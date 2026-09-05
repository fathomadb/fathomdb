//! 0.8.25 Slice 50 — compact, eligibility-bound source evidence.

use std::collections::BTreeSet;

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, EvidenceArtifactLifecycleV1,
    EvidenceErrorReasonV1, EvidenceGraphOriginV1, EvidenceResolveRequestV1,
    EvidenceSearchRequestV1, InitialState, LifecycleState, PreparedWrite, ProjectionRole,
    ProjectionSpec, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SoftFallbackBranch, SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

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
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "entity".into(),
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

#[test]
fn resolves_exact_utf8_source_span_and_associates_sidecar_by_position() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("evidence{SQLITE_SUFFIX}"));
    let source_body = "AéB canonical source";
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[canonical(source_body), derived(source_body)]).unwrap();

    let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
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
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let result = opened.engine.search_with_evidence(&request("needle", frozen.clone())).unwrap();
    let reference = &result.evidence[0].evidence_ref;

    let mut tampered_text = reference.as_str().to_string();
    let last = tampered_text.pop().unwrap();
    tampered_text.push(if last == '0' { '1' } else { '0' });
    let tampered = fathomdb_engine::EvidenceRefV1::new(tampered_text).unwrap();

    let mut different_filter = SearchFilter::default();
    different_filter.kind = Some("claim".into());
    let different = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), different_filter).unwrap())
        .unwrap();

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
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();

    let result = opened.engine.search_with_evidence(&request("needle", frozen)).unwrap();
    let token = result.evidence[0].evidence_ref.as_str();
    let payload_hex = token.split('.').nth(1).unwrap();
    let payload = (0..payload_hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&payload_hex[index..index + 2], 16).unwrap())
        .collect::<Vec<_>>();

    for secret in [
        "source-low-entropy",
        "source-revision-low-entropy",
        "derived-revision-low-entropy",
        source_body,
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
    let opened = Engine::open(&path).unwrap();
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
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
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
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter).unwrap())
        .unwrap();
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
    let mut view = ReadView::default();
    view.valid_as_of = Some(1_700_000_000);
    let context = ReadContextV1::new(view, SearchFilter::default()).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
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
    let equivalent = reopened.engine.freeze_read_context(&context).unwrap();
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
    let mut view = ReadView::default();
    view.valid_as_of = Some(1_700_000_000);
    let context = ReadContextV1::new(view, SearchFilter::default()).unwrap();
    let first = opened.engine.freeze_read_context(&context).unwrap();
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
    let current = opened.engine.freeze_read_context(&context).unwrap();
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
    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
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
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
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
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
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
    let hidden_context = reopened.engine.freeze_read_context(&context).unwrap();
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
    let visible_context = reopened.engine.freeze_read_context(&context).unwrap();
    let visible = reopened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: evidence,
            context: visible_context,
        })
        .unwrap_err();
    assert!(matches!(
        visible,
        EngineError::Evidence(ref error)
            if error.reason == EvidenceErrorReasonV1::EvidenceIncomplete
                && error.field_path == "/provenance"
    ));
}

#[test]
fn evidence_search_collapses_frozen_context_failure_to_nondisclosure() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("search-context-error{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let mut frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
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
fn unsupported_resolve_schema_precedes_context_authentication() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("resolve-schema-precedence{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let mut frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
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
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
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
    let equivalent = reopened.engine.freeze_read_context(&context).unwrap();
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
