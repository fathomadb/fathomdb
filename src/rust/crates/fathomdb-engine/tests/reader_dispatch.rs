//! EP-18 part A — reader dispatch tries an idle worker before falling back
//! to the round-robin worker.
//!
//! Workers are held inside a real read transaction through the `test-hooks`
//! hold seams. Every hold auto-releases after five seconds, so each wait in
//! these tests stays well under that bound.

#![cfg(debug_assertions)]

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_engine::{Engine, OpenedEngine, PreparedWrite};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const READER_POOL_SIZE: usize = 8;
const QUERY: &str = "dispatch";

fn seeded_engine(name: &str) -> (TempDir, OpenedEngine) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).expect("engine open");
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_string(),
            body: "dispatch witness".to_string(),
            source_id: fathomdb_engine::SourceId::new("test:reader-dispatch")
                .expect("test source id"),
            logical_id: None,
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("seed write");
    (dir, opened)
}

fn align_cursor_to_zero(engine: &Engine) {
    while engine.next_reader_worker_index_for_test() != 0 {
        engine.search(QUERY).expect("cursor alignment search");
    }
}

#[test]
fn a_slow_read_does_not_delay_reads_while_a_reader_is_idle() {
    let (_dir, opened) = seeded_engine("idle_first");
    align_cursor_to_zero(&opened.engine);

    let (snapshot_ready, release) = opened.engine.pause_reader_with_timeout_for_test();
    let held = snapshot_ready
        .recv_timeout(Duration::from_secs(2))
        .expect("worker zero must acquire its snapshot");
    assert_eq!(held, 0);

    let engine = Arc::new(opened.engine);
    let (done_tx, done_rx) = mpsc::channel();
    let reader_engine = Arc::clone(&engine);
    let started = Instant::now();
    let reader = thread::spawn(move || {
        for _ in 0..(READER_POOL_SIZE - 1) {
            let result = reader_engine.search(QUERY);
            if done_tx.send(result).is_err() {
                return;
            }
        }
    });

    for read in 0..(READER_POOL_SIZE - 1) {
        let result = done_rx
            .recv_timeout(Duration::from_millis(1_500))
            .unwrap_or_else(|_| panic!("read {read} is stuck behind the held worker"));
        assert!(!result.expect("search").results.is_empty());
    }
    let elapsed = started.elapsed();

    release.send(()).expect("release held reader");
    reader.join().expect("reader thread");
    assert!(elapsed < Duration::from_secs(3), "seven reads took {elapsed:?} while one was held");
}
