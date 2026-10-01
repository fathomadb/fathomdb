//! Projection capacity stays durable while a timed-out provider still owns its slot.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, EngineError, PreparedWrite, SourceId};
use tempfile::TempDir;

const DIMENSION: u32 = 8;
const _: fn(&Engine, u64) = Engine::set_embed_timeout_ms_for_test;

#[derive(Debug)]
struct FirstCallParked {
    calls: AtomicUsize,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
}

#[derive(Debug)]
struct FailOnceTimed {
    calls: AtomicUsize,
    first_failed: mpsc::Sender<Instant>,
    retry_started: mpsc::Sender<Instant>,
}

impl Embedder for FailOnceTimed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("fail-once-timed", "rev-a", DIMENSION)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        let now = Instant::now();
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.first_failed.send(now).expect("report first failure");
            return Err(EmbedderError::Failed { message: "first call fails".to_owned() });
        }
        self.retry_started.send(now).expect("report retry entry");
        Ok(vec![2.0; DIMENSION as usize])
    }
}

impl Embedder for FirstCallParked {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("first-call-parked", "rev-a", DIMENSION)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        if self.calls.fetch_add(1, Ordering::SeqCst) == 0 {
            self.entered.send(()).expect("report first call entry");
            self.release
                .lock()
                .expect("release lock")
                .recv_timeout(Duration::from_secs(10))
                .expect("release parked provider");
            return Ok(vec![1.0; DIMENSION as usize]);
        }
        Ok(vec![2.0; DIMENSION as usize])
    }
}

fn node(body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_owned(),
        body: body.to_owned(),
        source_id: SourceId::new("test:projection-capacity").expect("source id"),
        logical_id: None,
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn timed_out_provider_keeps_slot_and_later_work_durable_until_return() {
    let directory = TempDir::new().expect("test directory");
    let database = directory.path().join("parked.fathomdb.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let provider = Arc::new(FirstCallParked {
        calls: AtomicUsize::new(0),
        entered,
        release: Mutex::new(release_rx),
    });
    let opened = Engine::open_with_choice_and_config(
        &database,
        EmbedderChoice::Caller(provider.clone()),
        EngineConfig {
            embedder_pool_size: Some(1),
            embedder_call_timeout_ms: Some(80),
            ..EngineConfig::default()
        },
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_embed_timeout_ms_for_test(80);
    engine.set_projection_retry_delays_for_test(&[10, 10]);
    let first = engine.write(&[node("first parked")]).expect("first write");
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("provider entered");
    let second = engine.write(&[node("second pending")]).expect("second write stays live");

    assert!(
        matches!(engine.drain(600), Err(EngineError::Scheduler)),
        "a timed-out call still occupies the only physical provider slot"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1, "no replacement call may start");
    assert_eq!(engine.projection_failure_count_for_test(second.cursor).expect("failures"), 0);
    assert!(!engine.has_vector_for_cursor_for_test(second.cursor).expect("second vector"));

    release.send(()).expect("release parked call");
    engine.drain(4_000).expect("pending work recovers after slot returns");
    assert!(engine.has_vector_for_cursor_for_test(first.cursor).expect("first vector"));
    assert!(engine.has_vector_for_cursor_for_test(second.cursor).expect("second vector"));
    let first_blob = engine.read_vector_blob_for_test(first.cursor as i64).expect("first blob");
    let expected: Vec<u8> = (0..DIMENSION).flat_map(|_| 2.0_f32.to_le_bytes()).collect();
    assert_eq!(first_blob, expected, "late timed-out output must never commit");
}

#[test]
fn projection_wait_cancels_on_close_and_reopen_recovers_pending_row() {
    let directory = TempDir::new().expect("test directory");
    let database = directory.path().join("close-pending.fathomdb.sqlite");
    let (entered, entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let provider = Arc::new(FirstCallParked {
        calls: AtomicUsize::new(0),
        entered,
        release: Mutex::new(release_rx),
    });
    let config = EngineConfig {
        embedder_pool_size: Some(1),
        embedder_call_timeout_ms: Some(5_000),
        ..EngineConfig::default()
    };
    let opened = Engine::open_with_choice_and_config(
        &database,
        EmbedderChoice::Caller(provider.clone()),
        config.clone(),
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let receipt = engine.write(&[node("pending across close")]).expect("write");
    entered_rx.recv_timeout(Duration::from_secs(2)).expect("provider entered");

    // Phase 2.7 will add truthful incomplete-close provider-drain reporting.
    let started = Instant::now();
    engine.close().expect("current close cancels projection waiter");
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "projection waiter cancellation must not await provider deadline"
    );
    release.send(()).expect("release old provider call");

    let reopened =
        Engine::open_with_choice_and_config(&database, EmbedderChoice::Caller(provider), config)
            .expect("reopen");
    reopened.engine.drain(4_000).expect("pending row recovers");
    assert!(reopened.engine.has_vector_for_cursor_for_test(receipt.cursor).expect("vector"));
    assert_eq!(
        reopened.engine.projection_failure_count_for_test(receipt.cursor).expect("failures"),
        0
    );
}

#[test]
fn provider_retry_delay_uses_one_absolute_deadline_during_wake_storm() {
    let directory = TempDir::new().expect("test directory");
    let database = directory.path().join("retry-wake-storm.fathomdb.sqlite");
    let (first_failed, first_rx) = mpsc::channel();
    let (retry_started, retry_rx) = mpsc::channel();
    let provider =
        Arc::new(FailOnceTimed { calls: AtomicUsize::new(0), first_failed, retry_started });
    let opened = Engine::open_with_choice_and_config(
        &database,
        EmbedderChoice::Caller(provider.clone()),
        EngineConfig { embedder_call_timeout_ms: Some(1_000), ..EngineConfig::default() },
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_retry_delays_for_test(&[300]);
    let receipt = engine.write(&[node("retry delay survives wakeups")]).expect("write");
    let first = first_rx.recv_timeout(Duration::from_secs(2)).expect("first provider failure");

    // Every freeze call notifies the same runtime condition variable used by
    // projection retry waits. The short pauses only pace the wake storm; the
    // provider timestamps are the deadline oracle.
    for _ in 0..30 {
        engine.set_projection_scheduler_frozen_for_test(true);
        std::thread::sleep(Duration::from_millis(3));
    }
    let retry = retry_rx.recv_timeout(Duration::from_secs(2)).expect("retry provider entry");
    assert!(
        retry.duration_since(first) >= Duration::from_millis(280),
        "runtime notifications must not shorten the 300ms provider retry delay"
    );
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(2_000).expect("successful retry drains");
    assert!(engine.has_vector_for_cursor_for_test(receipt.cursor).expect("vector"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}

#[test]
fn close_interrupts_long_provider_retry_wait_and_preserves_pending_row() {
    let directory = TempDir::new().expect("test directory");
    let database = directory.path().join("retry-close.fathomdb.sqlite");
    let (first_failed, first_rx) = mpsc::channel();
    let (retry_started, retry_rx) = mpsc::channel();
    let provider =
        Arc::new(FailOnceTimed { calls: AtomicUsize::new(0), first_failed, retry_started });
    let opened = Engine::open_with_choice_and_config(
        &database,
        EmbedderChoice::Caller(provider.clone()),
        EngineConfig::default(),
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_retry_delays_for_test(&[5_000]);
    let receipt = engine.write(&[node("retry survives close")]).expect("write");
    first_rx.recv_timeout(Duration::from_secs(2)).expect("first provider failure");
    assert!(matches!(engine.drain(80), Err(EngineError::Scheduler)));

    let started = Instant::now();
    engine.close().expect("current close cancels retry wait");
    assert!(started.elapsed() < Duration::from_secs(1), "close must wake retry wait promptly");
    assert!(retry_rx.try_recv().is_err(), "retry may not start after close");

    let reopened = Engine::open_with_choice_and_config(
        &database,
        EmbedderChoice::Caller(provider.clone()),
        EngineConfig::default(),
    )
    .expect("reopen");
    reopened.engine.drain(2_000).expect("pending row recovers");
    assert!(reopened.engine.has_vector_for_cursor_for_test(receipt.cursor).expect("vector"));
    assert_eq!(
        reopened.engine.projection_failure_count_for_test(receipt.cursor).expect("failures"),
        0
    );
}
