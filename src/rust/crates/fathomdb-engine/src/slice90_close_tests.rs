use super::reader_pool::ReaderRequest;
use super::{EmbedderChoice, Engine, EngineConfig, EngineError, ReadView};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn close_discards_queued_reader_work_after_active_snapshot_reaches_boundary() {
    let dir = TempDir::new().expect("temp db");
    let opened = Arc::new(Engine::open(dir.path().join("readers.sqlite")).expect("open"));
    let (active, release) = opened.engine.pause_reader_after_wal_snapshot_for_test();
    active.wait();

    let worker_count =
        opened.engine.reader_pool.live_workers_for_test().load(std::sync::atomic::Ordering::SeqCst);
    let mut queued = Vec::new();
    for index in 0..worker_count * 4 {
        let (respond, received) = mpsc::sync_channel(1);
        assert!(
            opened
                .engine
                .reader_pool
                .dispatch(ReaderRequest::GetById {
                    logical_ids: vec![format!("absent-{index}")],
                    view: ReadView::default(),
                    respond,
                })
                .is_ok(),
            "enqueue bounded reader request"
        );
        if index % worker_count == 0 {
            queued.push(received);
        }
    }

    let (closed_tx, closed_rx) = mpsc::channel();
    let closing = Arc::clone(&opened);
    let close_thread = std::thread::spawn(move || {
        closed_tx.send(closing.engine.close()).expect("report close");
    });
    let started = Instant::now();
    while !opened.engine.reader_pool.shutdown_started_for_test() {
        assert!(started.elapsed() < Duration::from_secs(2), "close did not stop admission");
        std::thread::yield_now();
    }
    let still_live = opened.engine.managed_connections.live.lock().expect("registry").len();
    release.wait();
    let close_result = closed_rx.recv_timeout(Duration::from_secs(2)).expect("close after release");
    close_thread.join().expect("close thread");
    let queued_outcomes: Vec<_> =
        queued.into_iter().map(|reply| reply.recv_timeout(Duration::from_secs(1))).collect();

    assert!(still_live > 0, "active snapshot must retain its SQLite owner");
    assert!(close_result.is_ok(), "close={close_result:?}");
    assert!(
        queued_outcomes.iter().all(Result::is_err),
        "queued work must be cancelled after close: {queued_outcomes:?}"
    );
    assert!(opened.engine.managed_connections.live.lock().expect("registry").is_empty());
}

struct HeldProvider {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

impl Embedder for HeldProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-close", "r1", 2)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.entered.send(()).expect("report provider entry");
        self.release.lock().expect("release lock").recv().expect("release provider");
        Ok(vec![1.0, 0.0])
    }
}

#[test]
fn incomplete_close_quiesces_database_and_later_close_reports_provider_exit() {
    let dir = TempDir::new().expect("temp db");
    let path = dir.path().join("close.sqlite");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(HeldProvider { entered: entered_tx, release: Mutex::new(release_rx) });
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(provider.clone()),
        EngineConfig { embedder_call_timeout_ms: Some(2_000), ..EngineConfig::default() },
    )
    .expect("open real database");
    let accounting = opened.engine.embed_dispatch.accounting().expect("provider accounting");
    let pending = opened.engine.embed_dispatch.submit_text("held".to_owned()).expect("admit");
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("provider entered");

    // Seed the existing one-deadline seam with a short test budget.
    assert!(!opened.engine.embed_dispatch.join_until(Instant::now() + Duration::from_millis(20)));
    let first = opened.engine.close();
    let live_at_return = accounting.snapshot().live_workers;
    let connections_at_return =
        opened.engine.managed_connections.live.lock().expect("registry").len();
    let reopened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(provider),
        EngineConfig::default(),
    )
    .expect("SQLite admission lock released after quiescence");
    reopened.engine.close().expect("close reopened database");
    let repeated = opened.engine.close();

    release_tx.send(()).expect("release held provider");
    drop(pending);
    let started = Instant::now();
    while accounting.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "provider worker did not exit");
        std::thread::yield_now();
    }
    let final_close = opened.engine.close();

    assert!(matches!(first, Err(EngineError::Scheduler)), "first={first:?}");
    assert_eq!(live_at_return, 1, "retained provider must still be counted");
    assert_eq!(connections_at_return, 0, "all SQLite owners must leave before provider drain");
    assert!(matches!(repeated, Err(EngineError::Scheduler)), "repeated={repeated:?}");
    assert!(final_close.is_ok(), "final={final_close:?}");
}

#[test]
fn timed_out_close_retains_provider_until_later_close_releases_it() {
    let dir = TempDir::new().expect("temp db");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(HeldProvider { entered: entered_tx, release: Mutex::new(release_rx) });
    let weak = Arc::downgrade(&provider);
    let opened = Engine::open_with_choice_and_config(
        dir.path().join("retain.sqlite"),
        EmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(2_000), ..EngineConfig::default() },
    )
    .expect("open real database");
    let accounting = opened.engine.embed_dispatch.accounting().expect("provider accounting");
    let pending = opened.engine.embed_dispatch.submit_text("held".to_owned()).expect("admit");
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("provider entered");

    assert!(!opened.engine.embed_dispatch.join_until(Instant::now() + Duration::from_millis(20)));
    let timed_out = opened.engine.close();
    let retained = weak.upgrade().is_some();

    release_tx.send(()).expect("release held provider");
    drop(pending);
    let started = Instant::now();
    while accounting.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "provider worker did not exit");
        std::thread::yield_now();
    }
    let final_close = opened.engine.close();

    assert!(matches!(timed_out, Err(EngineError::Scheduler)), "timed_out={timed_out:?}");
    assert!(retained, "timed-out close must not release an active provider");
    assert!(final_close.is_ok(), "final={final_close:?}");
    assert!(weak.upgrade().is_none(), "successful close must release the provider");
}
