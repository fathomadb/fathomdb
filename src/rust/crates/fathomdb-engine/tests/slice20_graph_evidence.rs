//! Slice 20 contract tests for exact frozen graph evidence.

use fathomdb_engine::{
    encode_graph_expand_result_v1, ArtifactRevisionId, CanonicalHash, Engine, EngineError,
    EvidenceErrorReasonV1, GraphEvidenceArtifactV1, GraphEvidenceRefV1,
    GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView,
    SearchFilter, SourceId, SourceLocator, SourceRevisionId, SourceVersionId, TraversalDirection,
    WriteProvenanceV1,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("graph-evidence.fdb");
    let opened = Engine::open(&path).unwrap();
    let source = "canonical source bytes";
    let derived = |revision: &str| {
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
                source_id: SourceId::new("owner").unwrap(),
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
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target".into()),
                kind: "claim".into(),
                body: "target".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("winner".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived("edge-r1"),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
                filter,
            )
            .unwrap(),
        )
        .unwrap();
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
        include_evidence: false,
    };
    (directory, opened.engine, request)
}

fn unavailable(error: EngineError) {
    assert!(matches!(error, EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && error.field_path == "/evidenceRef"));
}

#[test]
fn opt_in_sidecar_resolves_exact_target_and_winning_edge() {
    let (_directory, engine, mut request) = fixture();
    let ordinary = engine.graph_expand(&request).unwrap();
    assert!(ordinary.evidence.is_none());
    let ordinary_bytes = encode_graph_expand_result_v1(&ordinary).unwrap();
    request.include_evidence = true;
    let expanded = engine.graph_expand(&request).unwrap();
    let sidecar = expanded.evidence.as_ref().unwrap();
    assert_eq!(sidecar.entries.len(), expanded.targets.len());
    assert_eq!(sidecar.entries[0].target_index, 0);
    assert_eq!(sidecar.entries[0].target_artifact_revision_id.as_str(), "target-r1");
    assert_eq!(sidecar.entries[0].terminal_edge_artifact_revision_id.as_str(), "edge-r1");
    assert!(sidecar.entries[0].target_evidence_ref.as_str().starts_with("fdbgev1."));
    assert!(!sidecar.entries[0].target_evidence_ref.as_str().contains("target-r1"));
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let target = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: sidecar.entries[0].target_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(target.canonical_source_body, "canonical source bytes");
    assert_eq!(target.evidence_text, "canonical source bytes");
    assert!(target.dependency.is_none());
    assert!(matches!(target.artifact, GraphEvidenceArtifactV1::Node {
        ref logical_id, ref body, ..
    } if logical_id == "target" && body == "target"));
    let edge = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: sidecar.entries[0].terminal_edge_evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert!(matches!(edge.artifact, GraphEvidenceArtifactV1::Edge {
        ref from, ref to, ..
    } if from == "root" && to == "target"));
    request.include_evidence = false;
    assert_eq!(
        encode_graph_expand_result_v1(&engine.graph_expand(&request).unwrap()).unwrap(),
        ordinary_bytes
    );
}

#[test]
fn graph_references_are_frozen_context_bound_and_tamper_safe() {
    let (_directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let expanded = engine.graph_expand(&request).unwrap();
    let reference = expanded.evidence.unwrap().entries[0].target_evidence_ref.clone();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let mut tampered = reference.as_str().as_bytes().to_vec();
    let last = tampered.len() - 1;
    tampered[last] = if tampered[last] == b'0' { b'1' } else { b'0' };
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: GraphEvidenceRefV1::new(String::from_utf8(tampered).unwrap())
                    .unwrap(),
                context: frozen,
            })
            .unwrap_err(),
    );
    request.context = GraphReadContextV1::Current {
        schema_version: 1,
        context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    };
    assert!(
        matches!(engine.graph_expand(&request).unwrap_err(), EngineError::GraphExpansion(ref error)
        if error.field_path == "/context")
    );
}
