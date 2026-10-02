//! Bounded projection batch fallback regression.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::ProjectionStatus;
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;

const DIMENSION: u32 = 8;

#[derive(Debug)]
struct BatchFailureEmbedder {
    batch_failure_marker: std::path::PathBuf,
    batch_calls: AtomicUsize,
    individual_calls: AtomicUsize,
}

impl Embedder for BatchFailureEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("batch-fallback", "rev-a", DIMENSION)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        self.individual_calls.fetch_add(1, Ordering::SeqCst);
        let mut vector = vec![0.0; DIMENSION as usize];
        vector[0] = 1.0;
        Ok(vector)
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        assert!(inputs.len() >= 2, "the batch failure must cover multiple rows");
        self.batch_calls.fetch_add(1, Ordering::SeqCst);
        std::fs::write(&self.batch_failure_marker, b"batch-returned-error")
            .expect("record batch failure path before returning the provider error");
        Err(EmbedderError::Failed { message: "controlled batch failure".to_owned() })
    }
}

#[derive(Debug)]
struct ParkedBatchEmbedder {
    marker: std::path::PathBuf,
    park: Arc<AtomicBool>,
    batch_calls: AtomicUsize,
    individual_calls: AtomicUsize,
}

impl Embedder for ParkedBatchEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("parked-batch", "rev-a", DIMENSION)
    }

    fn embed(&self, _input: &str) -> Result<Vector, EmbedderError> {
        self.individual_calls.fetch_add(1, Ordering::SeqCst);
        Ok(vec![1.0; DIMENSION as usize])
    }

    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        assert!(inputs.len() >= 2, "the retained slot must follow a real batch call");
        self.batch_calls.fetch_add(1, Ordering::SeqCst);
        std::fs::write(&self.marker, b"batch-started").expect("record entered batch call");
        let started = Instant::now();
        while self.park.load(Ordering::SeqCst) && started.elapsed() < Duration::from_secs(20) {
            thread::sleep(Duration::from_millis(10));
        }
        Ok(vec![vec![1.0; DIMENSION as usize]; inputs.len()])
    }
}

fn node(index: usize) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_owned(),
        body: format!("batch fallback document {index}"),
        source_id: SourceId::new("test:batch-fallback").expect("source id"),
        logical_id: None,
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn batch_failure_releases_permit_before_per_row_fallback() {
    let directory = tempfile::tempdir().expect("test directory");
    let marker = directory.path().join("batch-failed.marker");
    let database = directory.path().join(format!("batch-fallback{SQLITE_SUFFIX}"));
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "batch_failure_child", "--ignored", "--nocapture"])
        .env("FATHOMDB_PROJECTION_BATCH", "1")
        .env("FATHOMDB_SLICE90_BATCH_DB", &database)
        .env("FATHOMDB_SLICE90_BATCH_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start bounded child");

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(7) {
            child.kill().expect("terminate deadlocked child");
            child.wait().expect("reap deadlocked child");
            assert_eq!(
                std::fs::read(&marker).expect("batch failure marker"),
                b"batch-returned-error",
                "timeout is the fallback deadlock only if the batch error path was reached"
            );
            panic!("batch failed, then per-row fallback deadlocked for seven seconds");
        }
        thread::sleep(Duration::from_millis(10));
    };

    assert_eq!(
        std::fs::read(&marker).expect("batch failure marker"),
        b"batch-returned-error",
        "the child must exercise the batch error path"
    );
    assert!(status.success(), "per-row fallback must project every row after the batch error");
}

#[test]
fn timed_out_batch_retains_slot_without_fallback_reacquire() {
    let directory = tempfile::tempdir().expect("test directory");
    let marker = directory.path().join("batch-started.marker");
    let database = directory.path().join(format!("batch-retained{SQLITE_SUFFIX}"));
    let mut child = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "retained_batch_child", "--ignored", "--nocapture"])
        .env("FATHOMDB_PROJECTION_BATCH", "1")
        .env("FATHOMDB_SLICE90_BATCH_DB", &database)
        .env("FATHOMDB_SLICE90_BATCH_MARKER", &marker)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("start bounded child");

    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().expect("poll child") {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(7) {
            child.kill().expect("terminate stuck child");
            child.wait().expect("reap stuck child");
            assert_eq!(
                std::fs::read(&marker).expect("batch started marker"),
                b"batch-started",
                "timeout is relevant only if the batch provider was entered"
            );
            panic!("timed-out batch recovery failed to drain within seven seconds");
        }
        thread::sleep(Duration::from_millis(10));
    };

    assert_eq!(std::fs::read(&marker).expect("batch started marker"), b"batch-started");
    assert!(status.success(), "batch slot retention must recover without a fallback deadlock");
}

#[test]
#[ignore = "run only in the bounded parent subprocess"]
fn batch_failure_child() {
    let database = std::env::var_os("FATHOMDB_SLICE90_BATCH_DB").expect("database path");
    let marker = std::env::var_os("FATHOMDB_SLICE90_BATCH_MARKER").expect("marker path");
    let embedder = Arc::new(BatchFailureEmbedder {
        batch_failure_marker: marker.into(),
        batch_calls: AtomicUsize::new(0),
        individual_calls: AtomicUsize::new(0),
    });
    let opened = Engine::open_with_embedder_for_test(database, embedder.clone()).expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_scheduler_frozen_for_test(true);
    let rows: Vec<_> = (0..16).map(node).collect();
    let receipt = engine.write(&rows).expect("write rows");
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(2_000).expect("fallback must drain after batch provider failure");

    assert_eq!(
        engine.projection_status_for_test("doc").expect("projection status"),
        ProjectionStatus::UpToDate,
        "batch failure must recover through successful per-row projection"
    );
    assert_eq!(receipt.row_cursors.len(), rows.len(), "every input row has a cursor");
    for cursor in receipt.row_cursors {
        assert!(
            engine.has_vector_for_cursor_for_test(cursor).expect("stored vector"),
            "per-row fallback must store a vector for cursor {cursor}"
        );
    }
    assert!(embedder.batch_calls.load(Ordering::SeqCst) > 0, "batch failure must be reached");
    assert_eq!(
        embedder.individual_calls.load(Ordering::SeqCst),
        rows.len(),
        "every row must use the individual fallback"
    );
}

#[test]
#[ignore = "run only in the bounded parent subprocess"]
fn retained_batch_child() {
    let database = std::env::var_os("FATHOMDB_SLICE90_BATCH_DB").expect("database path");
    let marker = std::env::var_os("FATHOMDB_SLICE90_BATCH_MARKER").expect("marker path");
    let park = Arc::new(AtomicBool::new(true));
    let embedder = Arc::new(ParkedBatchEmbedder {
        marker: marker.into(),
        park: Arc::clone(&park),
        batch_calls: AtomicUsize::new(0),
        individual_calls: AtomicUsize::new(0),
    });
    let opened = Engine::open_with_choice_and_config(
        database,
        EmbedderChoice::Caller(embedder.clone()),
        EngineConfig { embedder_call_timeout_ms: Some(50), ..EngineConfig::default() },
    )
    .expect("open");
    let engine = opened.engine;
    assert_eq!(
        engine.config().embedder_pool_size,
        Some(1),
        "retained-slot fixture uses one worker"
    );
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    engine.set_projection_retry_delays_for_test(&[]);
    engine.set_projection_scheduler_frozen_for_test(true);
    let rows: Vec<_> = (0..16).map(node).collect();
    engine.write(&rows).expect("write rows");
    engine.set_projection_scheduler_frozen_for_test(false);
    let drained = engine.drain(200);
    let batch_calls = embedder.batch_calls.load(Ordering::SeqCst);
    let individual_calls = embedder.individual_calls.load(Ordering::SeqCst);
    assert!(matches!(drained, Err(fathomdb_engine::EngineError::Scheduler)));
    assert_eq!(batch_calls, 1, "a timed-out batch still occupies the only provider slot");
    assert_eq!(individual_calls, 0, "fallback cannot start before slot release");
    park.store(false, Ordering::SeqCst);
    engine.drain(3_000).expect("fallback must recover after batch returns");
    assert_eq!(
        engine.projection_status_for_test("doc").expect("projection status"),
        ProjectionStatus::UpToDate
    );
    assert!(embedder.individual_calls.load(Ordering::SeqCst) > 0);
}
