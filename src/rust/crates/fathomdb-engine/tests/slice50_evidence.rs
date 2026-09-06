//! 0.8.25 Slice 50 — compact, eligibility-bound source evidence.

use std::collections::BTreeSet;
use std::fs;
use std::sync::{Arc, Barrier};
use std::thread;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_evidence_before_resolve_return_hook_for_test, arm_evidence_before_sidecar_hook_for_test,
    arm_frozen_after_validation_hook_for_test, arm_reader_search_hook_for_test, ArtifactRevisionId,
    CanonicalHash, Engine, EngineError, EvidenceArmV1, EvidenceArtifactLifecycleV1,
    EvidenceErrorReasonV1, EvidenceGraphOriginV1, EvidenceResolveRequestV1,
    EvidenceSearchRequestV1, EvidenceSearchResultV1, InitialState, LifecycleState, PreparedWrite,
    ProjectionRole, ProjectionSpec, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView,
    ResolvedEvidenceV1, SearchFilter, SoftFallbackBranch, SourceDependencyRegistrationV1, SourceId,
    SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice50-fixed", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 8];
        for (index, byte) in text.bytes().enumerate() {
            vector[index % 8] += f32::from(byte) / 255.0;
        }
        if vector.iter().all(|value| *value == 0.0) {
            vector[0] = 1.0;
        }
        Ok(vector)
    }
}

fn open_with_fixed_embedder(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).unwrap()
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn canonical(body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new("source-low-entropy").unwrap(),
        logical_id: Some("source-logical-low-entropy".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-revision-low-entropy").unwrap(),
            SourceVersionId::new("source-version-low-entropy").unwrap(),
        ),
    })
}

fn derived(source_body: &str) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "claim".into(),
        body: "needle derived claim".into(),
        source_id: SourceId::new("source-low-entropy").unwrap(),
        logical_id: Some("derived-logical-low-entropy".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("derived-revision-low-entropy").unwrap(),
            SourceVersionId::new("source-version-low-entropy").unwrap(),
            SourceRevisionId::new("source-revision-low-entropy").unwrap(),
            SourceLocator::utf8_bytes(1, 3),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        ),
    })
}

fn canonical_named(
    logical: &str,
    revision: &str,
    version: &str,
    source_id: &str,
    body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
        ),
    })
}

fn derived_named(
    logical: &str,
    revision: &str,
    source_revision: &str,
    version: &str,
    source_id: &str,
    source_body: &str,
    body: &str,
) -> PreparedWrite {
    derived_kind_named(
        "entity",
        logical,
        revision,
        source_revision,
        version,
        source_id,
        source_body,
        body,
    )
}

#[allow(clippy::too_many_arguments)]
fn derived_kind_named(
    kind: &str,
    logical: &str,
    revision: &str,
    source_revision: &str,
    version: &str,
    source_id: &str,
    source_body: &str,
    body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(version).unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        ),
    })
}

fn request(query: &str, context: fathomdb_engine::FrozenReadContextV1) -> EvidenceSearchRequestV1 {
    EvidenceSearchRequestV1 {
        schema_version: 1,
        query: query.into(),
        context,
        rerank_depth: 0,
        use_graph_arm: false,
        alpha: 0.3,
        pool_n: 0,
        include_explanation: false,
        limit: 10,
    }
}

fn freeze_stable(engine: &Engine, context: &ReadContextV1) -> fathomdb_engine::FrozenReadContextV1 {
    engine.drain(30_000).unwrap();
    engine.freeze_read_context(context).unwrap()
}

fn assert_unavailable(error: EngineError) {
    assert!(
        matches!(
            error,
            EngineError::Evidence(ref evidence)
                if evidence.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                    && evidence.field_path == "/evidenceRef"
                    && evidence.to_string() == "evidence_unavailable at /evidenceRef"
        ),
        "expected the canonical non-disclosure outcome, got {error:?}",
    );
}

fn insert_active_closure_barrier(path: &std::path::Path, source_revision: &str, sequence: u64) {
    let connection = rusqlite::Connection::open(path).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 \
             WHERE key='_fathomdb_closure_sequence'",
            [sequence.to_string()],
        )
        .unwrap();
    connection
        .execute(
            "INSERT INTO _fathomdb_dependency_closures(\
               schema_version,closure_operation_id,root_kind,root_value,cause,\
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,\
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,\
               structural_proof_write_boundary,proof_json\
             ) VALUES(1,?1,'source_revision',?2,'soft_deleted',0,100,0,?3,?4,\
                      'proving',1,NULL,NULL,NULL)",
            rusqlite::params![
                format!("_fdb:c:{}", format!("{sequence:064x}")),
                source_revision,
                sequence,
                format!("{sequence:064x}"),
            ],
        )
        .unwrap();
}

fn seed_graph_matrix(engine: &Engine) {
    let node_source = r#"{"owner":"alice","text":"node source"}"#;
    let edge_source = r#"{"owner":"alice","text":"edge source"}"#;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "owner".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    engine
        .write(&[
            canonical_named(
                "graph-node-source",
                "graph-node-source-r1",
                "graph-node-v1",
                "graph-node-source-id",
                node_source,
            ),
            canonical_named(
                "graph-edge-source",
                "graph-edge-source-r1",
                "graph-edge-v1",
                "graph-edge-source-id",
                edge_source,
            ),
            derived_named(
                "graph-alpha",
                "graph-alpha-r1",
                "graph-node-source-r1",
                "graph-node-v1",
                "graph-node-source-id",
                node_source,
                r#"{"owner":"alice","text":"alpha endpoint"}"#,
            ),
            derived_named(
                "graph-beta",
                "graph-beta-r1",
                "graph-node-source-r1",
                "graph-node-v1",
                "graph-node-source-id",
                node_source,
                r#"{"owner":"alice","text":"beta endpoint"}"#,
            ),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                kind: "related".into(),
                from: "graph-alpha".into(),
                to: "graph-beta".into(),
                source_id: SourceId::new("graph-edge-source-id").unwrap(),
                logical_id: Some("graph-edge".into()),
                body: Some(r#"{"owner":"alice","text":"graphmatrixneedle"}"#.into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("graph-edge-r1").unwrap(),
                    SourceVersionId::new("graph-edge-v1").unwrap(),
                    SourceRevisionId::new("graph-edge-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest(edge_source)).unwrap(),
                ),
            }),
        ])
        .unwrap();
}

fn graph_matrix_context() -> ReadContextV1 {
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("owner".into(), "alice".into())];
    ReadContextV1::new(ReadView { valid_as_of: Some(1_000), ..ReadView::default() }, filter)
        .unwrap()
}

fn mint_graph_matrix_reference(
    engine: &Engine,
    context: &ReadContextV1,
) -> (fathomdb_engine::EvidenceRefV1, String) {
    let frozen = freeze_stable(engine, context);
    let mut evidence_request = request("graphmatrixneedle", frozen);
    evidence_request.use_graph_arm = true;
    let result = engine.search_with_evidence(&evidence_request).unwrap();
    let (index, hit) = result
        .search_result
        .results
        .iter()
        .enumerate()
        .find(|(_, hit)| hit.branch == SoftFallbackBranch::GraphArm)
        .expect("edge-body seed must return a graph endpoint");
    (result.evidence[index].evidence_ref.clone(), hit.id.value.clone())
}

fn assert_contribution_matches_explanation(
    engine: &Engine,
    result: &EvidenceSearchResultV1,
    context: &fathomdb_engine::FrozenReadContextV1,
    index: usize,
) -> ResolvedEvidenceV1 {
    let hit = &result.search_result.results[index];
    let sidecar = &result.evidence[index];
    let per_hit = &result.search_result.explanation.as_ref().unwrap().per_hit[index];
    assert_eq!(sidecar.result_index as usize, index);
    assert_eq!(per_hit.id, hit.write_cursor);
    assert_eq!(per_hit.arm, hit.branch);
    let resolved = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: sidecar.evidence_ref.clone(),
            context: context.clone(),
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, sidecar.artifact_revision_id);
    assert_eq!(resolved.retrieval_contribution.vector_rank, per_hit.vector_rank);
    assert_eq!(resolved.retrieval_contribution.text_rank, per_hit.text_rank);
    assert_eq!(resolved.retrieval_contribution.graph_rank, per_hit.graph_rank);
    assert_eq!(resolved.retrieval_contribution.fused_score, per_hit.fused_score);
    assert_eq!(resolved.retrieval_contribution.ce_score, per_hit.ce_score);
    assert_eq!(resolved.retrieval_contribution.blended_score, per_hit.blended);
    assert_eq!(resolved.retrieval_contribution.importance, per_hit.importance);
    assert_eq!(resolved.retrieval_contribution.confidence, per_hit.confidence);
    let expected_arm = match hit.branch {
        SoftFallbackBranch::Vector => EvidenceArmV1::Vector,
        SoftFallbackBranch::Text => EvidenceArmV1::Text,
        SoftFallbackBranch::TextEdge => EvidenceArmV1::TextEdge,
        SoftFallbackBranch::GraphArm => EvidenceArmV1::GraphArm,
    };
    assert_eq!(resolved.projection_origin.representative_arm, expected_arm);
    resolved
}

#[test]
fn resolves_exact_utf8_source_span_and_associates_sidecar_by_position() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("evidence{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();

    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();

    assert_eq!(result.schema_version, 1);
    assert_eq!(result.search_result.results.len(), 1);
    assert_eq!(result.evidence.len(), 1);
    assert_eq!(result.evidence[0].result_index, 0);
    assert_eq!(result.evidence[0].artifact_revision_id, "derived-revision-low-entropy");

    let resolved = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved.artifact_revision_id, "derived-revision-low-entropy");
    assert_eq!(resolved.source_id, "source-low-entropy");
    assert_eq!(resolved.source_version_id, "source-version-low-entropy");
    assert_eq!(resolved.source_revision_id, "source-revision-low-entropy");
    assert_eq!(resolved.canonical_source_body, source_body);
    assert_eq!(resolved.evidence_text, "é");
    assert_eq!(resolved.locator, SourceLocator::utf8_bytes(1, 3));
    assert_eq!(
        resolved.artifact_lifecycle,
        EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: false }
    );
    assert_eq!(resolved.source_lifecycle_state, LifecycleState::Active);
}

#[test]
fn tamper_and_context_mismatch_share_the_exact_nondisclosure_error() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("nondisclosure{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();

    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();
    let reference = &result.evidence[0].evidence_ref;

    let mut tampered_text = reference.as_str().to_string();
    let last = tampered_text.pop().unwrap();
    tampered_text.push(if last == '0' { '1' } else { '0' });
    let tampered = fathomdb_engine::EvidenceRefV1::new(tampered_text).unwrap();

    let mut different_filter = SearchFilter::default();
    different_filter.kind = Some("claim".into());
    let different = freeze_stable(
        &opened.engine,
        &ReadContextV1::new(ReadView::default(), different_filter).unwrap(),
    );

    let errors = [tampered, reference.clone()]
        .into_iter()
        .zip([frozen, different])
        .map(|(evidence_ref, context)| {
            opened
                .engine
                .resolve_evidence(&EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref,
                    context,
                })
                .unwrap_err()
        })
        .collect::<Vec<_>>();

    for error in errors {
        match error {
            EngineError::Evidence(error) => {
                assert_eq!(error.reason, EvidenceErrorReasonV1::EvidenceUnavailable);
                assert_eq!(error.field_path, "/evidenceRef");
            }
            other => panic!("unexpected error: {other:?}"),
        }
    }
}

#[test]
fn visible_reference_payload_uses_keyed_not_dictionary_matchable_commitments() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("privacy{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();
    let frozen = freeze_stable(
        &opened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );

    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();
    let second = opened.engine.search_with_evidence(&request("needle", frozen)).unwrap();
    assert_ne!(
        result.evidence[0].evidence_ref, second.evidence[0].evidence_ref,
        "generation protection requires a fresh per-reference nonce",
    );
    let token = result.evidence[0].evidence_ref.as_str();
    let payload_hex = token.split('.').nth(1).unwrap();
    let payload = (0..payload_hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&payload_hex[index..index + 2], 16).unwrap())
        .collect::<Vec<_>>();

    let generation: String = rusqlite::Connection::open(&path)
        .unwrap()
        .query_row(
            "SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    for secret in [
        "needle",
        "source-low-entropy",
        "source-version-low-entropy",
        "source-logical-low-entropy",
        "source-revision-low-entropy",
        "derived-logical-low-entropy",
        "derived-revision-low-entropy",
        source_body,
        generation.as_str(),
    ] {
        assert!(!String::from_utf8_lossy(&payload).contains(secret));
        assert!(!payload
            .windows(32)
            .any(|window| window == Sha256::digest(secret.as_bytes()).as_slice()));
    }
}

#[test]
fn graph_arm_resolves_node_body_source_and_separate_edge_origin() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("graph{SQLITE_SUFFIX}"));
    let opened = open_with_fixed_embedder(&path);
    let node_source = "node source body";
    let edge_source = "edge source body";
    opened
        .engine
        .write(&[
            canonical_named(
                "node-source",
                "node-source-r1",
                "node-v1",
                "node-source-id",
                node_source,
            ),
            canonical_named(
                "edge-source",
                "edge-source-r1",
                "edge-v1",
                "edge-source-id",
                edge_source,
            ),
            derived_named(
                "alpha",
                "alpha-r1",
                "node-source-r1",
                "node-v1",
                "node-source-id",
                node_source,
                "alpha entity",
            ),
            derived_named(
                "beta",
                "beta-r1",
                "node-source-r1",
                "node-v1",
                "node-source-id",
                node_source,
                "beta entity",
            ),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                kind: "related".into(),
                from: "alpha".into(),
                to: "beta".into(),
                source_id: SourceId::new("edge-source-id").unwrap(),
                logical_id: Some("edge-logical".into()),
                body: Some("graphneedle relationship".into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("edge-r1").unwrap(),
                    SourceVersionId::new("edge-v1").unwrap(),
                    SourceRevisionId::new("edge-source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest(edge_source)).unwrap(),
                ),
            }),
        ])
        .unwrap();
    let frozen = freeze_stable(
        &opened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );
    let mut evidence_request = request("graphneedle", frozen.clone());
    evidence_request.use_graph_arm = true;
    let result = opened.engine.search_with_evidence(&evidence_request).unwrap();
    let (index, hit) = result
        .search_result
        .results
        .iter()
        .enumerate()
        .find(|(_, hit)| hit.branch == SoftFallbackBranch::GraphArm)
        .expect("graph arm should return an endpoint node");
    let resolved = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence[index].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();

    assert_eq!(resolved.artifact_revision_id, format!("{}-r1", hit.id.value));
    assert_eq!(resolved.source_revision_id, "node-source-r1");
    assert_eq!(resolved.canonical_source_body, node_source);
    assert!(matches!(
        resolved.projection_origin.graph_origin,
        Some(EvidenceGraphOriginV1::EdgeSeed { ref edge_artifact_revision_id })
            if edge_artifact_revision_id == "edge-r1"
    ));

    let evidence_ref = result.evidence[index].evidence_ref.clone();
    drop(opened.engine);
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_versions SET source_version_id='edge-corrupt-v2' \
         WHERE source_revision_id='edge-source-r1'",
        [],
    )
    .unwrap();
    drop(raw);
    let reopened = open_with_fixed_embedder(&path);
    let equivalent = freeze_stable(
        &reopened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );
    let mut corrupt_graph_request = request("alpha", equivalent.clone());
    corrupt_graph_request.use_graph_arm = true;
    let mint_error = reopened.engine.search_with_evidence(&corrupt_graph_request).unwrap_err();
    assert!(matches!(
        mint_error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                && error.field_path.starts_with("/results/")
                && error.field_path.ends_with("/graphOrigin")
    ));
    let error = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence_ref.clone(),
            context: equivalent,
        })
        .unwrap_err();
    assert!(
        matches!(
            error,
            EngineError::Evidence(ref error)
                if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                    && error.field_path == "/projectionOrigin/graphOrigin"
        ),
        "unexpected graph corruption error: {error:?}"
    );
    drop(reopened.engine);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_versions SET source_version_id='edge-v1' \
         WHERE source_revision_id='edge-source-r1'",
        [],
    )
    .unwrap();
    raw.execute("UPDATE canonical_edges SET superseded_at=1 WHERE logical_id='edge-logical'", [])
        .unwrap();
    drop(raw);
    let reopened = open_with_fixed_embedder(&path);
    let equivalent = freeze_stable(
        &reopened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );
    let error = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref,
            context: equivalent,
        })
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                && error.field_path == "/evidenceRef"
    ));
}

#[test]
fn source_bytes_must_match_access_bearing_attribute_terms() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("source-eligibility{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "owner".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    let source_body = r#"{"owner":"bob","text":"canonical"}"#;
    opened
        .engine
        .write(&[
            canonical_named(
                "source",
                "source-owner-r1",
                "source-owner-v1",
                "source-owner-id",
                source_body,
            ),
            derived_named(
                "claim",
                "claim-owner-r1",
                "source-owner-r1",
                "source-owner-v1",
                "source-owner-id",
                source_body,
                r#"{"owner":"alice","text":"needle"}"#,
            ),
        ])
        .unwrap();
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("owner".into(), "alice".into())];
    let frozen =
        freeze_stable(&opened.engine, &ReadContextV1::new(ReadView::default(), filter).unwrap());
    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();
    let error = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                && error.field_path == "/evidenceRef"
    ));
}

#[test]
fn equivalent_context_survives_restart_and_retired_projection_generation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("restart-generation{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source_body = "stable source body";
    opened
        .engine
        .write(&[
            canonical_named(
                "source",
                "restart-source-r1",
                "restart-v1",
                "restart-source",
                source_body,
            ),
            derived_named(
                "claim",
                "restart-claim-r1",
                "restart-source-r1",
                "restart-v1",
                "restart-source",
                source_body,
                "restartneedle",
            ),
        ])
        .unwrap();
    let view = ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() };
    let context = ReadContextV1::new(view, SearchFilter::default()).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let evidence =
        opened.engine.search_with_evidence(&request("restartneedle", frozen)).unwrap().evidence[0]
            .evidence_ref
            .clone();
    drop(opened.engine);

    let reopened = Engine::open(&path).unwrap();
    reopened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "later_generation".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    let equivalent = freeze_stable(&reopened.engine, &context);
    let resolved = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence,
            context: equivalent,
        })
        .unwrap();

    assert_eq!(resolved.artifact_revision_id, "restart-claim-r1");
    assert_eq!(resolved.canonical_source_body, source_body);
}

#[test]
fn superseded_artifact_reference_is_nondisclosing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("superseded{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source_body = "supersession source";
    opened
        .engine
        .write(&[
            canonical_named("source", "sup-source-r1", "sup-v1", "sup-source", source_body),
            derived_named(
                "claim",
                "sup-claim-r1",
                "sup-source-r1",
                "sup-v1",
                "sup-source",
                source_body,
                "supersessionneedle old",
            ),
        ])
        .unwrap();
    let view = ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() };
    let context = ReadContextV1::new(view, SearchFilter::default()).unwrap();
    let first = freeze_stable(&opened.engine, &context);
    let evidence =
        opened.engine.search_with_evidence(&request("supersessionneedle", first)).unwrap().evidence
            [0]
        .evidence_ref
        .clone();
    opened
        .engine
        .write(&[derived_named(
            "claim",
            "sup-claim-r2",
            "sup-source-r1",
            "sup-v1",
            "sup-source",
            source_body,
            "supersessionneedle new",
        )])
        .unwrap();
    let current = freeze_stable(&opened.engine, &context);
    let error = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence,
            context: current,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                && error.field_path == "/evidenceRef"
    ));
}

#[test]
fn resolved_derived_evidence_includes_its_registered_dependency() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("dependency{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source_body = "dependency source";
    opened
        .engine
        .write(&[
            canonical_named("source", "dep-source-r1", "dep-v1", "dep-source", source_body),
            derived_named(
                "claim",
                "dep-claim-r1",
                "dep-source-r1",
                "dep-v1",
                "dep-source",
                source_body,
                "dependencyneedle",
            ),
        ])
        .unwrap();
    let registered = opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "dep-source-r1", "dep-claim-r1").unwrap(),
        )
        .unwrap();
    let frozen = freeze_stable(
        &opened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );
    let result =
        opened.engine.search_with_evidence(&request("dependencyneedle", frozen.clone())).unwrap();
    let resolved = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();

    assert_eq!(resolved.dependency, Some(registered));
}

#[test]
fn relaxed_validity_view_remains_authorized_during_resolution() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("relaxed-validity{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let source_body = "future source";
    let future: i64 = 4_102_444_800;
    opened
        .engine
        .write(&[
            canonical_named(
                "future-source-logical",
                "future-source-r1",
                "future-v1",
                "future-source",
                source_body,
            ),
            derived_named(
                "future-claim",
                "future-claim-r1",
                "future-source-r1",
                "future-v1",
                "future-source",
                source_body,
                "futureevidenceneedle",
            ),
        ])
        .unwrap();
    drop(opened.engine);
    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE canonical_nodes SET valid_from=?1 WHERE logical_id IN (?2,?3)",
        rusqlite::params![future, "future-source-logical", "future-claim"],
    )
    .unwrap();
    drop(raw);
    let opened = Engine::open(&path).unwrap();
    let context = ReadContextV1::new(
        ReadView { include_out_of_window: true, ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let result = opened
        .engine
        .search_with_evidence(&request("futureevidenceneedle", frozen.clone()))
        .unwrap();

    let resolved = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: result.evidence[0].evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();

    assert_eq!(resolved.artifact_revision_id, "future-claim-r1");
    assert_eq!(resolved.source_revision_id, "future-source-r1");
}

#[test]
fn incomplete_detail_is_disclosed_only_after_current_source_authorization() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("incomplete-precedence{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "owner".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    let source_body = r#"{"owner":"alice","text":"canonical"}"#;
    opened
        .engine
        .write(&[
            canonical_named(
                "precedence-source",
                "precedence-source-r1",
                "precedence-v1",
                "precedence-source-id",
                source_body,
            ),
            derived_named(
                "precedence-claim",
                "precedence-claim-r1",
                "precedence-source-r1",
                "precedence-v1",
                "precedence-source-id",
                source_body,
                r#"{"owner":"alice","text":"precedenceneedle"}"#,
            ),
        ])
        .unwrap();
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("owner".into(), "alice".into())];
    let context = ReadContextV1::new(ReadView::default(), filter).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let evidence =
        opened.engine.search_with_evidence(&request("precedenceneedle", frozen)).unwrap().evidence
            [0]
        .evidence_ref
        .clone();
    drop(opened.engine);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE _fathomdb_artifact_revisions SET completeness='migrated_incomplete' \
         WHERE revision_id='precedence-claim-r1'",
        [],
    )
    .unwrap();
    raw.execute(
        "UPDATE canonical_nodes SET body=?1 WHERE logical_id='precedence-source'",
        [r#"{"owner":"bob","text":"canonical"}"#],
    )
    .unwrap();
    drop(raw);

    let reopened = Engine::open(&path).unwrap();
    let hidden_context = freeze_stable(&reopened.engine, &context);
    let hidden = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence.clone(),
            context: hidden_context,
        })
        .unwrap_err();
    assert!(matches!(
        hidden,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                && error.field_path == "/evidenceRef"
    ));
    drop(reopened.engine);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE canonical_nodes SET body=?1 WHERE logical_id='precedence-source'",
        [source_body],
    )
    .unwrap();
    drop(raw);
    let reopened = Engine::open(&path).unwrap();
    let visible_context = freeze_stable(&reopened.engine, &context);
    let visible = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence,
            context: visible_context,
        })
        .unwrap_err();
    assert!(
        matches!(
            visible,
            EngineError::Evidence(ref error)
                if error.reason == EvidenceErrorReasonV1::EvidenceIncomplete
                    && error.field_path == "/provenance"
        ),
        "unexpected visible-incomplete outcome: {visible:?}"
    );
}

#[test]
fn evidence_search_collapses_frozen_context_failure_to_nondisclosure() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("search-context-error{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let mut frozen = freeze_stable(
        &opened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );
    frozen.token.push('0');

    let error = opened.engine.search_with_evidence(&request("needle", frozen)).unwrap_err();

    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                && error.field_path == "/evidenceRef"
    ));
}

#[test]
fn evidence_search_authenticates_before_existence_axis_refusal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("search-precedence{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();

    for existence_view in [
        ReadView { include_superseded: true, ..ReadView::default() },
        ReadView { include_inactive: true, ..ReadView::default() },
    ] {
        let context = ReadContextV1::new(existence_view, SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);

        let authenticated =
            opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap_err();
        assert!(matches!(authenticated, EngineError::InvalidArgument { .. }));

        let mut forged = frozen;
        forged.token.push('0');
        let error = opened.engine.search_with_evidence(&request("needle", forged)).unwrap_err();
        assert_unavailable(error);
    }
}

#[test]
fn unsupported_resolve_schema_precedes_context_authentication() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("resolve-schema-precedence{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let mut frozen = freeze_stable(
        &opened.engine,
        &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    );
    frozen.token.push('0');

    let error = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 2,
            evidence_ref: fathomdb_engine::EvidenceRefV1::new("opaque").unwrap(),
            context: frozen,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::UnsupportedSchemaVersion
                && error.field_path == "/schemaVersion"
    ));
}

#[test]
fn authorized_source_identity_corruption_is_typed_evidence_corrupt() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("source-chain-corruption{SQLITE_SUFFIX}"));
    let source_body = "authoritative source body";
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[
            canonical_named(
                "chain-source",
                "chain-source-r1",
                "chain-source-v1",
                "chain-source-id",
                source_body,
            ),
            derived_named(
                "chain-claim",
                "chain-claim-r1",
                "chain-source-r1",
                "chain-source-v1",
                "chain-source-id",
                source_body,
                "chaincorruptionneedle",
            ),
        ])
        .unwrap();
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let evidence = opened
        .engine
        .search_with_evidence(&request("chaincorruptionneedle", frozen))
        .unwrap()
        .evidence[0]
        .evidence_ref
        .clone();
    drop(opened.engine);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_versions SET source_version_id='different-version' \
         WHERE source_revision_id='chain-source-r1'",
        [],
    )
    .unwrap();
    drop(raw);

    let reopened = Engine::open(&path).unwrap();
    let equivalent = freeze_stable(&reopened.engine, &context);
    let mint_error = reopened
        .engine
        .search_with_evidence(&request("chaincorruptionneedle", equivalent.clone()))
        .unwrap_err();
    assert!(matches!(
        mint_error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                && error.field_path == "/results/0/provenance"
    ));
    let error = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence,
            context: equivalent,
        })
        .unwrap_err();

    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                && error.field_path == "/provenance"
    ));
}

#[test]
fn authorized_dependency_corruption_is_typed_evidence_corrupt() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("dependency-corruption{SQLITE_SUFFIX}"));
    let source_body = "dependency authority";
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[
            canonical_named(
                "corrupt-dep-source",
                "corrupt-dep-source-r1",
                "corrupt-dep-v1",
                "corrupt-dep-source-id",
                source_body,
            ),
            derived_named(
                "corrupt-dep-claim",
                "corrupt-dep-claim-r1",
                "corrupt-dep-source-r1",
                "corrupt-dep-v1",
                "corrupt-dep-source-id",
                source_body,
                "dependencycorruptionneedle",
            ),
        ])
        .unwrap();
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                "corrupt-dep-1",
                "corrupt-dep-source-r1",
                "corrupt-dep-claim-r1",
            )
            .unwrap(),
        )
        .unwrap();
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let evidence = opened
        .engine
        .search_with_evidence(&request("dependencycorruptionneedle", frozen))
        .unwrap()
        .evidence[0]
        .evidence_ref
        .clone();
    drop(opened.engine);

    let raw = rusqlite::Connection::open(&path).unwrap();
    raw.pragma_update(None, "ignore_check_constraints", "ON").unwrap();
    raw.execute(
        "UPDATE _fathomdb_source_dependencies SET schema_version=2 \
         WHERE dependency_id='corrupt-dep-1'",
        [],
    )
    .unwrap();
    drop(raw);

    let reopened = Engine::open(&path).unwrap();
    let equivalent = freeze_stable(&reopened.engine, &context);
    let error = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence,
            context: equivalent,
        })
        .unwrap_err();
    assert!(matches!(
        error,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                && error.field_path == "/dependency"
    ));
}

#[test]
fn authorized_hash_locator_and_generation_corruption_is_typed() {
    // A body mutation that leaves the committed source-link hash untouched is
    // visible only as post-authorization structural corruption.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("hash-corruption{SQLITE_SUFFIX}"));
        let source_body = "hash authority source";
        let opened = Engine::open(&path).unwrap();
        opened
            .engine
            .write(&[
                canonical_named(
                    "hash-source",
                    "hash-source-r1",
                    "hash-v1",
                    "hash-source-id",
                    source_body,
                ),
                derived_named(
                    "hash-claim",
                    "hash-claim-r1",
                    "hash-source-r1",
                    "hash-v1",
                    "hash-source-id",
                    source_body,
                    "hashcorruptionneedle",
                ),
            ])
            .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let evidence = opened
            .engine
            .search_with_evidence(&request("hashcorruptionneedle", frozen))
            .unwrap()
            .evidence
            .into_iter()
            .find(|entry| entry.artifact_revision_id == "hash-claim-r1")
            .unwrap()
            .evidence_ref;
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            "UPDATE canonical_nodes SET body='tampered source bytes' \
             WHERE logical_id='hash-source'",
            [],
        )
        .unwrap();
        drop(raw);
        let equivalent = freeze_stable(&opened.engine, &context);
        let error = opened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: evidence,
                context: equivalent,
            })
            .unwrap_err();
        assert!(
            matches!(
                error,
                EngineError::Evidence(ref evidence)
                    if evidence.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                        && evidence.field_path == "/canonicalSourceHash"
            ),
            "unexpected hash-corruption outcome: {error:?}"
        );
    }

    // Minting refuses an already-visible locator whose exact UTF-8 slice
    // cannot be resolved. No malformed reference is allowed to escape.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("locator-corruption{SQLITE_SUFFIX}"));
        let source_body = "locator authority source";
        let opened = Engine::open(&path).unwrap();
        opened
            .engine
            .write(&[
                canonical_named(
                    "locator-source",
                    "locator-source-r1",
                    "locator-v1",
                    "locator-source-id",
                    source_body,
                ),
                derived_named(
                    "locator-claim",
                    "locator-claim-r1",
                    "locator-source-r1",
                    "locator-v1",
                    "locator-source-id",
                    source_body,
                    "locatorcorruptionneedle",
                ),
            ])
            .unwrap();
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            "UPDATE _fathomdb_source_links \
             SET locator_kind='utf8_bytes',start_byte=1,end_byte=999 \
             WHERE artifact_revision_id='locator-claim-r1'",
            [],
        )
        .unwrap();
        drop(raw);
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let error = opened
            .engine
            .search_with_evidence(&request("locatorcorruptionneedle", frozen))
            .unwrap_err();
        assert!(matches!(
            error,
            EngineError::Evidence(ref evidence)
                if evidence.reason == EvidenceErrorReasonV1::EvidenceCorrupt
                    && evidence.field_path == "/results/0/provenance"
        ));
    }

    // Projection-generation storage is part of the frozen authority itself.
    // A malformed row therefore fails closed before row-level evidence detail
    // is authorized, even if the raw-fault visibility trigger is bypassed.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("generation-corruption{SQLITE_SUFFIX}"));
        let opened = Engine::open(&path).unwrap();
        opened.engine.write(&[canonical("generationcorruptionneedle")]).unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let evidence = opened
            .engine
            .search_with_evidence(&request("generationcorruptionneedle", frozen.clone()))
            .unwrap()
            .evidence[0]
            .evidence_ref
            .clone();
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute_batch(
            "DROP TRIGGER _fathomdb_projection_generation_immutable;
             DROP TRIGGER _fathomdb_read_visibility_pg_au;
             UPDATE _fathomdb_projection_generations
                SET declaration_sha256='not-a-sha256'
              WHERE role='serving';",
        )
        .unwrap();
        drop(raw);
        let error = opened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: evidence,
                context: frozen,
            })
            .unwrap_err();
        assert_unavailable(error);
    }
}

#[test]
fn ordinary_search_is_equivalent_and_evidence_is_stateless() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("stateless{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical("statelessneedle")]).unwrap();
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = freeze_stable(&opened.engine, &context);
    let wal_path = std::path::PathBuf::from(format!("{}-wal", path.display()));
    let database_before = fs::read(&path).unwrap();
    let wal_before = fs::read(&wal_path).ok();
    let evidence_tables_before: Vec<String> = {
        let raw = rusqlite::Connection::open(&path).unwrap();
        let mut statement = raw
            .prepare(
                "SELECT name FROM sqlite_master WHERE lower(name) LIKE '%evidence%' ORDER BY name",
            )
            .unwrap();
        statement.query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };

    let ordinary = opened.engine.search("statelessneedle").unwrap();
    let frozen_ordinary = opened
        .engine
        .search_frozen("statelessneedle", &frozen, 0, false, 0.3, 0, false, 10)
        .unwrap();
    let with_evidence =
        opened.engine.search_with_evidence(&request("statelessneedle", frozen.clone())).unwrap();
    assert_eq!(ordinary.results, frozen_ordinary.results);
    assert_eq!(frozen_ordinary, with_evidence.search_result);
    assert_eq!(with_evidence.evidence.len(), with_evidence.search_result.results.len());
    for entry in with_evidence.evidence {
        opened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry.evidence_ref,
                context: frozen.clone(),
            })
            .unwrap();
    }

    let evidence_tables_after: Vec<String> = {
        let raw = rusqlite::Connection::open(&path).unwrap();
        let mut statement = raw
            .prepare(
                "SELECT name FROM sqlite_master WHERE lower(name) LIKE '%evidence%' ORDER BY name",
            )
            .unwrap();
        statement.query_map([], |row| row.get(0)).unwrap().collect::<Result<_, _>>().unwrap()
    };
    assert_eq!(evidence_tables_before, evidence_tables_after);
    assert_eq!(fs::read(&path).unwrap(), database_before);
    assert_eq!(fs::read(&wal_path).ok(), wal_before);
}

#[test]
fn lifecycle_deletion_and_source_erasure_revoke_evidence() {
    for erased in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("revocation-{erased}{SQLITE_SUFFIX}"));
        let source_id = format!("revocation-source-{erased}");
        let source_revision = format!("revocation-source-r1-{erased}");
        let source_body = "revocation source body";
        let opened = Engine::open(&path).unwrap();
        opened
            .engine
            .write(&[
                canonical_named(
                    "revocation-source",
                    &source_revision,
                    "revocation-v1",
                    &source_id,
                    source_body,
                ),
                derived_named(
                    "revocation-claim",
                    &format!("revocation-claim-r1-{erased}"),
                    &source_revision,
                    "revocation-v1",
                    &source_id,
                    source_body,
                    "revocationneedle",
                ),
            ])
            .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let evidence = opened
            .engine
            .search_with_evidence(&request("revocationneedle", frozen))
            .unwrap()
            .evidence[0]
            .evidence_ref
            .clone();
        if erased {
            opened.engine.erase_source(&source_id).unwrap();
        } else {
            opened.engine.transition("revocation-source", LifecycleState::Deleted, None).unwrap();
        }
        let equivalent = freeze_stable(&opened.engine, &context);
        let error = opened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: evidence,
                context: equivalent,
            })
            .unwrap_err();
        assert!(matches!(
            error,
            EngineError::Evidence(ref error)
                if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
                    && error.field_path == "/evidenceRef"
        ));
    }
}

#[test]
fn evidence_search_races_are_snapshot_atomic_or_wholly_refused() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("evidence-race{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let engine = Arc::new(opened.engine);
    engine.write(&[canonical("race needle")]).unwrap();
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();

    let frozen = freeze_stable(&engine, &context);
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_reader_search_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.search_with_evidence(&request("race", frozen)))
    };
    ready.wait();
    engine
        .write(&[canonical_named(
            "race-later-1",
            "race-later-r1",
            "race-later-v1",
            "race-later-source",
            "later",
        )])
        .unwrap();
    release.wait();
    assert!(matches!(
        worker.join().unwrap(),
        Err(EngineError::Evidence(ref error))
            if error.reason == EvidenceErrorReasonV1::EvidenceUnavailable
    ));

    let frozen = freeze_stable(&engine, &context);
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_frozen_after_validation_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.search_with_evidence(&request("race", frozen)))
    };
    ready.wait();
    engine
        .write(&[canonical_named(
            "race-later-2",
            "race-later-r2",
            "race-later-v2",
            "race-later-source-2",
            "later again",
        )])
        .unwrap();
    release.wait();
    let result = worker.join().unwrap().unwrap();
    assert_eq!(result.search_result.results.len(), result.evidence.len());
    assert!(!result.evidence.is_empty());
}

#[test]
fn evidence_linearizes_at_sidecar_and_resolver_return_seams() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("evidence-linearization{SQLITE_SUFFIX}"));
    let source_body = "linearization source body";
    let opened = Engine::open(&path).unwrap();
    let engine = Arc::new(opened.engine);
    engine
        .write(&[
            canonical_named(
                "linear-source",
                "linear-source-r1",
                "linear-v1",
                "linear-source-id",
                source_body,
            ),
            derived_named(
                "linear-claim",
                "linear-claim-r1",
                "linear-source-r1",
                "linear-v1",
                "linear-source-id",
                source_body,
                "linearizationneedle",
            ),
        ])
        .unwrap();
    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();

    let frozen = freeze_stable(&engine, &context);
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_evidence_before_sidecar_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let search_worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || engine.search_with_evidence(&request("linearizationneedle", frozen)))
    };
    ready.wait();
    engine
        .write(&[canonical_named(
            "linear-unrelated",
            "linear-unrelated-r1",
            "linear-unrelated-v1",
            "linear-unrelated-source",
            "committed after ranked results",
        )])
        .unwrap();
    release.wait();
    let search = search_worker.join().unwrap().unwrap();
    assert_eq!(search.search_result.results.len(), search.evidence.len());
    assert_eq!(search.evidence[0].artifact_revision_id, "linear-claim-r1");

    let equivalent = freeze_stable(&engine, &context);
    let evidence_ref = search.evidence[0].evidence_ref.clone();
    let ready = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let hook_ready = Arc::clone(&ready);
    let hook_release = Arc::clone(&release);
    arm_evidence_before_resolve_return_hook_for_test(Box::new(move || {
        hook_ready.wait();
        hook_release.wait();
    }));
    let resolve_worker = {
        let engine = Arc::clone(&engine);
        thread::spawn(move || {
            engine.resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref,
                context: equivalent,
            })
        })
    };
    ready.wait();
    let external = rusqlite::Connection::open(&path).unwrap();
    external
        .execute(
            "UPDATE canonical_nodes SET body='changed after resolution' \
             WHERE logical_id='linear-source'",
            [],
        )
        .unwrap();
    drop(external);
    release.wait();
    let resolved = resolve_worker.join().unwrap().unwrap();
    assert_eq!(resolved.canonical_source_body, source_body);
    assert_eq!(resolved.evidence_text, source_body);
}

#[test]
fn ordinary_authorization_revocation_matrix_is_indistinguishable() {
    let context = ReadContextV1::new(
        ReadView { valid_as_of: Some(1_000), ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap();

    // A reference and even an equivalent-looking context from another database
    // convey no authority.
    {
        let dir = TempDir::new().unwrap();
        let first_path = dir.path().join(format!("foreign-a{SQLITE_SUFFIX}"));
        let second_path = dir.path().join(format!("foreign-b{SQLITE_SUFFIX}"));
        let first = Engine::open(&first_path).unwrap();
        let source_body = "foreign source";
        first
            .engine
            .write(&[
                canonical_named(
                    "matrix-source",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                ),
                derived_named(
                    "matrix-claim",
                    "matrix-claim-r1",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                    "matrixneedle",
                ),
            ])
            .unwrap();
        let frozen = freeze_stable(&first.engine, &context);
        let reference =
            first.engine.search_with_evidence(&request("matrixneedle", frozen)).unwrap().evidence
                [0]
            .evidence_ref
            .clone();
        let second = Engine::open(&second_path).unwrap();
        let foreign_context = freeze_stable(&second.engine, &context);
        assert_unavailable(
            second
                .engine
                .resolve_evidence(&EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: reference,
                    context: foreign_context,
                })
                .unwrap_err(),
        );
    }

    // Both broader and narrower envelopes differ from the originating policy,
    // even when all three would admit the same current row.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("context-width{SQLITE_SUFFIX}"));
        let opened = Engine::open(&path).unwrap();
        let source_body = "context source";
        opened
            .engine
            .write(&[
                canonical_named(
                    "matrix-source",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                ),
                derived_named(
                    "matrix-claim",
                    "matrix-claim-r1",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                    "matrixneedle",
                ),
            ])
            .unwrap();
        let mut origin_filter = SearchFilter::default();
        origin_filter.kind = Some("entity".into());
        let origin = ReadContextV1::new(context.view, origin_filter.clone()).unwrap();
        let frozen = freeze_stable(&opened.engine, &origin);
        let reference =
            opened.engine.search_with_evidence(&request("matrixneedle", frozen)).unwrap().evidence
                [0]
            .evidence_ref
            .clone();
        let broader = ReadContextV1::new(context.view, SearchFilter::default()).unwrap();
        origin_filter.created_after = Some(0);
        let narrower = ReadContextV1::new(context.view, origin_filter).unwrap();
        for replacement in [broader, narrower] {
            let mismatch = freeze_stable(&opened.engine, &replacement);
            assert_unavailable(
                opened
                    .engine
                    .resolve_evidence(&EvidenceResolveRequestV1 {
                        schema_version: 1,
                        evidence_ref: reference.clone(),
                        context: mismatch,
                    })
                    .unwrap_err(),
            );
        }
    }

    for case in ["validity", "closure", "replacement"] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("ordinary-{case}{SQLITE_SUFFIX}"));
        let opened = Engine::open(&path).unwrap();
        let source_body = "ordinary authority source";
        opened
            .engine
            .write(&[
                canonical_named(
                    "matrix-source",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                ),
                derived_named(
                    "matrix-claim",
                    "matrix-claim-r1",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                    "matrixneedle",
                ),
            ])
            .unwrap();
        opened
            .engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new(
                    "matrix-dependency",
                    "matrix-source-r1",
                    "matrix-claim-r1",
                )
                .unwrap(),
            )
            .unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let reference =
            opened.engine.search_with_evidence(&request("matrixneedle", frozen)).unwrap().evidence
                [0]
            .evidence_ref
            .clone();
        match case {
            "validity" => {
                let raw = rusqlite::Connection::open(&path).unwrap();
                raw.execute(
                    "UPDATE canonical_nodes SET valid_until=1000 \
                     WHERE logical_id='matrix-claim'",
                    [],
                )
                .unwrap();
            }
            "closure" => insert_active_closure_barrier(&path, "matrix-source-r1", 1),
            "replacement" => {
                opened
                    .engine
                    .write(&[canonical_named(
                        "matrix-source",
                        "matrix-source-r2",
                        "matrix-v2",
                        "matrix-id",
                        "replacement source",
                    )])
                    .unwrap();
            }
            _ => unreachable!(),
        }
        let equivalent = freeze_stable(&opened.engine, &context);
        assert_unavailable(
            opened
                .engine
                .resolve_evidence(&EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: reference,
                    context: equivalent,
                })
                .unwrap_err(),
        );
    }

    // An access-bearing attribute missing on the source denies bytes even when
    // the returned artifact itself carries the required value.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("missing-source-attribute{SQLITE_SUFFIX}"));
        let opened = Engine::open(&path).unwrap();
        opened
            .engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "owner".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                }],
                &[],
            )
            .unwrap();
        let source_body = r#"{"text":"source without owner"}"#;
        opened
            .engine
            .write(&[
                canonical_named(
                    "matrix-source",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                ),
                derived_named(
                    "matrix-claim",
                    "matrix-claim-r1",
                    "matrix-source-r1",
                    "matrix-v1",
                    "matrix-id",
                    source_body,
                    r#"{"owner":"alice","text":"missingattributeneedle"}"#,
                ),
            ])
            .unwrap();
        let mut filter = SearchFilter::default();
        filter.attributes = vec![("owner".into(), "alice".into())];
        let filtered = ReadContextV1::new(context.view, filter).unwrap();
        let frozen = freeze_stable(&opened.engine, &filtered);
        let reference = opened
            .engine
            .search_with_evidence(&request("missingattributeneedle", frozen.clone()))
            .unwrap()
            .evidence[0]
            .evidence_ref
            .clone();
        assert_unavailable(
            opened
                .engine
                .resolve_evidence(&EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: reference,
                    context: frozen,
                })
                .unwrap_err(),
        );
    }
}

#[test]
fn graph_origin_revocation_matrix_is_indistinguishable() {
    let context = graph_matrix_context();
    for case in ["erasure", "replacement", "validity", "endpoint", "closure"] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("graph-revocation-{case}{SQLITE_SUFFIX}"));
        let opened = open_with_fixed_embedder(&path);
        seed_graph_matrix(&opened.engine);
        let (reference, endpoint) = mint_graph_matrix_reference(&opened.engine, &context);

        match case {
            "erasure" => {
                opened.engine.erase_source("graph-edge-source-id").unwrap();
            }
            "replacement" => {
                let edge_source = r#"{"owner":"alice","text":"edge source"}"#;
                opened
                    .engine
                    .write(&[PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                        kind: "related".into(),
                        from: "graph-alpha".into(),
                        to: "graph-beta".into(),
                        source_id: SourceId::new("graph-edge-source-id").unwrap(),
                        logical_id: Some("graph-edge".into()),
                        body: Some("replacement relationship".into()),
                        t_valid: None,
                        t_invalid: None,
                        confidence: Some(0.9),
                        extractor_model_id: None,
                        temporal_fallback: None,
                        provenance: WriteProvenanceV1::derived(
                            ArtifactRevisionId::new("graph-edge-r2").unwrap(),
                            SourceVersionId::new("graph-edge-v1").unwrap(),
                            SourceRevisionId::new("graph-edge-source-r1").unwrap(),
                            SourceLocator::whole_body(),
                            CanonicalHash::sha256(digest(edge_source)).unwrap(),
                        ),
                    })])
                    .unwrap();
            }
            "validity" => {
                let raw = rusqlite::Connection::open(&path).unwrap();
                raw.execute(
                    "UPDATE canonical_edges SET t_invalid=1000 WHERE logical_id='graph-edge'",
                    [],
                )
                .unwrap();
            }
            "endpoint" => {
                let raw = rusqlite::Connection::open(&path).unwrap();
                raw.execute(
                    "UPDATE canonical_attributes SET attr_value='bob' \
                     WHERE attr_name='owner' AND write_cursor=(\
                       SELECT write_cursor FROM canonical_nodes WHERE logical_id=?1\
                     )",
                    [endpoint],
                )
                .unwrap();
            }
            "closure" => insert_active_closure_barrier(&path, "graph-edge-source-r1", 2),
            _ => unreachable!(),
        }

        let equivalent = freeze_stable(&opened.engine, &context);
        assert_unavailable(
            opened
                .engine
                .resolve_evidence(&EvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: reference,
                    context: equivalent,
                })
                .unwrap_err(),
        );
    }
}

#[test]
fn all_reachable_origin_contributions_and_atomic_sidecars() {
    // Node FTS.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("origin-node-text{SQLITE_SUFFIX}"));
        let opened = Engine::open(&path).unwrap();
        let source_body = "node text source";
        opened
            .engine
            .write(&[
                canonical_named(
                    "node-text-source",
                    "node-text-source-r1",
                    "node-text-v1",
                    "node-text-source-id",
                    source_body,
                ),
                derived_named(
                    "node-text",
                    "node-text-r1",
                    "node-text-source-r1",
                    "node-text-v1",
                    "node-text-source-id",
                    source_body,
                    "nodetextneedle",
                ),
            ])
            .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let mut evidence_request = request("nodetextneedle", frozen.clone());
        evidence_request.include_explanation = true;
        let result = opened.engine.search_with_evidence(&evidence_request).unwrap();
        let index = result
            .search_result
            .results
            .iter()
            .position(|hit| hit.branch == SoftFallbackBranch::Text)
            .unwrap();
        let resolved =
            assert_contribution_matches_explanation(&opened.engine, &result, &frozen, index);
        assert_eq!(resolved.projection_origin.graph_origin, None);
    }

    // Node vector; only the entity kind is enrolled, so the source document
    // cannot consume the single vector result.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("origin-node-vector{SQLITE_SUFFIX}"));
        let opened = open_with_fixed_embedder(&path);
        let source_body = "vector source";
        opened
            .engine
            .write(&[canonical_named(
                "node-vector-source",
                "node-vector-source-r1",
                "node-vector-v1",
                "node-vector-source-id",
                source_body,
            )])
            .unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        opened
            .engine
            .write(&[derived_kind_named(
                "doc",
                "node-vector",
                "node-vector-r1",
                "node-vector-source-r1",
                "node-vector-v1",
                "node-vector-source-id",
                source_body,
                "semantic candidate without lexical overlap",
            )])
            .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let mut evidence_request = request("unrelated vector query", frozen.clone());
        evidence_request.include_explanation = true;
        let result = opened.engine.search_with_evidence(&evidence_request).unwrap();
        let index = result
            .search_result
            .results
            .iter()
            .position(|hit| hit.branch == SoftFallbackBranch::Vector)
            .unwrap_or_else(|| {
                panic!(
                    "expected vector origin; hits={:?}, explanation={:?}",
                    result.search_result.results, result.search_result.explanation
                )
            });
        let resolved =
            assert_contribution_matches_explanation(&opened.engine, &result, &frozen, index);
        assert!(resolved.retrieval_contribution.vector_rank.is_some());
    }

    // Edge FTS and edge-vector hydration share the stable `text_edge` artifact
    // arm; their contribution ranks distinguish the producing candidate arm.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("origin-edge{SQLITE_SUFFIX}"));
        let opened = open_with_fixed_embedder(&path);
        opened.engine.configure_vector_kind_for_test("edge_fact").unwrap();
        seed_graph_matrix(&opened.engine);
        let context = ReadContextV1::new(
            ReadView { valid_as_of: Some(1_000), ..ReadView::default() },
            SearchFilter::default(),
        )
        .unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let mut fts_request = request("graphmatrixneedle", frozen.clone());
        fts_request.include_explanation = true;
        let fts = opened.engine.search_with_evidence(&fts_request).unwrap();
        let fts_index = fts
            .search_result
            .results
            .iter()
            .position(|hit| hit.branch == SoftFallbackBranch::TextEdge)
            .unwrap();
        let fts_resolved =
            assert_contribution_matches_explanation(&opened.engine, &fts, &frozen, fts_index);
        assert!(fts_resolved.retrieval_contribution.text_rank.is_some());

        let vector_context = freeze_stable(&opened.engine, &context);
        opened.engine.set_vector_stage_only_for_test(true);
        let mut vector_request = request("no lexical edge overlap", vector_context.clone());
        vector_request.include_explanation = true;
        let vector = opened.engine.search_with_evidence(&vector_request).unwrap();
        opened.engine.set_vector_stage_only_for_test(false);
        let vector_index = vector
            .search_result
            .results
            .iter()
            .position(|hit| hit.branch == SoftFallbackBranch::TextEdge)
            .unwrap();
        let vector_resolved = assert_contribution_matches_explanation(
            &opened.engine,
            &vector,
            &vector_context,
            vector_index,
        );
        assert!(vector_resolved.retrieval_contribution.vector_rank.is_some());
        assert_eq!(vector_resolved.retrieval_contribution.text_rank, None);
    }

    // Edge seed and multi-hop traversal origins.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("origin-graph-edge{SQLITE_SUFFIX}"));
        let opened = open_with_fixed_embedder(&path);
        seed_graph_matrix(&opened.engine);
        let context = graph_matrix_context();
        let frozen = freeze_stable(&opened.engine, &context);
        let mut evidence_request = request("graphmatrixneedle", frozen.clone());
        evidence_request.use_graph_arm = true;
        evidence_request.include_explanation = true;
        let result = opened.engine.search_with_evidence(&evidence_request).unwrap();
        let index = result
            .search_result
            .results
            .iter()
            .position(|hit| hit.branch == SoftFallbackBranch::GraphArm)
            .unwrap();
        let resolved =
            assert_contribution_matches_explanation(&opened.engine, &result, &frozen, index);
        assert!(matches!(
            resolved.projection_origin.graph_origin,
            Some(EvidenceGraphOriginV1::EdgeSeed { .. })
        ));
    }

    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("origin-graph-traversal{SQLITE_SUFFIX}"));
        let opened = open_with_fixed_embedder(&path);
        let source_body = "traversal source";
        opened
            .engine
            .write(&[
                canonical_named(
                    "traversal-source",
                    "traversal-source-r1",
                    "traversal-v1",
                    "traversal-source-id",
                    source_body,
                ),
                derived_named(
                    "traversal-a",
                    "traversal-a-r1",
                    "traversal-source-r1",
                    "traversal-v1",
                    "traversal-source-id",
                    source_body,
                    "traversalseedneedle",
                ),
                derived_named(
                    "traversal-b",
                    "traversal-b-r1",
                    "traversal-source-r1",
                    "traversal-v1",
                    "traversal-source-id",
                    source_body,
                    "middle endpoint",
                ),
                derived_named(
                    "traversal-c",
                    "traversal-c-r1",
                    "traversal-source-r1",
                    "traversal-v1",
                    "traversal-source-id",
                    source_body,
                    "far endpoint",
                ),
                PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                    kind: "related".into(),
                    from: "traversal-a".into(),
                    to: "traversal-b".into(),
                    source_id: SourceId::new("traversal-source-id").unwrap(),
                    logical_id: Some("traversal-edge-1".into()),
                    body: Some("first relation".into()),
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("traversal-edge-r1").unwrap(),
                        SourceVersionId::new("traversal-v1").unwrap(),
                        SourceRevisionId::new("traversal-source-r1").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(digest(source_body)).unwrap(),
                    ),
                }),
                PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                    kind: "related".into(),
                    from: "traversal-b".into(),
                    to: "traversal-c".into(),
                    source_id: SourceId::new("traversal-source-id").unwrap(),
                    logical_id: Some("traversal-edge-2".into()),
                    body: Some("second relation".into()),
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("traversal-edge-r2").unwrap(),
                        SourceVersionId::new("traversal-v1").unwrap(),
                        SourceRevisionId::new("traversal-source-r1").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(digest(source_body)).unwrap(),
                    ),
                }),
            ])
            .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = freeze_stable(&opened.engine, &context);
        let mut evidence_request = request("traversalseedneedle", frozen.clone());
        evidence_request.use_graph_arm = true;
        evidence_request.include_explanation = true;
        let result = opened.engine.search_with_evidence(&evidence_request).unwrap();
        assert!(result.search_result.results.iter().any(|hit| {
            hit.id.value == "traversal-a" && hit.branch != SoftFallbackBranch::GraphArm
        }));
        assert!(!result.search_result.results.iter().any(|hit| {
            hit.id.value == "traversal-a" && hit.branch == SoftFallbackBranch::GraphArm
        }));
        let index = result
            .search_result
            .results
            .iter()
            .position(|hit| {
                hit.branch == SoftFallbackBranch::GraphArm && hit.id.value == "traversal-c"
            })
            .unwrap();
        let resolved =
            assert_contribution_matches_explanation(&opened.engine, &result, &frozen, index);
        assert!(matches!(
            resolved.projection_origin.graph_origin,
            Some(EvidenceGraphOriginV1::Traversal { hop_count: 2, .. })
        ));
    }

    // One unrepresentable hit rejects the whole opt-in operation; the Result
    // type exposes no partial sidecar or hit prefix.
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("origin-atomic{SQLITE_SUFFIX}"));
        let opened = Engine::open(&path).unwrap();
        for suffix in ["a", "b"] {
            let source_body = format!("atomic source {suffix}");
            opened
                .engine
                .write(&[
                    canonical_named(
                        &format!("atomic-source-{suffix}"),
                        &format!("atomic-source-{suffix}-r1"),
                        &format!("atomic-{suffix}-v1"),
                        &format!("atomic-source-{suffix}-id"),
                        &source_body,
                    ),
                    derived_named(
                        &format!("atomic-claim-{suffix}"),
                        &format!("atomic-claim-{suffix}-r1"),
                        &format!("atomic-source-{suffix}-r1"),
                        &format!("atomic-{suffix}-v1"),
                        &format!("atomic-source-{suffix}-id"),
                        &source_body,
                        &format!("atomicsidecarneedle {suffix}"),
                    ),
                ])
                .unwrap();
        }
        opened.engine.drain(30_000).unwrap();
        let raw = rusqlite::Connection::open(&path).unwrap();
        raw.execute(
            "UPDATE _fathomdb_artifact_revisions SET completeness='migrated_incomplete' \
             WHERE revision_id='atomic-claim-b-r1'",
            [],
        )
        .unwrap();
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = opened.engine.freeze_read_context(&context).unwrap();
        let error = opened
            .engine
            .search_with_evidence(&request("atomicsidecarneedle", frozen))
            .unwrap_err();
        assert!(matches!(
            error,
            EngineError::Evidence(ref evidence)
                if evidence.reason == EvidenceErrorReasonV1::EvidenceIncomplete
                    && evidence.field_path.starts_with("/results/")
        ));
    }
}
