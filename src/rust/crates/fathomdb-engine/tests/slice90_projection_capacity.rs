//! Projection capacity stays durable while a timed-out provider still owns its slot.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, EngineError, PreparedWrite, SourceId};
use tempfile::TempDir;

const DIMENSION: u32 = 8;

#[derive(Debug)]
struct FirstCallParked {
    calls: AtomicUsize,
    entered: mpsc::Sender<()>,
    release: Mutex<mpsc::Receiver<()>>,
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
    #[cfg(feature = "test-hooks")]
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
