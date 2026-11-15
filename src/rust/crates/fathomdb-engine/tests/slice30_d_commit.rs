//! Tiny real projection commits and session custody; no scale evidence.
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct FixedVector;
impl Embedder for FixedVector {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30-d-commit-unit", "fixed", 768)
    }
    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0; 768];
        vector[0] = 1.0;
        Ok(vector)
    }
}

struct Fixture {
    engine: Engine,
    path: PathBuf,
    _directory: tempfile::TempDir,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("commit.sqlite");
        let engine =
            Engine::open_with_embedder_for_test(&path, Arc::new(FixedVector)).unwrap().engine;
        engine.configure_vector_kind_for_test("doc").unwrap();
        Self { engine, path, _directory: directory }
    }

    fn write(&self, ids: &[&str]) -> Vec<u64> {
        let writes = ids
            .iter()
            .map(|id| PreparedWrite::Node {
                kind: "doc".into(),
                body: format!("literal {id}"),
                logical_id: Some((*id).into()),
                source_id: SourceId::new("test:slice30-d-commit").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        self.engine.write(&writes).unwrap().row_cursors
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

fn physical_counts(path: &Path) -> (u64, u64, u64, u64) {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    connection
        .query_row(
            "SELECT (SELECT COUNT(*) FROM vector_default), \
         (SELECT COUNT(*) FROM _fathomdb_vector_rows), \
         (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date'), \
         (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='failed')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap()
}

#[test]
fn successful_commit_observes_actual_ordered_gate_transaction_and_completion() {
    let fixture = Fixture::new();
    let frozen = Frozen::new(&fixture.engine);
    fixture.engine.drain(5_000).unwrap();
    let observer =
        fixture.engine.begin_slice30_projection_observation_for_test(512, Instant::now()).unwrap();
    let mut scan = fixture.engine.pause_projection_scan_for_test();
    let cursors = fixture.write(&["alpha", "beta"]);
    drop(frozen);
    let scan_ready = scan.wait_ready(Duration::from_secs(5));
    // Hold only future scans; the captured real scan still admits both rows.
    let captured_scan_hold = Frozen::new(&fixture.engine);
    scan.release();
    let drained = fixture.engine.drain(5_000);
    let pending = fixture.engine.read_embedding_readiness();
    let batch = observer.take();
    drop(captured_scan_hold);
    let closed = fixture.engine.close();
    drop(observer);

    scan_ready.expect("bounded first scan must reach the real dispatcher rendezvous");
    drained.expect("actual two-row projection must drain");
    closed.expect("engine must close before observation assertions");
    assert_eq!(pending.unwrap().pending_count, 0);
    assert_eq!(physical_counts(&fixture.path), (2, 2, 2, 0));
    assert_eq!(batch.dropped_records, 0);
    assert!(batch.issues.is_empty());
    let processing = batch
        .records
        .iter()
        .filter(|record| record.phase == "projection_processing")
        .collect::<Vec<_>>();
    assert_eq!(processing.len(), 1);
    let parent = processing[0];
    let mut previous_end = parent.started_ns.unwrap();
    let expected = cursors.into_iter().collect::<BTreeSet<_>>();
    for phase in [
        "projection_commit_gate_wait",
        "projection_commit_begin",
        "projection_commit_prepare",
        "projection_commit_publication",
        "projection_commit_sqlite",
        "projection_commit_completion",
    ] {
        let records =
            batch.records.iter().filter(|record| record.phase == phase).collect::<Vec<_>>();
        assert_eq!(records.len(), 1, "actual commit phase is missing or duplicated: {phase}");
        let record = records[0];
        assert_eq!(record.status, "ok", "successful actual phase is not qualified: {phase}");
        assert_eq!(
            (record.engine_id, record.session_id, record.operation_id),
            (parent.engine_id, parent.session_id, parent.operation_id)
        );
        assert_eq!(record.parent_span_id, Some(parent.span_id));
        let actual = record.facts["cursors"]
            .as_array()
            .expect("actual bounded commit cursors")
            .iter()
            .map(|cursor| cursor.as_u64().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected);
        let start = record.started_ns.expect("actual phase start");
        let end = record.ended_ns.expect("actual phase end");
        assert!(previous_end <= start && start <= end && end <= parent.ended_ns.unwrap());
        previous_end = end;
    }
}

#[test]
fn retired_session_work_cannot_be_relabelled_into_the_next_collector() {
    let fixture = Fixture::new();
    let frozen = Frozen::new(&fixture.engine);
    fixture.engine.drain(5_000).unwrap();
    let old =
        fixture.engine.begin_slice30_projection_observation_for_test(512, Instant::now()).unwrap();
    let mut pause = fixture.engine.pause_projection_worker_after_wal_transaction_for_test();
    let old_cursor = fixture.write(&["old"])[0];
    drop(frozen);
    let reached = pause.wait_ready(Duration::from_secs(5));
    let old_batch = old.take();
    drop(old);
    let next =
        fixture.engine.begin_slice30_projection_observation_for_test(512, Instant::now()).unwrap();
    pause.release();
    let old_drained = fixture.engine.drain(5_000);
    let first = next.take();
    let new_cursor = fixture.write(&["new"])[0];
    let new_drained = fixture.engine.drain(5_000);
    let second = next.take();
    let closed = fixture.engine.close();
    drop(next);

    reached.expect("bounded old worker rendezvous must be observed");
    old_drained.expect("retired session work must finish unchanged");
    new_drained.expect("new session job must finish");
    closed.expect("engine must close before custody assertions");
    assert_eq!(physical_counts(&fixture.path), (2, 2, 2, 0));
    let old_session = old_batch.records.first().expect("actual old session scan").session_id;
    assert!(first
        .records
        .iter()
        .chain(&second.records)
        .all(|record| record.session_id != old_session));
    assert_eq!(first.dropped_records, 0);
    assert_eq!(second.dropped_records, 0);
    assert!(first.issues.is_empty() && second.issues.is_empty());
    for record in first.records.iter().chain(&second.records) {
        if let Some(cursors) = record.facts["cursors"].as_array() {
            assert!(cursors.iter().all(|cursor| cursor.as_u64() != Some(old_cursor)));
        }
    }
    assert!(second.records.iter().any(|record| record.phase == "projection_processing"
        && record.facts["cursors"].as_array().is_some_and(|cursors| cursors
            .iter()
            .any(|cursor| cursor.as_u64() == Some(new_cursor)))));
}
