//! Numeric runtime custody on a tiny real store; no measured D execution.
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
use std::sync::Arc;
use std::time::Duration;

struct FixedVector;
impl Embedder for FixedVector {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30-d-state-unit", "fixed", 768)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 768];
        vector[0] = 1.0;
        Ok(vector)
    }
}

struct Frozen<'a>(&'a Engine);
impl<'a> Frozen<'a> {
    fn new(engine: &'a Engine) -> Self {
        engine.set_projection_scheduler_frozen_for_test(true);
        Self(engine)
    }
}
impl Drop for Frozen<'_> {
    fn drop(&mut self) {
        self.0.set_projection_scheduler_frozen_for_test(false);
    }
}

#[test]
fn projection_snapshot_reports_actual_held_and_busy_state_without_sql_inference() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Engine::open_with_embedder_for_test(
        directory.path().join("state.sqlite"),
        Arc::new(FixedVector),
    )
    .unwrap()
    .engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let frozen = Frozen::new(&engine);
    engine.drain(5_000).unwrap();
    let initial = engine.slice30_projection_snapshot_for_test();
    let writes = ["alpha", "beta"].map(|id| PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("literal {id}"),
        logical_id: Some(id.into()),
        source_id: SourceId::new("test:slice30-d-state").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    });
    engine.write(&writes).unwrap();
    let held = engine.slice30_projection_snapshot_for_test();
    let pending = engine.read_embedding_readiness().unwrap().pending_count;
    let mut pause = engine.pause_projection_worker_after_wal_transaction_for_test();
    drop(frozen);
    let reached = pause.wait_ready(Duration::from_secs(5));
    let busy = reached.as_ref().ok().map(|_| engine.slice30_projection_snapshot_for_test());
    // Release every worker before assertions; Drop also releases on unwind.
    pause.release();
    let settled_freeze = Frozen::new(&engine);
    let drained = engine.drain(5_000);
    let settled = engine.slice30_projection_snapshot_for_test();
    drop(settled_freeze);
    let closed = engine.close();
    let after_close = engine.slice30_projection_snapshot_for_test();

    reached.expect("tiny worker must reach the bounded WAL rendezvous");
    drained.expect("released tiny jobs must actually drain");
    closed.expect("tiny engine must close before numeric assertions");
    assert_eq!(pending, 2);
    let initial = initial.expect("numeric snapshot is unavailable for a frozen empty runtime");
    assert_eq!((initial.scheduler_runtime_threads, initial.embedder_pool_size), (2, 5));
    assert_eq!(initial.admission_capacity, 128);
    assert_eq!(
        (initial.active_jobs, initial.active_scans, initial.queued_jobs, initial.in_flight_count),
        (0, 0, 0, 0)
    );
    assert!(initial.frozen && !initial.stopping);
    let held = held.expect("frozen pending rows still require actual numeric runtime state");
    assert_eq!(
        (held.active_jobs, held.active_scans, held.queued_jobs, held.in_flight_count),
        (0, 0, 0, 0)
    );
    assert!(held.frozen && held.pending_scan && !held.stopping);
    let busy = busy
        .expect("bounded worker readiness was observed")
        .expect("busy runtime state is unavailable");
    assert_eq!((busy.active_jobs, busy.queued_jobs, busy.in_flight_count), (2, 0, 2));
    assert!(busy.active_scans <= 1);
    assert!(!busy.frozen && !busy.stopping);
    let settled = settled.expect("settled runtime state is unavailable");
    assert_eq!(
        (settled.active_jobs, settled.active_scans, settled.queued_jobs, settled.in_flight_count),
        (0, 0, 0, 0)
    );
    assert!(settled.frozen && !settled.stopping);
    assert!(matches!(after_close, Err(EngineError::Closing)));
}
