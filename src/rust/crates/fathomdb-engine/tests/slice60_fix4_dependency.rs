//! Slice 60 FIX-4 RED: graph expansion observes real cross-owner closure barriers.

#![cfg(all(feature = "test-hooks", feature = "operator"))]

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, IdSpace, InitialState, PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, StructuralDependencyStateV1, TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
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
        result_limit: 10,
        max_work_units: 10,
        include_explanation: true,
    }
}

fn digest(value: &str) -> String {
    Sha256::digest(value.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture(directory: &TempDir, name: &str) -> (std::path::PathBuf, Engine) {
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
                source_id: SourceId::new("control-owner").unwrap(),
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
                source_id: SourceId::new("derived-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest(source_body)).unwrap(),
                ),
            }),
            PreparedWrite::Edge {
                logical_id: Some("root-derived".into()),
                kind: "link".into(),
                from: "root".into(),
                to: "derived".into(),
                source_id: SourceId::new("control-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    (path, opened.engine)
}

fn dependency_state(engine: &Engine) -> StructuralDependencyStateV1 {
    engine.graph_expand(&request()).unwrap().explanation.unwrap().per_target[0].dependency_state
}

#[test]
fn cross_owner_registered_closure_barrier_excludes_a_surviving_derived_target() {
    let directory = TempDir::new().unwrap();
    let (path, engine) = fixture(&directory, "closure-barrier");
    assert_eq!(dependency_state(&engine), StructuralDependencyStateV1::NotRegistered);

    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dependency", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    assert_eq!(dependency_state(&engine), StructuralDependencyStateV1::Registered);

    engine.erase_source("source-owner").unwrap();
    let surviving: i64 = Connection::open(path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='derived'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(
        surviving, 1,
        "the independently owned derived row must survive the closure barrier"
    );
    assert!(engine.graph_expand(&request()).unwrap().targets.is_empty());
}

#[test]
fn erase_and_excise_remain_separate_cross_owner_disappearance_controls() {
    let directory = TempDir::new().unwrap();
    let (_, erased) = fixture(&directory, "erase");
    erased.erase_source("derived-owner").unwrap();
    assert!(erased.graph_expand(&request()).unwrap().targets.is_empty());

    let (_, excised) = fixture(&directory, "excise");
    excised.excise_source("derived-owner").unwrap();
    assert!(excised.graph_expand(&request()).unwrap().targets.is_empty());
}
