//! Fixed provider-slot timeout and projection recovery regression tests.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::ProjectionStatus;
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, PreparedWrite};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const DIM: u32 = 8;

fn unit_vector(dim: u32) -> Vector {
    let mut v = vec![0.0_f32; dim as usize];
    v[0] = 1.0;
    v
}

/// Embedder whose `embed()` blocks while `park` is true (simulating a hung
/// forward pass), then returns a valid unit vector once the test releases
/// it. `calls` counts entries so a test can assert the embed was actually
/// attempted. The park loop sleeps in small increments so releasing it is
/// observed promptly.
#[derive(Debug)]
struct ParkingEmbedder {
    identity: EmbedderIdentity,
    park: Arc<AtomicBool>,
    calls: AtomicU64,
}

impl ParkingEmbedder {
    fn new(park: Arc<AtomicBool>) -> Self {
        Self {
            identity: EmbedderIdentity::new("parking", "rev-a", DIM),
            park,
            calls: AtomicU64::new(0),
        }
    }
}

impl Embedder for ParkingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        // A timed-out call retains the fixed provider slot until this park
        // ends; the safety cap keeps a failed test binary bounded.
        let mut waited = Duration::ZERO;
        let cap = Duration::from_secs(20);
        while self.park.load(Ordering::Relaxed) && waited < cap {
            thread::sleep(Duration::from_millis(10));
            waited += Duration::from_millis(10);
        }
        Ok(unit_vector(DIM))
    }
}

/// Embedder that sleeps a fixed sub-budget delay then returns Ok — models a
/// legitimately-slow cold embed or repeated started timeouts.
#[derive(Debug)]
struct SlowEmbedder {
    identity: EmbedderIdentity,
    delay: Duration,
    calls: AtomicU64,
}

impl SlowEmbedder {
    fn new(delay: Duration) -> Self {
        Self {
            identity: EmbedderIdentity::new("slow", "rev-a", DIM),
            delay,
            calls: AtomicU64::new(0),
        }
    }
}

impl Embedder for SlowEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        thread::sleep(self.delay);
        Ok(unit_vector(DIM))
    }
}

fn fixture_path(name: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    (dir, path)
}

fn doc(body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: body.to_string(),
        source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
        logical_id: None,
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

/// A timed-out call retains the only provider slot until the provider returns.
/// Later rows remain pending without terminal failure or replacement threads.
#[test]
fn hung_embed_retains_slot_and_pending_work_recovers() {
    let (_dir, path) = fixture_path("pr9_fixed_slot_hang");
    let park = Arc::new(AtomicBool::new(true));
    let embedder = Arc::new(ParkingEmbedder::new(park.clone()));
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder.clone()),
        EngineConfig { embedder_call_timeout_ms: Some(80), ..EngineConfig::default() },
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_retry_delays_for_test(&[10, 10]);
    let first = engine.write(&[doc("hung-doc")]).expect("first write");
    assert!(wait_for_calls(&embedder, 1, Duration::from_secs(2)), "provider must start");
    let second = engine.write(&[doc("later-doc")]).expect("second write");
    assert!(matches!(engine.drain(500), Err(fathomdb_engine::EngineError::Scheduler)));
    assert_eq!(embedder.calls.load(Ordering::Relaxed), 1, "no replacement provider call");
    assert_eq!(engine.projection_failure_count_for_test(second.cursor).expect("failures"), 0);
    assert!(!engine.has_vector_for_cursor_for_test(second.cursor).expect("vector"));
    park.store(false, Ordering::Relaxed);
    engine.drain(5_000).expect("recover after provider returns");
    assert!(engine.has_vector_for_cursor_for_test(first.cursor).expect("first vector"));
    assert!(engine.has_vector_for_cursor_for_test(second.cursor).expect("second vector"));
}

#[test]
fn timed_out_embed_does_not_corrupt_subsequent_writes() {
    let (_dir, path) = fixture_path("pr9_fixed_slot_nocorrupt");
    let park = Arc::new(AtomicBool::new(true));
    let embedder = Arc::new(ParkingEmbedder::new(park.clone()));
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder.clone()),
        EngineConfig { embedder_call_timeout_ms: Some(80), ..EngineConfig::default() },
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_retry_delays_for_test(&[10, 10]);
    engine.write(&[doc("hung-doc")]).expect("first write");
    assert!(wait_for_calls(&embedder, 1, Duration::from_secs(2)), "provider must start");
    let healthy = engine.write(&[doc("healthy-doc")]).expect("write while provider is parked");
    assert!(matches!(engine.drain(500), Err(fathomdb_engine::EngineError::Scheduler)));
    park.store(false, Ordering::Relaxed);
    engine.drain(5_000).expect("drain after release");
    assert!(engine.has_vector_for_cursor_for_test(healthy.cursor).expect("healthy vector"));
    assert_eq!(
        engine.projection_status_for_test("doc").expect("status"),
        ProjectionStatus::UpToDate
    );
}

#[test]
fn slow_but_under_budget_embed_succeeds() {
    let (_dir, path) = fixture_path("pr9_watchdog_slow_ok");
    let embedder = Arc::new(SlowEmbedder::new(Duration::from_millis(100)));
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder),
        EngineConfig { embedder_call_timeout_ms: Some(5_000), ..EngineConfig::default() },
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let receipt = engine.write(&[doc("slow-doc")]).expect("write");
    engine.drain(8_000).expect("drain slow embed");
    assert!(engine.has_vector_for_cursor_for_test(receipt.cursor).expect("vector"));
}

#[test]
fn each_started_timeout_spends_one_provider_retry_and_never_commits_late_vector() {
    let (_dir, path) = fixture_path("pr9_started_timeout_budget");
    let embedder = Arc::new(SlowEmbedder::new(Duration::from_millis(60)));
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(embedder.clone()),
        EngineConfig { embedder_call_timeout_ms: Some(20), ..EngineConfig::default() },
    )
    .expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_retry_delays_for_test(&[0, 0]);
    let receipt = engine.write(&[doc("times out on every started call")]).expect("write");
    engine.drain(2_000).expect("three started timeouts exhaust the retry budget");
    assert_eq!(embedder.calls.load(Ordering::Relaxed), 3);
    assert_eq!(engine.projection_status_for_test("doc").expect("status"), ProjectionStatus::Failed);
    assert_eq!(engine.projection_failure_count_for_test(receipt.cursor).expect("failures"), 1);
    assert!(!engine.has_vector_for_cursor_for_test(receipt.cursor).expect("late vector"));
}

fn wait_for_calls(embedder: &ParkingEmbedder, calls: u64, timeout: Duration) -> bool {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if embedder.calls.load(Ordering::Relaxed) >= calls {
            return true;
        }
        thread::sleep(Duration::from_millis(5));
    }
    false
}
