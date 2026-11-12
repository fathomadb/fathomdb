use fathomdb_engine::{
    EmbedderChoice, Engine, EngineError, FrozenReadErrorReason, InitialState, PreparedWrite,
    ReadContextV1, ReadView, SearchFilter, SourceId,
};
use proptest::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

fn node(body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:slice30-outcomes").unwrap(),
        logical_id: Some("alpha".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn fixture(path: &std::path::Path) -> Engine {
    let engine = Engine::open_with_choice(path, EmbedderChoice::None).unwrap().engine;
    engine.write(&[node("alphaonly evidence")]).unwrap();
    engine.slice30_observe_queries_for_test(true).unwrap();
    engine
}

fn call(engine: &Engine, invalid_filter: bool) {
    if invalid_filter {
        let mut filter = SearchFilter::default();
        filter.attributes.push(("not_declared".into(), "alice".into()));
        assert!(matches!(
            engine.search_filtered("alphaonly", Some(filter)),
            Err(EngineError::InvalidFilter { .. })
        ));
    } else {
        let result = engine
            .search_text_only_filtered_with_snippets(
                "alphaonly",
                None,
                &ReadView::default(),
                1,
                None,
            )
            .unwrap();
        assert_eq!(result.results[0].id.to_prefixed(), "l:alpha");
    }
}

fn one(engine: &Engine) -> Value {
    let rows = engine.take_slice30_measurements_for_test().unwrap();
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().unwrap()
}

fn not_reached(row: &Value) {
    for name in
        ["has_source_dependencies", "vector_arm_requires_fallback", "all_nodes_directly_eligible"]
    {
        let span = &row["predicates"][name];
        assert_eq!(span["invoked"], false);
        assert_eq!(span["reason"], "not_reached");
        assert!(span["value"].is_null());
        assert!(span["elapsed_ns"].is_null());
    }
    for name in ["vector", "text"] {
        let span = &row["blocks"][name];
        assert_eq!(span["invoked"], false);
        assert_eq!(span["reason"], "not_reached");
        assert!(span["elapsed_ns"].is_null());
        assert!(span["rows"].is_null());
    }
    assert!(row["blocks"]["vector"]["degraded"].is_null());
}

#[test]
fn successful_real_query_reports_success_and_available_snapshot() {
    let root = TempDir::new().unwrap();
    let engine = fixture(&root.path().join("success.sqlite"));
    call(&engine, false);
    let row = one(&engine);
    assert_eq!(row["outcome"], "ok", "missing successful query disposition");
    assert!(row["error_class"].is_null());
    assert_eq!(row["snapshot_available"], true);
    assert!(row["snapshot_unavailable_reason"].is_null());
    assert_eq!(row["snapshot"]["event"], "SnapshotStamp");
    engine.close().unwrap();
}

#[test]
fn ordinary_invalid_filter_preserves_typed_error_and_marks_unreached_work() {
    let root = TempDir::new().unwrap();
    let engine = fixture(&root.path().join("invalid-filter.sqlite"));
    call(&engine, true);
    let row = one(&engine);
    assert_eq!(row["outcome"], "error", "missing actual error disposition");
    assert_eq!(row["error_class"], "InvalidFilter");
    assert_eq!(row["snapshot_available"], true);
    assert_eq!(row["snapshot"]["event"], "SnapshotStamp");
    not_reached(&row);
    engine.close().unwrap();
}

#[test]
fn governed_write_stales_frozen_context_before_stamp_without_adjacent_substitution() {
    let root = TempDir::new().unwrap();
    let engine = fixture(&root.path().join("stale-frozen.sqlite"));
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(100), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    engine.write(&[node("newonly evidence")]).unwrap();
    assert!(matches!(engine.search_frozen("alphaonly", &frozen, 0, false, 0.3, 0, false, 10),
        Err(EngineError::FrozenRead(error)) if error.reason == FrozenReadErrorReason::StateDrifted));
    let row = one(&engine);
    assert_eq!(row["outcome"], "error", "missing frozen refusal disposition");
    assert_eq!(row["error_class"], "FrozenRead");
    assert_eq!(row["snapshot_available"], false);
    assert_eq!(row["snapshot_unavailable_reason"], "not_reached");
    assert!(row["snapshot"].is_null());
    not_reached(&row);
    engine.close().unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 8, max_shrink_iters: 32, failure_persistence: None, ..ProptestConfig::default() })]
    #[test]
    fn quiescent_real_call_sequence_preserves_success_error_projection(flags in proptest::collection::vec(any::<bool>(), 1..9)) {
        let root = TempDir::new().unwrap();
        let engine = fixture(&root.path().join("projection.sqlite"));
        for invalid in &flags {
            call(&engine, *invalid);
        }
        let rows = engine.take_slice30_measurements_for_test().unwrap();
        prop_assert_eq!(rows.len(), flags.len());
        for (row, invalid) in rows.iter().zip(&flags) {
            prop_assert_eq!(row["outcome"].as_str(), Some(if *invalid { "error" } else { "ok" }), "missing real outcome projection");
            prop_assert_eq!(row["error_class"].as_str(), if *invalid { Some("InvalidFilter") } else { None });
        }
        prop_assert_eq!(engine.slice30_measurement_stats_for_test().unwrap()["dropped"].as_u64(), Some(0));
        engine.close().unwrap();
    }
}
