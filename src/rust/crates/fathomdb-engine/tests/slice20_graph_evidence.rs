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

#[test]
fn empty_evidence_expansion_has_present_empty_sidecar() {
    let (_directory, engine, mut request) = fixture();
    request.target_kinds = vec!["absent-kind".into()];
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    assert!(result.targets.is_empty());
    assert!(result.evidence.unwrap().entries.is_empty());
}

#[test]
fn opaque_graph_references_survive_clean_restart() {
    let (directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    let reference = result.evidence.unwrap().entries[0].target_evidence_ref.clone();
    let frozen = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    engine.close().unwrap();
    let reopened = Engine::open(directory.path().join("graph-evidence.fdb")).unwrap().engine;
    let resolved = reopened
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: reference,
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id.as_str(), "target-r1");
}

#[test]
fn graph_preflights_remain_two_indexed_class_specific_statements() {
    let (_directory, engine, _request) = fixture();
    let plans = engine.explain_graph_evidence_preflights_for_test().unwrap();
    assert_eq!(plans.len(), 2);
    assert!(plans[0].contains("canonical_nodes_write_cursor_idx"));
    assert!(plans[1].contains("canonical_edges_write_cursor_idx"));
    assert!(plans.iter().all(|plan| plan.contains("USING INDEX")));
}

#[test]
fn canonical_source_graph_target_resolves_without_dependency() {
    let directory = TempDir::new().unwrap();
    let opened = Engine::open(directory.path().join("canonical-target.fdb")).unwrap();
    let source = "canonical graph target";
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
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge".into()),
                kind: "cites".into(),
                from: "root".into(),
                to: "source".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived("edge-r1"),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let result = opened
        .engine
        .graph_expand(&GraphExpandRequestV1 {
            schema_version: 1,
            seed: GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            direction: TraversalDirection::Outgoing,
            edge_kinds: vec!["cites".into()],
            target_kinds: vec!["document".into()],
            context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
            max_depth: 1,
            result_limit: 1,
            max_work_units: 10,
            include_explanation: false,
            include_evidence: true,
        })
        .unwrap();
    let target = opened
        .engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence.unwrap().entries[0].target_evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(target.artifact_revision_id.as_str(), "source-r1");
    assert!(target.dependency.is_none());
}
