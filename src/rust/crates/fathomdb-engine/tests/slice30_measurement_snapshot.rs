use std::sync::{mpsc, Arc};
use std::time::Duration;

use fathomdb_engine::{
    arm_frozen_after_validation_hook_for_test, EmbedderChoice, Engine, InitialState, PreparedWrite,
    ReadContextV1, ReadView, SearchFilter, SourceId,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;
use tempfile::TempDir;

fn node(id: &str, body: &str, valid_from: i64) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:slice30-stamp").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: Some(valid_from),
        valid_until: None,
    }
}

fn adjacent(path: &std::path::Path) -> (u64, u64, String) {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let generation = connection
        .query_row(
            "SELECT generation FROM _fathomdb_read_visibility_state WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .unwrap();
    let cursor: Option<String> = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key='projection_cursor'",
            [],
            |row| row.get(0),
        )
        .optional()
        .unwrap();
    let body = connection
        .query_row(
            "SELECT body FROM canonical_nodes WHERE logical_id='alpha' AND superseded_at IS NULL",
            [],
            |row| row.get(0),
        )
        .unwrap();
    (generation, cursor.map_or(0, |value| value.parse().unwrap()), body)
}

fn stamp(records: &[Value], generation: u64, cursor: u64) {
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["event"], "QueryComplete");
    let snapshot = &records[0]["snapshot"];
    assert_eq!(snapshot["event"], "SnapshotStamp", "missing reader snapshot stamp");
    assert_eq!(snapshot["generation"], generation);
    assert_eq!(snapshot["projection_cursor"], cursor);
    assert_eq!(snapshot["effective_at"], 100);
    assert!(snapshot["observer_ns"].as_u64().unwrap() > 0);
}

fn fixture(path: &std::path::Path) -> Engine {
    let engine = Engine::open_with_choice(path, EmbedderChoice::None).unwrap().engine;
    engine
        .write(&[node("alpha", "oldonly evidence", 90), node("beta", "futureonly evidence", 200)])
        .unwrap();
    engine.slice30_observe_queries_for_test(true).unwrap();
    engine
}

#[test]
fn quiescent_stamp_matches_literal_result_time_and_independent_store_state() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("quiescent.sqlite");
    let engine = fixture(&path);
    let (generation, cursor, body) = adjacent(&path);
    assert_eq!(body, "oldonly evidence");
    let result = engine
        .search_text_only_filtered_with_snippets(
            "evidence",
            None,
            &ReadView { valid_as_of: Some(100), ..ReadView::default() },
            10,
            None,
        )
        .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.to_prefixed(), "l:alpha");
    assert_eq!(result.results[0].body, "oldonly evidence");
    stamp(&engine.take_slice30_measurements_for_test().unwrap(), generation, cursor);
    engine.close().unwrap();
}

struct ReleaseOnDrop(Option<mpsc::Sender<()>>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

#[test]
fn post_pin_stamp_stays_old_after_governed_commit_while_adjacent_reader_is_new() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("pinned.sqlite");
    let engine = Arc::new(fixture(&path));
    let (g0, cursor0, _) = adjacent(&path);
    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(100), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    let (ready_tx, ready_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    arm_frozen_after_validation_hook_for_test(
        &engine,
        Box::new(move || {
            ready_tx.send(()).unwrap();
            let _ = release_rx.recv_timeout(Duration::from_secs(15));
        }),
    );
    std::thread::scope(|scope| {
        let searching = Arc::clone(&engine);
        let worker = scope.spawn(move || {
            searching.search_frozen("evidence", &frozen, 0, false, 0.3, 0, false, 10)
        });
        // This local is dropped before the scope's implicit joins on every exit.
        let release = ReleaseOnDrop(Some(release_tx));
        ready_rx.recv_timeout(Duration::from_secs(15)).unwrap();
        assert!(engine.take_slice30_measurements_for_test().unwrap().is_empty());
        engine.write(&[node("alpha", "newonly evidence", 90)]).unwrap();
        let (g1, _, body1) = adjacent(&path);
        assert!(g1 > g0);
        assert_eq!(body1, "newonly evidence");
        drop(release);
        let result = worker.join().unwrap().unwrap();
        assert_eq!(result.results.len(), 1);
        assert_eq!(result.results[0].id.to_prefixed(), "l:alpha");
        assert_eq!(result.results[0].body, "oldonly evidence");
        stamp(&engine.take_slice30_measurements_for_test().unwrap(), g0, cursor0);
    });
    engine.close().unwrap();
}
