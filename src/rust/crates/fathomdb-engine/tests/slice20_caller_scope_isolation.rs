//! 0.8.32 Slice 20 (`WP-20C`) — Caller-scope isolation TDD suite.
//!
//! Covers fail-closed `ScopePolicy` open validation, `ScopePredicate` routing
//! across every engine read/search/graph/frozen/evidence/dependency/write/
//! actuation/erasure/lifecycle/status path, D-01 nondisclosure invariants,
//! multi-read-scope BFS non-jumping & 8-col `vec0` K-merge, `AsOf` tombstone
//! isolation, grant revocation, and `EXPLAIN QUERY PLAN` `SEARCH`-only guards.

use std::collections::BTreeSet;
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ActuationRefusalReasonV1,
    ArtifactRevisionId, Bm25fQueryPlan, CanonicalHash, DependencyDerivedLookupV1,
    DependencySourceLookupV1, DependencyTraceDirectionV1, DependencyTraceRequestV1, EmbedderChoice,
    Engine, EngineConfig, EngineConfigurationError, EngineError, EngineOpenError,
    EvidenceErrorReasonV1, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    FrozenReadErrorReason, GraphArmOptions, GraphEvidenceResolveRequestV1, GraphExpandRequestV1,
    GraphExpansionDegradationCodeV1, GraphProjectionReadinessV1, GraphReadContextV1, GraphSeedMode,
    GraphSeedV1, IdSpace, InitialState, LifecycleActuationV1, LifecycleState,
    MutationProjectionStatusRequestV1, PageRequestV1, PhysicalErasurePolicy,
    PhysicalErasureSelector, PreparedWrite, ProjectionFts, ProjectionGenerationErrorReason,
    ProjectionReadinessV1, ProjectionRole, ProjectionSpec, ProjectionVector, PropertyEntityClass,
    PropertyMap, PropertyValue, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView,
    ScopePolicy, ScopePolicyError, SearchFilter, SoftFallback, SoftFallbackBranch,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    StructuralDegradationCodeV1, StructuralInclusionStateV1, TestEmbedderChoice,
    TraversalDirection, WriteProvenanceV1, MAX_READ_SCOPES, MAX_SCOPE_TOKEN_BYTES,
    MEAN_VEC_PIN_THRESHOLD,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn open_scoped(path: &std::path::Path, policy: ScopePolicy) -> Engine {
    Engine::open_with_choice_and_config(
        path,
        EmbedderChoice::None,
        EngineConfig::default().with_scope_policy(policy),
    )
    .expect("open scoped engine")
    .engine
}

fn open_scoped_with_config(
    path: &std::path::Path,
    policy: ScopePolicy,
    erasure_policy: Option<PhysicalErasurePolicy>,
) -> Engine {
    Engine::open_with_choice_and_config(
        path,
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: erasure_policy,
            scope_policy: policy,
            ..EngineConfig::default()
        },
    )
    .unwrap()
    .engine
}

fn open_scoped_with_embedder(
    path: &std::path::Path,
    policy: ScopePolicy,
    embedder: Arc<dyn Embedder>,
) -> Engine {
    Engine::open_with_test_choice_and_config_for_test(
        path,
        TestEmbedderChoice::Caller(embedder),
        EngineConfig::default().with_scope_policy(policy),
    )
    .expect("open scoped engine with embedder")
    .engine
}

fn body_vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "memory".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None, kinds: None }),
        source: None,
        entity_class: PropertyEntityClass::Node,
    }
}

struct MeanCenteredTestEmbedder;

impl Embedder for MeanCenteredTestEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new(
            "fathomdb-bge-small-en-v1.5",
            "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
            384,
        )
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vec = vec![0.0_f32; 384];
        vec[0] = 0.8;
        vec[1] = if text.contains("act_item") { 0.6 } else { 0.2 };
        vec[2] = ((text.len() % 7) as f32) * 0.05;
        Ok(vec)
    }
}

#[derive(Debug)]
struct DeterministicEmbedder;

impl Embedder for DeterministicEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice20-scope-test", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let vec = if text.contains("vec_a_near") || text.contains("vec_query") {
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        } else if text.contains("vec_b_mid") {
            vec![0.8, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        } else if text.contains("vec_c_exact") {
            vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        } else {
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0]
        };
        Ok(vec)
    }
}

fn plain_node(
    kind: &str,
    body: &str,
    source_id: &str,
    logical_id: Option<&str>,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: logical_id.map(str::to_string),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties,
    }
}

fn canonical_node(
    kind: &str,
    body: &str,
    source_id: &str,
    logical_id: &str,
    rev_id: &str,
    src_ver_id: &str,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(rev_id).unwrap(),
            SourceVersionId::new(src_ver_id).unwrap(),
        ),
        properties,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_node(
    kind: &str,
    body: &str,
    source_id: &str,
    logical_id: &str,
    rev_id: &str,
    src_ver_id: &str,
    src_rev_id: &str,
    source_body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev_id).unwrap(),
            SourceVersionId::new(src_ver_id).unwrap(),
            SourceRevisionId::new(src_rev_id).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body.as_bytes())).unwrap(),
        ),
        properties: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_edge(
    kind: &str,
    from: &str,
    to: &str,
    body: Option<&str>,
    source_id: &str,
    logical_id: &str,
    rev_id: &str,
    src_ver_id: &str,
    src_rev_id: &str,
    source_body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical_id.to_string()),
        body: body.map(str::to_string),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev_id).unwrap(),
            SourceVersionId::new(src_ver_id).unwrap(),
            SourceRevisionId::new(src_rev_id).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(source_body.as_bytes())).unwrap(),
        ),
    })
}

fn plain_edge(
    from: &str,
    to: &str,
    kind: &str,
    body: &str,
    logical_id: &str,
    source_id: &str,
) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        body: Some(body.to_string()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical_id.to_string()),
        temporal_fallback: None,
        properties: None,
    }
}

fn strict_frozen(engine: &Engine) -> fathomdb_engine::FrozenReadContextV1 {
    engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap()
}

// =============================================================================
// 1. Open-time ScopePolicy validation, canonicalization, config immutability
// =============================================================================

#[test]
fn scope_policy_validation_rejects_invalid_configs_before_creating_database_file() {
    let dir = TempDir::new().unwrap();
    let long_token = "a".repeat(MAX_SCOPE_TOKEN_BYTES + 1);
    let too_many: Vec<String> = (0..=MAX_READ_SCOPES).map(|i| format!("scope-{i:02}")).collect();

    let invalid_cases = vec![
        (
            "empty_reads",
            ScopePolicy::read_only(Vec::<String>::new()),
            ScopePolicyError::EmptyReadScopes,
        ),
        (
            "too_many_reads",
            ScopePolicy::read_only(too_many),
            ScopePolicyError::TooManyReadScopes {
                count: MAX_READ_SCOPES + 1,
                max: MAX_READ_SCOPES,
            },
        ),
        (
            "empty_read_token",
            ScopePolicy::single(""),
            ScopePolicyError::EmptyToken { is_write: false },
        ),
        (
            "wildcard_read",
            ScopePolicy::read_only(["*"]),
            ScopePolicyError::WildcardForbidden { is_write: false },
        ),
        (
            "wildcard_write",
            ScopePolicy::read_write(["agent-a", "agent-b"], "*"),
            ScopePolicyError::WildcardForbidden { is_write: true },
        ),
        (
            "long_token",
            ScopePolicy::single(long_token),
            ScopePolicyError::TokenTooLong {
                is_write: false,
                len: MAX_SCOPE_TOKEN_BYTES + 1,
                max: MAX_SCOPE_TOKEN_BYTES,
            },
        ),
        (
            "nul_byte",
            ScopePolicy::read_only(["bad\x00tok"]),
            ScopePolicyError::ControlCharacter { is_write: false },
        ),
        (
            "ctrl_newline",
            ScopePolicy::read_only(["bad\ntok"]),
            ScopePolicyError::ControlCharacter { is_write: false },
        ),
        (
            "del_byte",
            ScopePolicy::read_only(["bad\x7ftok"]),
            ScopePolicyError::ControlCharacter { is_write: false },
        ),
        (
            "record_sep_0x1e",
            ScopePolicy::read_only(["bad\x1etok"]),
            ScopePolicyError::DelimiterForbidden { is_write: false },
        ),
        (
            "leading_space",
            ScopePolicy::read_only([" padded"]),
            ScopePolicyError::LeadingOrTrailingWhitespace { is_write: false },
        ),
        (
            "trailing_space",
            ScopePolicy::read_only(["padded "]),
            ScopePolicyError::LeadingOrTrailingWhitespace { is_write: false },
        ),
        (
            "write_not_in_reads",
            ScopePolicy::read_write(["agent-a"], "agent-b"),
            ScopePolicyError::WriteScopeNotInReadScopes,
        ),
    ];

    for (name, policy, expected_reason) in invalid_cases {
        let path = db_path(&dir, name);
        let err = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig::default().with_scope_policy(policy),
        )
        .unwrap_err();
        assert_eq!(
            err,
            EngineOpenError::EngineConfiguration(EngineConfigurationError::ScopePolicy {
                reason: expected_reason,
            }),
            "case {name}"
        );
        assert!(
            !path.exists(),
            "case {name}: invalid scope_policy must fail BEFORE creating DB file {path:?}"
        );
    }
}

#[test]
fn scope_policy_canonicalizes_for_engine_getter_while_keeping_engine_config_unmutated() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "canonicalize");
    let requested = EngineConfig::default()
        .with_scope_policy(ScopePolicy::read_write(["agent-b", "agent-a", "agent-b"], "agent-a"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, requested.clone())
            .expect("open");
    assert_eq!(opened.engine.config(), &requested);
    assert_eq!(
        opened.engine.scope_policy(),
        &ScopePolicy::Scoped {
            read_scopes: vec!["agent-a".to_string(), "agent-b".to_string()],
            write_scope: Some("agent-a".to_string()),
        }
    );
    opened.engine.close().unwrap();
}

// =============================================================================
// 2. Read, Operational State, & Pagination Isolation
// =============================================================================

#[test]
fn read_get_list_collection_pages_and_operational_state_are_scope_isolated() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "read_isolation");

    {
        let e = open_scoped(&path, ScopePolicy::Unscoped);
        e.write(&[
            PreparedWrite::AdminSchema {
                name: "agent_state".into(),
                kind: "latest_state".into(),
                schema_json: "{}".into(),
                retention_json: "{}".into(),
            },
            PreparedWrite::AdminSchema {
                name: "agent_log".into(),
                kind: "append_only_log".into(),
                schema_json: "{}".into(),
                retention_json: "{}".into(),
            },
        ])
        .unwrap();
        let rec = e
            .write(&[
                plain_node("doc", "unscoped body", "src-shared", Some("shared-node"), None),
                PreparedWrite::OpStore {
                    collection: "agent_state".into(),
                    record_key: "k1".into(),
                    schema_id: None,
                    body: r#"{"scope":"unscoped"}"#.into(),
                },
                PreparedWrite::OpStore {
                    collection: "agent_log".into(),
                    record_key: "log-1".into(),
                    schema_id: None,
                    body: r#"{"scope":"unscoped"}"#.into(),
                },
            ])
            .unwrap();
        e.write_node_importance(rec.row_cursors[0], 0.11).unwrap();
        e.close().unwrap();
    }

    let cursor_a = {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        assert!(a.read_operational_collection_info("agent_state").unwrap().is_some());
        let receipt = a
            .write(&[
                plain_node("doc", "agent-a body", "src-shared", Some("shared-node"), None),
                PreparedWrite::OpStore {
                    collection: "agent_state".into(),
                    record_key: "k1".into(),
                    schema_id: None,
                    body: r#"{"scope":"a"}"#.into(),
                },
                PreparedWrite::OpStore {
                    collection: "agent_log".into(),
                    record_key: "log-1".into(),
                    schema_id: None,
                    body: r#"{"scope":"a"}"#.into(),
                },
            ])
            .unwrap();
        a.write_node_importance(receipt.row_cursors[0], 0.91).unwrap();
        a.close().unwrap();
        receipt.row_cursors[0]
    };

    let cursor_b = {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        let receipt = b
            .write(&[
                plain_node("doc", "agent-b body", "src-shared", Some("shared-node"), None),
                PreparedWrite::OpStore {
                    collection: "agent_state".into(),
                    record_key: "k1".into(),
                    schema_id: None,
                    body: r#"{"scope":"b"}"#.into(),
                },
                PreparedWrite::OpStore {
                    collection: "agent_log".into(),
                    record_key: "log-1".into(),
                    schema_id: None,
                    body: r#"{"scope":"b"}"#.into(),
                },
            ])
            .unwrap();
        b.write_node_importance(receipt.row_cursors[0], 0.55).unwrap();
        b.close().unwrap();
        receipt.row_cursors[0]
    };

    let a = open_scoped(&path, ScopePolicy::single("agent-a"));
    let got_a = a.read_get("shared-node", &ReadView::default()).unwrap().unwrap();
    assert_eq!(got_a.body, "agent-a body");
    let many_a =
        a.read_get_many(&["shared-node".into(), "missing".into()], &ReadView::default()).unwrap();
    assert_eq!(many_a[0].as_ref().unwrap().body, "agent-a body");
    assert!(many_a[1].is_none());

    let list_a = a.read_list("doc", &[], 20, &ReadView::default()).unwrap();
    assert_eq!(list_a.len(), 1);
    assert_eq!(list_a[0].body, "agent-a body");

    let coll_a = a.read_collection("agent_log", None, 20).unwrap();
    assert_eq!(coll_a.len(), 1);
    assert_eq!(coll_a[0].payload, r#"{"scope":"a"}"#);

    let frozen_a = strict_frozen(&a);
    let page_a = a
        .read_canonical_page(
            "doc",
            &frozen_a,
            &PageRequestV1 { schema_version: 1, limit: 20, cursor: None },
        )
        .unwrap();
    assert_eq!(page_a.items.len(), 1);
    assert_eq!(page_a.items[0].body, "agent-a body");

    let op_state_a =
        a.read_operational_state("agent_state", "k1", Some(&frozen_a)).unwrap().unwrap();
    assert_eq!(op_state_a.payload, r#"{"scope":"a"}"#);
    let op_page_a = a
        .read_operational_state_page(
            "agent_state",
            &frozen_a,
            &PageRequestV1 { schema_version: 1, limit: 20, cursor: None },
        )
        .unwrap();
    assert_eq!(op_page_a.items.len(), 1);
    assert_eq!(op_page_a.items[0].payload, r#"{"scope":"a"}"#);

    assert_eq!(a.node_importance(cursor_a).unwrap(), Some(0.91));
    assert_eq!(a.node_importance(cursor_b).unwrap(), None);
    a.close().unwrap();

    let u = open_scoped(&path, ScopePolicy::Unscoped);
    assert_eq!(
        u.read_get("shared-node", &ReadView::default()).unwrap().unwrap().body,
        "unscoped body"
    );
    assert_eq!(u.read_list("doc", &[], 20, &ReadView::default()).unwrap().len(), 1);
    u.close().unwrap();

    let ab = open_scoped(&path, ScopePolicy::read_only(["agent-a", "agent-b"]));
    assert_eq!(ab.read_list("doc", &[], 20, &ReadView::default()).unwrap().len(), 2);
    ab.close().unwrap();
}

#[test]
fn crossed_boundary_since_is_scope_isolated() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "boundary");

    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        a.write(&[PreparedWrite::Node {
            kind: "event".into(),
            body: "a event".into(),
            source_id: SourceId::new("src").unwrap(),
            logical_id: Some("evt".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: Some(100),
            valid_until: Some(500),
            properties: None,
        }])
        .unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[PreparedWrite::Node {
            kind: "event".into(),
            body: "b event".into(),
            source_id: SourceId::new("src").unwrap(),
            logical_id: Some("evt".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: Some(100),
            valid_until: Some(200),
            properties: None,
        }])
        .unwrap();
        b.close().unwrap();
    }

    let a = open_scoped(&path, ScopePolicy::single("agent-a"));
    let view = ReadView { valid_as_of: Some(250), ..ReadView::default() };
    assert!(a.crossed_boundary_since(150, &view).unwrap().is_empty());
    a.close().unwrap();

    let b = open_scoped(&path, ScopePolicy::single("agent-b"));
    assert_eq!(b.crossed_boundary_since(150, &view).unwrap().len(), 1);
    b.close().unwrap();
}

// =============================================================================
// 3. Search Isolation: FTS, 8-col vec0 + multi-scope K-merge, ownerless,
//    edge FTS, BM25F, projected text (+ historical BM25 non-leak), graph arm
// =============================================================================

#[test]
fn vec0_and_text_search_isolate_scopes_and_merge_topk_across_multi_read_scopes() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "vec0_isolation");
    let emb: Arc<dyn Embedder> = Arc::new(DeterministicEmbedder);

    {
        let u = open_scoped_with_embedder(&path, ScopePolicy::Unscoped, Arc::clone(&emb));
        u.configure_projections(&[body_vector_spec()], &[]).unwrap();
        u.write(&[plain_node(
            "doc",
            "vec_query unscoped sharedtok",
            "src-u",
            Some("shared"),
            None,
        )])
        .unwrap();
        u.drain(10_000).unwrap();
        u.close().unwrap();
    }
    {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        a.write(&[plain_node(
            "doc",
            "vec_a_near sharedtok agent_a_alpha",
            "src-a",
            Some("shared"),
            None,
        )])
        .unwrap();
        a.drain(10_000).unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped_with_embedder(&path, ScopePolicy::single("agent-b"), Arc::clone(&emb));
        b.write(&[plain_node(
            "doc",
            "vec_b_mid sharedtok agent_b_beta",
            "src-b",
            Some("shared"),
            None,
        )])
        .unwrap();
        b.drain(10_000).unwrap();
        b.close().unwrap();
    }
    {
        let c = open_scoped_with_embedder(&path, ScopePolicy::single("agent-c"), Arc::clone(&emb));
        c.write(&[plain_node(
            "doc",
            "vec_c_exact sharedtok agent_c_gamma",
            "src-c",
            Some("shared"),
            None,
        )])
        .unwrap();
        c.drain(10_000).unwrap();
        c.close().unwrap();
    }

    {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        let res = a.search("vec_query sharedtok").unwrap();
        assert_eq!(res.results.len(), 1);
        assert!(res.results[0].body.contains("agent_a_alpha"));
        a.close().unwrap();
    }

    {
        let ab = open_scoped_with_embedder(
            &path,
            ScopePolicy::read_only(["agent-a", "agent-b"]),
            Arc::clone(&emb),
        );
        let res = ab.search("vec_query sharedtok").unwrap();
        assert_eq!(
            res.results.len(),
            2,
            "must return both agent-a & agent-b, never c or unscoped: {:?}",
            res.results
        );
        assert!(res.results[0].body.contains("agent_a_alpha"));
        assert!(res.results[1].body.contains("agent_b_beta"));
        ab.close().unwrap();
    }

    {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        a.write(&[plain_node(
            "doc",
            "defertextident commonterm onlyina",
            "src-a",
            Some("defer-1"),
            None,
        )])
        .unwrap();
        a.drain(10_000).unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped_with_embedder(&path, ScopePolicy::single("agent-b"), Arc::clone(&emb));
        b.write(&[plain_node(
            "doc",
            "defertextident commonterm onlyinb",
            "src-b",
            Some("defer-1"),
            None,
        )])
        .unwrap();
        b.drain(10_000).unwrap();
        b.close().unwrap();
    }
    {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        let text_res = a.search_text_only("defertextident").unwrap();
        assert_eq!(text_res.results.len(), 1);
        assert!(text_res.results[0].body.contains("onlyina"));
        a.close().unwrap();
    }
}

#[test]
fn bm25f_projected_text_historical_idf_and_ownerless_rows_do_not_leak() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "text_surfaces");

    let prop_spec = ProjectionSpec {
        name: "summary".into(),
        roles: BTreeSet::from([ProjectionRole::Searchable, ProjectionRole::Filterable]),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: None,
        source: None,
        entity_class: PropertyEntityClass::Node,
    };

    let mut props_a = PropertyMap::new();
    props_a.insert("summary".into(), PropertyValue::String("histterm alpha_only".into()));

    let mut props_b = PropertyMap::new();
    props_b.insert("summary".into(), PropertyValue::String("histterm beta_only".into()));

    {
        let u = open_scoped(&path, ScopePolicy::Unscoped);
        u.configure_projections(&[prop_spec], &[]).unwrap();
        let mut p_u = PropertyMap::new();
        p_u.insert("summary".into(), PropertyValue::String("histterm unscoped_only".into()));
        u.write(&[plain_node("doc", "bm25tok unscoped", "src-u", Some("u1"), Some(p_u))]).unwrap();
        u.close().unwrap();
    }
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute("UPDATE canonical_nodes SET source_id = NULL WHERE logical_id = 'u1'", [])
            .unwrap();
    }

    let cursor_a = {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        let rec = a
            .write(&[plain_node("doc", "bm25tok alpha", "src-a", Some("shared"), Some(props_a))])
            .unwrap();
        a.close().unwrap();
        rec.row_cursors[0]
    };
    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[plain_node("doc", "bm25tok beta", "src-b", Some("shared"), Some(props_b))])
            .unwrap();
        for idx in 0..10 {
            let mut hp = PropertyMap::new();
            hp.insert("summary".into(), PropertyValue::String(format!("histterm flood_{idx}")));
            b.write(&[plain_node(
                "doc",
                "flood",
                "src-b",
                Some(&format!("b-hist-{idx}")),
                Some(hp),
            )])
            .unwrap();
        }
        b.close().unwrap();
    }

    let u = open_scoped(&path, ScopePolicy::Unscoped);
    assert_eq!(u.search_text_only("bm25tok").unwrap().results.len(), 1);
    u.close().unwrap();

    let a = open_scoped(&path, ScopePolicy::single("agent-a"));
    let bm25f_a = a.bm25f_search("bm25tok", &Bm25fQueryPlan::default()).unwrap();
    assert_eq!(bm25f_a.len(), 1);
    assert_eq!(bm25f_a[0].0, cursor_a);

    let proj_cur =
        a.search_projected_text("histterm", "summary", None, &ReadView::default()).unwrap();
    assert_eq!(proj_cur.results.len(), 1);
    assert!(proj_cur.results[0].body.contains("alpha"));

    let hist_view = ReadView { include_superseded: true, ..ReadView::default() };
    let proj_hist = a.search_projected_text("histterm", "summary", None, &hist_view).unwrap();
    assert_eq!(proj_hist.results.len(), 1);
    assert!(proj_hist.results[0].body.contains("alpha"));
    a.close().unwrap();
}

#[test]
fn multi_read_scope_graph_arm_and_graph_expand_never_jump_across_scopes_on_shared_logical_id() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "graph_non_jump");

    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        a.write(&[
            plain_node("doc", "anchor_alpha_needle root", "src-a", Some("root-a"), None),
            plain_node("doc", "ALPHA_MID", "src-a", Some("shared-mid"), None),
            plain_node("doc", "ALPHA_LEAF", "src-a", Some("leaf-a"), None),
            plain_edge("root-a", "shared-mid", "links", "edgetok_a", "e-rm-a", "src-a"),
            plain_edge("shared-mid", "leaf-a", "links", "edgetok_a2", "e-ml-a", "src-a"),
        ])
        .unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[
            plain_node("doc", "BETA_MID", "src-b", Some("shared-mid"), None),
            plain_node("doc", "BETA_LEAF", "src-b", Some("leaf-b"), None),
            plain_edge("shared-mid", "leaf-b", "links", "edgetok_b", "e-ml-b", "src-b"),
        ])
        .unwrap();
        b.close().unwrap();
    }

    let ab = open_scoped(&path, ScopePolicy::read_only(["agent-a", "agent-b"]));

    let neighbors = ab
        .graph_neighbors("root-a", 3, TraversalDirection::Outgoing, &ReadView::default())
        .unwrap();
    let neighbor_bodies: Vec<&str> = neighbors.iter().map(|n| n.body.as_str()).collect();
    assert!(neighbor_bodies.contains(&"ALPHA_MID"));
    assert!(neighbor_bodies.contains(&"ALPHA_LEAF"));
    assert!(
        !neighbor_bodies.iter().any(|b| b.contains("BETA")),
        "multi-scope BFS jumped across scopes: {neighbor_bodies:?}"
    );

    let se = ab.search_expand("anchor_alpha_needle", None, 3, None).unwrap();
    assert!(
        se.expanded.iter().all(|(n, _)| !n.body.contains("BETA")),
        "search_expand jumped across scopes: {:?}",
        se.expanded
    );

    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("root-a")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        3,
        50,
        1_000,
    );
    let expanded = ab.graph_expand(&req).unwrap();
    assert!(
        expanded.targets.iter().all(|t| !t.body.contains("BETA")),
        "graph_expand jumped across scopes: {:?}",
        expanded.targets
    );

    let mut graph_opts = GraphArmOptions::default();
    graph_opts.graph_seed = GraphSeedMode::Both;
    let arm_res = ab
        .search_reranked_view_with_graph_options(
            "anchor_alpha_needle",
            None,
            0,
            true,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
            None,
            &graph_opts,
        )
        .unwrap();
    assert!(arm_res.results.iter().all(|h| !h.body.contains("BETA")));

    ab.close().unwrap();
}

// =============================================================================
// 4. AsOf Tombstone Isolation, Frozen Read, Cursors, Evidence & Grant Revocation
// =============================================================================

#[test]
fn as_of_tombstone_in_scope_b_does_not_suppress_scope_a_and_search_expand_uses_read_view() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "as_of_tombstone");

    let t_hist: i64 = 1_000_000;
    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        a.write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "asof_marker alpha_hist".into(),
                source_id: SourceId::new("src-a").unwrap(),
                logical_id: Some("shared-hist".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(500_000),
                valid_until: Some(2_000_000),
                properties: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "asof_leaf alpha_leaf".into(),
                source_id: SourceId::new("src-a").unwrap(),
                logical_id: Some("hist-leaf".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(500_000),
                valid_until: Some(2_000_000),
                properties: None,
            },
            plain_edge("shared-hist", "hist-leaf", "rel", "a hist edge", "e-ha", "src-a"),
        ])
        .unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "asof_marker beta_hist".into(),
                source_id: SourceId::new("src-b").unwrap(),
                logical_id: Some("shared-hist".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(500_000),
                valid_until: Some(2_000_000),
                properties: None,
            },
            PreparedWrite::Node {
                kind: "doc".into(),
                body: "asof_leaf beta_leaf".into(),
                source_id: SourceId::new("src-b").unwrap(),
                logical_id: Some("hist-leaf".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: Some(500_000),
                valid_until: Some(2_000_000),
                properties: None,
            },
            plain_edge("shared-hist", "hist-leaf", "rel", "b hist edge", "e-hb", "src-b"),
        ])
        .unwrap();
        b.transition("shared-hist", LifecycleState::Deleted, Some("delete b root".into())).unwrap();
        b.transition("hist-leaf", LifecycleState::Deleted, Some("delete b leaf".into())).unwrap();
        b.close().unwrap();
    }

    let ab = open_scoped(&path, ScopePolicy::read_only(["agent-a", "agent-b"]));
    let as_of_view = ReadView { valid_as_of: Some(t_hist), ..ReadView::default() };

    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("shared-hist")],
        },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(as_of_view, SearchFilter::default()).unwrap(),
        },
        1,
        50,
        1_000,
    );
    let exp = ab.graph_expand(&req).unwrap();
    assert!(
        exp.targets.iter().any(|t| t.body.contains("alpha_leaf")),
        "Scope B delete must NOT suppress Scope A AsOf traversal: {:?}",
        exp.targets
    );
    assert!(
        !exp.targets.iter().any(|t| t.body.contains("beta_leaf")),
        "Scope B deleted node leaked into AsOf: {:?}",
        exp.targets
    );

    let se = ab.search_expand("asof_marker", None, 1, Some(as_of_view)).unwrap();
    assert!(se.search_hits.iter().any(|h| h.body.contains("alpha_hist")));
    ab.close().unwrap();
}

#[test]
fn frozen_contexts_cursors_evidence_and_grant_revocation_are_authenticated_and_nondisclosing() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "frozen_evidence");

    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        a.write(&[
            canonical_node("doc", "evid_marker alpha", "src-a", "node-a", "rev-a1", "v1", None),
            derived_node(
                "fact",
                "evid_marker leaf_a",
                "src-a",
                "leaf-a",
                "rev-a2",
                "v1",
                "rev-a1",
                "evid_marker alpha",
            ),
            derived_edge(
                "supports",
                "node-a",
                "leaf-a",
                Some("edge a"),
                "src-a",
                "edge-a",
                "rev-ae",
                "v1",
                "rev-a1",
                "evid_marker alpha",
            ),
        ])
        .unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[canonical_node(
            "doc",
            "evid_marker beta",
            "src-b",
            "node-b",
            "rev-b1",
            "v1",
            None,
        )])
        .unwrap();
        b.close().unwrap();
    }

    let ab = open_scoped(&path, ScopePolicy::read_only(["agent-a", "agent-b"]));
    let frozen_ab = strict_frozen(&ab);
    let ev_search = ab
        .search_with_evidence(&EvidenceSearchRequestV1::new(
            "evid_marker".into(),
            frozen_ab.clone(),
        ))
        .unwrap();
    let ref_a = ev_search
        .evidence
        .iter()
        .find(|e| {
            e.artifact_revision_id.as_str() == "rev-a1"
                || e.artifact_revision_id.as_str() == "rev-a2"
        })
        .expect("agent-a evidence present under AB")
        .evidence_ref
        .clone();

    let ge_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("node-a")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen_ab.clone() },
        1,
        20,
        1_000,
    )
    .with_evidence(true);
    let ge_res = ab.graph_expand(&ge_req).unwrap();
    let graph_ref_a =
        ge_res.evidence.expect("graph evidence present").entries[0].target_evidence_ref.clone();
    ab.close().unwrap();

    let b = open_scoped(&path, ScopePolicy::single("agent-b"));
    let frozen_b = strict_frozen(&b);
    let auth_err =
        b.search_frozen("evid_marker", &frozen_ab, 0, false, 0.3, 0, false, 10).unwrap_err();
    assert!(matches!(
        auth_err,
        EngineError::FrozenRead(ref e) if e.reason == FrozenReadErrorReason::TokenAuthenticationFailed
    ));

    let ev_err = b
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: ref_a,
            context: frozen_b.clone(),
        })
        .unwrap_err();
    assert!(matches!(
        ev_err,
        EngineError::Evidence(ref e) if e.reason == EvidenceErrorReasonV1::EvidenceUnavailable
    ));

    let gev_err = b
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: graph_ref_a,
            context: frozen_b,
        })
        .unwrap_err();
    assert!(matches!(
        gev_err,
        EngineError::Evidence(ref e) if e.reason == EvidenceErrorReasonV1::EvidenceUnavailable
    ));
    b.close().unwrap();

    let u = open_scoped(&path, ScopePolicy::Unscoped);
    let auth_u = u
        .read_canonical_page(
            "doc",
            &frozen_ab,
            &PageRequestV1 { schema_version: 1, limit: 10, cursor: None },
        )
        .unwrap_err();
    assert!(matches!(
        auth_u,
        EngineError::FrozenRead(ref e) if e.reason == FrozenReadErrorReason::TokenAuthenticationFailed
    ));
    u.close().unwrap();
}

// =============================================================================
// 5. Dependency Registry, Dependency Trace & Actuation / Provenance / Write /
//    Lifecycle / Erasure Isolation + Read-only ScopeViolation & Nondisclosure
// =============================================================================

#[test]
fn read_only_scope_refuses_every_mutating_verb_atomically_with_scope_violation() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "read_only_guard");

    {
        let a = open_scoped_with_config(
            &path,
            ScopePolicy::single("agent-a"),
            Some(PhysicalErasurePolicy::ExplicitMaintenance),
        );
        a.write(&[canonical_node("doc", "src", "s1", "l1", "r1", "v1", None)]).unwrap();
        a.close().unwrap();
    }

    let ro = Arc::new(open_scoped_with_config(
        &path,
        ScopePolicy::read_only(["agent-a"]),
        Some(PhysicalErasurePolicy::ExplicitMaintenance),
    ));

    assert!(matches!(
        ro.write(&[plain_node("doc", "nope", "s1", Some("l2"), None)]),
        Err(EngineError::ScopeViolation { .. })
    ));
    assert!(matches!(
        ro.transition("l1", LifecycleState::Deleted, None),
        Err(EngineError::ScopeViolation { .. })
    ));
    assert!(matches!(ro.purge("l1"), Err(EngineError::ScopeViolation { .. })));
    assert!(matches!(ro.erase_source("s1"), Err(EngineError::ScopeViolation { .. })));
    assert!(matches!(
        ro.register_source_dependency(
            SourceDependencyRegistrationV1::new("d1", "r1", "r2").unwrap()
        ),
        Err(EngineError::ScopeViolation { .. })
    ));
    assert!(matches!(
        ro.actuate(
            ActuationBatchV1::new(
                "op-ro",
                vec![ActuationOperationV1::TransitionLifecycle(
                    LifecycleActuationV1::new(
                        "s1",
                        ArtifactRevisionId::new("r1").unwrap(),
                        LifecycleState::Deleted,
                        None,
                    )
                    .unwrap()
                )],
            )
            .unwrap()
        ),
        Err(EngineError::ScopeViolation { .. })
    ));
    assert!(matches!(
        ro.queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "s1".into() },
            "ddcfe624b1464d388b6ec5bfe6483c45"
        ),
        Err(EngineError::ScopeViolation { .. })
    ));
    assert!(matches!(
        ro.withdraw_queued_erasure("nonexistent-ticket"),
        Err(EngineError::ScopeViolation { .. })
    ));

    assert_eq!(ro.read_get("l1", &ReadView::default()).unwrap().unwrap().body, "src");
    Arc::try_unwrap(ro).unwrap_or_else(|_| panic!("sole")).close().unwrap();
}

#[test]
fn cross_scope_edge_endpoints_and_derived_provenance_preserve_d01_nondisclosure() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "d01_nondisclosure");

    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        a.write(&[canonical_node(
            "doc",
            "secret a body",
            "src-a",
            "target-a",
            "rev-a",
            "v1",
            None,
        )])
        .unwrap();
        a.close().unwrap();
    }

    let b = open_scoped(&path, ScopePolicy::single("agent-b"));
    b.write(&[canonical_node("doc", "b local", "src-b", "local-b", "rev-b", "v1", None)]).unwrap();

    let err_missing = b
        .write(&[plain_edge("local-b", "never-existed", "rel", "b", "e-miss", "src-b")])
        .unwrap_err();
    let err_cross =
        b.write(&[plain_edge("local-b", "target-a", "rel", "b", "e-cross", "src-b")]).unwrap_err();
    assert_eq!(err_missing, EngineError::WriteValidation);
    assert_eq!(err_cross, EngineError::WriteValidation);

    let act_miss = b
        .actuate(
            ActuationBatchV1::new(
                "act-miss",
                vec![ActuationOperationV1::PutDerivedEdge(ProvenancedEdgeV1 {
                    kind: "rel".into(),
                    from: "local-b".into(),
                    to: "never-existed".into(),
                    source_id: SourceId::new("src-b").unwrap(),
                    logical_id: Some("e-act-m".into()),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("rev-eb-m").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                        SourceRevisionId::new("rev-b").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(sha256_hex(b"b local")).unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap();
    let act_cross = b
        .actuate(
            ActuationBatchV1::new(
                "act-cross",
                vec![ActuationOperationV1::PutDerivedEdge(ProvenancedEdgeV1 {
                    kind: "rel".into(),
                    from: "local-b".into(),
                    to: "target-a".into(),
                    source_id: SourceId::new("src-b").unwrap(),
                    logical_id: Some("e-act-c".into()),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("rev-eb-c").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                        SourceRevisionId::new("rev-b").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(sha256_hex(b"b local")).unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(act_miss.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(act_miss.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);
    assert_eq!(act_cross.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(act_cross.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);

    let prov_missing = b
        .write(&[derived_node(
            "fact",
            "f",
            "src-b",
            "f1",
            "rev-f1",
            "v1",
            "nonexistent-rev",
            "secret a body",
        )])
        .unwrap_err();
    let prov_cross = b
        .write(&[derived_node(
            "fact",
            "f",
            "src-b",
            "f2",
            "rev-f2",
            "v1",
            "rev-a",
            "secret a body",
        )])
        .unwrap_err();
    assert_eq!(prov_missing, prov_cross);
    b.close().unwrap();
}

#[test]
fn dependency_registry_trace_erasure_and_status_respect_scope_boundaries() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "dep_erasure_status");

    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        a.write(&[
            canonical_node(
                "doc",
                "src body a",
                "shared-src",
                "src-node",
                "shared-src-rev",
                "v1",
                None,
            ),
            derived_node(
                "fact",
                "der body a",
                "shared-src",
                "der-node",
                "shared-der-rev",
                "v1",
                "shared-src-rev",
                "src body a",
            ),
        ])
        .unwrap();
        a.register_source_dependency(
            SourceDependencyRegistrationV1::new("shared-dep", "shared-src-rev", "shared-der-rev")
                .unwrap(),
        )
        .unwrap();
        let act_a_req = ActuationBatchV1::new(
            "shared-op",
            vec![ActuationOperationV1::TransitionLifecycle(
                LifecycleActuationV1::new(
                    "src-node",
                    ArtifactRevisionId::new("shared-src-rev").unwrap(),
                    LifecycleState::Deleted,
                    Some("a note".into()),
                )
                .unwrap(),
            )],
        )
        .unwrap();
        let rec_a = a.actuate(act_a_req.clone()).unwrap();
        assert_eq!(rec_a.outcome, ActuationOutcomeV1::CommittedClosurePending);
        // Restore active state so Scope A still exposes live records while keeping receipt `shared-op`
        // backed by `shared-src` / `shared-src-rev`.
        a.transition("src-node", LifecycleState::Active, None).unwrap();
        a.transition("der-node", LifecycleState::Active, None).unwrap();
        a.close().unwrap();
    }
    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[
            canonical_node(
                "doc",
                "src body b",
                "shared-src",
                "src-node",
                "shared-src-rev",
                "v1",
                None,
            ),
            derived_node(
                "fact",
                "der body b",
                "shared-src",
                "der-node",
                "shared-der-rev",
                "v1",
                "shared-src-rev",
                "src body b",
            ),
        ])
        .unwrap();
        b.register_source_dependency(
            SourceDependencyRegistrationV1::new("shared-dep", "shared-src-rev", "shared-der-rev")
                .unwrap(),
        )
        .unwrap();

        let frozen_b = strict_frozen(&b);
        let trace_b = b
            .trace_dependency(
                DependencyTraceRequestV1::new(
                    "shared-src-rev",
                    DependencyTraceDirectionV1::ToDependents,
                    frozen_b,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(trace_b.dependency_edges.len(), 1);
        assert_eq!(
            b.dependencies_for_source(DependencySourceLookupV1::new("shared-src-rev").unwrap())
                .unwrap()
                .items
                .len(),
            1
        );
        assert!(b
            .dependency_for_derived(DependencyDerivedLookupV1::new("shared-der-rev").unwrap())
            .unwrap()
            .is_some());

        let receipt_act = b
            .actuate(
                ActuationBatchV1::new(
                    "shared-op",
                    vec![ActuationOperationV1::TransitionLifecycle(
                        LifecycleActuationV1::new(
                            "src-node",
                            ArtifactRevisionId::new("shared-src-rev").unwrap(),
                            LifecycleState::Deleted,
                            Some("b erase".into()),
                        )
                        .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(receipt_act.outcome, ActuationOutcomeV1::CommittedClosurePending);
        b.purge("der-node").unwrap();
        b.erase_source("shared-src").unwrap();
        // Scope B's own `"shared-op"` receipt referencing `"shared-src"` MUST now be redacted,
        // while Scope A's same `"shared-op"` receipt referencing `"shared-src"` stays intact.
        let redacted_b = b
            .actuate(
                ActuationBatchV1::new(
                    "shared-op",
                    vec![ActuationOperationV1::TransitionLifecycle(
                        LifecycleActuationV1::new(
                            "src-node",
                            ArtifactRevisionId::new("shared-src-rev").unwrap(),
                            LifecycleState::Deleted,
                            Some("b erase".into()),
                        )
                        .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .unwrap_err();
        assert!(matches!(redacted_b, EngineError::Actuation(_)), "got {redacted_b:?}");
        b.close().unwrap();
    }

    {
        let a = open_scoped(&path, ScopePolicy::single("agent-a"));
        // Scope A's receipt `"shared-op"` MUST still replay cleanly!
        let replay_a = a
            .actuate(
                ActuationBatchV1::new(
                    "shared-op",
                    vec![ActuationOperationV1::TransitionLifecycle(
                        LifecycleActuationV1::new(
                            "src-node",
                            ArtifactRevisionId::new("shared-src-rev").unwrap(),
                            LifecycleState::Deleted,
                            Some("a note".into()),
                        )
                        .unwrap(),
                    )],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(replay_a.outcome, ActuationOutcomeV1::CommittedClosurePending);
        assert_eq!(
            a.read_get("src-node", &ReadView::default()).unwrap().unwrap().body,
            "src body a"
        );
        assert_eq!(
            a.read_get("der-node", &ReadView::default()).unwrap().unwrap().body,
            "der body a"
        );
        assert_eq!(
            a.dependencies_for_source(DependencySourceLookupV1::new("shared-src-rev").unwrap())
                .unwrap()
                .items
                .len(),
            1
        );
        a.close().unwrap();
    }

    // Separate ExplicitMaintenance database verifying queued physical erasure (`guard_plain`,
    // `physical_erasure_status`, `withdraw_queued_erasure`) isolation across overlapping scopes.
    let maint_path = db_path(&dir, "dep_erasure_queue");
    let ticket_a = {
        let a = open_scoped_with_config(
            &maint_path,
            ScopePolicy::single("agent-a"),
            Some(PhysicalErasurePolicy::ExplicitMaintenance),
        );
        a.write(&[canonical_node(
            "doc",
            "src body a",
            "shared-src",
            "src-node",
            "shared-src-rev",
            "v1",
            None,
        )])
        .unwrap();
        let a_arc = Arc::new(a);
        let maint = a_arc.begin_erasure_maintenance(1_000).unwrap();
        let q = maint
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "shared-src".into() },
                "11111111111111111111111111111111",
            )
            .unwrap();
        let ticket = match q {
            fathomdb_engine::QueueResult::Accepted { ticket, .. } => ticket,
            other => panic!("expected Accepted queue result, got {other:?}"),
        };
        drop(maint);
        let a_inner = Arc::try_unwrap(a_arc).unwrap_or_else(|_| panic!("sole"));
        assert!(matches!(
            a_inner.write(&[plain_node("doc", "blocked a", "shared-src", Some("a-fenced"), None)]),
            Err(EngineError::Maintenance(_))
        ));
        a_inner.close().unwrap();
        ticket
    };

    // Scope B MUST NOT be fenced by Scope A's queued physical erasure (`guard_plain`),
    // MUST NOT see Scope A's ticket in `physical_erasure_status`, and MUST NOT be able to
    // withdraw Scope A's ticket.
    let b2 = open_scoped_with_config(
        &maint_path,
        ScopePolicy::single("agent-b"),
        Some(PhysicalErasurePolicy::ExplicitMaintenance),
    );
    b2.write(&[plain_node("doc", "unblocked b", "shared-src", Some("b-free"), None)]).unwrap();
    assert_eq!(b2.physical_erasure_status(&ticket_a).unwrap(), None);
    assert_eq!(
        b2.withdraw_queued_erasure(&ticket_a).unwrap(),
        fathomdb_engine::WithdrawResult::Unknown
    );
    b2.close().unwrap();

    let a2 = open_scoped_with_config(
        &maint_path,
        ScopePolicy::single("agent-a"),
        Some(PhysicalErasurePolicy::ExplicitMaintenance),
    );
    assert!(a2.physical_erasure_status(&ticket_a).unwrap().is_some());
    assert_eq!(
        a2.withdraw_queued_erasure(&ticket_a).unwrap(),
        fathomdb_engine::WithdrawResult::WithdrawnBeforeRemoval
    );
    a2.close().unwrap();
}

#[test]
fn projection_generation_and_mean_recompute_preserve_scope() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "mean_and_status");
    let emb: Arc<dyn Embedder> = Arc::new(MeanCenteredTestEmbedder);

    let act_cursor_a = {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        a.configure_projections(&[body_vector_spec()], &[]).unwrap();
        let mut ops = Vec::with_capacity(MEAN_VEC_PIN_THRESHOLD as usize + 2);
        for i in 0..(MEAN_VEC_PIN_THRESHOLD as usize + 2) {
            ops.push(plain_node(
                "doc",
                &format!("vec_a_near item {i}"),
                "src-a",
                Some(&format!("a-{i}")),
                None,
            ));
        }
        a.write(&ops).unwrap();
        let node = match canonical_node(
            "doc",
            "vec_a_near act_item",
            "src-a",
            "a-act",
            "rev-a-act",
            "v1",
            None,
        ) {
            PreparedWrite::ProvenancedNode(n) => n,
            _ => unreachable!(),
        };
        let receipt = a
            .actuate(
                ActuationBatchV1::new(
                    "shared-op",
                    vec![ActuationOperationV1::PutCanonicalNode(node)],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
        let tracked_cursor = *receipt
            .pending_projection_write_cursors
            .first()
            .expect("canonical node queued for vector projection");
        a.drain(15_000).unwrap();
        let gen_id = a.read_projection_generation_status().unwrap().generation_id;
        let _status = a
            .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: "shared-op".into(),
                write_cursor: tracked_cursor,
                expected_generation_id: gen_id.clone(),
            })
            .unwrap();
        #[cfg(feature = "operator")]
        {
            a.recompute_mean().unwrap();
        }
        a.close().unwrap();
        tracked_cursor
    };

    {
        let b = open_scoped_with_embedder(&path, ScopePolicy::single("agent-b"), Arc::clone(&emb));
        let gen_id = b.read_projection_generation_status().unwrap().generation_id;
        let err = b
            .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: "shared-op".into(),
                write_cursor: act_cursor_a,
                expected_generation_id: gen_id,
            })
            .unwrap_err();
        assert!(matches!(err, EngineError::ProjectionGeneration(_)), "got {err:?}");
        b.close().unwrap();
    }

    let conn = Connection::open(&path).unwrap();
    let non_a: i64 = conn
        .query_row("SELECT COUNT(*) FROM vector_default WHERE scope != 'agent-a'", [], |r| r.get(0))
        .unwrap();
    let a_cnt: i64 = conn
        .query_row("SELECT COUNT(*) FROM vector_default WHERE scope = 'agent-a'", [], |r| r.get(0))
        .unwrap();
    assert_eq!(non_a, 0);
    assert!(a_cnt >= MEAN_VEC_PIN_THRESHOLD as i64);
}

#[test]
fn explain_query_plan_uses_search_only_for_unscoped_and_scoped_queries() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "eqp");
    let emb: Arc<dyn Embedder> = Arc::new(DeterministicEmbedder);
    {
        let u = open_scoped_with_embedder(&path, ScopePolicy::Unscoped, Arc::clone(&emb));
        u.configure_projections(&[body_vector_spec()], &[]).unwrap();
        u.write(&[plain_node("doc", "vec_a_near eqp node", "s1", Some("n1"), None)]).unwrap();
        u.drain(10_000).unwrap();
        u.close().unwrap();
    }

    for label in ["unscoped", "scoped"] {
        let handle = if label == "unscoped" {
            open_scoped_with_embedder(&path, ScopePolicy::Unscoped, Arc::clone(&emb))
        } else {
            open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb))
        };
        let vec_plan = handle.explain_vector_phase1_for_test(None).unwrap();
        assert!(
            !vec_plan
                .iter()
                .any(|p| p.contains("SCAN canonical_nodes") || p.contains("SCAN canonical_edges")),
            "{label} vector phase1 must never SCAN canonical tables: {vec_plan:?}"
        );
        assert!(
            vec_plan.iter().any(|p| p.contains("VIRTUAL TABLE INDEX")),
            "{label} vector phase1 must use vec0 virtual index: {vec_plan:?}"
        );
        let ge_plan = handle.explain_graph_expand_for_test(TraversalDirection::Outgoing).unwrap();
        assert!(
            ge_plan.iter().all(|p| !p.starts_with("SCAN ")),
            "{label} graph_expand must not SCAN: {ge_plan:?}"
        );
        handle.close().unwrap();
    }
}

#[test]
fn historical_dense_incomplete_does_not_leak_unembedded_scope_b_history_into_scope_a_search() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "hist_dense_scope");
    let emb: Arc<dyn Embedder> = Arc::new(DeterministicEmbedder);

    {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        a.configure_projections(&[body_vector_spec()], &[]).unwrap();
        a.write(&[
            plain_node("doc", "vec_a_near alpha_v1", "src-a", Some("a-hist"), None),
            plain_node("doc", "vec_a_near alpha_del", "src-a", Some("a-del"), None),
            plain_node("doc", "vec_a_near alpha_end", "src-a", Some("a-end"), None),
        ])
        .unwrap();
        a.write(&[plain_node("doc", "vec_a_near alpha_v2", "src-a", Some("a-hist"), None)])
            .unwrap();
        a.write(&[plain_edge(
            "a-hist",
            "a-end",
            "rel",
            "vec_a_near alpha_edge_v1",
            "e-a",
            "src-a",
        )])
        .unwrap();
        a.write(&[plain_edge(
            "a-hist",
            "a-end",
            "rel",
            "vec_a_near alpha_edge_v2",
            "e-a",
            "src-a",
        )])
        .unwrap();
        a.transition("a-del", LifecycleState::Deleted, Some("deleted in a".into())).unwrap();
        a.drain(10_000).unwrap();
        a.close().unwrap();
    }

    {
        let b = open_scoped(&path, ScopePolicy::single("agent-b"));
        b.write(&[
            plain_node("doc", "vec_a_near beta_v1", "src-b", Some("b-hist"), None),
            plain_node("doc", "vec_a_near beta_del", "src-b", Some("b-del"), None),
            plain_node("doc", "vec_a_near beta_end", "src-b", Some("b-end"), None),
        ])
        .unwrap();
        b.write(&[plain_node("doc", "vec_a_near beta_v2", "src-b", Some("b-hist"), None)]).unwrap();
        b.write(&[plain_edge("b-hist", "b-end", "rel", "vec_a_near beta_edge_v1", "e-b", "src-b")])
            .unwrap();
        b.write(&[plain_edge("b-hist", "b-end", "rel", "vec_a_near beta_edge_v2", "e-b", "src-b")])
            .unwrap();
        b.transition("b-del", LifecycleState::Deleted, Some("deleted in b".into())).unwrap();
        b.close().unwrap();
    }

    let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
    for view in [
        ReadView { include_superseded: true, ..ReadView::default() },
        ReadView { include_inactive: true, ..ReadView::default() },
        ReadView { include_superseded: true, include_inactive: true, ..ReadView::default() },
    ] {
        let res =
            a.search_reranked_view("vec_a_near", None, 0, false, 0.3, 0, false, &view).unwrap();
        assert_eq!(
            res.soft_fallback, None,
            "Scope B un-embedded history must NOT mark Scope A relaxed search as dense-incomplete ({view:?})"
        );
        assert!(
            res.results.iter().all(|hit| hit.body.contains("alpha")),
            "Scope B hit leaked into Scope A relaxed search: {:?}",
            res.results
        );
        assert!(
            res.results.iter().any(|hit| hit.branch == SoftFallbackBranch::Vector),
            "Scope A vector arm must execute cleanly under relaxed ReadView ({view:?}): {:?}",
            res.results
        );
    }
    a.close().unwrap();

    // Positive control: Scope B itself DOES observe dense incompleteness under relaxed ReadView
    // when its own historical rows remain un-embedded.
    let b = open_scoped_with_embedder(&path, ScopePolicy::single("agent-b"), Arc::clone(&emb));
    b.set_projection_scheduler_frozen_for_test(true);
    b.write(&[plain_node("doc", "vec_a_near beta_v3", "src-b", Some("b-hist-frozen"), None)])
        .unwrap();
    b.write(&[plain_node("doc", "vec_a_near beta_v4", "src-b", Some("b-hist-frozen"), None)])
        .unwrap();
    let res_b = b
        .search_reranked_view(
            "vec_a_near",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView { include_superseded: true, ..ReadView::default() },
        )
        .unwrap();
    assert_eq!(res_b.soft_fallback, Some(SoftFallback { branch: SoftFallbackBranch::Vector }));
    b.close().unwrap();
}

#[test]
fn cross_scope_pending_or_failed_projection_work_does_not_degrade_clean_scope_search_or_graph_expand(
) {
    let dir = TempDir::new().unwrap();
    let pending_path = db_path(&dir, "pending_cross_scope");
    let emb: Arc<dyn Embedder> = Arc::new(DeterministicEmbedder);

    {
        let a = open_scoped_with_embedder(
            &pending_path,
            ScopePolicy::single("agent-a"),
            Arc::clone(&emb),
        );
        a.configure_projections(&[body_vector_spec()], &[]).unwrap();
        a.write(&[
            plain_node("doc", "vec_a_near graphanchor alpha_root", "src-a", Some("a-root"), None),
            plain_node("doc", "alpha_leaf", "src-a", Some("a-leaf"), None),
            plain_edge("a-root", "a-leaf", "rel", "alpha_edge", "e-a", "src-a"),
        ])
        .unwrap();
        a.drain(10_000).unwrap();
        a.close().unwrap();
    }
    {
        // Scope B writes pending un-embedded work.
        let b = open_scoped(&pending_path, ScopePolicy::single("agent-b"));
        b.write(&[plain_node("doc", "beta_pending", "src-b", Some("b-pending"), None)]).unwrap();
        b.close().unwrap();
    }

    // Scope A opens with Absent embedder runtime: Scope A is fully projected, Scope B is pending.
    let a_absent = open_scoped(&pending_path, ScopePolicy::single("agent-a"));
    let status_a = a_absent.read_projection_generation_status().unwrap();
    assert_eq!(status_a.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status_a.pending_count, 0);
    assert_eq!(status_a.failed_count, 0);

    let explained_a = a_absent
        .search_reranked_view("graphanchor", None, 0, false, 0.3, 0, true, &ReadView::default())
        .unwrap();
    let per_hit_a = &explained_a.explanation.as_ref().expect("explanation").per_hit;
    assert!(!per_hit_a.is_empty());
    for hit in per_hit_a {
        assert_eq!(
            hit.structural.inclusion_state,
            StructuralInclusionStateV1::Included,
            "Scope B pending work must NOT degrade Scope A search hit: {:?}",
            hit.structural
        );
        assert!(
            !hit.structural
                .degradation_codes
                .contains(&StructuralDegradationCodeV1::ProjectionBlocked),
            "Scope B pending work must NOT emit ProjectionBlocked on Scope A: {:?}",
            hit.structural
        );
    }

    let expand_req = GraphExpandRequestV1::new(
        GraphSeedV1::Query { schema_version: 1, text: "graphanchor".into(), ranked_limit: 5 },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        1,
        20,
        1_000,
    )
    .with_explanation(true);
    let expanded_a = a_absent.graph_expand(&expand_req).unwrap();
    assert_eq!(
        expanded_a.explanation.as_ref().expect("explanation").projection_readiness,
        GraphProjectionReadinessV1::Ready,
        "Scope B pending work must NOT mark Scope A GraphSeedV1::Query as Blocked"
    );
    assert!(
        !expanded_a.degradation_codes.contains(&GraphExpansionDegradationCodeV1::ProjectionBlocked),
        "Scope A graph_expand degradation_codes must not carry ProjectionBlocked: {:?}",
        expanded_a.degradation_codes
    );
    a_absent.close().unwrap();

    // Positive control: Scope B sees Blocked readiness under Absent runtime.
    let b_absent = open_scoped(&pending_path, ScopePolicy::single("agent-b"));
    let status_b = b_absent.read_projection_generation_status().unwrap();
    assert_eq!(status_b.readiness, ProjectionReadinessV1::Blocked);
    assert!(status_b.pending_count >= 1);
    let expand_req_b = GraphExpandRequestV1::new(
        GraphSeedV1::Query { schema_version: 1, text: "beta_pending".into(), ranked_limit: 5 },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        1,
        20,
        1_000,
    )
    .with_explanation(true);
    let expanded_b = b_absent.graph_expand(&expand_req_b).unwrap();
    assert_eq!(
        expanded_b.explanation.as_ref().expect("explanation").projection_readiness,
        GraphProjectionReadinessV1::Blocked
    );
    b_absent.close().unwrap();

    // Failed dense projection work in Scope B must NOT mark Scope A as Degraded.
    let failed_path = db_path(&dir, "failed_cross_scope");
    {
        let a = open_scoped_with_embedder(
            &failed_path,
            ScopePolicy::single("agent-a"),
            Arc::clone(&emb),
        );
        a.configure_projections(&[body_vector_spec()], &[]).unwrap();
        a.write(&[plain_node(
            "doc",
            "vec_a_near graphanchor alpha_clean",
            "src-a",
            Some("a-clean"),
            None,
        )])
        .unwrap();
        a.drain(10_000).unwrap();
        a.close().unwrap();
    }
    let failed_cursor_b = {
        let b = open_scoped(&failed_path, ScopePolicy::single("agent-b"));
        let cursor = b
            .write(&[plain_node("doc", "beta_fail broken_doc", "src-b", Some("b-fail"), None)])
            .unwrap()
            .row_cursors[0];
        b.close().unwrap();
        cursor
    };
    Connection::open(&failed_path)
        .unwrap()
        .execute(
            "INSERT INTO _fathomdb_projection_terminal(write_cursor,state) VALUES(?1,'failed')",
            [failed_cursor_b],
        )
        .unwrap();
    {
        let b = open_scoped_with_embedder(
            &failed_path,
            ScopePolicy::single("agent-b"),
            Arc::clone(&emb),
        );
        let status_b = b.read_projection_generation_status().unwrap();
        assert_eq!(status_b.readiness, ProjectionReadinessV1::Degraded);
        assert!(status_b.failed_count >= 1);
        b.close().unwrap();
    }
    let a_usable =
        open_scoped_with_embedder(&failed_path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
    let status_a_usable = a_usable.read_projection_generation_status().unwrap();
    assert_eq!(status_a_usable.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status_a_usable.failed_count, 0);
    let expanded_a_usable = a_usable.graph_expand(&expand_req).unwrap();
    assert_eq!(
        expanded_a_usable.explanation.as_ref().expect("explanation").projection_readiness,
        GraphProjectionReadinessV1::Ready
    );
    assert!(
        !expanded_a_usable
            .degradation_codes
            .contains(&GraphExpansionDegradationCodeV1::ProjectionDegraded)
            && !expanded_a_usable
                .degradation_codes
                .contains(&GraphExpansionDegradationCodeV1::ProjectionBlocked),
        "Scope A must not observe cross-scope projection degradation: {:?}",
        expanded_a_usable.degradation_codes
    );
    a_usable.close().unwrap();
}

#[test]
fn multi_read_scope_resolves_shared_operation_id_and_detects_ambiguous_derived_lookup() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "multi_read_shadowing");
    let emb: Arc<dyn Embedder> = Arc::new(DeterministicEmbedder);

    let (cursor_a, gen_id) = {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        a.configure_projections(&[body_vector_spec()], &[]).unwrap();
        let node_a = match canonical_node(
            "doc",
            "vec_a_near src a",
            "src-a",
            "node-a",
            "shared-src-rev",
            "v1",
            None,
        ) {
            PreparedWrite::ProvenancedNode(n) => n,
            _ => unreachable!(),
        };
        let der_a = match derived_node(
            "fact",
            "vec_a_near der a",
            "src-a",
            "der-a",
            "shared-der-rev",
            "v1",
            "shared-src-rev",
            "vec_a_near src a",
        ) {
            PreparedWrite::ProvenancedNode(n) => n,
            _ => unreachable!(),
        };
        let rec_a = a
            .actuate(
                ActuationBatchV1::new(
                    "shared-op",
                    vec![
                        ActuationOperationV1::PutCanonicalNode(node_a),
                        ActuationOperationV1::PutDerivedNode(der_a),
                        ActuationOperationV1::RegisterSourceDependency(
                            SourceDependencyRegistrationV1::new(
                                "dep-a",
                                "shared-src-rev",
                                "shared-der-rev",
                            )
                            .unwrap(),
                        ),
                    ],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(rec_a.outcome, ActuationOutcomeV1::Committed);
        a.drain(10_000).unwrap();
        let gen_id = a.read_projection_generation_status().unwrap().generation_id;
        let cursor = rec_a.pending_projection_write_cursors[0];
        a.close().unwrap();
        (cursor, gen_id)
    };

    let cursor_b = {
        let b = open_scoped_with_embedder(&path, ScopePolicy::single("agent-b"), Arc::clone(&emb));
        let node_b = match canonical_node(
            "doc",
            "vec_b_mid src b",
            "src-b",
            "node-b",
            "shared-src-rev",
            "v1",
            None,
        ) {
            PreparedWrite::ProvenancedNode(n) => n,
            _ => unreachable!(),
        };
        let der_b = match derived_node(
            "fact",
            "vec_b_mid der b",
            "src-b",
            "der-b",
            "shared-der-rev",
            "v1",
            "shared-src-rev",
            "vec_b_mid src b",
        ) {
            PreparedWrite::ProvenancedNode(n) => n,
            _ => unreachable!(),
        };
        let der_b_unique = match derived_node(
            "fact",
            "vec_b_mid only b",
            "src-b",
            "der-b-only",
            "only-b-der-rev",
            "v1",
            "shared-src-rev",
            "vec_b_mid src b",
        ) {
            PreparedWrite::ProvenancedNode(n) => n,
            _ => unreachable!(),
        };
        let rec_b = b
            .actuate(
                ActuationBatchV1::new(
                    "shared-op",
                    vec![
                        ActuationOperationV1::PutCanonicalNode(node_b),
                        ActuationOperationV1::PutDerivedNode(der_b),
                        ActuationOperationV1::PutDerivedNode(der_b_unique),
                        ActuationOperationV1::RegisterSourceDependency(
                            SourceDependencyRegistrationV1::new(
                                "dep-b",
                                "shared-src-rev",
                                "shared-der-rev",
                            )
                            .unwrap(),
                        ),
                        ActuationOperationV1::RegisterSourceDependency(
                            SourceDependencyRegistrationV1::new(
                                "dep-b-only",
                                "shared-src-rev",
                                "only-b-der-rev",
                            )
                            .unwrap(),
                        ),
                    ],
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(rec_b.outcome, ActuationOutcomeV1::Committed);
        b.drain(10_000).unwrap();
        let cursor = rec_b.pending_projection_write_cursors[0];
        b.close().unwrap();
        cursor
    };

    {
        let ab = open_scoped_with_embedder(
            &path,
            ScopePolicy::read_only(["agent-a", "agent-b"]),
            Arc::clone(&emb),
        );
        // Both Scope A's cursor and Scope B's cursor under shared `operation_id = "shared-op"` must resolve!
        let status_a = ab
            .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: "shared-op".into(),
                write_cursor: cursor_a,
                expected_generation_id: gen_id.clone(),
            })
            .unwrap();
        assert_eq!(status_a.write_cursor, cursor_a);
        let status_b = ab
            .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: "shared-op".into(),
                write_cursor: cursor_b,
                expected_generation_id: gen_id.clone(),
            })
            .unwrap();
        assert_eq!(status_b.write_cursor, cursor_b);

        let missing_cursor_err = ab
            .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: "shared-op".into(),
                write_cursor: cursor_b + 9_999,
                expected_generation_id: gen_id.clone(),
            })
            .unwrap_err();
        assert!(matches!(
            missing_cursor_err,
            EngineError::ProjectionGeneration(ref e)
                if e.reason == ProjectionGenerationErrorReason::MutationNotTracked
                    && e.field_path == "/writeCursor"
        ));

        // Unambiguous derived lookup resolves Scope B cleanly.
        let unique_b = ab
            .dependency_for_derived(DependencyDerivedLookupV1::new("only-b-der-rev").unwrap())
            .unwrap()
            .expect("only-b-der-rev exists in agent-b");
        assert_eq!(unique_b.dependency_id.as_str(), "dep-b-only");

        // Shared `derived_revision_id` across multiple readable scopes must error deterministically.
        let ambig_dep = ab
            .dependency_for_derived(DependencyDerivedLookupV1::new("shared-der-rev").unwrap())
            .unwrap_err();
        assert!(
            matches!(ambig_dep, EngineError::InvalidArgument { ref msg } if msg.contains("derived_revision_id") && msg.contains("multiple")),
            "expected explicit ambiguity error, got {ambig_dep:?}"
        );

        // `trace_dependency` on shared revision_id across multiple readable scopes must also error deterministically.
        let frozen_ab = strict_frozen(&ab);
        let ambig_trace = ab
            .trace_dependency(
                DependencyTraceRequestV1::new(
                    "shared-src-rev",
                    DependencyTraceDirectionV1::ToDependents,
                    frozen_ab.clone(),
                )
                .unwrap(),
            )
            .unwrap_err();
        assert!(
            matches!(ambig_trace, EngineError::InvalidArgument { ref msg } if msg.contains("multiple")),
            "expected explicit trace ambiguity error, got {ambig_trace:?}"
        );
        let unique_trace = ab
            .trace_dependency(
                DependencyTraceRequestV1::new(
                    "only-b-der-rev",
                    DependencyTraceDirectionV1::ToSource,
                    frozen_ab,
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(unique_trace.dependency_edges.len(), 1);
        ab.close().unwrap();
    }

    // Erasing Scope A's source redacts Scope A's `"shared-op"` receipt to `outcome = 'erased'`,
    // which must NOT shadow Scope B's `"shared-op"` receipt or shared derived revision!
    {
        let a = open_scoped_with_embedder(&path, ScopePolicy::single("agent-a"), Arc::clone(&emb));
        a.transition("der-a", LifecycleState::Deleted, None).unwrap();
        a.purge("der-a").unwrap();
        a.erase_source("src-a").unwrap();
        a.close().unwrap();
    }
    {
        let ab = open_scoped_with_embedder(
            &path,
            ScopePolicy::read_only(["agent-a", "agent-b"]),
            Arc::clone(&emb),
        );
        let status_b_after_erase = ab
            .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
                schema_version: 1,
                operation_id: "shared-op".into(),
                write_cursor: cursor_b,
                expected_generation_id: gen_id,
            })
            .unwrap();
        assert_eq!(status_b_after_erase.write_cursor, cursor_b);

        let der_b_after_erase = ab
            .dependency_for_derived(DependencyDerivedLookupV1::new("shared-der-rev").unwrap())
            .unwrap()
            .expect("agent-b dependency remains unambiguous once agent-a erased");
        assert_eq!(der_b_after_erase.dependency_id.as_str(), "dep-b");
        ab.close().unwrap();
    }
}

// =============================================================================
// 6. Proptest property tests for ScopePolicy grammar & cardinality
// =============================================================================

proptest! {
    #[test]
    fn prop_valid_scope_policies_canonicalize_deterministically_at_open(
        tokens in prop::collection::vec("[a-z0-9][a-z0-9:_-]{0,16}", 1..=MAX_READ_SCOPES),
    ) {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir, "prop_valid");
        let write_tok = tokens[0].clone();
        let policy = ScopePolicy::read_write(tokens.clone(), write_tok.clone());
        let opened = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig::default().with_scope_policy(policy),
        )
        .unwrap();
        let mut expected = tokens;
        expected.sort();
        expected.dedup();
        prop_assert_eq!(
            opened.engine.scope_policy(),
            &ScopePolicy::Scoped { read_scopes: expected, write_scope: Some(write_tok) }
        );
        opened.engine.close().unwrap();
    }

    #[test]
    fn prop_invalid_tokens_always_fail_open_without_creating_db(
        bad in prop_oneof![
            Just(String::new()),
            Just("*".to_string()),
            "[a-z]{1,8}".prop_map(|s| format!(" {s}")),
            "[a-z]{1,8}".prop_map(|s| format!("{s}\x1e")),
            "[a-z]{1,8}".prop_map(|s| format!("{s}\x00")),
        ],
    ) {
        let dir = TempDir::new().unwrap();
        let path = db_path(&dir, "prop_invalid");
        let res = Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig::default().with_scope_policy(ScopePolicy::single(bad)),
        );
        let is_policy_err = matches!(
            res,
            Err(EngineOpenError::EngineConfiguration(EngineConfigurationError::ScopePolicy { .. }))
        );
        prop_assert!(is_policy_err);
        prop_assert!(!path.exists());
    }
}
