//! Bounded projection batch fallback regression.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, PreparedWrite, SourceId};
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
    engine.write(&rows).expect("write rows");
    engine.set_projection_scheduler_frozen_for_test(false);
    engine.drain(2_000).expect("fallback must drain after batch provider failure");

    assert!(embedder.batch_calls.load(Ordering::SeqCst) > 0, "batch failure must be reached");
    assert_eq!(
        embedder.individual_calls.load(Ordering::SeqCst),
        rows.len(),
        "every row must use the individual fallback"
    );
}
