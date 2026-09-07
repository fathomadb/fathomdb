//! Slice 60 FIX-5 RED: persisted provenance mismatches fail closed.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, DependencyTraceDirectionV1, DependencyTraceRequestV1,
    Engine, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState,
    PreparedWrite, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    StructuralDependencyStateV1, TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[derive(Deserialize)]
struct Fixture {
    faults: Vec<Fault>,
}

#[derive(Deserialize)]
struct Fault {
    name: String,
    sql: String,
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn graph_request() -> GraphExpandRequestV1 {
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
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        max_depth: 1,
        result_limit: 10,
        max_work_units: 10,
        include_explanation: true,
    }
}

fn seeded() -> (TempDir, Engine) {
    let directory = TempDir::new().unwrap();
    let opened = Engine::open(directory.path().join(format!("fix5{SQLITE_SUFFIX}"))).unwrap();
    opened
        .engine
        .write(&[
            PreparedWrite::Node {
                logical_id: Some("root".into()),
                kind: "fact".into(),
                body: "root".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "doc".into(),
                body: "canonical source".into(),
                source_id: SourceId::new("owner").unwrap(),
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
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest("canonical source")).unwrap(),
                ),
            }),
            PreparedWrite::Edge {
                logical_id: Some("root-derived".into()),
                kind: "link".into(),
                from: "root".into(),
                to: "derived".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    (directory, opened.engine)
}

fn trace_edges(engine: &Engine) -> usize {
    let context = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    engine
        .trace_dependency(
            DependencyTraceRequestV1::new(
                "source-r1",
                DependencyTraceDirectionV1::ToDependents,
                context,
            )
            .unwrap(),
        )
        .unwrap()
        .dependency_edges
        .len()
}

#[test]
fn persisted_source_id_and_link_mismatches_fail_closed_for_classification_and_trace() {
    let fixture: Fixture = serde_json::from_str(include_str!(
        "../../../../../dev/fixtures/slice60-fix5-provenance-v1.json"
    ))
    .unwrap();
    for fault in fixture.faults {
        let (_directory, engine) = seeded();
        engine.execute_for_test(&fault.sql).unwrap();
        let dependency_state =
            engine.graph_expand(&graph_request()).unwrap().explanation.unwrap().per_target[0]
                .dependency_state;
        assert_eq!(
            dependency_state,
            StructuralDependencyStateV1::NotRegistered,
            "{} must not classify a corrupted relation as registered",
            fault.name
        );
        assert_eq!(trace_edges(&engine), 0, "{} must not enter dependency trace", fault.name);
    }
}
