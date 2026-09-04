use std::sync::{Arc, Barrier};
use std::thread;

use fathomdb_engine::{
    arm_reader_search_hook_for_test, Engine, EngineError, FrozenReadErrorReason, InitialState,
    PreparedWrite, ReadContextV1, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

fn node(id: &str, body: &str, valid_from: Option<i64>) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.to_string()),
        kind: "doc".to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:slice35-races").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until: None,
    }
}

fn edge(id: &str, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new("test:slice35-races").unwrap(),
        logical_id: Some(id.to_string()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn strict_context() -> ReadContextV1 {
    ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap()
}

#[test]
fn authentication_precedes_ordinary_limit_and_depth_validation() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("precedence{SQLITE_SUFFIX}"))).unwrap();
    let mut frozen = opened.engine.freeze_read_context(&strict_context()).unwrap();
    frozen.token.push('0');

    for result in [
        opened.engine.search_frozen("", &frozen, 0, false, 0.3, 0, false, 0).map(|_| ()),
        opened.engine.search_expand_frozen("", &frozen, 4, 0).map(|_| ()),
    ] {
        assert!(matches!(
            result,
            Err(EngineError::FrozenRead(error))
                if error.reason == FrozenReadErrorReason::TokenMalformed
        ));
    }
}

#[test]
fn consume_rejects_visibility_generation_regression_observed_in_process() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("regression{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[node("root", "needle", None)]).unwrap();
    let old = opened.engine.freeze_read_context(&strict_context()).unwrap();

    let connection = Connection::open(&path).unwrap();
    let generation: i64 = connection
        .query_row(
            "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_read_visibility_state SET generation=?1 WHERE singleton=1",
            [generation + 1],
        )
        .unwrap();
    opened.engine.freeze_read_context(&strict_context()).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_read_visibility_state SET generation=?1 WHERE singleton=1",
            [generation],
        )
        .unwrap();

    assert!(matches!(
        opened.engine.search_frozen("needle", &old, 0, false, 0.3, 0, false, 10),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::StateUnavailable
    ));
}

#[test]
fn frozen_expansion_honors_out_of_window_relaxation() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("validity{SQLITE_SUFFIX}"))).unwrap();
    let future = 4_102_444_800_i64;
    opened
        .engine
        .write(&[
            node("root", "unique relaxed root", None),
            node("future", "future neighbor", Some(future)),
            edge("edge", "root", "future"),
        ])
        .unwrap();
    let context = ReadContextV1::new(
        ReadView { include_out_of_window: true, ..ReadView::default() },
        SearchFilter::default(),
    )
    .unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();

    let result = opened.engine.search_expand_frozen("unique relaxed root", &frozen, 1, 10).unwrap();
    assert!(result.expanded.iter().any(|(node, _)| node.logical_id == "future"));
}

#[test]
fn mutation_committed_before_snapshot_pin_causes_drift_without_results() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("before-pin{SQLITE_SUFFIX}"))).unwrap();
    let engine = Arc::new(opened.engine);
    engine.write(&[node("root", "needle", None)]).unwrap();
    let frozen = engine.freeze_read_context(&strict_context()).unwrap();
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
        thread::spawn(move || engine.search_frozen("needle", &frozen, 0, false, 0.3, 0, false, 10))
    };
    ready.wait();
    engine.write(&[node("later", "needle later", None)]).unwrap();
    release.wait();

    assert!(matches!(
        worker.join().unwrap(),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::StateDrifted
    ));
}

#[test]
fn centering_state_change_invalidates_a_frozen_context() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("mean-drift{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let frozen = opened.engine.freeze_read_context(&strict_context()).unwrap();

    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_embedder_profiles SET mean_vec=X'00000000'
             WHERE profile='default'",
            [],
        )
        .unwrap();

    assert!(matches!(
        opened.engine.search_frozen("needle", &frozen, 0, false, 0.3, 0, false, 10),
        Err(EngineError::FrozenRead(error))
            if error.reason == FrozenReadErrorReason::StateDrifted
    ));
}
