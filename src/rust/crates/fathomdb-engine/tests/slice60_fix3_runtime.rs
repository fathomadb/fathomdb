//! Slice 60 FIX-3 RED: graph expansion observes real lifecycle and RSS state.

#![cfg(all(feature = "test-hooks", feature = "operator"))]

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, IdSpace, InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn request() -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: Vec::new(),
        target_kinds: Vec::new(),
        context: GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        },
        max_depth: 1,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: true,
    }
}

fn hash(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn open_graph(directory: &TempDir, name: &str, registered: bool) -> Engine {
    let path = directory.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source_body = "source body";
    opened
        .engine
        .write(&[
            PreparedWrite::Node {
                logical_id: Some("root".into()),
                kind: "fact".into(),
                body: "root".into(),
                source_id: SourceId::new("control").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "doc".into(),
                body: source_body.into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("derived".into()),
                kind: "fact".into(),
                body: "derived".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(hash(source_body)).unwrap(),
                ),
            }),
            PreparedWrite::Edge {
                logical_id: Some("root-derived".into()),
                kind: "link".into(),
                from: "root".into(),
                to: "derived".into(),
                source_id: SourceId::new("control").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    if registered {
        opened
            .engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("dependency", "source-r1", "derived-r1")
                    .unwrap(),
            )
            .unwrap();
    }
    opened.engine
}

fn target_ids(engine: &Engine) -> Vec<String> {
    engine
        .graph_expand(&request())
        .unwrap()
        .targets
        .into_iter()
        .map(|target| target.logical_id)
        .collect()
}

#[test]
fn graph_expand_executes_registered_unregistered_and_closure_fenced_dependency_states() {
    let directory = TempDir::new().unwrap();
    let registered = open_graph(&directory, "registered", true);
    assert_eq!(target_ids(&registered), ["derived"]);
    registered.erase_source("source-owner").unwrap();
    assert!(
        target_ids(&registered).is_empty(),
        "registered closure must fence derived graph target"
    );

    let unregistered = open_graph(&directory, "unregistered", false);
    unregistered.erase_source("source-owner").unwrap();
    assert!(
        target_ids(&unregistered).is_empty(),
        "unregistered source-owned target disappears through the same real erasure path"
    );
}

#[test]
fn graph_expand_executes_erase_and_excise_disappearance() {
    let directory = TempDir::new().unwrap();
    let erased = open_graph(&directory, "erase", false);
    erased.erase_source("source-owner").unwrap();
    assert!(target_ids(&erased).is_empty());

    let excised = open_graph(&directory, "excise", false);
    excised.excise_source("source-owner").unwrap();
    assert!(target_ids(&excised).is_empty());
}

#[test]
fn measured_expansion_rss_is_an_immediate_proportional_delta() {
    let directory = TempDir::new().unwrap();
    let small = open_graph(&directory, "small", false);
    let _ = small.graph_expand(&request()).unwrap();
    let small_rss = small.measure_graph_expand_for_test().peak_rss_delta_bytes;

    let large = open_graph(&directory, "large", false);
    let mut writes = Vec::new();
    for index in 0..10_000 {
        writes.push(PreparedWrite::Node {
            logical_id: Some(format!("extra-{index}")),
            kind: "fact".into(),
            body: "extra".into(),
            source_id: SourceId::new("bulk").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        });
    }
    large.write(&writes).unwrap();
    let _ = large.graph_expand(&request()).unwrap();
    let large_rss = large.measure_graph_expand_for_test().peak_rss_delta_bytes;
    assert!(
        large_rss <= small_rss.saturating_add(4 * 1024 * 1024),
        "small={small_rss} large={large_rss}"
    );
    assert!(
        large_rss < 2 * 1024 * 1024,
        "the immediate graph-expand delta, not process-history peak, must stay bounded: {large_rss}"
    );
}
