//! Slice 41 (`WP-41B`) — Zero-Schema-Change FTS/Evidence & `read.get(include_superseded)`
//! `O(N)` Elimination (`AC32-QUERY-SCALE-01..06`).
#![cfg(feature = "test-hooks")]

use std::sync::{Arc, Mutex};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    canonical_body_hash_calls_for_test, set_slice41_sql_trace_enabled_for_test,
    take_slice41_sql_trace_for_test, ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine,
    EngineConfig, EngineError, EvidenceResolveRequestV1, EvidenceSearchRequestV1,
    GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, LifecycleState, PreparedWrite, ProjectionFts, ProjectionGenerationErrorReason,
    ProjectionRole, ProjectionSpec, ProjectionVector, PropertyEntityClass, ProvenancedEdgeV1,
    ProvenancedNodeV1, ReadContextV1, ReadView, ScopePolicy, SearchFilter,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    TestEmbedderChoice, TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_test() -> std::sync::MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

struct SqlTraceGuard;

impl SqlTraceGuard {
    fn start() -> Self {
        take_slice41_sql_trace_for_test();
        set_slice41_sql_trace_enabled_for_test(true);
        Self
    }

    fn take(&self) -> Vec<String> {
        take_slice41_sql_trace_for_test()
    }
}

impl Drop for SqlTraceGuard {
    fn drop(&mut self) {
        set_slice41_sql_trace_enabled_for_test(false);
        take_slice41_sql_trace_for_test();
    }
}

struct EnvGuard(Vec<(&'static str, Option<String>)>);

impl EnvGuard {
    fn update(values: &[(&'static str, Option<String>)]) -> Self {
        let prior = values
            .iter()
            .map(|(key, value)| {
                let prior = std::env::var(key).ok();
                match value {
                    Some(value) => unsafe { std::env::set_var(key, value) },
                    None => unsafe { std::env::remove_var(key) },
                }
                (*key, prior)
            })
            .collect();
        Self(prior)
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

#[derive(Clone, Debug)]
struct FixedTestEmbedder;

impl Embedder for FixedTestEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice41-query-scale-test", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        if text.contains("alpha") {
            Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else if text.contains("beta") {
            Ok(vec![0.8, 0.6, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        } else {
            Ok(vec![0.6, 0.8, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
        }
    }
}

fn sha256_hex(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "memory".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None, kinds: None }),
        source: None,
        entity_class: PropertyEntityClass::Node,
    }
}

fn jsonl(path: &std::path::Path) -> Vec<serde_json::Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("JSONL row"))
        .collect()
}

fn plain_node(kind: &str, logical_id: &str, body: &str, source_id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source_id).expect("source_id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn canonical_source_write(
    kind: &str,
    logical_id: &str,
    body: &str,
    source_id: &str,
    rev_id: &str,
    ver_id: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source_id).expect("source_id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(rev_id).expect("rev"),
            SourceVersionId::new(ver_id).expect("ver"),
        ),
        properties: None,
    })
}

#[allow(clippy::too_many_arguments)]
fn derived_node_write(
    kind: &str,
    logical_id: &str,
    body: &str,
    source_id: &str,
    rev_id: &str,
    ver_id: &str,
    source_rev_id: &str,
    source_body: &str,
) -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source_id).expect("source_id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev_id).expect("rev"),
            SourceVersionId::new(ver_id).expect("ver"),
            SourceRevisionId::new(source_rev_id).expect("src rev"),
            SourceLocator::WholeBody,
            CanonicalHash::sha256(sha256_hex(source_body)).expect("hash"),
        ),
        properties: None,
    })
}

/// `AC32-QUERY-SCALE-01`: `search_with_evidence(include_explanation = false)` skips
/// full `explain` overhead (`status_cache`/`status_in_snapshot`, `structural_*`,
/// `vec_distance_cosine`), while producing bit-identical `EvidenceContributionV1` to
/// `include_explanation = true`.
#[test]
fn search_with_evidence_explain_false_skips_status_structural_cosine_and_matches_contributions() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac01-evidence{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedTestEmbedder)).expect("open");
    opened.engine.configure_projections(&[vector_spec()], &[]).expect("configure projections");

    let src_body_a = "alpha quantum telemetry canonical source record";
    let src_body_b = "beta quantum telemetry canonical source record";
    let receipt = opened
        .engine
        .write(&[
            canonical_source_write("doc", "src-a", src_body_a, "src:a", "rev-src-a", "ver-a"),
            canonical_source_write("doc", "src-b", src_body_b, "src:b", "rev-src-b", "ver-b"),
            derived_node_write(
                "doc",
                "derived-a",
                "alpha quantum telemetry derived summary",
                "src:a",
                "rev-der-a",
                "ver-a",
                "rev-src-a",
                src_body_a,
            ),
        ])
        .expect("write");
    opened.engine.write_node_importance(receipt.row_cursors[0], 0.85).expect("importance");
    opened.engine.drain(30_000).expect("drain");
    opened.engine.close().expect("close");

    // Reopen fresh so status_cache is cold; verify include_explanation = false does NOT
    // trigger a status_cache full owner scan or vec_distance_cosine or structural queries.
    let reopened = Engine::open_with_test_choice_and_config_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(FixedTestEmbedder)),
        EngineConfig::default(),
    )
    .expect("reopen");
    reopened.engine.drain(30_000).expect("drain after reopen");

    let frozen = reopened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("freeze");

    let scans_before_false =
        reopened.engine.projection_generation_status_full_owner_scan_count_for_test();
    let trace = SqlTraceGuard::start();
    let mut req_false =
        EvidenceSearchRequestV1::new("alpha quantum telemetry".into(), frozen.clone());
    req_false.include_explanation = false;
    req_false.limit = 10;
    let res_false = reopened.engine.search_with_evidence(&req_false).expect("evidence false");
    let sql_false = trace.take();
    let scans_after_false =
        reopened.engine.projection_generation_status_full_owner_scan_count_for_test();

    assert!(res_false.search_result.explanation.is_none());
    assert!(!res_false.evidence.is_empty());
    assert_eq!(
        scans_after_false - scans_before_false,
        0,
        "include_explanation=false must not invoke status_cache/status_in_snapshot"
    );
    assert!(
        !sql_false.iter().any(|s| s.contains("vec_distance_cosine")),
        "include_explanation=false must not evaluate vec_distance_cosine: {sql_false:?}"
    );
    assert!(
        !sql_false
            .iter()
            .any(|s| s.contains("SELECT state FROM canonical_nodes WHERE write_cursor=?1")),
        "include_explanation=false must not query structural_lifecycle_state: {sql_false:?}"
    );
    assert!(
        !sql_false.iter().any(|s| s.contains(
            "SELECT artifact_role, completeness FROM _fathomdb_artifact_revisions WHERE write_cursor=?1"
        )),
        "include_explanation=false must not query structural_dependency_state: {sql_false:?}"
    );

    // Now run with include_explanation = true and compare resolved EvidenceContributionV1.
    let mut req_true =
        EvidenceSearchRequestV1::new("alpha quantum telemetry".into(), frozen.clone());
    req_true.include_explanation = true;
    req_true.limit = 10;
    let res_true = reopened.engine.search_with_evidence(&req_true).expect("evidence true");
    let sql_true = trace.take();
    drop(trace);

    assert!(res_true.search_result.explanation.is_some());
    assert!(
        sql_true.iter().any(|s| s.contains("vec_distance_cosine")),
        "control include_explanation=true should compute vec_distance_cosine"
    );
    assert_eq!(res_false.evidence.len(), res_true.evidence.len());
    assert_eq!(res_false.search_result.results, res_true.search_result.results);

    for (entry_f, entry_t) in res_false.evidence.iter().zip(res_true.evidence.iter()) {
        let resolved_f = reopened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry_f.evidence_ref.clone(),
                context: frozen.clone(),
            })
            .expect("resolve false");
        let resolved_t = reopened
            .engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry_t.evidence_ref.clone(),
                context: frozen.clone(),
            })
            .expect("resolve true");
        assert_eq!(
            resolved_f.retrieval_contribution, resolved_t.retrieval_contribution,
            "EvidenceContributionV1 must be bit-identical between include_explanation=false and true"
        );
    }

    reopened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-02` (part 1): filtered FTS-only `search_frozen` and
/// `search_with_evidence` push `search_index.kind = ?` and stream `ORDER BY rank`,
/// matching full-sort `(bm25 score, write_cursor)` prefix ordering bit-for-bit.
#[test]
fn filtered_fts_only_pushes_search_index_kind_and_streams_order_by_rank() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let route_witness = dir.path().join("routes.jsonl");
    let plan_witness = dir.path().join("plans.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", None),
        ("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", None),
        ("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", Some(route_witness.display().to_string())),
        ("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", Some(plan_witness.display().to_string())),
    ]);

    // Open WITH an embedder so query_vector.is_some(), while `_fathomdb_vector_kinds` stays empty!
    let path = dir.path().join(format!("ac02-filtered-fts{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedTestEmbedder)).expect("open");

    let mut writes = Vec::new();
    for i in 1..=60 {
        writes.push(canonical_source_write(
            "doc",
            &format!("doc-{i}"),
            &format!("{} doc-entry-{i}", "qscaleneedle ".repeat((i % 7) + 1)),
            &format!("src:doc-{i}"),
            &format!("rev-doc-{i}"),
            &format!("ver-doc-{i}"),
        ));
        writes.push(canonical_source_write(
            "noise",
            &format!("noise-{i}"),
            &format!("{} noise-entry-{i}", "qscaleneedle ".repeat(10)),
            &format!("src:noise-{i}"),
            &format!("rev-noise-{i}"),
            &format!("ver-noise-{i}"),
        ));
    }
    opened.engine.write(&writes).expect("write");
    opened.engine.drain(10_000).expect("drain");

    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".into());
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), filter).expect("ctx"))
        .expect("freeze");

    let trace = SqlTraceGuard::start();
    let streamed_frozen = opened
        .engine
        .search_frozen("qscaleneedle", &frozen, 0, false, 0.3, 0, false, 15)
        .expect("search_frozen");
    let mut ev_req = EvidenceSearchRequestV1::new("qscaleneedle".into(), frozen.clone());
    ev_req.limit = 15;
    ev_req.include_explanation = false;
    let streamed_ev = opened.engine.search_with_evidence(&ev_req).expect("search_with_evidence");
    let traced_sql = trace.take();
    drop(trace);

    assert_eq!(streamed_frozen.results.len(), 15);
    assert_eq!(streamed_ev.search_result.results, streamed_frozen.results);
    assert!(streamed_frozen.results.iter().all(|h| h.kind == "doc"));

    let routes: Vec<String> = jsonl(&route_witness)
        .into_iter()
        .map(|row| row["route"].as_str().expect("route").to_string())
        .collect();
    assert!(
        routes.len() >= 2 && routes.iter().all(|r| r.starts_with("rank_stream_")),
        "filtered FTS-only search_frozen and search_with_evidence must use rank_stream_*: {routes:?}"
    );
    let plans = jsonl(&plan_witness);
    assert!(plans.len() >= 2, "must emit EXPLAIN QUERY PLAN witness for streamed filtered FTS");
    for plan in &plans {
        assert_eq!(
            plan["uses_temp_btree_for_order_by"], false,
            "filtered FTS stream must not USE TEMP B-TREE FOR ORDER BY: {plan}"
        );
        let details = plan["details"].as_array().expect("details");
        assert!(
            !details.iter().any(|d| d.as_str().unwrap_or("").contains("SCAN canonical_nodes")),
            "filtered FTS stream must not SCAN canonical_nodes: {plan}"
        );
    }
    assert!(
        traced_sql.iter().any(|s| s.contains("search_index.kind = ?")),
        "filtered FTS must push search_index.kind = ?: {traced_sql:?}"
    );

    // Compare against forced full-sort control.
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let full_frozen = opened
        .engine
        .search_frozen("qscaleneedle", &frozen, 0, false, 0.3, 0, false, 15)
        .expect("full sort control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(
        streamed_frozen.results, full_frozen.results,
        "streamed filtered FTS results must match full-sort prefix bit-for-bit"
    );

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-02` (part 2): post-`ANALYZE` on a heavily skewed `kind` distribution,
/// `CROSS JOIN canonical_nodes cn` keeps `search_index` as the outer loop with zero
/// `SCAN canonical_nodes` and zero `USE TEMP B-TREE FOR ORDER BY`.
#[test]
fn filtered_fts_only_post_analyze_skewed_kind_preserves_stream_plan() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let route_witness = dir.path().join("skew-routes.jsonl");
    let plan_witness = dir.path().join("skew-plans.jsonl");
    let _env = EnvGuard::update(&[
        ("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", None),
        ("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", None),
        ("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", Some(route_witness.display().to_string())),
        ("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", Some(plan_witness.display().to_string())),
    ]);

    let path = dir.path().join(format!("ac02-skewed{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open");

    let mut writes = Vec::new();
    for i in 0..400 {
        writes.push(plain_node(
            "common",
            &format!("common-{i}"),
            &format!("skewtoken common document payload {i}"),
            &format!("src:c-{i}"),
        ));
    }
    for i in 0..5 {
        writes.push(plain_node(
            "rare",
            &format!("rare-{i}"),
            &format!("{} rare document {i}", "skewtoken ".repeat(i + 1)),
            &format!("src:r-{i}"),
        ));
    }
    opened.engine.write(&writes).expect("write skewed corpus");
    opened.engine.execute_for_test("ANALYZE;").expect("ANALYZE");

    let mut rare_filter = SearchFilter::default();
    rare_filter.kind = Some("rare".into());
    let frozen_rare = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), rare_filter).expect("ctx"))
        .expect("freeze rare");

    let streamed = opened
        .engine
        .search_frozen("skewtoken", &frozen_rare, 0, false, 0.3, 0, false, 5)
        .expect("stream rare");
    assert_eq!(streamed.results.len(), 5);
    assert!(streamed.results.iter().all(|h| h.kind == "rare"));

    let plans = jsonl(&plan_witness);
    assert!(!plans.is_empty(), "must capture post-ANALYZE EXPLAIN QUERY PLAN");
    for plan in &plans {
        assert_eq!(
            plan["uses_temp_btree_for_order_by"], false,
            "post-ANALYZE skewed kind must not USE TEMP B-TREE FOR ORDER BY: {plan}"
        );
        let details = plan["details"].as_array().expect("details");
        assert!(
            !details.iter().any(|d| d.as_str().unwrap_or("").contains("SCAN canonical_nodes")),
            "post-ANALYZE skewed kind must not SCAN canonical_nodes: {plan}"
        );
        assert!(
            details.iter().any(|d| {
                let s = d.as_str().unwrap_or("");
                s.contains("SEARCH cn USING INTEGER PRIMARY KEY")
                    || s.contains("SEARCH cn USING INDEX canonical_nodes_write_cursor_idx")
            }),
            "post-ANALYZE skewed kind must probe canonical_nodes by write_cursor point search: {plan}"
        );
    }

    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let control = opened
        .engine
        .search_frozen("skewtoken", &frozen_rare, 0, false, 0.3, 0, false, 5)
        .expect("control rare");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(streamed.results, control.results);

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-02` (part 3): `fts_only_limit` when `_fathomdb_vector_kinds` is empty
/// obeys `!use_graph_arm && !recency_enabled && !importance_enabled`, expands for
/// `rerank_depth > 0` without upfront OOM even at `rerank_depth = u32::MAX` / `pool_n = u32::MAX`,
/// and `defer_full_hydration` never applies an `ORDER BY`-less `LIMIT`.
#[test]
fn empty_vector_kinds_cap_respects_graph_recency_importance_rerank_guards_and_large_pool_no_oom() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac02-guards{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedTestEmbedder)).expect("open");

    let mut writes = Vec::new();
    // Write 10 docs with strictly decreasing BM25 score (doc-1 has 10x term, doc-10 has 1x term).
    for i in 1..=10 {
        let reps = 11 - i;
        writes.push(plain_node(
            "doc",
            &format!("gdoc-{i}"),
            &format!("{} guardbody-{i}", "guardneedle ".repeat(reps)),
            &format!("src:g-{i}"),
        ));
    }
    // Add an edge from gdoc-10 to gdoc-neighbor (which doesn't contain guardneedle at all!)
    writes.push(plain_node("doc", "gdoc-neighbor", "unrelated graph neighbor body", "src:gn"));
    writes.push(PreparedWrite::Edge {
        kind: "links".into(),
        from: "gdoc-10".into(),
        to: "gdoc-neighbor".into(),
        source_id: SourceId::new("src:ge").expect("src"),
        logical_id: Some("edge-10-nb".into()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: Some(0.9),
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
    });
    let receipt = opened.engine.write(&writes).expect("write");
    opened.engine.drain(10_000).expect("drain");

    // Give gdoc-10 (lowest BM25, rank 10) huge importance so when importance_enabled=true
    // and limit=2, gdoc-10 MUST NOT be truncated before importance reweighting!
    let cursor_10 = receipt.row_cursors[9];
    opened.engine.write_node_importance(cursor_10, 1.0).expect("set high importance");
    for c in &receipt.row_cursors[0..9] {
        opened.engine.write_node_importance(*c, 0.0).expect("set zero importance");
    }

    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("freeze");

    // 1. importance_enabled = true -> must NOT cap before reweighting; matches full-sort.
    opened.engine.set_importance_reweight_enabled_for_test(true);
    let imp_res = opened
        .engine
        .search_frozen("guardneedle", &frozen, 0, false, 0.3, 0, false, 2)
        .expect("importance search");
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let imp_ctrl = opened
        .engine
        .search_frozen("guardneedle", &frozen, 0, false, 0.3, 0, false, 2)
        .expect("importance control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(imp_res.results, imp_ctrl.results, "importance_enabled must match full-sort");
    assert!(
        imp_res.results.iter().any(|h| h.write_cursor == cursor_10),
        "high-importance rank-10 node must survive importance reweight at limit=2: {:?}",
        imp_res.results
    );
    opened.engine.set_importance_reweight_enabled_for_test(false);

    // 2. recency_enabled = true -> must NOT cap before reweighting; matches full-sort.
    opened.engine.set_recency_reweight_enabled_for_test(true);
    let rec_res = opened
        .engine
        .search_frozen("guardneedle", &frozen, 0, false, 0.3, 0, false, 2)
        .expect("recency search");
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let rec_ctrl = opened
        .engine
        .search_frozen("guardneedle", &frozen, 0, false, 0.3, 0, false, 2)
        .expect("recency control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(rec_res.results, rec_ctrl.results, "recency_enabled must match full-sort");
    opened.engine.set_recency_reweight_enabled_for_test(false);

    // 3. use_graph_arm = true -> must NOT cap before graph seeding; matches full-sort.
    let graph_res = opened
        .engine
        .search_frozen("guardneedle", &frozen, 0, true, 0.3, 0, false, 5)
        .expect("graph search");
    unsafe { std::env::set_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1") };
    let graph_ctrl = opened
        .engine
        .search_frozen("guardneedle", &frozen, 0, true, 0.3, 0, false, 5)
        .expect("graph control");
    unsafe { std::env::remove_var("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST") };
    assert_eq!(graph_res.results, graph_ctrl.results, "use_graph_arm must match full-sort");

    // 4. Huge rerank_depth = u32::MAX and pool_n = u32::MAX -> must NOT OOM!
    let huge_rerank = opened
        .engine
        .search_frozen(
            "guardneedle",
            &frozen,
            u32::MAX as usize,
            false,
            0.3,
            u32::MAX as usize,
            false,
            5,
        )
        .expect("large rerank_depth / pool_n must not OOM");
    assert_eq!(huge_rerank.results.len(), 5);

    opened.engine.close().expect("close");

    // 5. Same corpus reopened with EmbedderChoice::None (query_vector.is_none()):
    //    importance_enabled = true at limit = 2 must STILL not truncate rank-10 before reweighting!
    let no_emb =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("reopen no embedder");
    let frozen_no_emb = no_emb
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("freeze no emb");
    no_emb.engine.set_importance_reweight_enabled_for_test(true);
    let imp_no_emb = no_emb
        .engine
        .search_frozen("guardneedle", &frozen_no_emb, 0, false, 0.3, 0, false, 2)
        .expect("importance search without embedder");
    assert!(
        imp_no_emb.results.iter().any(|h| h.write_cursor == cursor_10),
        "with query_vector=None and importance_enabled=true, high-importance rank-10 node must survive limit=2: {:?}",
        imp_no_emb.results
    );
    no_emb.engine.close().expect("close no emb");
}

/// `AC32-QUERY-SCALE-02` (part 4): `build_importance_confidence_maps` runs strictly
/// post-truncation when `!importance_enabled`, reducing lookups to `<= final_limit` while
/// propagating `EngineError::Storage` for corrupt surviving hits `<= final_limit` and
/// preserving pre-truncation behavior when `importance_enabled = true`.
#[test]
fn importance_confidence_maps_run_post_truncation_and_preserve_storage_errors() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac02-importance-post{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(FixedTestEmbedder)).expect("open");

    let mut writes = Vec::new();
    for i in 1..=10 {
        let reps = 11 - i;
        writes.push(canonical_source_write(
            "doc",
            &format!("idoc-{i}"),
            &format!("{} impbody-{i}", "impneedle ".repeat(reps)),
            &format!("src:i-{i}"),
            &format!("rev-i-{i}"),
            &format!("ver-i-{i}"),
        ));
    }
    let receipt = opened.engine.write(&writes).expect("write");
    opened.engine.drain(10_000).expect("drain");

    // Inject corrupt importance blob on rank-10 node (beyond final_limit = 3).
    let rank10_cursor = receipt.row_cursors[9];
    opened
        .engine
        .execute_for_test(&format!(
            "UPDATE canonical_nodes SET importance = x'ff' WHERE write_cursor = {rank10_cursor}"
        ))
        .expect("corrupt rank-10 importance");

    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("freeze");

    // With !importance_enabled: limit = 3 must succeed and query importance <= 3 times.
    let trace = SqlTraceGuard::start();
    let explained = opened
        .engine
        .search_frozen("impneedle", &frozen, 0, false, 0.3, 0, true, 3)
        .expect("!importance_enabled must not touch corrupt row beyond final_limit");
    let sqls = trace.take();
    assert_eq!(explained.results.len(), 3);
    let imp_queries = sqls
        .iter()
        .filter(|s| s.contains("SELECT importance FROM canonical_nodes WHERE write_cursor=?1"))
        .count();
    assert!(
        imp_queries <= 3,
        "post-truncation importance lookups must be <= final_limit (3), got {imp_queries}"
    );

    let mut ev_req = EvidenceSearchRequestV1::new("impneedle".into(), frozen.clone());
    ev_req.limit = 3;
    ev_req.include_explanation = false;
    let ev_res = opened
        .engine
        .search_with_evidence(&ev_req)
        .expect("evidence search with !importance_enabled must ignore corrupt row beyond limit");
    assert_eq!(ev_res.search_result.results.len(), 3);
    drop(trace);

    // With importance_enabled = true: must run before truncation and fail with Storage.
    opened.engine.set_importance_reweight_enabled_for_test(true);
    let err_enabled = opened.engine.search_frozen("impneedle", &frozen, 0, false, 0.3, 0, true, 3);
    assert!(
        matches!(err_enabled, Err(EngineError::Storage)),
        "importance_enabled=true must still decode all candidates and fail with Storage: {err_enabled:?}"
    );
    opened.engine.set_importance_reweight_enabled_for_test(false);

    // Now corrupt rank-1 node (surviving hit <= final_limit = 3): must return Storage!
    let rank1_cursor = receipt.row_cursors[0];
    opened
        .engine
        .execute_for_test(&format!(
            "UPDATE canonical_nodes SET importance = x'ff' WHERE write_cursor = {rank1_cursor}"
        ))
        .expect("corrupt rank-1 importance");

    let frozen2 = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("re-freeze after rank-1 corruption");
    let err_surviving =
        opened.engine.search_frozen("impneedle", &frozen2, 0, false, 0.3, 0, true, 3);
    assert!(
        matches!(err_surviving, Err(EngineError::Storage)),
        "corrupt surviving hit <= final_limit must fail with EngineError::Storage: {err_surviving:?}"
    );
    let mut ev_req2 = EvidenceSearchRequestV1::new("impneedle".into(), frozen2);
    ev_req2.limit = 3;
    ev_req2.include_explanation = false;
    let err_ev_surviving = opened.engine.search_with_evidence(&ev_req2);
    assert!(
        matches!(err_ev_surviving, Err(EngineError::Storage)),
        "evidence search must propagate Storage on corrupt surviving hit: {err_ev_surviving:?}"
    );

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-03`: inside `empty_physical_fast_path`, skip internal `node_sql`
/// when `!(node_arm_declared || has_explicit_node_enrolment)` while preserving
/// `edge_count` and `edge_count != 0 && !edge_enrolled` corruption checks.
#[test]
fn empty_physical_fast_path_skips_node_sql_when_unenrolled_and_preserves_edge_corruption_check() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac03-fast-path{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open");

    // Seed canonical_nodes with no vector projections declared and no enrolled vector kinds,
    // and ensure physical/terminal tables are empty so empty_physical_fast_path executes.
    opened
        .engine
        .write(&[
            plain_node("doc", "n1", "first unenrolled node", "src:1"),
            plain_node("doc", "n2", "second unenrolled node", "src:2"),
        ])
        .expect("write");
    opened
        .engine
        .execute_for_test("DELETE FROM _fathomdb_projection_terminal")
        .expect("clear terminal rows for fast-path entry");

    // Authorizer denies all reads on `canonical_nodes`: empty_physical_fast_path must succeed
    // because neither `node_arm_declared` nor `has_explicit_node_enrolment` holds!
    let fast = opened
        .engine
        .run_empty_physical_fast_path_with_node_read_denied_for_test()
        .expect("empty_physical_fast_path must skip canonical_nodes scan when unenrolled");
    assert_eq!(fast, Some((0, 0, None)));

    // Now inject an unenrolled body-bearing canonical_edges row -> must still fail closed
    // with EngineError::Corruption!
    opened
        .engine
        .execute_for_test(
            "INSERT INTO canonical_edges(\
                write_cursor, scope, logical_id, kind, from_id, to_id, source_id, body\
             ) VALUES (999, '', 'e-corrupt', 'relates', 'n1', 'n2', 'src:e', 'unenrolled edge body')",
        )
        .expect("inject unenrolled edge");
    let err = opened.engine.run_empty_physical_fast_path_with_node_read_denied_for_test();
    assert!(
        matches!(
            err,
            Err(EngineError::ProjectionGeneration(ref pg))
                if pg.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
        ),
        "unenrolled body-bearing edge must still trigger ProjectionGenerationCorrupt in empty_physical_fast_path: {err:?}"
    );

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-04`: `store_single_unscoped` uses covering index range probes
/// (`scope < ''` / `scope > ''`) gated behind cheap booleans, and `detect_vector_soft_fallback`
/// short-circuits in `O(1)` when `_fathomdb_vector_kinds` is empty.
#[test]
fn store_single_unscoped_uses_covering_range_search_and_soft_fallback_short_circuits() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");

    // Part A: store_single_unscoped covering range search & cheap-boolean gating.
    let path_vec = dir.path().join(format!("ac04-unscoped-vec{SQLITE_SUFFIX}"));
    let opened_vec =
        Engine::open_with_embedder_for_test(&path_vec, Arc::new(FixedTestEmbedder)).expect("open");
    opened_vec.engine.configure_projections(&[vector_spec()], &[]).expect("configure");
    opened_vec
        .engine
        .write(&[plain_node("doc", "u1", "alpha range probe document", "src:u1")])
        .expect("write");
    opened_vec.engine.drain(30_000).expect("drain");

    let trace = SqlTraceGuard::start();
    // Unfiltered hybrid search: can_defer_text_identity_cheap is true -> runs range probe.
    let res = opened_vec.engine.search("alpha").expect("hybrid search");
    assert_eq!(res.results.len(), 1);
    let hybrid_sql = trace.take();

    assert!(
        !hybrid_sql.iter().any(|s| s.contains("WHERE scope != ''")),
        "store_single_unscoped must NOT run full-index `WHERE scope != ''`: {hybrid_sql:?}"
    );
    let range_sql = hybrid_sql
        .iter()
        .find(|s| s.contains("scope < ''") && s.contains("scope > ''"))
        .cloned()
        .unwrap_or_else(|| {
            panic!("expected range probe SQL with scope < '' and scope > '': {hybrid_sql:?}")
        });

    // Verify EXPLAIN QUERY PLAN on that exact SQL is strictly SEARCH USING COVERING INDEX.
    let raw = rusqlite::Connection::open(&path_vec).expect("raw sqlite");
    let plan: Vec<String> = raw
        .prepare(&format!("EXPLAIN QUERY PLAN {range_sql}"))
        .expect("prepare explain")
        .query_map([], |row| row.get::<_, String>(3))
        .expect("query explain")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect explain");
    assert!(
        !plan.iter().any(|d| d.contains("SCAN canonical_nodes")),
        "store_single_unscoped must not SCAN canonical_nodes: {plan:?}"
    );
    assert!(
        plan.iter()
            .any(|d| d.contains("canonical_nodes_scope_cursor_idx") && d.contains("scope<?")),
        "store_single_unscoped must SEARCH canonical_nodes_scope_cursor_idx (scope<?): {plan:?}"
    );
    assert!(
        plan.iter()
            .any(|d| d.contains("canonical_nodes_scope_cursor_idx") && d.contains("scope>?")),
        "store_single_unscoped must SEARCH canonical_nodes_scope_cursor_idx (scope>?): {plan:?}"
    );
    drop(raw);

    // Filtered search: cheap boolean `filter.is_none()` is false -> must NOT run scope probe at all!
    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".into());
    let _ = opened_vec.engine.search_filtered_with_limit("alpha", Some(filter), 10).expect("filt");
    let filtered_sql = trace.take();
    assert!(
        !filtered_sql
            .iter()
            .any(|s| s.contains("scope < ''") || s.contains("scope != ''")),
        "filtered search must skip store_single_unscoped query behind cheap booleans: {filtered_sql:?}"
    );
    opened_vec.engine.close().expect("close vec");

    // Part B: detect_vector_soft_fallback O(1) short-circuit when _fathomdb_vector_kinds is empty.
    let path_novec = dir.path().join(format!("ac04-soft-fallback{SQLITE_SUFFIX}"));
    let opened_novec =
        Engine::open_with_embedder_for_test(&path_novec, Arc::new(FixedTestEmbedder))
            .expect("open novec");
    opened_novec
        .engine
        .write(&[plain_node("doc", "nv1", "alpha no vector kinds document", "src:nv1")])
        .expect("write novec");
    opened_novec.engine.drain(10_000).expect("drain novec");

    let _ = trace.take();
    let nv_res = opened_novec.engine.search("alpha").expect("novec search");
    let nv_sql = trace.take();
    drop(trace);

    assert_eq!(nv_res.results.len(), 1);
    assert!(nv_res.soft_fallback.is_none());
    assert!(
        !nv_sql.iter().any(|s| {
            s.contains("JOIN _fathomdb_vector_kinds ON _fathomdb_vector_kinds.kind = search_index.kind")
        }),
        "detect_vector_soft_fallback must short-circuit without querying search_index when _fathomdb_vector_kinds is empty: {nv_sql:?}"
    );
    opened_novec.engine.close().expect("close novec");
}

/// `AC32-QUERY-SCALE-05` (part 1): `frozen_read::mint_inner` and `graph_expand` (`Current`)
/// pin the DEFERRED snapshot via `_fathomdb_read_visibility_state WHERE singleton = 1`
/// instead of `SELECT COUNT(*) FROM canonical_nodes`.
#[test]
fn mint_and_current_graph_expand_pin_without_count_star() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac05-pin{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open");
    opened
        .engine
        .write(&[
            plain_node("doc", "p1", "pin root node", "src:p1"),
            plain_node("doc", "p2", "pin target node", "src:p2"),
            PreparedWrite::Edge {
                kind: "relates".into(),
                from: "p1".into(),
                to: "p2".into(),
                source_id: SourceId::new("src:pe").expect("src"),
                logical_id: Some("pe1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .expect("write");

    let trace = SqlTraceGuard::start();
    let _frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("freeze");
    let mint_sql = trace.take();
    assert!(
        !mint_sql.iter().any(|s| s.contains("SELECT COUNT(*) FROM canonical_nodes")),
        "freeze_read_context must NOT run SELECT COUNT(*) FROM canonical_nodes: {mint_sql:?}"
    );
    assert!(
        mint_sql.iter().any(|s| {
            s.contains("SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton = 1")
        }),
        "freeze_read_context must pin via _fathomdb_read_visibility_state WHERE singleton = 1: {mint_sql:?}"
    );

    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("p1")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        },
        1,
        10,
        100,
    );
    let expand = opened.engine.graph_expand(&req).expect("graph_expand current");
    let expand_sql = trace.take();
    drop(trace);

    assert_eq!(expand.targets.len(), 1);
    assert!(
        !expand_sql.iter().any(|s| s.contains("SELECT COUNT(*) FROM canonical_nodes")),
        "Current graph_expand must NOT run SELECT COUNT(*) FROM canonical_nodes: {expand_sql:?}"
    );
    assert!(
        expand_sql.iter().any(|s| {
            s.contains("SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton = 1")
        }),
        "Current graph_expand must pin via _fathomdb_read_visibility_state WHERE singleton = 1: {expand_sql:?}"
    );

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-05` (part 2): `historical_dense_incomplete` short-circuits immediately
/// when `query_vector.is_none()`.
#[test]
fn historical_dense_incomplete_short_circuits_without_query_vector() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac05-hist{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open");
    opened
        .engine
        .write(&[plain_node("doc", "h1", "histneedle v1 body", "src:h1")])
        .expect("write v1");
    opened
        .engine
        .write(&[plain_node("doc", "h1", "histneedle v2 body", "src:h1")])
        .expect("write v2");

    let view = ReadView { include_superseded: true, ..ReadView::default() };
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(view, SearchFilter::default()).expect("ctx"))
        .expect("freeze");

    let trace = SqlTraceGuard::start();
    let res = opened
        .engine
        .search_frozen("histneedle", &frozen, 0, false, 0.3, 0, false, 10)
        .expect("search_frozen historical");
    let sqls = trace.take();
    drop(trace);

    assert_eq!(res.results.len(), 2);
    assert!(
        !sqls.iter().any(|s| s.contains("SELECT n.write_cursor,n.kind,0 FROM canonical_nodes n")),
        "historical_dense_incomplete must short-circuit when query_vector.is_none(): {sqls:?}"
    );

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-05` (part 3): `build_search_result`, `resolve_evidence`, and
/// `resolve_graph_evidence` hash each unique `source_body` strictly ONCE (including for
/// canonical sources, derived nodes, and derived nodes with registered source dependencies).
#[test]
fn evidence_resolution_hashes_source_body_once() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac05-sha256{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open");

    let src_body = "singlepasscanonicalsource unique source text payload";
    let src_hash = CanonicalHash::sha256(sha256_hex(src_body)).expect("hash");

    opened
        .engine
        .write(&[
            canonical_source_write(
                "source_doc",
                "src-node",
                src_body,
                "src:once",
                "rev-src-once",
                "ver-once",
            ),
            derived_node_write(
                "claim",
                "der-node",
                "singlepassderived derived claim payload",
                "src:once",
                "rev-der-once",
                "ver-once",
                "rev-src-once",
                src_body,
            ),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                kind: "supports".into(),
                from: "src-node".into(),
                to: "der-node".into(),
                source_id: SourceId::new("src:once").expect("src"),
                logical_id: Some("edge-once".into()),
                body: Some("edge linking source to claim".into()),
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("rev-edge-once").expect("rev"),
                    SourceVersionId::new("ver-once").expect("ver"),
                    SourceRevisionId::new("rev-src-once").expect("src rev"),
                    SourceLocator::WholeBody,
                    src_hash,
                ),
                properties: None,
            }),
        ])
        .expect("write provenance fixture");

    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-once-1", "rev-src-once", "rev-der-once")
                .expect("dep reg"),
        )
        .expect("register dependency");

    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).expect("ctx"),
        )
        .expect("freeze");

    // 1. Canonical source via search_with_evidence + resolve_evidence: exactly 1 hash each!
    let before = canonical_body_hash_calls_for_test();
    let mut req_src =
        EvidenceSearchRequestV1::new("singlepasscanonicalsource".into(), frozen.clone());
    req_src.include_explanation = false;
    let res_src = opened.engine.search_with_evidence(&req_src).expect("search canonical source");
    let after = canonical_body_hash_calls_for_test();
    assert_eq!(res_src.evidence.len(), 1);
    assert_eq!(
        after - before,
        1,
        "build_search_result for canonical source must hash source_body exactly once"
    );

    let before = canonical_body_hash_calls_for_test();
    let _resolved_src = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: res_src.evidence[0].evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve canonical source");
    let after = canonical_body_hash_calls_for_test();
    assert_eq!(
        after - before,
        1,
        "resolve_evidence for canonical source must hash source_body exactly once"
    );

    // 2. Derived node WITH registered source dependency via search_with_evidence + resolve_evidence:
    //    exactly 1 hash each!
    let before = canonical_body_hash_calls_for_test();
    let mut req_der = EvidenceSearchRequestV1::new("singlepassderived".into(), frozen.clone());
    req_der.include_explanation = false;
    let res_der = opened.engine.search_with_evidence(&req_der).expect("search derived");
    let after = canonical_body_hash_calls_for_test();
    assert_eq!(res_der.evidence.len(), 1);
    assert_eq!(
        after - before,
        1,
        "build_search_result for derived node (+ registered dependency) must hash source_body once"
    );

    let before = canonical_body_hash_calls_for_test();
    let _resolved_der = opened
        .engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: res_der.evidence[0].evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve derived");
    let after = canonical_body_hash_calls_for_test();
    assert_eq!(
        after - before,
        1,
        "resolve_evidence for derived node (+ registered dependency) must hash source_body once"
    );

    // 3. Graph evidence resolution via resolve_graph_evidence: exactly 1 hash!
    let expand_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("src-node")],
        },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        10,
        100,
    )
    .with_evidence(true);
    let expanded = opened.engine.graph_expand(&expand_req).expect("graph_expand evidence");
    let sidecar = expanded.evidence.expect("graph evidence sidecar");
    assert_eq!(sidecar.entries.len(), 1);

    let before = canonical_body_hash_calls_for_test();
    let _resolved_graph = opened
        .engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: sidecar.entries[0].target_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .expect("resolve_graph_evidence");
    let after = canonical_body_hash_calls_for_test();
    assert_eq!(after - before, 1, "resolve_graph_evidence must hash source_body exactly once");

    opened.engine.close().expect("close");
}

/// `AC32-QUERY-SCALE-06`: `read.get` and `read.get_many` with `include_superseded = true`
/// emit `UNION ALL` over `canonical_nodes_scope_logical_{active,sup}_idx` (zero `SCAN`,
/// zero `USE TEMP B-TREE FOR ORDER BY`), gate `read_eligibility_sql` on
/// `has_source_dependencies`, perform max-`write_cursor` tiebreaking in Rust, and preserve
/// exact semantics across single-scope, multi-scope, active + superseded + missing +
/// dependency-revoked rows.
#[test]
fn read_get_and_get_many_include_superseded_use_search_only_union_all_and_match_semantics() {
    let _lock = lock_test();
    let dir = TempDir::new().expect("tempdir");

    // Part A: Unscoped single-ID & multi-ID plans and semantics (without and with dependencies).
    let path = dir.path().join(format!("ac06-read-get{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open");

    // Create active-only, superseded-twice, and superseded-with-deleted-head nodes.
    let r1 = opened
        .engine
        .write(&[
            plain_node("doc", "active-only", "active body v1", "src:a"),
            plain_node("doc", "multi-ver", "multi body v1", "src:m"),
            plain_node("doc", "deleted-head", "deleted-head body v1", "src:d"),
        ])
        .expect("write v1");
    let r2 = opened
        .engine
        .write(&[
            plain_node("doc", "multi-ver", "multi body v2", "src:m"),
            plain_node("doc", "deleted-head", "deleted-head body v2", "src:d"),
        ])
        .expect("write v2");
    let r3 = opened
        .engine
        .write(&[plain_node("doc", "multi-ver", "multi body v3", "src:m")])
        .expect("write v3");
    // Transition deleted-head's current version (v2) to deleted -> only v1 (superseded, active)
    // is eligible under include_superseded=true, include_inactive=false!
    opened
        .engine
        .transition("deleted-head", LifecycleState::Deleted, Some("retire head".into()))
        .expect("delete head");

    let sup_view = ReadView { include_superseded: true, ..ReadView::default() };
    let default_view = ReadView::default();

    // Check EXPLAIN QUERY PLAN for single-ID and multi-ID when has_source_dependencies == false.
    for ids in [
        vec!["multi-ver".to_string()],
        vec!["active-only".to_string(), "multi-ver".to_string(), "deleted-head".to_string()],
    ] {
        let (sql, plan) = opened
            .engine
            .read_get_by_id_explain_plan_for_test(&ids, sup_view)
            .expect("explain include_superseded=true");
        assert!(sql.contains("UNION ALL"), "expected UNION ALL SQL: {sql}");
        assert!(
            !sql.contains("ORDER BY write_cursor"),
            "must drop SQL ORDER BY write_cursor: {sql}"
        );
        assert!(
            !sql.contains("_fathomdb_source_dependencies"),
            "must omit read_eligibility_sql when has_source_dependencies is false: {sql}"
        );
        assert!(
            !plan.iter().any(|d| d.contains("SCAN canonical_nodes")),
            "include_superseded=true must not SCAN canonical_nodes: {plan:?}"
        );
        assert!(
            !plan.iter().any(|d| d.contains("USE TEMP B-TREE FOR ORDER BY")),
            "include_superseded=true must not USE TEMP B-TREE FOR ORDER BY: {plan:?}"
        );
        assert!(
            plan.iter().any(|d| d.contains("canonical_nodes_scope_logical_active_idx")),
            "must use canonical_nodes_scope_logical_active_idx: {plan:?}"
        );
        assert!(
            plan.iter().any(|d| d.contains("canonical_nodes_scope_logical_sup_idx")),
            "must use canonical_nodes_scope_logical_sup_idx: {plan:?}"
        );
    }

    // Check semantic parity on read_get & read_get_many.
    let single_multi = opened.engine.read_get("multi-ver", &sup_view).expect("read_get multi");
    assert_eq!(single_multi.as_ref().map(|r| r.body.as_str()), Some("multi body v3"));
    assert_eq!(single_multi.as_ref().map(|r| r.write_cursor), Some(r3.row_cursors[0]));

    let single_del = opened.engine.read_get("deleted-head", &sup_view).expect("read_get del");
    assert_eq!(single_del.as_ref().map(|r| r.body.as_str()), Some("deleted-head body v1"));
    assert_eq!(single_del.as_ref().map(|r| r.write_cursor), Some(r1.row_cursors[2]));
    assert!(opened.engine.read_get("deleted-head", &default_view).expect("default del").is_none());

    let many_ids = vec![
        "missing".to_string(),
        "multi-ver".to_string(),
        "deleted-head".to_string(),
        "active-only".to_string(),
        "multi-ver".to_string(),
    ];
    let many = opened.engine.read_get_many(&many_ids, &sup_view).expect("read_get_many sup");
    assert_eq!(many.len(), 5);
    assert!(many[0].is_none());
    assert_eq!(many[1].as_ref().map(|r| r.write_cursor), Some(r3.row_cursors[0]));
    assert_eq!(many[2].as_ref().map(|r| r.write_cursor), Some(r1.row_cursors[2]));
    assert_eq!(many[3].as_ref().map(|r| r.write_cursor), Some(r1.row_cursors[0]));
    assert_eq!(many[4], many[1], "duplicate logical_id must echo identical winning revision");
    let _ = r2;

    // Now test dependency revocation when _fathomdb_source_dependencies is non-empty!
    let dep_src_body = "revokable source for read.get";
    opened
        .engine
        .write(&[
            canonical_source_write(
                "source_doc",
                "dep-src",
                dep_src_body,
                "src:dep",
                "rev-dep-src",
                "ver-dep",
            ),
            derived_node_write(
                "claim",
                "dep-der",
                "derived node depending on dep-src",
                "src:dep",
                "rev-dep-der",
                "ver-dep",
                "rev-dep-src",
                dep_src_body,
            ),
        ])
        .expect("write dep pair");
    opened
        .engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-read-get-1", "rev-dep-src", "rev-dep-der")
                .expect("dep reg"),
        )
        .expect("register dep");

    let (dep_sql, dep_plan) = opened
        .engine
        .read_get_by_id_explain_plan_for_test(&["dep-der".to_string()], sup_view)
        .expect("explain with deps");
    assert!(
        dep_sql.contains("_fathomdb_source_dependencies"),
        "must include read_eligibility_sql when has_source_dependencies is true: {dep_sql}"
    );
    assert!(
        !dep_plan.iter().any(|d| d.contains("SCAN canonical_nodes")),
        "with deps must still not SCAN canonical_nodes: {dep_plan:?}"
    );
    assert!(opened.engine.read_get("dep-der", &sup_view).expect("before revoke").is_some());

    // Delete source -> derived node becomes ineligible under include_inactive = false!
    opened
        .engine
        .transition("dep-src", LifecycleState::Deleted, Some("revoke".into()))
        .expect("delete dep-src");
    assert!(
        opened.engine.read_get("dep-der", &sup_view).expect("after revoke").is_none(),
        "revoked dependency must exclude derived node even with include_superseded=true"
    );
    opened.engine.close().expect("close unscoped");

    // Part B: Multi-scope `ScopePolicy::Scoped` plan and semantics.
    let scoped_path = dir.path().join(format!("ac06-multi-scope{SQLITE_SUFFIX}"));
    let open_scoped = |write_scope: &str, read_scopes: Vec<&str>| {
        let cfg = EngineConfig {
            scope_policy: ScopePolicy::Scoped {
                write_scope: Some(write_scope.to_string()),
                read_scopes: read_scopes.into_iter().map(String::from).collect(),
            },
            ..EngineConfig::default()
        };
        Engine::open_with_choice_and_config(&scoped_path, EmbedderChoice::None, cfg)
            .expect("open scoped")
    };
    let s1 = open_scoped("tenant-a", vec!["tenant-a"]);
    s1.engine.write(&[plain_node("doc", "shared-id", "tenant-a v1", "src:ta")]).expect("ta v1");
    s1.engine.write(&[plain_node("doc", "shared-id", "tenant-a v2", "src:ta")]).expect("ta v2");
    s1.engine.write(&[plain_node("doc", "only-a", "only-a body", "src:ta")]).expect("only-a");
    s1.engine.close().expect("close s1");

    let s2 = open_scoped("tenant-b", vec!["tenant-b"]);
    let tb_v1 =
        s2.engine.write(&[plain_node("doc", "shared-id", "tenant-b v1", "src:tb")]).expect("tb v1");
    s2.engine.write(&[plain_node("doc", "only-b", "only-b body", "src:tb")]).expect("only-b");
    s2.engine.close().expect("close s2");

    let multi = open_scoped("tenant-a", vec!["tenant-a", "tenant-b"]);
    let (m_sql, m_plan) = multi
        .engine
        .read_get_by_id_explain_plan_for_test(
            &["shared-id".to_string(), "only-a".to_string(), "only-b".to_string()],
            sup_view,
        )
        .expect("explain multi-scope");
    assert!(m_sql.contains("UNION ALL"), "multi-scope must use UNION ALL: {m_sql}");
    assert!(
        !m_plan.iter().any(|d| d.contains("SCAN canonical_nodes")),
        "multi-scope include_superseded=true must not SCAN canonical_nodes: {m_plan:?}"
    );
    assert!(
        !m_plan.iter().any(|d| d.contains("USE TEMP B-TREE FOR ORDER BY")),
        "multi-scope include_superseded=true must not USE TEMP B-TREE FOR ORDER BY: {m_plan:?}"
    );
    let m_res = multi
        .engine
        .read_get_many(
            &["shared-id".to_string(), "only-a".to_string(), "only-b".to_string()],
            &sup_view,
        )
        .expect("multi-scope read_get_many");
    assert_eq!(m_res[0].as_ref().map(|r| r.write_cursor), Some(tb_v1.row_cursors[0]));
    assert_eq!(m_res[0].as_ref().map(|r| r.body.as_str()), Some("tenant-b v1"));
    assert_eq!(m_res[1].as_ref().map(|r| r.body.as_str()), Some("only-a body"));
    assert_eq!(m_res[2].as_ref().map(|r| r.body.as_str()), Some("only-b body"));
    multi.engine.close().expect("close multi");
}
