#![cfg(feature = "test-hooks")]

use std::collections::BTreeSet;
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use fathomdb_engine::{
    arm_property_corpus_hook_for_test, Engine, EngineError, InitialState, PreparedWrite,
    ProjectionFts, ProjectionRole, ProjectionSpec, ReadView, SourceId,
};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(5);

fn node(body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("property-source").unwrap(),
        logical_id: Some("item".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn history() -> ReadView {
    ReadView { include_superseded: true, ..ReadView::default() }
}
fn setup(path: &std::path::Path) -> Arc<Engine> {
    let opened = Engine::open(path).unwrap();
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "summary".into(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .unwrap();
    opened.engine.write(&[node(r#"{"summary":"corpusneedle old"}"#)]).unwrap();
    Arc::new(opened.engine)
}
fn query(engine: &Engine) -> Result<fathomdb_engine::SearchResult, EngineError> {
    engine.search_projected_text("corpusneedle", "summary", None, &history())
}

#[test]
fn populated_property_corpus_returns_one_snapshot_across_supersession() {
    let dir = TempDir::new().unwrap();
    let engine = setup(&dir.path().join("snapshot.sqlite"));
    let pause = arm_property_corpus_hook_for_test(&engine, false);
    let reader_engine = Arc::clone(&engine);
    let reader = thread::spawn(move || query(&reader_engine));
    pause.wait_ready(TIMEOUT).unwrap();
    engine.write(&[node(r#"{"summary":"corpusneedle new"}"#)]).unwrap();
    pause.release();
    let result = reader.join().unwrap().unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].body, r#"{"summary":"corpusneedle old"}"#);
    assert_eq!(result.results[0].source_id.as_deref(), Some("property-source"));
    assert_eq!(query(&engine).unwrap().results.len(), 2);
    let strict = engine
        .search_projected_text("corpusneedle", "summary", None, &ReadView::default())
        .unwrap();
    assert_eq!(strict.results.len(), 1);
    assert_eq!(strict.results[0].body, r#"{"summary":"corpusneedle new"}"#);
    engine.close().unwrap();
}

#[test]
fn populated_property_snapshot_reports_incomplete_erasure_then_releases_for_retry() {
    let dir = TempDir::new().unwrap();
    let engine = setup(&dir.path().join("erase.sqlite"));
    let pause = arm_property_corpus_hook_for_test(&engine, false);
    let reader_engine = Arc::clone(&engine);
    let reader = thread::spawn(move || query(&reader_engine));
    pause.wait_ready(TIMEOUT).unwrap();
    let erased = engine.erase_source("property-source");
    pause.release();
    let result = reader.join().unwrap().unwrap();
    assert!(
        matches!(erased, Err(EngineError::ErasureIncomplete { ref stage, .. }) if stage == "wal_checkpoint"),
        "{erased:?}"
    );
    assert_eq!(
        result.results.len(),
        1,
        "the pinned pre-erasure snapshot remains internally consistent"
    );
    engine.erase_source("property-source").unwrap();
    #[cfg(debug_assertions)]
    let workers = engine.reader_worker_count_for_test();
    #[cfg(not(debug_assertions))]
    let workers = 8;
    for _ in 0..workers * 2 {
        assert!(query(&engine).unwrap().results.is_empty());
    }
    engine.close().unwrap();
}

#[test]
fn postpopulation_fault_releases_all_readers_and_strict_queries_do_not_consume_it() {
    let dir = TempDir::new().unwrap();
    let engine = setup(&dir.path().join("fault.sqlite"));
    #[cfg(debug_assertions)]
    let workers = engine.reader_worker_count_for_test();
    #[cfg(not(debug_assertions))]
    let workers = 8;
    let pause = arm_property_corpus_hook_for_test(&engine, true);
    pause.release();
    assert_eq!(
        engine
            .search_projected_text("corpusneedle", "summary", None, &ReadView::default())
            .unwrap()
            .results
            .len(),
        1
    );
    assert!(matches!(query(&engine), Err(EngineError::Storage)));
    pause.wait_ready(TIMEOUT).unwrap();
    drop(pause);
    for _ in 0..workers {
        let pause = arm_property_corpus_hook_for_test(&engine, true);
        pause.release();
        assert!(matches!(query(&engine), Err(EngineError::Storage)));
        pause.wait_ready(TIMEOUT).unwrap();
    }
    engine.erase_source("property-source").unwrap();
    engine.write(&[node(r#"{"summary":"corpusneedle replacement"}"#)]).unwrap();
    for _ in 0..workers * 2 {
        let result = query(&engine).unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].body, r#"{"summary":"corpusneedle replacement"}"#);
    }
    engine.close().unwrap();
}

#[test]
fn dropping_property_pause_releases_active_reader_and_cancels_unconsumed_fault() {
    let dir = TempDir::new().unwrap();
    let engine = setup(&dir.path().join("drop.sqlite"));
    let pause = arm_property_corpus_hook_for_test(&engine, false);
    let reader_engine = Arc::clone(&engine);
    let reader = thread::spawn(move || query(&reader_engine));
    pause.wait_ready(TIMEOUT).unwrap();
    drop(pause);
    assert_eq!(reader.join().unwrap().unwrap().results.len(), 1);
    drop(arm_property_corpus_hook_for_test(&engine, true));
    assert_eq!(query(&engine).unwrap().results.len(), 1);
    engine.close().unwrap();
}

#[test]
fn close_completes_after_populated_property_reader_is_released() {
    let dir = TempDir::new().unwrap();
    let engine = setup(&dir.path().join("close.sqlite"));
    let pause = arm_property_corpus_hook_for_test(&engine, false);
    let reader_engine = Arc::clone(&engine);
    let reader = thread::spawn(move || query(&reader_engine));
    pause.wait_ready(TIMEOUT).unwrap();
    let close_engine = Arc::clone(&engine);
    let (started_tx, started_rx) = mpsc::channel();
    let closer = thread::spawn(move || {
        started_tx.send(()).unwrap();
        close_engine.close()
    });
    started_rx.recv_timeout(TIMEOUT).unwrap();
    drop(pause);
    assert_eq!(reader.join().unwrap().unwrap().results.len(), 1);
    closer.join().unwrap().unwrap();
    assert!(query(&engine).is_err());
}

#[test]
fn private_corpus_is_only_used_by_property_existence_views() {
    let dir = TempDir::new().unwrap();
    let engine = setup(&dir.path().join("routes.sqlite"));
    let pause = arm_property_corpus_hook_for_test(&engine, true);
    pause.release();
    for view in
        [ReadView::default(), ReadView { include_out_of_window: true, ..ReadView::default() }]
    {
        assert_eq!(
            engine
                .search_projected_text("corpusneedle", "summary", None, &view)
                .unwrap()
                .results
                .len(),
            1
        );
    }
    for view in [history(), ReadView { include_inactive: true, ..ReadView::default() }] {
        assert_eq!(engine.search_text_only_view("corpusneedle", &view).unwrap().results.len(), 1);
        assert_eq!(engine.search_view("corpusneedle", &view).unwrap().results.len(), 1);
    }
    let inactive = ReadView { include_inactive: true, ..ReadView::default() };
    assert!(matches!(
        engine.search_projected_text("corpusneedle", "summary", None, &inactive),
        Err(EngineError::Storage)
    ));
    pause.wait_ready(TIMEOUT).unwrap();
    drop(pause);
    assert_eq!(
        engine
            .search_projected_text("corpusneedle", "summary", None, &inactive)
            .unwrap()
            .results
            .len(),
        1
    );
    engine.close().unwrap();
}
