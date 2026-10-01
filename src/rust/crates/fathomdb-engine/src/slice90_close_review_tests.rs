use super::{
    acquire_lock_without_metadata_mutation, install_post_probe_startup_fault_for_test,
    EmbedderChoice, Engine, EngineConfig, EngineError, EngineOpenError, READER_POOL_SIZE,
};
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};
use tempfile::TempDir;

struct HeldProvider {
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

struct FixedProvider;

impl Embedder for FixedProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-drain-clock", "r1", 2)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0])
    }
}

#[test]
fn failed_startup_after_timed_out_probe_preserves_error_and_cleans_sqlite_profiles() {
    let dir = TempDir::new().expect("temp db");
    let path = dir.path().join("post-probe.sqlite");
    let initial = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(Arc::new(FixedProvider)),
        EngineConfig::default(),
    )
    .expect("first open");
    initial.engine.configure_vector_kind_for_test("doc").expect("enrol vector kind");
    initial.engine.close().expect("close first session");

    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(HeldProvider { entered: entered_tx, release: Mutex::new(release_rx) });
    let (observed_tx, observed_rx) = mpsc::channel();
    install_post_probe_startup_fault_for_test(path.clone(), observed_tx);
    let release_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        release_tx.send(()).expect("release probe provider if still held");
    });
    let result = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(provider),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    );
    let error = match result {
        Ok(opened) => {
            opened.engine.close().expect("close unexpected open");
            panic!("injected post-probe failure was ignored")
        }
        Err(error) => error,
    };
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("probe called provider");
    let observation = observed_rx.recv_timeout(Duration::from_secs(2)).expect("fault observation");
    let retained_at_return = observation.accounting.snapshot().live_workers;
    let owners_at_return = observation.registry.live.lock().expect("registry").len();
    let releases = observation.profile_releases.releases.lock().expect("profile facts");
    let released_profiles = releases.len();
    let profiles_safe = releases.iter().all(|fact| {
        fact.callback_uninstalled && fact.live_connections.is_empty() && fact.live_workers == 0
    });
    drop(releases);
    let admission = acquire_lock_without_metadata_mutation(&path);
    let admission_released = admission.is_ok();
    drop(admission);
    release_thread.join().expect("release thread");
    let started = Instant::now();
    while observation.accounting.snapshot().live_workers != 0 {
        assert!(started.elapsed() < Duration::from_secs(2), "provider worker did not exit");
        std::thread::yield_now();
    }

    assert!(
        matches!(error, EngineOpenError::Io { message } if message == "injected post-probe startup failure")
    );
    assert_eq!(observation.profiles_at_fault, READER_POOL_SIZE + 1);
    assert_eq!(released_profiles, READER_POOL_SIZE + 1);
    assert!(profiles_safe, "profile contexts released before SQLite callback owners");
    assert_eq!(owners_at_return, 0, "all managed SQLite owners must exit");
    assert_eq!(retained_at_return, 1, "timed-out provider remains counted after failed open");
    assert!(admission_released, "failed open must release admission lock");
}

impl Embedder for HeldProvider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice90-drain-clock", "r1", 2)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.entered.send(()).expect("report provider entry");
        self.release.lock().expect("release lock").recv().expect("release provider");
        Ok(vec![1.0, 0.0])
    }
}

#[test]
fn provider_deadline_starts_after_sqlite_quiescence_and_is_never_renewed() {
    let dir = TempDir::new().expect("temp db");
    let (entered_tx, entered_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let provider = Arc::new(HeldProvider { entered: entered_tx, release: Mutex::new(release_rx) });
    let opened = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("drain-clock.sqlite"),
            EmbedderChoice::Caller(provider),
            EngineConfig { embedder_call_timeout_ms: Some(2_000), ..EngineConfig::default() },
        )
        .expect("open real database"),
    );
    opened.engine.embed_dispatch.set_drain_budget_ms_for_test(40);
    let accounting = opened.engine.embed_dispatch.accounting().expect("provider accounting");
    let pending = opened.engine.embed_dispatch.submit_text("held".to_owned()).expect("admit");
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("provider entered");
    let connection = opened.engine.connection.lock().expect("primary SQLite owner");
    connection
        .as_ref()
        .expect("live SQLite owner")
        .execute_batch("BEGIN IMMEDIATE")
        .expect("begin real SQLite transaction");

    let (first_tx, first_rx) = mpsc::channel();
    let first_engine = Arc::clone(&opened);
    let first = std::thread::spawn(move || {
        first_tx.send(first_engine.engine.close()).expect("report first close");
    });
    let started = Instant::now();
    while !opened.engine.reader_pool.shutdown_started_for_test() {
        assert!(started.elapsed() < Duration::from_secs(2), "close did not reach SQLite drain");
        std::thread::yield_now();
    }
    let (second_tx, second_rx) = mpsc::channel();
    let second_engine = Arc::clone(&opened);
    let second = std::thread::spawn(move || {
        second_tx.send(second_engine.engine.close()).expect("report second close");
    });
    std::thread::sleep(Duration::from_millis(120));
    connection
        .as_ref()
        .expect("live SQLite owner")
        .execute_batch("ROLLBACK")
        .expect("finish real SQLite transaction");
    let quiesced_at = Instant::now();
    drop(connection);
    let release_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        release_tx.send(()).expect("release provider if close still waits");
    });
    let first_result = first_rx.recv_timeout(Duration::from_secs(2)).expect("first close");
    let first_after_quiescence = quiesced_at.elapsed();
    let second_result = second_rx.recv_timeout(Duration::from_secs(2)).expect("second close");
    let repeated = opened.engine.close();
    let live_after_close = accounting.snapshot().live_workers;
    let owned_connections = opened.engine.managed_connections.live.lock().expect("registry").len();
    first.join().expect("first close thread");
    second.join().expect("second close thread");
    let drop_started = Instant::now();
    drop(opened);
    let drop_elapsed = drop_started.elapsed();
    release_thread.join().expect("release thread");
    drop(pending);
    let settle_started = Instant::now();
    while accounting.snapshot().live_workers != 0 {
        assert!(settle_started.elapsed() < Duration::from_secs(2), "provider did not exit");
        std::thread::yield_now();
    }

    assert!(matches!(first_result, Err(EngineError::Scheduler)), "first={first_result:?}");
    assert!(matches!(second_result, Err(EngineError::Scheduler)), "second={second_result:?}");
    assert!(matches!(repeated, Err(EngineError::Scheduler)), "repeated={repeated:?}");
    assert!(
        first_after_quiescence >= Duration::from_millis(20),
        "clock started before SQLite quiescence: {first_after_quiescence:?}"
    );
    assert!(
        first_after_quiescence < Duration::from_millis(200),
        "provider drain exceeded one budget: {first_after_quiescence:?}"
    );
    assert!(drop_elapsed < Duration::from_millis(100), "Drop renewed deadline: {drop_elapsed:?}");
    assert_eq!(live_after_close, 1, "retained provider remains counted");
    assert_eq!(owned_connections, 0, "SQLite owners exit before provider deadline");
}
