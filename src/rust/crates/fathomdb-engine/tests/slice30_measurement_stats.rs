use fathomdb_engine::{EmbedderChoice, Engine, InitialState, PreparedWrite, ReadView, SourceId};
use proptest::prelude::*;
use tempfile::TempDir;

fn fixture(path: &std::path::Path) -> Engine {
    let engine = Engine::open_with_choice(path, EmbedderChoice::None).unwrap().engine;
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "alphaonly evidence".into(),
            source_id: SourceId::new("test:slice30-stats").unwrap(),
            logical_id: Some("alpha".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine.slice30_observe_queries_for_test(true).unwrap();
    engine
}

fn query(engine: &Engine) {
    let result = engine
        .search_text_only_filtered_with_snippets("alphaonly", None, &ReadView::default(), 1, None)
        .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.to_prefixed(), "l:alpha");
}

#[test]
fn fixed_capacity_drop_report_survives_drain_toggle_and_resets_only_on_new_engine() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("bounded.sqlite");
    let engine = fixture(&path);
    for _ in 0..32_769 {
        query(&engine);
    }
    // No intermediate drain: bounded retention is an existing correctness control.
    let records = engine.take_slice30_measurements_for_test().unwrap();
    assert_eq!(records.len(), 32_768);
    assert_eq!(records.first().unwrap()["call_id"], 1);
    assert_eq!(records.last().unwrap()["call_id"], 32_768);
    let stats = engine.slice30_measurement_stats_for_test().unwrap();
    assert_eq!(stats["dropped"], 1, "missing cumulative drop report");
    assert_eq!(stats["capacity"], 32_768);
    drop(records);
    assert!(engine.take_slice30_measurements_for_test().unwrap().is_empty());
    assert_eq!(engine.slice30_measurement_stats_for_test().unwrap()["dropped"], 1);
    engine.slice30_observe_queries_for_test(false).unwrap();
    query(&engine);
    engine.slice30_observe_queries_for_test(true).unwrap();
    query(&engine);
    assert_eq!(engine.take_slice30_measurements_for_test().unwrap().len(), 1);
    assert_eq!(engine.slice30_measurement_stats_for_test().unwrap()["dropped"], 1);
    engine.close().unwrap();
    let reopened = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
    assert_eq!(reopened.slice30_measurement_stats_for_test().unwrap()["dropped"], 0);
    assert_eq!(reopened.slice30_measurement_stats_for_test().unwrap()["capacity"], 32_768);
    reopened.close().unwrap();
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 8, max_shrink_iters: 32, failure_persistence: None, ..ProptestConfig::default() })]
    #[test]
    fn quiescent_zero_drop_report_conserves_completed_real_queries(calls in 0usize..17) {
        let root = TempDir::new().unwrap();
        let engine = fixture(&root.path().join("conservation.sqlite"));
        for _ in 0..calls {
            query(&engine);
        }
        let records = engine.take_slice30_measurements_for_test().unwrap();
        let report = engine.slice30_measurement_stats_for_test().unwrap();
        prop_assert_eq!(report["dropped"].as_u64(), Some(0), "missing explicit zero-drop projection");
        prop_assert_eq!(report["capacity"].as_u64(), Some(32_768));
        prop_assert_eq!(records.len() + report["dropped"].as_u64().unwrap() as usize, calls);
        engine.close().unwrap();
    }
}
