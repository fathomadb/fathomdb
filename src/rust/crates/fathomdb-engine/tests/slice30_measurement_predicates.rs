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
