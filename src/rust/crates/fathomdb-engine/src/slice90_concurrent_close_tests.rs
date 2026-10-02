use super::{EmbedderChoice, Engine, EngineConfig, EngineError};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct HeldProvider {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Embedder for HeldProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-drop", "r1", 2)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.entered.send(()).expect("report provider entry");
        self.release.lock().expect("release lock").recv().expect("release provider");
        Ok(vec![1.0, 0.0])
    }
}

#[test]
fn drop_after_incomplete_close_retains_worker_accounting_without_new_deadline() {
    let dir = TempDir::new().expect("temp db");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(HeldProvider { entered: entered_tx, release: Mutex::new(release_rx) });
    let engine = Engine::open_with_choice_and_config(
        dir.path().join("drop.sqlite"),
        EmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(2_000), ..EngineConfig::default() },
    )
    .expect("open real database")
    .engine;
    let accounting = engine.embed_dispatch.accounting().expect("provider accounting");
    let pending = engine.embed_dispatch.submit_text("held".to_owned()).expect("admit");
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("provider entered");
    assert!(!engine.embed_dispatch.join_until(Instant::now() + Duration::from_millis(20)));

    let close_result = engine.close();
    let drop_started = Instant::now();
    drop(engine);
    let drop_elapsed = drop_started.elapsed();
    let retained_after_drop = accounting.snapshot().live_workers;
    release_tx.send(()).expect("release provider");
    drop(pending);
    let started = Instant::now();
    while accounting.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "provider worker did not exit");
        std::thread::yield_now();
    }

    assert!(matches!(close_result, Err(EngineError::Scheduler)), "close={close_result:?}");
    assert!(drop_elapsed < Duration::from_secs(1), "Drop renewed drain budget: {drop_elapsed:?}");
    assert_eq!(retained_after_drop, 1, "worker-owned accounting must survive Engine Drop");
}

#[test]
fn close_waits_for_active_primary_sqlite_operation_before_releasing_lock() {
    let dir = TempDir::new().expect("temp db");
    let opened = Arc::new(Engine::open(dir.path().join("primary.sqlite")).expect("open"));
    let connection = opened.engine.connection.lock().expect("primary connection");
    connection
        .as_ref()
        .expect("live SQLite owner")
        .execute_batch("BEGIN IMMEDIATE")
        .expect("begin real SQLite transaction");
    let (closed_tx, closed_rx) = mpsc::channel();
    let closing = Arc::clone(&opened);
    let close_thread = std::thread::spawn(move || {
        closed_tx.send(closing.engine.close()).expect("report close");
    });
    let started = Instant::now();
    while !opened.engine.reader_pool.shutdown_started_for_test() {
        assert!(started.elapsed() < Duration::from_secs(2), "close did not reach reader shutdown");
        std::thread::yield_now();
    }
    let premature = closed_rx.recv_timeout(Duration::from_millis(50));
    connection
        .as_ref()
        .expect("live SQLite owner")
        .execute_batch("ROLLBACK")
        .expect("finish real SQLite transaction");
    drop(connection);
    let result = closed_rx.recv_timeout(Duration::from_secs(2)).expect("close after transaction");
    close_thread.join().expect("close thread");

    assert!(premature.is_err(), "close returned before primary transaction finished");
    assert!(result.is_ok(), "close={result:?}");
    assert!(opened.engine.managed_connections.live.lock().expect("registry").is_empty());
}

#[test]
fn concurrent_close_waits_for_active_reader_before_releasing_database_owners() {
    let dir = TempDir::new().expect("temp db");
    let opened = Arc::new(Engine::open(dir.path().join("concurrent-close.sqlite")).expect("open"));
    let (active, release) = opened.engine.pause_reader_after_wal_snapshot_for_test();
    active.wait();

    let (first_tx, first_rx) = mpsc::channel();
    let first_engine = Arc::clone(&opened);
    let first = std::thread::spawn(move || {
        first_tx.send(first_engine.engine.close()).expect("report first close");
    });
    let started = Instant::now();
    while !opened.engine.reader_pool.shutdown_started_for_test() {
        assert!(started.elapsed() < Duration::from_secs(2), "reader shutdown did not begin");
        std::thread::yield_now();
    }

    let (second_tx, second_rx) = mpsc::channel();
    let second_engine = Arc::clone(&opened);
    let second = std::thread::spawn(move || {
        second_tx.send(second_engine.engine.close()).expect("report second close");
    });
    let premature = second_rx.recv_timeout(Duration::from_millis(50));
    let was_premature = premature.is_ok();
    let owners_while_active =
        opened.engine.managed_connections.live.lock().expect("registry").len();
    release.wait();
    let first_result = first_rx.recv_timeout(Duration::from_secs(2)).expect("first close finishes");
    let second_result = match premature {
        Ok(result) => result,
        Err(_) => second_rx.recv_timeout(Duration::from_secs(2)).expect("second close finishes"),
    };
    first.join().expect("first close thread");
    second.join().expect("second close thread");

    assert!(!was_premature, "second close released owners before active reader exited");
    assert!(owners_while_active > 0, "reader connection must remain registered");
    assert!(first_result.is_ok(), "first close={first_result:?}");
    assert!(second_result.is_ok(), "second close={second_result:?}");
    assert!(opened.engine.managed_connections.live.lock().expect("registry").is_empty());
}
