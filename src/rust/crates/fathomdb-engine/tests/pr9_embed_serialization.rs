//! The configured provider pool bounds projection inference concurrency.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::lifecycle::ProjectionStatus;
use fathomdb_engine::{Engine, PreparedWrite};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const DIM: u32 = 8;

fn unit_vector() -> Vector {
    let mut v = vec![0.0_f32; DIM as usize];
    v[0] = 1.0;
    v
}

/// Records peak concurrency of `embed()`. Each call bumps an in-flight
/// counter, holds it for `delay` (so overlapping calls are observable),
/// and updates the shared `max_in_flight` high-water mark.
#[derive(Debug)]
struct ConcurrencyProbeEmbedder {
    identity: EmbedderIdentity,
    in_flight: AtomicUsize,
    max_in_flight: Arc<AtomicUsize>,
    delay: Duration,
}

impl ConcurrencyProbeEmbedder {
    fn new(max_in_flight: Arc<AtomicUsize>, delay: Duration) -> Self {
        Self {
            identity: EmbedderIdentity::new("probe", "rev-a", DIM),
            in_flight: AtomicUsize::new(0),
            max_in_flight,
            delay,
        }
    }
}

impl Embedder for ConcurrencyProbeEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let cur = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(cur, Ordering::SeqCst);
        thread::sleep(self.delay);
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok(unit_vector())
    }
}

fn fixture_path(name: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    (dir, path)
}

/// The default provider pool has one slot even with two projection workers.
#[test]
fn default_provider_pool_has_one_active_slot() {
    let (_dir, path) = fixture_path("pr9_serialize");
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let embedder =
        Arc::new(ConcurrencyProbeEmbedder::new(max_in_flight.clone(), Duration::from_millis(50)));
    let opened = Engine::open_with_embedder_for_test(&path, embedder).expect("open");
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");

    // A worker grabs a whole PROJECTION_COMMIT_BATCH (64) at once, and the
    // dispatcher enqueues up to PROJECTION_INFLIGHT_LIMIT (128) per scan — so
    // we write > one batch (80 docs) to ensure both workers pick up work and
    // would embed concurrently if the engine permitted it.
    let nodes: Vec<PreparedWrite> = (0..80)
        .map(|i| PreparedWrite::Node {
            kind: "doc".to_string(),
            body: format!("serialize-doc-{i}"),
            source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
            logical_id: None,
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    engine.write(&nodes).expect("write");
    engine.drain(30_000).expect("drain");
    assert_eq!(
        engine.projection_status_for_test("doc").expect("status"),
        ProjectionStatus::UpToDate,
        "all docs must project successfully"
    );

    let peak = max_in_flight.load(Ordering::SeqCst);
    assert_eq!(
        peak, 1,
        "embeds must run one at a time engine-side (PR-9 oversubscription fix); \
         observed {peak} concurrent embed() calls"
    );
}
