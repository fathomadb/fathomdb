//! Bounded rendezvous proves the overlap classifier, never a scientific schedule.
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    arm_reader_search_hook_for_test, Engine, InitialState, PreparedWrite, ProjectionRole,
    ProjectionSpec, ProjectionVector, SourceId,
};
use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

struct FixedVector;
impl Embedder for FixedVector {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30-d-overlap-unit", "fixed", 768)
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

fn elapsed_ns(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_nanos()).unwrap()
}

#[test]
fn actual_pending_bracket_inside_same_call_and_worker_requires_observed_overlap() {
    let directory = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_embedder_for_test(
            directory.path().join("overlap.sqlite"),
            Arc::new(FixedVector),
        )
        .unwrap()
        .engine,
    );
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "doc".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(ProjectionVector::default()),
                source: None,
            }],
            &[],
        )
        .unwrap();
    let frozen = Frozen::new(&engine);
    engine.drain(5_000).unwrap();
    let origin = Instant::now();
    let observer =
        Arc::new(engine.begin_slice30_projection_observation_for_test(512, origin).unwrap());
    let mut scan = engine.pause_projection_scan_for_test();
    let mut worker_pause = engine.pause_projection_worker_after_wal_transaction_for_test();
    let writes = ["alpha", "beta"].map(|id| PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("needle {id}"),
        logical_id: Some(id.into()),
        source_id: SourceId::new("test:slice30-d-overlap").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    });
    engine.write(&writes).unwrap();
    drop(frozen);
    let scan_ready = scan.wait_ready(Duration::from_secs(5));
    let captured_scan_hold = Frozen::new(&engine);
    scan.release();
    let worker_ready = worker_pause.wait_ready(Duration::from_secs(5));
    let (reader_ready, pending_start, pending, pending_end, joined) = std::thread::scope(|scope| {
        let mut worker_pause = worker_pause;
        let reader_pause = arm_reader_search_hook_for_test(&engine);
        let reader_engine = Arc::clone(&engine);
        let reader_observer = Arc::clone(&observer);
        let query = scope.spawn(move || {
            reader_observer.with_operation(17, || reader_engine.search_text_only("needle"))
        });
        let reader_ready = reader_pause.wait_ready(Duration::from_secs(5));
        let pending_start = elapsed_ns(origin);
        let pending = engine.read_embedding_readiness();
        let pending_end = elapsed_ns(origin);
        reader_pause.release();
        let joined = query.join();
        worker_pause.release();
        (reader_ready, pending_start, pending, pending_end, joined)
    });
    let drained = engine.drain(5_000);
    let final_pending = engine.read_embedding_readiness();
    let batch = observer.take();
    drop(captured_scan_hold);
    let closed = engine.close();
    drop(observer);

    scan_ready.expect("bounded captured scan must be observed");
    worker_ready.expect("actual projection worker must reach its WAL rendezvous");
    reader_ready.expect("same natural call must reach its bounded reader rendezvous");
    let results =
        joined.expect("natural caller must join").expect("natural text query must succeed");
    let ids = results.results.iter().map(|hit| hit.id.value.as_str()).collect::<BTreeSet<_>>();
    assert_eq!(ids, ["alpha", "beta"].into_iter().collect());
    drained.expect("released real projections must drain");
    closed.expect("engine must close before classification assertions");
    assert_eq!(final_pending.unwrap().pending_count, 0);
    let physical = Connection::open_with_flags(
        directory.path().join("overlap.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let counts: (u64, u64, u64) = physical.query_row(
        "SELECT (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_vector_rows), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date')",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(counts, (2, 2, 2));
    let pending = pending.expect("actual bracketed readiness read must succeed").pending_count;
    assert_eq!(pending, 2);
    assert_eq!(batch.dropped_records, 0);
    assert!(batch.issues.is_empty());
    let query = batch
        .records
        .iter()
        .find(|record| record.phase == "api_text" && record.operation_id == 17)
        .expect("actual same-call API interval");
    let worker = batch
        .records
        .iter()
        .find(|record| record.phase == "projection_processing")
        .expect("actual worker processing interval");
    assert_eq!((query.engine_id, query.session_id), (worker.engine_id, worker.session_id));
    assert_eq!((query.status, worker.status), ("ok", "ok"));
    let start = query.started_ns.unwrap().max(worker.started_ns.unwrap());
    let end = query.ended_ns.unwrap().min(worker.ended_ns.unwrap());
    assert!(
        start < end && start <= pending_start && pending_start <= pending_end && pending_end <= end
    );
    println!(
        "actual overlap engine={} session={} query=[{},{}] worker=[{},{}] pending=[{},{}] count={}",
        query.engine_id,
        query.session_id,
        query.started_ns.unwrap(),
        query.ended_ns.unwrap(),
        worker.started_ns.unwrap(),
        worker.ended_ns.unwrap(),
        pending_start,
        pending_end,
        pending
    );
    let cell = serde_json::json!({"cell_id":"tiny-c-overlap-n2-r0","attempt_id":"tiny-rendezvous-one","n":2,"repeat":0,"layer":"C"});
    let records = batch
        .records
        .iter()
        .map(|record| serde_json::json!({"cell":cell,"record":record}))
        .collect::<Vec<_>>();
    let case = serde_json::json!({"cell":cell,"engine_id":query.engine_id,"session_id":query.session_id,
        "operation_id":17,"records":records,"dropped_records":batch.dropped_records,"issues":batch.issues,
        "pending":[{"cell":cell,"engine_id":query.engine_id,"session_id":query.session_id,
            "started_ns":pending_start,"ended_ns":pending_end,"pending_count":pending}]});
    let path = directory.path().join("actual-case.json");
    std::fs::write(&path, serde_json::to_vec(&case).unwrap()).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../..");
    let output = std::process::Command::new(root.join(".venv/bin/python"))
        .arg(root.join("dev/experiments/0.8.29-slice30-measurements/d_overlap.py"))
        .arg(&path)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        report["verdict"], "OBSERVED_OVERLAP",
        "actual exact-execution overlap was not qualified"
    );
}
