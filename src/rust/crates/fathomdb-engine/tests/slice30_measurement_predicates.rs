use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, EmbedderChoice, Engine, InitialState, PreparedWrite,
    ProvenancedNodeV1, ReadView, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn plain() -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: "source evidence".into(),
        source_id: SourceId::new("test:slice30-dependencies").unwrap(),
        logical_id: Some("plain".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn canonical() -> PreparedWrite {
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "source source-r1".into(),
        source_id: SourceId::new("test:slice30-dependencies").unwrap(),
        logical_id: Some("source".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
        ),
    })
}

fn derived() -> PreparedWrite {
    let hash =
        Sha256::digest(b"source source-r1").iter().map(|b| format!("{b:02x}")).collect::<String>();
    PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
        kind: "fact".into(),
        body: "derived source-r1".into(),
        source_id: SourceId::new("test:slice30-dependencies").unwrap(),
        logical_id: Some("derived".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("derived-r1").unwrap(),
            SourceVersionId::new("v1").unwrap(),
            SourceRevisionId::new("source-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(hash).unwrap(),
        ),
    })
}

fn dependencies(engine: &Engine) {
    engine.write(&[canonical(), derived()]).unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
}

fn check_dependency_span(with_dependency: bool) {
    let root = TempDir::new().unwrap();
    let engine = Engine::open_with_choice(root.path().join("store.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine;
    if with_dependency {
        dependencies(&engine);
    } else {
        engine.write(&[plain()]).unwrap();
    }
    engine.slice30_observe_queries_for_test(true).unwrap();
    for expected_call in 1..=2 {
        let result = engine
            .search_text_only_filtered_with_snippets("source", None, &ReadView::default(), 10, None)
            .unwrap();
        let mut ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
        ids.sort();
        assert_eq!(
            ids,
            if with_dependency { vec!["l:derived", "l:source"] } else { vec!["l:plain"] }
        );
        let records = engine.take_slice30_measurements_for_test().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["call_id"], expected_call);
        assert!(records[0]["engine_id"].as_u64().unwrap() > 0);
        let observed = &records[0]["predicates"]["has_source_dependencies"];
        assert_eq!(observed["invoked"], true, "missing dependency-presence invocation span");
        assert_eq!(observed["value"], with_dependency);
        assert!(observed["elapsed_ns"].as_u64().unwrap() > 0);
    }
    engine.close().unwrap();
}

#[test]
fn dependency_presence_is_called_false_on_ordinary_real_store() {
    check_dependency_span(false);
}

#[test]
fn dependency_presence_is_called_true_on_governed_dependency_store() {
    check_dependency_span(true);
}

struct TinyEmbedder;

impl fathomdb_embedder_api::Embedder for TinyEmbedder {
    fn identity(&self) -> fathomdb_embedder_api::EmbedderIdentity {
        fathomdb_embedder_api::EmbedderIdentity::new("slice30-fixture", "normalized-768", 768)
    }

    fn embed(&self, _text: &str) -> Result<Vec<f32>, fathomdb_embedder_api::EmbedderError> {
        let mut vector = vec![0.0; 768];
        vector[0] = 1.0;
        Ok(vector)
    }

    fn truncation_offset(
        &self,
        text: &str,
    ) -> Result<Option<usize>, fathomdb_embedder_api::EmbedderError> {
        Ok(Some(text.len()))
    }
}

fn dense_engine(path: &std::path::Path, with_dependency: bool) -> Engine {
    use fathomdb_engine::{ProjectionRole, ProjectionSpec, ProjectionVector};
    let engine = Engine::open_with_embedder_for_test(path, std::sync::Arc::new(TinyEmbedder))
        .unwrap()
        .engine;
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "s30_dense".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                source: None,
            }],
            &[],
        )
        .unwrap();
    if with_dependency {
        dependencies(&engine);
    } else {
        engine.write(&[plain()]).unwrap();
    }
    engine.drain(5000).unwrap();
    engine.slice30_observe_queries_for_test(true).unwrap();
    engine
}

fn one_observation(engine: &Engine) -> serde_json::Value {
    let records = engine.take_slice30_measurements_for_test().unwrap();
    assert_eq!(records.len(), 1);
    records.into_iter().next().unwrap()
}

fn called_fallback(record: &serde_json::Value, expected: bool, sql: bool) {
    let span = &record["predicates"]["vector_arm_requires_fallback"];
    assert_eq!(span["invoked"], true, "missing called vector fallback span");
    assert_eq!(span["value"], expected);
    assert!(span["elapsed_ns"].as_u64().unwrap() > 0);
    let sql_span = &span["sql"];
    assert_eq!(sql_span["invoked"], sql);
    if sql {
        assert_eq!(sql_span["value"], expected);
        assert!(sql_span["elapsed_ns"].as_u64().unwrap() > 0);
    } else {
        assert!(sql_span["elapsed_ns"].is_null());
        assert!(sql_span["value"].is_null());
        assert_eq!(sql_span["reason"], "no_dependencies");
    }
}

#[test]
fn vectorless_direct_text_skips_fallback_without_zero_false_substitution() {
    let root = TempDir::new().unwrap();
    let engine = Engine::open_with_choice(root.path().join("text.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine;
    engine.write(&[plain()]).unwrap();
    engine.slice30_observe_queries_for_test(true).unwrap();
    let result = engine
        .search_text_only_filtered_with_snippets("source", None, &ReadView::default(), 10, None)
        .unwrap();
    assert_eq!(result.results[0].id.to_prefixed(), "l:plain");
    let record = one_observation(&engine);
    let span = &record["predicates"]["vector_arm_requires_fallback"];
    assert_eq!(span["invoked"], false, "missing explicit NOT_CALLED fallback disposition");
    assert_eq!(span["reason"], "query_vector_absent");
    assert!(span["value"].is_null());
    assert!(span["elapsed_ns"].is_null());
    engine.close().unwrap();
}

#[test]
fn history_incomplete_short_circuits_fallback_before_vector_presence() {
    let root = TempDir::new().unwrap();
    let engine = Engine::open_with_choice(root.path().join("history.sqlite"), EmbedderChoice::None)
        .unwrap()
        .engine;
    engine.write(&[plain()]).unwrap();
    engine.slice30_observe_queries_for_test(true).unwrap();
    let result = engine
        .search_view("source", &ReadView { include_inactive: true, ..ReadView::default() })
        .unwrap();
    assert_eq!(result.results[0].id.to_prefixed(), "l:plain");
    let record = one_observation(&engine);
    let span = &record["predicates"]["vector_arm_requires_fallback"];
    assert_eq!(span["invoked"], false, "missing history-short-circuit disposition");
    assert_eq!(span["reason"], "history_incomplete");
    assert!(span["value"].is_null());
    assert!(span["elapsed_ns"].is_null());
    engine.close().unwrap();
}

#[test]
fn dense_no_dependency_calls_false_without_executing_dependency_sql() {
    let root = TempDir::new().unwrap();
    let engine = dense_engine(&root.path().join("dense.sqlite"), false);
    let result = engine.search("source").unwrap();
    assert_eq!(result.results[0].id.to_prefixed(), "l:plain");
    called_fallback(&one_observation(&engine), false, false);
    engine.close().unwrap();
}

#[test]
fn dense_valid_dependency_calls_sql_and_reports_false() {
    let root = TempDir::new().unwrap();
    let engine = dense_engine(&root.path().join("valid.sqlite"), true);
    let result = engine.search("source").unwrap();
    let mut ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
    ids.sort();
    assert_eq!(ids, ["l:derived", "l:source"]);
    called_fallback(&one_observation(&engine), false, true);
    engine.close().unwrap();
}

#[test]
fn governed_source_lifecycle_loss_calls_sql_and_reports_fallback_true() {
    let root = TempDir::new().unwrap();
    let engine = dense_engine(&root.path().join("invalid.sqlite"), true);
    engine.transition("source", fathomdb_engine::LifecycleState::Deleted, None).unwrap();
    let result = engine.search("source").unwrap();
    assert!(result.results.is_empty());
    called_fallback(&one_observation(&engine), true, true);
    engine.close().unwrap();
}

fn all_nodes_case(case: &str) {
    use fathomdb_engine::SearchFilter;
    let root = TempDir::new().unwrap();
    let engine = dense_engine(&root.path().join("all-nodes.sqlite"), case == "dependencies");
    let mut view = ReadView { valid_as_of: Some(100), ..ReadView::default() };
    if case == "future" || case == "expired" {
        engine.slice30_observe_queries_for_test(false).unwrap();
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "source outside".into(),
                source_id: SourceId::new("test:slice30-window").unwrap(),
                logical_id: Some("outside".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: (case == "future").then_some(200),
                valid_until: (case == "expired").then_some(50),
            }])
            .unwrap();
        engine.drain(5000).unwrap();
        engine.slice30_observe_queries_for_test(true).unwrap();
    }
    view.include_superseded = case == "include_superseded";
    view.include_inactive = case == "include_inactive";
    view.include_out_of_window = case == "include_out_of_window";
    engine.set_recency_reweight_enabled_for_test(case == "recency");
    engine.set_importance_reweight_enabled_for_test(case == "importance");
    let filter = (case == "filter").then(|| {
        let mut filter = SearchFilter::default();
        filter.kind = Some("doc".into());
        filter
    });
    let result = if case == "direct_text" {
        engine.search_text_only_filtered_with_snippets("source", None, &view, 10, None).unwrap()
    } else {
        engine
            .search_reranked_view(
                "source",
                filter,
                usize::from(case == "rerank"),
                case == "graph",
                0.3,
                0,
                case == "explain",
                &view,
            )
            .unwrap()
    };
    let mut ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
    ids.sort();
    assert_eq!(
        ids,
        if case == "dependencies" { vec!["l:derived", "l:source"] } else { vec!["l:plain"] }
    );
    let record = one_observation(&engine);
    let span = &record["predicates"]["all_nodes_directly_eligible"];
    if matches!(case, "default" | "future" | "expired") {
        assert_eq!(span["invoked"], true, "missing called all-nodes predicate span");
        assert_eq!(span["value"], case == "default");
        assert!(span["elapsed_ns"].as_u64().unwrap() > 0);
    } else {
        let reason = match case {
            "direct_text" => "query_vector_absent",
            "filter" => "filter",
            "perf_limit" => "perf_limit",
            "dependencies" => "source_dependencies",
            "include_superseded" => "include_superseded",
            "include_inactive" => "include_inactive",
            "include_out_of_window" => "include_out_of_window",
            "recency" => "recency",
            "importance" => "importance",
            "rerank" => "rerank",
            "graph" => "graph",
            "explain" => "explain",
            "full_sort" => "full_sort",
            _ => panic!("unknown literal case"),
        };
        assert_eq!(span["invoked"], false, "missing guarded all-nodes NOT_CALLED disposition");
        assert_eq!(span["reason"], reason);
        assert!(span["value"].is_null());
        assert!(span["elapsed_ns"].is_null());
    }
    engine.close().unwrap();
}

struct OwnedChild(Option<std::process::Child>);

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn bounded_control_output(command: &mut std::process::Command) -> std::process::Output {
    command.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    let mut owned = OwnedChild(Some(command.spawn().unwrap()));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while owned.0.as_mut().unwrap().try_wait().unwrap().is_none() {
        assert!(std::time::Instant::now() < deadline, "tiny control exceeded its30s bound");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    owned.0.take().unwrap().wait_with_output().unwrap()
}

#[test]
fn all_nodes_guard_controls_preserve_invocation_and_null_skip_distinction() {
    const SELECTOR: &str = "S30_GD_ALL_NODES_CASE";
    if let Ok(case) = std::env::var(SELECTOR) {
        all_nodes_case(&case);
        return;
    }
    let mut failed = Vec::new();
    for case in [
        "default",
        "future",
        "expired",
        "direct_text",
        "filter",
        "perf_limit",
        "dependencies",
        "include_superseded",
        "include_inactive",
        "include_out_of_window",
        "recency",
        "importance",
        "rerank",
        "graph",
        "explain",
        "full_sort",
    ] {
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command
            .args([
                "--exact",
                "all_nodes_guard_controls_preserve_invocation_and_null_skip_distinction",
                "--nocapture",
            ])
            .env(SELECTOR, case)
            .env_remove("FATHOMDB_PERF_EXPERIMENTS")
            .env_remove("FATHOMDB_PERF_SEARCH_LIMIT")
            .env_remove("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
        if case == "perf_limit" {
            command.env("FATHOMDB_PERF_EXPERIMENTS", "1").env("FATHOMDB_PERF_SEARCH_LIMIT", "10");
        }
        if case == "full_sort" {
            command.env("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1");
        }
        let output = bounded_control_output(&mut command);
        println!(
            "case={case} exit={}\n{}{}",
            output.status,
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap()
        );
        if !output.status.success() {
            failed.push(case);
        }
    }
    assert!(failed.is_empty(), "all-nodes observer literal controls failed: {failed:?}");
}

fn two_vector_engine(path: &std::path::Path) -> Engine {
    let engine = dense_engine(path, false);
    engine.slice30_observe_queries_for_test(false).unwrap();
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "betaonly evidence".into(),
            source_id: SourceId::new("test:slice30-block").unwrap(),
            logical_id: Some("beta".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine.drain(5000).unwrap();
    let independent =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let count: i64 =
        independent.query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 2);
    drop(independent);
    engine.slice30_observe_queries_for_test(true).unwrap();
    engine
}

#[test]
fn actual_vector_block_brackets_known_two_vector_hybrid_hydration() {
    let root = TempDir::new().unwrap();
    let engine = two_vector_engine(&root.path().join("vectors.sqlite"));
    let result = engine.search_with_limit("source", 2).unwrap();
    let mut ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
    ids.sort();
    assert_eq!(ids, ["l:beta", "l:plain"]);
    let record = one_observation(&engine);
    let vector = &record["blocks"]["vector"];
    assert_eq!(vector["invoked"], true, "missing actual vector block span");
    assert!(vector["elapsed_ns"].as_u64().unwrap() > 0);
    assert_eq!(vector["rows"], 2);
    assert_eq!(vector["degraded"], false);
    engine.close().unwrap();
}

#[test]
fn direct_text_on_vector_store_reports_vector_block_not_run() {
    let root = TempDir::new().unwrap();
    let engine = two_vector_engine(&root.path().join("text-on-vectors.sqlite"));
    let result = engine
        .search_text_only_filtered_with_snippets("source", None, &ReadView::default(), 2, None)
        .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.to_prefixed(), "l:plain");
    let record = one_observation(&engine);
    let vector = &record["blocks"]["vector"];
    assert_eq!(vector["invoked"], false, "missing vector block NOT_RUN disposition");
    assert_eq!(vector["reason"], "query_vector_absent");
    assert!(vector["elapsed_ns"].is_null());
    assert!(vector["rows"].is_null());
    assert_eq!(vector["degraded"], false);
    engine.close().unwrap();
}

#[test]
fn source_lifecycle_degradation_is_reported_as_skipped_vector_arm() {
    let root = TempDir::new().unwrap();
    let engine = dense_engine(&root.path().join("degraded-vectors.sqlite"), true);
    engine.transition("source", fathomdb_engine::LifecycleState::Deleted, None).unwrap();
    let result = engine.search("source").unwrap();
    assert!(result.results.is_empty());
    let record = one_observation(&engine);
    let vector = &record["blocks"]["vector"];
    assert_eq!(vector["invoked"], false, "missing degraded vector arm disposition");
    assert_eq!(vector["reason"], "eligibility_degraded");
    assert!(vector["elapsed_ns"].is_null());
    assert!(vector["rows"].is_null());
    assert_eq!(vector["degraded"], true);
    engine.close().unwrap();
}
