//! Transient Slice 15 RED oracles for the graph-evidence decision spike.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    encode_graph_expand_result_v1, ArtifactRevisionId, CanonicalHash, Engine,
    GraphArtifactClassForTest, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceId, SourceLocator, SourceRevisionId, SourceVersionId, TraversalDirection,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("slice15{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source = "canonical source bytes";
    let provenance = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "document".into(),
                body: source.into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("source-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: "root".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target".into()),
                kind: "claim".into(),
                body: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: provenance("target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("z-later".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.7),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("edge-later-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("a-winner".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: provenance("edge-winner-r1"),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        filter,
    )
    .unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["claim".into()],
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        max_depth: 1,
        result_limit: 50,
        max_work_units: 10_000,
        include_explanation: false,
    };
    (directory, opened.engine, request)
}

#[test]
fn treatment_off_preserves_literal_bytes_and_winning_parallel_edge() {
    let (_directory, engine, request) = fixture();
    let ordinary = engine.graph_expand(&request).unwrap();
    let bytes = encode_graph_expand_result_v1(&ordinary).unwrap();
    assert_eq!(bytes, br#"{"schemaVersion":1,"seeds":[{"schemaVersion":1,"logicalId":"root","seedOrdinal":0,"queryScore":null}],"targets":[{"schemaVersion":1,"logicalId":"target","kind":"claim","body":"target","writeCursor":"3","origin":{"schemaVersion":1,"seedLogicalId":"root","seedOrdinal":0,"predecessorLogicalId":"root","targetLogicalId":"target","hopCount":1,"terminalEdgeKind":"supports","terminalDirection":"outgoing"}}],"complete":true,"workUnits":"2","degradationCodes":[],"explanation":null}"#);
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    assert_eq!(treated.graph, ordinary);
    assert_eq!(treated.evidence.len(), 1);
    assert_eq!(treated.evidence[0].target_revision_id, "target-r1");
    assert_eq!(treated.evidence[0].terminal_edge_revision_id.as_deref(), Some("edge-winner-r1"));
}

#[test]
fn authenticated_target_and_edge_refs_resolve_intrinsic_evidence() {
    let (_directory, engine, request) = fixture();
    let treated = engine.graph_expand_with_graph_evidence_for_test(&request).unwrap();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        _ => unreachable!(),
    };
    let target =
        engine.resolve_graph_evidence_for_test(&treated.evidence[0].target_ref, frozen).unwrap();
    let edge = engine
        .resolve_graph_evidence_for_test(
            treated.evidence[0].terminal_edge_ref.as_ref().unwrap(),
            frozen,
        )
        .unwrap();
    assert_eq!(target.artifact_class, GraphArtifactClassForTest::Node);
    assert_eq!(target.artifact_revision_id, "target-r1");
    assert_eq!(edge.artifact_class, GraphArtifactClassForTest::Edge);
    assert_eq!(edge.artifact_revision_id, "edge-winner-r1");
    assert_eq!(target.canonical_source_body, "canonical source bytes");
    assert_eq!(edge.canonical_source_body, "canonical source bytes");
}

#[test]
fn current_context_and_incomplete_provenance_refuse_atomically() {
    let (_directory, engine, mut request) = fixture();
    let context = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context.clone(),
        _ => unreachable!(),
    };
    request.context = GraphReadContextV1::Current { schema_version: 1, context };
    assert!(engine.graph_expand_with_graph_evidence_for_test(&request).is_err());
}
