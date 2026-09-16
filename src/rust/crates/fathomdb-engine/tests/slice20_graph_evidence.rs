//! Slice 20 contract tests for exact frozen graph evidence.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    decode_graph_expand_result_v1, encode_graph_expand_result_v1, ArtifactRevisionId,
    CanonicalHash, ConsolidateAxis, Engine, EngineError, EvidenceErrorReasonV1, EvidenceRefV1,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, FrozenReadErrorReason,
    GraphEvidenceArtifactV1, GraphEvidenceRefV1, GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1, GraphExpansionErrorReasonV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, LifecycleState, PreparedWrite, ProjectionRole, ProjectionSpec, ProvenancedEdgeV1,
    ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter, SourceDependencyRegistrationV1,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, TraversalDirection,
    WriteProvenanceV1,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tempfile::TempDir;

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn fixture_with_pre_freeze_sql(sql: Option<&str>) -> (TempDir, Engine, GraphExpandRequestV1) {
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
    let engine = match sql {
        Some(sql) => reopen_after_sql(&directory, opened.engine, sql),
        None => opened.engine,
    };
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let frozen = engine
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
    (directory, engine, request)
}

fn fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    fixture_with_pre_freeze_sql(None)
}

fn fixture_with_two_targets() -> (TempDir, Engine, GraphExpandRequestV1) {
    let (directory, engine, request) = fixture();
    let source = "second canonical source bytes";
    let derived = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("source-v2").unwrap(),
            SourceRevisionId::new("source-r2").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source-two".into()),
                kind: "document".into(),
                body: source.into(),
                source_id: SourceId::new("owner-two").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r2").unwrap(),
                    SourceVersionId::new("source-v2").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target-two".into()),
                kind: "claim".into(),
                body: "target two".into(),
                source_id: SourceId::new("owner-two").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("target-two-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("winner-two".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target-two".into(),
                source_id: SourceId::new("owner-two").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived("edge-two-r1"),
            }),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();
    let context = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    (directory, engine, request)
}

#[test]
fn evidence_hydration_executes_zero_or_exactly_two_sql_statements() {
    let (_directory, engine, mut one) = fixture();
    one.include_evidence = true;
    let (without_one, baseline_one) = engine
        .graph_expand_with_statement_count_for_test(&GraphExpandRequestV1 {
            include_evidence: false,
            ..one.clone()
        })
        .unwrap();
    assert_eq!(without_one.targets.len(), 1);
    let (with_one, evidence_one) = engine.graph_expand_with_statement_count_for_test(&one).unwrap();
    assert_eq!(with_one.targets.len(), 1);
    assert_eq!(evidence_one - baseline_one, 2);

    let mut empty = one;
    empty.target_kinds = vec!["no-such-kind".into()];
    let (without_empty, baseline_empty) = engine
        .graph_expand_with_statement_count_for_test(&GraphExpandRequestV1 {
            include_evidence: false,
            ..empty.clone()
        })
        .unwrap();
    assert!(without_empty.targets.is_empty());
    let (with_empty, evidence_empty) =
        engine.graph_expand_with_statement_count_for_test(&empty).unwrap();
    assert!(with_empty.targets.is_empty());
    assert_eq!(evidence_empty - baseline_empty, 0);

    let (_directory, engine, mut multiple) = fixture_with_two_targets();
    multiple.include_evidence = true;
    let (without_multiple, baseline_multiple) = engine
        .graph_expand_with_statement_count_for_test(&GraphExpandRequestV1 {
            include_evidence: false,
            ..multiple.clone()
        })
        .unwrap();
    assert_eq!(without_multiple.targets.len(), 2);
    let (with_multiple, evidence_multiple) =
        engine.graph_expand_with_statement_count_for_test(&multiple).unwrap();
    assert_eq!(with_multiple.targets.len(), 2);
    assert_eq!(evidence_multiple - baseline_multiple, 2);
}

#[test]
fn graph_evidence_response_sidecar_and_entries_are_closed() {
    let (_directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    let mut value: serde_json::Value =
        serde_json::from_slice(&encode_graph_expand_result_v1(&result).unwrap()).unwrap();
    value["evidence"]["z/future~field"] = serde_json::json!(true);
    let error = decode_graph_expand_result_v1(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
    assert_eq!(error.field_path, "/evidence/z~1future~0field");

    let mut value: serde_json::Value =
        serde_json::from_slice(&encode_graph_expand_result_v1(&result).unwrap()).unwrap();
    value["evidence"]["entries"][0]["a/future~field"] = serde_json::json!(true);
    let error = decode_graph_expand_result_v1(&serde_json::to_vec(&value).unwrap()).unwrap_err();
    assert_eq!(error.reason, GraphExpansionErrorReasonV1::GraphCorrupt);
    assert_eq!(error.field_path, "/evidence/entries/0/a~1future~0field");
}

fn temporal_fixture(
    effective_valid_at: i64,
    source_window: (Option<i64>, Option<i64>),
    target_window: (Option<i64>, Option<i64>),
    edge_window: (Option<i64>, Option<i64>),
) -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let opened = Engine::open(directory.path().join("temporal-graph-evidence.fdb")).unwrap();
    let source = "temporal canonical bytes";
    let derived = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("temporal-v1").unwrap(),
            SourceRevisionId::new("temporal-source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source)).unwrap(),
        )
    };
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("temporal-source".into()),
                kind: "document".into(),
                body: source.into(),
                source_id: SourceId::new("temporal-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: source_window.0,
                valid_until: source_window.1,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("temporal-source-r1").unwrap(),
                    SourceVersionId::new("temporal-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("temporal-root".into()),
                kind: "claim".into(),
                body: "root".into(),
                source_id: SourceId::new("temporal-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("temporal-root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("temporal-target".into()),
                kind: "claim".into(),
                body: "target".into(),
                source_id: SourceId::new("temporal-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: target_window.0,
                valid_until: target_window.1,
                provenance: derived("temporal-target-r1"),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("temporal-edge".into()),
                kind: "supports".into(),
                from: "temporal-root".into(),
                to: "temporal-target".into(),
                source_id: SourceId::new("temporal-owner").unwrap(),
                body: None,
                t_valid: edge_window.0,
                t_invalid: edge_window.1,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived("temporal-edge-r1"),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(effective_valid_at), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("temporal-root")],
        },
        direction: TraversalDirection::Outgoing,
        edge_kinds: vec!["supports".into()],
        target_kinds: vec!["claim".into()],
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        max_depth: 1,
        result_limit: 1,
        max_work_units: 10,
        include_explanation: false,
        include_evidence: true,
    };
    (directory, opened.engine, request)
}

fn unavailable(error: EngineError) {
    assert!(matches!(error, EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && error.field_path == "/evidenceRef"));
}

fn expansion_unavailable(error: EngineError) {
    assert!(
        matches!(&error, EngineError::Evidence(error)
        if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && error.field_path == "/evidence"),
        "{error:?}"
    );
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
    assert_eq!(sidecar.entries[0].target_evidence_ref.as_str().len(), 704);
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
fn anonymous_winning_terminal_edge_resolves_with_no_logical_id() {
    let (_directory, engine, mut request) = fixture();
    engine
        .write(&[PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            logical_id: None,
            kind: "supports".into(),
            from: "root".into(),
            to: "target".into(),
            source_id: SourceId::new("owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.8),
            extractor_model_id: None,
            temporal_fallback: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("anonymous-edge-r1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest("canonical source bytes")).unwrap(),
            ),
        })])
        .unwrap();
    engine.drain(30_000).unwrap();
    let GraphReadContextV1::Frozen { context, .. } = &request.context else { unreachable!() };
    let frozen = engine.freeze_read_context(&context.context).unwrap();
    request.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() };
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    assert_eq!(result.targets.len(), 1);
    let sidecar = result.evidence.unwrap();
    assert_eq!(sidecar.entries.len(), 1);
    let entry = &sidecar.entries[0];
    assert_eq!(entry.terminal_edge_artifact_revision_id.as_str(), "anonymous-edge-r1");
    let resolved = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.terminal_edge_evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, entry.terminal_edge_artifact_revision_id);
    assert_eq!(resolved.canonical_source_body, "canonical source bytes");
    assert!(matches!(resolved.artifact, GraphEvidenceArtifactV1::Edge {
        logical_id: None, ref from, ref to, ..
    } if from == "root" && to == "target"));
}

#[test]
fn evidence_pins_the_traversal_winner_among_parallel_edges() {
    let (_directory, engine, mut request) = fixture();
    let source = "canonical source bytes";
    engine
        .write(&[PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            logical_id: Some("aaa-first-by-order".into()),
            kind: "supports".into(),
            from: "root".into(),
            to: "target".into(),
            source_id: SourceId::new("owner").unwrap(),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.8),
            extractor_model_id: None,
            temporal_fallback: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("edge-parallel-r1").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest(source)).unwrap(),
            ),
        })])
        .unwrap();
    engine.drain(30_000).unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
                filter,
            )
            .unwrap(),
        )
        .unwrap();
    request.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen };
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    let entry = &result.evidence.unwrap().entries[0];
    assert_eq!(entry.terminal_edge_artifact_revision_id.as_str(), "edge-parallel-r1");
    assert_eq!(result.targets[0].origin.terminal_edge_kind, "supports");
}

fn reopen_after_sql(directory: &TempDir, engine: Engine, sql: &str) -> Engine {
    engine.close().unwrap();
    let path = directory.path().join("graph-evidence.fdb");
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch(sql).unwrap();
    drop(connection);
    Engine::open(path).unwrap().engine
}

#[test]
fn post_freeze_denied_source_and_corrupt_locator_is_state_drift() {
    let (directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let engine = reopen_after_sql(
        &directory,
        engine,
        "UPDATE canonical_nodes SET state='deleted' WHERE logical_id='source';
         UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 \
           WHERE artifact_revision_id='target-r1';",
    );
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(matches!(error, EngineError::FrozenRead(ref error)
        if error.reason == FrozenReadErrorReason::StateDrifted
            && error.field_path == "/token"));
}

#[test]
fn denied_source_precedes_visible_corrupt_locator_globally() {
    let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(
        "UPDATE canonical_nodes SET state='deleted' WHERE logical_id='source';
         UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 \
           WHERE artifact_revision_id='target-r1';",
    ));
    request.include_evidence = true;
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(matches!(error, EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && error.field_path == "/evidence"));
}

#[test]
fn post_freeze_missing_source_storage_is_state_drift() {
    let (directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let engine = reopen_after_sql(
        &directory,
        engine,
        "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='target-r1';",
    );
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(matches!(error, EngineError::FrozenRead(ref error)
        if error.reason == FrozenReadErrorReason::StateDrifted
            && error.field_path == "/token"));

    let (directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let engine = reopen_after_sql(
        &directory,
        engine,
        "DELETE FROM canonical_nodes WHERE logical_id='source';",
    );
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(matches!(error, EngineError::FrozenRead(ref error)
        if error.reason == FrozenReadErrorReason::StateDrifted
            && error.field_path == "/token"));
}

#[test]
fn missing_link_is_incomplete_but_linked_missing_source_is_unavailable() {
    let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(
        "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='target-r1';",
    ));
    request.include_evidence = true;
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(matches!(error, EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceIncomplete
            && error.field_path == "/targets/0/provenance"));

    let (_directory, engine, mut request) =
        fixture_with_pre_freeze_sql(Some("DELETE FROM canonical_nodes WHERE logical_id='source';"));
    request.include_evidence = true;
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(matches!(error, EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && error.field_path == "/evidence"));
}

#[test]
fn phase_two_faults_use_exact_target_then_terminal_edge_paths() {
    let cases = [
        (
            "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 WHERE artifact_revision_id='target-r1';",
            "/targets/0/provenance/sourceLocator",
        ),
        (
            "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 WHERE artifact_revision_id='edge-r1';",
            "/targets/0/terminalEdge/provenance/sourceLocator",
        ),
        (
            "UPDATE _fathomdb_source_links SET hash_digest='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' WHERE artifact_revision_id='target-r1';",
            "/targets/0/provenance/canonicalSourceHash",
        ),
        (
            "UPDATE _fathomdb_source_links SET hash_digest='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' WHERE artifact_revision_id='edge-r1';",
            "/targets/0/terminalEdge/provenance/canonicalSourceHash",
        ),
        (
            "DELETE FROM _fathomdb_source_versions WHERE source_revision_id='source-r1';",
            "/targets/0/provenance",
        ),
        (
            "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='source-r1';",
            "/targets/0/provenance",
        ),
    ];
    for (sql, expected_path) in cases {
        let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(sql));
        request.include_evidence = true;
        let error = engine.graph_expand(&request).unwrap_err();
        assert!(
            matches!(error, EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                && error.field_path == expected_path),
            "unexpected error for {sql}: {error:?}"
        );
    }

    let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(
        "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 WHERE artifact_revision_id IN ('target-r1','edge-r1');",
    ));
    request.include_evidence = true;
    assert!(matches!(engine.graph_expand(&request).unwrap_err(), EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
            && error.field_path == "/targets/0/provenance/sourceLocator"));
}

#[test]
fn canonical_source_registry_authority_precedes_malformed_provenance_detail() {
    for registry_fault in [
        "artifact_role='derived_semantic'",
        "artifact_class='edge'",
        "completeness='migrated_incomplete'",
    ] {
        let sql = format!(
            "UPDATE _fathomdb_artifact_revisions SET {registry_fault} \
             WHERE revision_id='source-r1'; \
             UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes', \
               start_byte=0,end_byte=999999, \
               hash_digest='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa' \
             WHERE artifact_revision_id='target-r1';"
        );
        let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(&sql));
        request.include_evidence = true;
        expansion_unavailable(engine.graph_expand(&request).unwrap_err());
    }
}

#[test]
fn missing_links_are_globally_ordered_and_use_exact_terminal_edge_path() {
    let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(
        "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='edge-r1';",
    ));
    request.include_evidence = true;
    let error = engine.graph_expand(&request).unwrap_err();
    assert!(
        matches!(&error, EngineError::Evidence(error)
        if error.reason == EvidenceErrorReasonV1::EvidenceIncomplete
            && error.field_path == "/targets/0/terminalEdge/provenance"),
        "{error:?}"
    );

    let (_directory, engine, mut request) = fixture_with_pre_freeze_sql(Some(
        "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id IN ('target-r1','edge-r1');",
    ));
    request.include_evidence = true;
    assert!(matches!(engine.graph_expand(&request).unwrap_err(), EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceIncomplete
            && error.field_path == "/targets/0/provenance"));

    let (directory, engine, request) = fixture_with_two_targets();
    let context = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let engine = reopen_after_sql(
        &directory,
        engine,
        "DELETE FROM _fathomdb_source_links \
         WHERE artifact_revision_id IN ('edge-r1','target-two-r1');",
    );
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        include_evidence: true,
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    assert!(matches!(engine.graph_expand(&request).unwrap_err(), EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceIncomplete
            && error.field_path == "/targets/0/terminalEdge/provenance"));
}

#[test]
fn lower_target_index_precedes_later_target_even_when_the_lower_fault_is_an_edge() {
    let (directory, engine, request) = fixture_with_two_targets();
    let context = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let engine = reopen_after_sql(
        &directory,
        engine,
        "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 WHERE artifact_revision_id IN ('edge-r1','target-two-r1');",
    );
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        include_evidence: true,
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    assert!(matches!(engine.graph_expand(&request).unwrap_err(), EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
            && error.field_path == "/targets/0/terminalEdge/provenance/sourceLocator"));
}

#[test]
fn source_attribute_eligibility_distinguishes_absent_from_present_empty() {
    let (_directory, engine, request) = fixture();
    let mut roles = BTreeSet::new();
    roles.insert(ProjectionRole::Filterable);
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "tenant".into(),
                roles,
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine
        .execute_for_test(
            "INSERT INTO canonical_attributes(write_cursor,attr_name,attr_value) \
             SELECT write_cursor,'tenant','' FROM canonical_nodes \
             WHERE logical_id IN ('root','target');",
        )
        .unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    filter.attributes = vec![("tenant".into(), "".into())];
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        filter,
    )
    .unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        include_evidence: true,
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    expansion_unavailable(engine.graph_expand(&request).unwrap_err());

    engine
        .execute_for_test(
            "INSERT INTO canonical_attributes(write_cursor,attr_name,attr_value) \
             SELECT write_cursor,'tenant','' FROM canonical_nodes WHERE logical_id='source';",
        )
        .unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    assert_eq!(engine.graph_expand(&request).unwrap().targets.len(), 1);
}

#[test]
fn source_status_and_created_after_require_vector_metadata_only_when_filtered() {
    let (_directory, engine, request) = fixture();
    engine
        .execute_for_test(
            "DROP TABLE vector_default; \
             CREATE TABLE vector_default( \
               rowid INTEGER PRIMARY KEY, embedding BLOB, embedding_bin BLOB, \
               source_type TEXT, kind TEXT, created_at INTEGER, status TEXT \
             ); \
             INSERT INTO vector_default(rowid,source_type,kind,created_at,status) \
             SELECT write_cursor,'node_body',kind,20,'ready' FROM canonical_nodes \
             WHERE logical_id IN ('root','target');",
        )
        .unwrap();
    let mut filter = SearchFilter::default();
    filter.kind = Some("claim".into());
    filter.created_after = Some(10);
    filter.status = Some("ready".into());
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        filter,
    )
    .unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        include_evidence: true,
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    expansion_unavailable(engine.graph_expand(&request).unwrap_err());

    engine
        .execute_for_test(
            "INSERT INTO vector_default(rowid,source_type,kind,created_at,status) \
             SELECT write_cursor,'node_body',kind,20,'ready' FROM canonical_nodes \
             WHERE logical_id='source';",
        )
        .unwrap();
    let frozen = engine.freeze_read_context(&context).unwrap();
    let request = GraphExpandRequestV1 {
        context: GraphReadContextV1::Frozen { schema_version: 1, context: frozen },
        ..request
    };
    assert_eq!(engine.graph_expand(&request).unwrap().targets.len(), 1);
}

fn resolver_linearizes_before_erasure(use_operator_spelling: bool) {
    let (_directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    let reference = result.evidence.unwrap().entries[0].target_evidence_ref.clone();
    let context = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let engine = Arc::new(engine);
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    engine.arm_graph_evidence_before_resolve_return_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let resolver = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || {
            engine.resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context,
            })
        })
    };
    ready.wait();

    let (lock_tx, lock_rx) = mpsc::channel();
    engine.arm_erasure_before_primary_lock_hook_for_test(Box::new(move || {
        lock_tx.send(()).unwrap();
    }));
    let (done_tx, done_rx) = mpsc::channel();
    let eraser = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || {
            let result = if use_operator_spelling {
                #[cfg(feature = "operator")]
                {
                    engine.excise_source("owner")
                }
                #[cfg(not(feature = "operator"))]
                {
                    unreachable!()
                }
            } else {
                engine.erase_source("owner")
            };
            done_tx.send(()).unwrap();
            result
        })
    };
    lock_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(matches!(done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    release.wait();
    assert_eq!(resolver.join().unwrap().unwrap().artifact_revision_id.as_str(), "target-r1");
    eraser.join().unwrap().unwrap();
}

#[test]
fn graph_resolver_and_erasure_spellings_linearize_under_the_primary_mutex() {
    resolver_linearizes_before_erasure(false);
    #[cfg(feature = "operator")]
    resolver_linearizes_before_erasure(true);
}

fn erasure_linearizes_before_resolver(use_operator_spelling: bool) {
    let (_directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let result = engine.graph_expand(&request).unwrap();
    let reference = result.evidence.unwrap().entries[0].target_evidence_ref.clone();
    let context = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    if use_operator_spelling {
        #[cfg(feature = "operator")]
        engine.excise_source("owner").unwrap();
        #[cfg(not(feature = "operator"))]
        unreachable!();
    } else {
        engine.erase_source("owner").unwrap();
    }
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context,
            })
            .unwrap_err(),
    );
}

#[test]
fn erase_and_excise_that_linearize_first_make_resolution_unavailable() {
    erasure_linearizes_before_resolver(false);
    #[cfg(feature = "operator")]
    erasure_linearizes_before_resolver(true);
}

#[test]
fn graph_resolver_rendezvous_is_engine_scoped() {
    let (_first_directory, first, mut first_request) = fixture();
    first_request.include_evidence = true;
    let first_reference = first.graph_expand(&first_request).unwrap().evidence.unwrap().entries[0]
        .target_evidence_ref
        .clone();
    let first_context = match first_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let (_second_directory, second, mut second_request) = fixture();
    second_request.include_evidence = true;
    let second_reference = second.graph_expand(&second_request).unwrap().evidence.unwrap().entries
        [0]
    .target_evidence_ref
    .clone();
    let second_context = match second_request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };

    let fired = Arc::new(AtomicUsize::new(0));
    let hook_fired = Arc::clone(&fired);
    first.arm_graph_evidence_before_resolve_return_hook_for_test(Box::new(move || {
        hook_fired.fetch_add(1, Ordering::SeqCst);
    }));
    second
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: second_reference,
            context: second_context,
        })
        .unwrap();
    assert_eq!(fired.load(Ordering::SeqCst), 0);
    first
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: first_reference,
            context: first_context,
        })
        .unwrap();
    assert_eq!(fired.load(Ordering::SeqCst), 1);
}

#[test]
fn erasure_rendezvous_is_engine_scoped() {
    let (_first_directory, first, _first_request) = fixture();
    let (_second_directory, second, _second_request) = fixture();
    let fired = Arc::new(AtomicUsize::new(0));
    let hook_fired = Arc::clone(&fired);
    first.arm_erasure_before_primary_lock_hook_for_test(Box::new(move || {
        hook_fired.fetch_add(1, Ordering::SeqCst);
    }));
    second.erase_source("owner").unwrap();
    assert_eq!(fired.load(Ordering::SeqCst), 0);
    first.erase_source("owner").unwrap();
    assert_eq!(fired.load(Ordering::SeqCst), 1);
}

#[test]
fn authenticated_relaxed_window_context_is_refused_before_mint() {
    let (_directory, engine, mut request) = fixture();
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView {
                    include_out_of_window: true,
                    valid_as_of: Some(1_800_000_000),
                    ..ReadView::default()
                },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    request.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen };
    request.include_evidence = true;
    assert!(
        matches!(engine.graph_expand(&request).unwrap_err(), EngineError::GraphExpansion(ref error)
        if error.reason == fathomdb_engine::GraphExpansionErrorReasonV1::GraphContextInvalid
            && error.field_path == "/context/context/view/includeOutOfWindow")
    );
}

#[test]
fn evidence_validity_is_start_inclusive_and_end_exclusive_for_all_three_artifacts() {
    let now =
        i64::try_from(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()).unwrap();
    let start = now - 10;
    let end = now + 100;
    let (_directory, engine, request) = temporal_fixture(
        start,
        (Some(start), Some(end)),
        (Some(start), Some(end)),
        (Some(start), Some(end)),
    );
    assert_eq!(engine.graph_expand(&request).unwrap().targets.len(), 1);

    let (_directory, engine, request) =
        temporal_fixture(end, (None, None), (None, Some(end)), (None, None));
    assert!(engine.graph_expand(&request).unwrap().targets.is_empty());

    let (_directory, engine, request) =
        temporal_fixture(end, (None, None), (None, None), (None, Some(end)));
    assert!(engine.graph_expand(&request).unwrap().targets.is_empty());

    let (_directory, engine, request) =
        temporal_fixture(end, (None, Some(end)), (None, None), (None, None));
    assert!(matches!(engine.graph_expand(&request).unwrap_err(), EngineError::Evidence(ref error)
        if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            && error.field_path == "/evidence"));
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
    let entry = result.evidence.unwrap().entries[0].clone();
    let frozen = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    engine.close().unwrap();
    let reopened = Engine::open(directory.path().join("graph-evidence.fdb")).unwrap().engine;
    let resolved = reopened
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.target_evidence_ref,
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id.as_str(), "target-r1");
    let edge = reopened
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.terminal_edge_evidence_ref,
            context: frozen,
        })
        .unwrap();
    assert_eq!(edge.artifact_revision_id.as_str(), "edge-r1");
}

#[test]
fn graph_evidence_carries_exact_registered_dependency_for_node_and_edge() {
    let (_directory, engine, mut request) = fixture();
    let target_dependency = engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("target-dep", "source-r1", "target-r1").unwrap(),
        )
        .unwrap();
    let edge_dependency = engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("edge-dep", "source-r1", "edge-r1").unwrap(),
        )
        .unwrap();
    assert_eq!(target_dependency.registered_dependency_generation, 1);
    assert_eq!(edge_dependency.registered_dependency_generation, 2);
    let context = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context.context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let frozen = engine.freeze_read_context(&context).unwrap();
    request.context = GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() };
    request.include_evidence = true;
    let entry = engine.graph_expand(&request).unwrap().evidence.unwrap().entries[0].clone();
    for (reference, expected) in [
        (entry.target_evidence_ref, target_dependency),
        (entry.terminal_edge_evidence_ref, edge_dependency),
    ] {
        let resolved = engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: frozen.clone(),
            })
            .unwrap();
        assert_eq!(resolved.source_revision_id.as_str(), "source-r1");
        assert_eq!(resolved.dependency, Some(expected));
    }
}

#[test]
fn graph_evidence_references_are_nondisclosing_across_authority_and_token_failures() {
    let (_directory, engine, mut request) = fixture();
    request.include_evidence = true;
    let expanded = engine.graph_expand(&request).unwrap();
    let graph_reference = expanded.evidence.unwrap().entries[0].target_evidence_ref.clone();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        GraphReadContextV1::Current { .. } => unreachable!(),
    };

    let (_foreign_directory, foreign, _) = fixture();
    unavailable(
        foreign
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: graph_reference.clone(),
                context: frozen.clone(),
            })
            .unwrap_err(),
    );

    let mut mismatch_filter = SearchFilter::default();
    mismatch_filter.kind = Some("document".into());
    let mismatched = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
                mismatch_filter,
            )
            .unwrap(),
        )
        .unwrap();
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: graph_reference.clone(),
                context: mismatched,
            })
            .unwrap_err(),
    );

    let token = graph_reference.as_str();
    for malformed in [
        token[..token.len() - 1].to_owned(),
        format!("{token}0"),
        format!("badgev1.{}", &token["fdbgev1.".len()..]),
    ] {
        unavailable(
            engine
                .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: GraphEvidenceRefV1::new(malformed).unwrap(),
                    context: frozen.clone(),
                })
                .unwrap_err(),
        );
    }

    let ranked = engine
        .search_with_evidence(&EvidenceSearchRequestV1 {
            schema_version: 1,
            query: "target".into(),
            context: frozen.clone(),
            rerank_depth: 0,
            use_graph_arm: false,
            alpha: 0.5,
            pool_n: 10,
            include_explanation: false,
            limit: 10,
        })
        .unwrap();
    let ranked_reference = ranked.evidence[0].evidence_ref.clone();
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: GraphEvidenceRefV1::new(ranked_reference.as_str()).unwrap(),
                context: frozen.clone(),
            })
            .unwrap_err(),
    );
    unavailable(
        engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: EvidenceRefV1::new(graph_reference.as_str()).unwrap(),
                context: frozen,
            })
            .unwrap_err(),
    );
}

fn minted_target_reference(
    engine: &Engine,
    request: &mut GraphExpandRequestV1,
) -> (GraphEvidenceRefV1, fathomdb_engine::FrozenReadContextV1) {
    request.include_evidence = true;
    let reference = engine.graph_expand(request).unwrap().evidence.unwrap().entries[0]
        .target_evidence_ref
        .clone();
    let frozen = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    (reference, frozen)
}

fn canonical_target_fixture() -> (TempDir, Engine, GraphExpandRequestV1) {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join("canonical-target-lifecycle.fdb");
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(NoopEmbedder::default())).unwrap();
    let edge_source = "independent edge source bytes";
    let derived = |revision: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("edge-source-v1").unwrap(),
            SourceRevisionId::new("edge-source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(edge_source)).unwrap(),
        )
    };
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("edge-source".into()),
                kind: "document".into(),
                body: edge_source.into(),
                source_id: SourceId::new("edge-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("edge-source-r1").unwrap(),
                    SourceVersionId::new("edge-source-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: "root".into(),
                source_id: SourceId::new("edge-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived("root-r1"),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target".into()),
                kind: "claim".into(),
                body: "canonical target bytes".into(),
                source_id: SourceId::new("target-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("target-r1").unwrap(),
                    SourceVersionId::new("target-v1").unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("winner".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("edge-owner").unwrap(),
                body: None,
                t_valid: Some(1_700_000_000),
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: Some(false),
                provenance: derived("edge-r1"),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_800_000_000), ..ReadView::default() },
        SearchFilter::default(),
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
        result_limit: 1,
        max_work_units: 10,
        include_explanation: false,
        include_evidence: true,
    };
    (directory, opened.engine, request)
}

#[test]
fn graph_reference_requires_current_active_artifact_and_source_after_equivalent_remint() {
    let (directory, engine, request) = canonical_target_fixture();
    let ordinary = engine
        .graph_expand(&GraphExpandRequestV1 { include_evidence: false, ..request.clone() })
        .unwrap();
    assert_eq!(ordinary.targets.len(), 1);
    let expanded = engine.graph_expand(&request).unwrap();
    let reference = expanded.evidence.unwrap().entries[0].target_evidence_ref.clone();
    let original = match &request.context {
        GraphReadContextV1::Frozen { context, .. } => context.clone(),
        GraphReadContextV1::Current { .. } => unreachable!(),
    };

    let equivalent = engine.freeze_read_context(&original.context).unwrap();
    assert_eq!(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference.clone(),
                context: equivalent,
            })
            .unwrap()
            .artifact_revision_id
            .as_str(),
        "target-r1"
    );

    engine.transition("target", LifecycleState::Deleted, Some("revoked".into())).unwrap();
    let stale_error = engine.graph_expand(&request).unwrap_err();
    assert!(
        matches!(stale_error, EngineError::FrozenRead(ref error)
            if error.reason == FrozenReadErrorReason::StateDrifted
                && error.field_path == "/token"),
        "unexpected stale-context error: {stale_error:?}"
    );

    let semantic_context = original.context.clone();
    engine.close().unwrap();
    let connection =
        Connection::open(directory.path().join("canonical-target-lifecycle.fdb")).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_source_links SET locator_kind='utf8_bytes',start_byte=0,end_byte=999999 \
             WHERE artifact_revision_id='target-r1'",
            [],
        )
        .unwrap();
    drop(connection);
    let reopened = Engine::open_with_embedder_for_test(
        directory.path().join("canonical-target-lifecycle.fdb"),
        Arc::new(NoopEmbedder::default()),
    )
    .unwrap()
    .engine;
    let reminted = reopened.freeze_read_context(&semantic_context).unwrap();
    unavailable(
        reopened
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: reminted,
            })
            .unwrap_err(),
    );
}

#[test]
fn graph_terminal_edge_reference_requires_current_temporal_validity() {
    let (_directory, engine, request) = canonical_target_fixture();
    let expanded = engine.graph_expand(&request).unwrap();
    let reference = expanded.evidence.unwrap().entries[0].terminal_edge_evidence_ref.clone();
    let original = match request.context {
        GraphReadContextV1::Frozen { context, .. } => context,
        GraphReadContextV1::Current { .. } => unreachable!(),
    };
    let derived = WriteProvenanceV1::derived(
        ArtifactRevisionId::new("newer-edge-r1").unwrap(),
        SourceVersionId::new("edge-source-v1").unwrap(),
        SourceRevisionId::new("edge-source-r1").unwrap(),
        SourceLocator::whole_body(),
        CanonicalHash::sha256(digest("independent edge source bytes")).unwrap(),
    );
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("other-target".into()),
                kind: "claim".into(),
                body: "other".into(),
                source_id: SourceId::new("edge-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("other-target-r1").unwrap(),
                    SourceVersionId::new("edge-source-v1").unwrap(),
                    SourceRevisionId::new("edge-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest("independent edge source bytes")).unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("newer-winner".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "other-target".into(),
                source_id: SourceId::new("edge-owner").unwrap(),
                body: Some("newer supporting fact".into()),
                t_valid: Some(1_750_000_000),
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived,
            }),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/slice15_consolidate/stub_consolidate_harness.py");
    let command = ["python3".to_string(), script.to_string_lossy().into_owned()];
    let command_refs = command.iter().map(String::as_str).collect::<Vec<_>>();
    let receipt = engine
        .consolidate_with_provider(
            &command_refs,
            &[ConsolidateAxis { subject_logical_id: "root".into(), relation: "supports".into() }],
        )
        .unwrap();
    assert_eq!(receipt.edges_invalidated, 1);
    let reminted = engine.freeze_read_context(&original.context).unwrap();
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: reminted,
            })
            .unwrap_err(),
    );
}

#[test]
fn superseded_revoked_closure_fenced_and_missing_artifacts_are_nondisclosing() {
    let (_directory, engine, mut request) = fixture();
    let (reference, frozen) = minted_target_reference(&engine, &mut request);
    engine
        .write(&[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some("target".into()),
            kind: "claim".into(),
            body: "superseding target".into(),
            source_id: SourceId::new("owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new("target-r2").unwrap(),
                SourceVersionId::new("source-v1").unwrap(),
                SourceRevisionId::new("source-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest("canonical source bytes")).unwrap(),
            ),
        })])
        .unwrap();
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: frozen,
            })
            .unwrap_err(),
    );

    let (_directory, engine, mut request) = fixture();
    let (reference, frozen) = minted_target_reference(&engine, &mut request);
    engine.erase_source("owner").unwrap();
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: frozen,
            })
            .unwrap_err(),
    );

    let (directory, engine, mut request) = fixture();
    let (reference, frozen) = minted_target_reference(&engine, &mut request);
    let engine = reopen_after_sql(
        &directory,
        engine,
        "UPDATE _fathomdb_open_state SET value='1' \
           WHERE key='_fathomdb_closure_sequence';
         INSERT INTO _fathomdb_dependency_closures(
           schema_version,closure_operation_id,root_kind,root_value,cause,
           effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,
           closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,
           structural_proof_write_boundary,proof_json
         ) VALUES(1,'_fdb:c:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
           'source_revision','source-r1','soft_deleted',0,4,0,1,
           'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
           'proving',1,NULL,NULL,NULL);",
    );
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: frozen,
            })
            .unwrap_err(),
    );

    let (directory, engine, mut request) = fixture();
    let (reference, frozen) = minted_target_reference(&engine, &mut request);
    let engine = reopen_after_sql(
        &directory,
        engine,
        "DELETE FROM canonical_nodes WHERE logical_id='target';",
    );
    unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: reference,
                context: frozen,
            })
            .unwrap_err(),
    );
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
    let reference = result.evidence.unwrap().entries[0].target_evidence_ref.clone();
    opened.engine.close().unwrap();
    let reopened = Engine::open(directory.path().join("canonical-target.fdb")).unwrap().engine;
    let target = reopened
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: reference,
            context: frozen,
        })
        .unwrap();
    assert_eq!(target.artifact_revision_id.as_str(), "source-r1");
    assert!(target.dependency.is_none());
}
