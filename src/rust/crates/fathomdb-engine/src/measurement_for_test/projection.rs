use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde_json::{json, Value};

use crate::projection_runtime::{ProjectionJob, ProjectionRuntimeShared};
use crate::{Engine, EngineError};

pub(crate) struct Collector {
    engine_id: u64,
    enabled: AtomicBool,
    next_batch: AtomicU64,
    buffer: Mutex<Buffer>,
}

struct Buffer {
    capacity: usize,
    records: Vec<Record>,
    dropped: u64,
}

struct StateSample {
    at: Instant,
    queued_jobs: usize,
    active_jobs: usize,
    in_flight_cursors: usize,
}

impl StateSample {
    fn read(shared: &ProjectionRuntimeShared) -> Option<Self> {
        let state = shared.state.lock().ok()?;
        Some(Self {
            at: Instant::now(),
            queued_jobs: state.queued_jobs,
            active_jobs: state.active_jobs,
            in_flight_cursors: state.in_flight.len(),
        })
    }

    fn report(self, origin: Instant) -> (Value, bool) {
        let at = offset(self.at, origin);
        (
            json!({"at_ns":at,"queued_jobs":self.queued_jobs,"active_jobs":self.active_jobs,
            "in_flight_cursors":self.in_flight_cursors}),
            at.is_none(),
        )
    }
}

struct Record {
    engine_id: u64,
    batch_id: u64,
    worker_idx: usize,
    cursors: Vec<(String, u64)>,
    started: Instant,
    ended: Instant,
    start_state: Option<StateSample>,
    end_state: Option<StateSample>,
    disposition: &'static str,
    commit_outcome: Option<&'static str>,
    caught_unwind: Option<bool>,
    cleanup: &'static str,
}

fn offset(at: Instant, origin: Instant) -> Option<u64> {
    at.checked_duration_since(origin).and_then(|duration| u64::try_from(duration.as_nanos()).ok())
}

impl Record {
    fn report(self, origin: Instant) -> Value {
        let start = offset(self.started, origin);
        let end = offset(self.ended, origin);
        let (start_state, start_issue) =
            self.start_state.map(|state| state.report(origin)).unwrap_or((Value::Null, false));
        let (end_state, end_issue) =
            self.end_state.map(|state| state.report(origin)).unwrap_or((Value::Null, false));
        let clock_issue = (start.is_none() || end.is_none() || start_issue || end_issue)
            .then_some("origin_after_event_or_duration_overflow");
        json!({"event":"ProjectionBatchEnd","engine_id":self.engine_id,"batch_id":self.batch_id,
            "worker_idx":self.worker_idx,"job_count":self.cursors.len(),
            "cursors":self.cursors.into_iter().map(|(kind,cursor)|json!({"kind":kind,"write_cursor":cursor})).collect::<Vec<_>>(),
            "start_ns":start,"end_ns":end,"clock_issue":clock_issue,"start_state":start_state,"end_state":end_state,
            "disposition":self.disposition,"commit_outcome":self.commit_outcome,
            "caught_unwind":self.caught_unwind,"cleanup":self.cleanup})
    }
}

impl Collector {
    pub(crate) fn new(engine_id: u64) -> Self {
        Self {
            engine_id,
            enabled: AtomicBool::new(false),
            next_batch: AtomicU64::new(1),
            buffer: Mutex::new(Buffer {
                capacity: super::CAPACITY,
                records: Vec::new(),
                dropped: 0,
            }),
        }
    }

    fn publish(&self, record: Record) {
        let mut buffer = self.buffer.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if buffer.records.len() < buffer.capacity {
            buffer.records.push(record);
        } else {
            buffer.dropped = buffer.dropped.saturating_add(1);
        }
    }
}

pub(crate) struct Batch {
    collector: Arc<Collector>,
    record: Option<Record>,
}

fn enter(
    collector: Arc<Collector>,
    worker: usize,
    cursors: impl FnOnce() -> Vec<(String, u64)>,
) -> Option<Batch> {
    if !collector.enabled.load(Ordering::Relaxed) {
        return None;
    }
    let started = Instant::now();
    let record = Record {
        engine_id: collector.engine_id,
        batch_id: collector.next_batch.fetch_add(1, Ordering::Relaxed),
        worker_idx: worker,
        cursors: cursors(),
        started,
        ended: started,
        start_state: None,
        end_state: None,
        disposition: "aborted",
        commit_outcome: None,
        caught_unwind: None,
        cleanup: "unconfirmed",
    };
    Some(Batch { collector, record: Some(record) })
}

#[cfg(test)]
fn begin(collector: Arc<Collector>, worker: usize, cursors: Vec<(String, u64)>) -> Option<Batch> {
    enter(collector, worker, || cursors)
}

pub(crate) fn begin_jobs(
    collector: Arc<Collector>,
    worker: usize,
    jobs: &[ProjectionJob],
) -> Option<Batch> {
    enter(collector, worker, || jobs.iter().map(|job| (job.kind.clone(), job.cursor)).collect())
}

impl Batch {
    pub(crate) fn capture_start_state(&mut self, shared: &ProjectionRuntimeShared) {
        if let Some(record) = self.record.as_mut() {
            record.start_state = StateSample::read(shared);
        }
    }

    pub(crate) fn finish(
        mut self,
        ended: Instant,
        outcome: &'static str,
        caught_unwind: bool,
        cleanup_confirmed: bool,
        shared: &ProjectionRuntimeShared,
    ) {
        if let Some(mut record) = self.record.take() {
            record.ended = ended;
            record.end_state = StateSample::read(shared);
            record.disposition = "finished";
            record.commit_outcome = Some(outcome);
            record.caught_unwind = Some(caught_unwind);
            record.cleanup = if cleanup_confirmed { "confirmed" } else { "unavailable" };
            self.collector.publish(record);
        }
    }
}

impl Drop for Batch {
    fn drop(&mut self) {
        if let Some(mut record) = self.record.take() {
            record.ended = Instant::now();
            self.collector.publish(record);
        }
    }
}

impl Engine {
    /// Enable or disable this Engine's separate private projection observations.
    ///
    /// Existing records and cumulative loss are retained. Batches sample the
    /// flag at entry; disabling does not cancel an already observed batch.
    /// Returns Closing when admission closes. No scheduler notification occurs.
    #[doc(hidden)]
    pub fn slice30_observe_projection_workers_for_test(
        &self,
        enabled: bool,
    ) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        self.wal_attribution.measurement.projection.enabled.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    /// Drain this Engine's separate ended projection-batch observations.
    ///
    /// The report has records, capacity and cumulative dropped fields. The
    /// fixed32768 bound and loss counter are independent of query observations.
    /// Typed timestamps normalize against the caller's attempt origin outside
    /// API clocks; checked conversion failure is explicit unavailable data.
    /// Drains/toggles do not reset loss. Returns Closing after admission closes,
    /// or Storage if the worker buffer is poisoned.
    #[doc(hidden)]
    pub fn take_slice30_projection_measurements_for_test(
        &self,
        origin: Instant,
    ) -> Result<Value, EngineError> {
        let _operation = self.ensure_open()?;
        let (records, capacity, dropped) = {
            let mut buffer = self
                .wal_attribution
                .measurement
                .projection
                .buffer
                .lock()
                .map_err(|_| EngineError::Storage)?;
            (std::mem::take(&mut buffer.records), buffer.capacity, buffer.dropped)
        };
        let records: Vec<_> = records.into_iter().map(|record| record.report(origin)).collect();
        Ok(json!({"records":records,"capacity":capacity,"dropped":dropped}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EmbedderChoice, InitialState, PreparedWrite, SourceId};
    use rusqlite::{Connection, OpenFlags};

    fn original_error_with_lifetime(
        engine: &Engine,
        connection: &Connection,
        during: impl FnOnce(),
    ) -> rusqlite::Result<()> {
        let _observation = begin(
            Arc::clone(&engine.wal_attribution.measurement.projection),
            0,
            vec![("doc".into(), 1)],
        );
        during();
        connection.query_row("SELECT write_cursor FROM canonical_nodes", [], |row| {
            row.get::<_, String>(0)
        })?;
        Ok(())
    }

    #[test]
    fn actual_readonly_sql_error_retains_aborted_observation_and_original_error() {
        let root = tempfile::Builder::new().prefix("s30-worker-error-").tempdir().unwrap().keep();
        println!("helper error proof: {}", root.display());
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "original SQL error witness".into(),
                source_id: SourceId::new("test:s30-worker-error").unwrap(),
                logical_id: Some("witness".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        engine.slice30_observe_projection_workers_for_test(true).unwrap();
        let origin = Instant::now();
        let connection =
            Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let error = original_error_with_lifetime(&engine, &connection, || {}).unwrap_err();
        assert!(
            matches!(&error, rusqlite::Error::InvalidColumnType(0, name, rusqlite::types::Type::Integer) if name == "write_cursor")
        );
        println!("original readonly InvalidColumnType propagated: {error:?}");
        drop(connection);
        let report = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        engine.close().unwrap();
        std::fs::write(root.join("observations.json"), serde_json::to_vec_pretty(&report).unwrap())
            .unwrap();
        assert_eq!(
            report["records"].as_array().unwrap().len(),
            1,
            "missing aborted helper observation after original SQLite error"
        );
        let row = &report["records"][0];
        assert_eq!(row["disposition"], "aborted");
        assert_eq!(row["cleanup"], "unconfirmed");
        assert!(row["commit_outcome"].is_null());
        assert!(row["end_ns"].as_u64().unwrap() > row["start_ns"].as_u64().unwrap());
        assert_eq!(report["dropped"], 0);
    }

    fn fixture() -> (std::path::PathBuf, Engine, Connection) {
        let root = tempfile::Builder::new().prefix("s30-worker-edges-").tempdir().unwrap().keep();
        println!("worker edge proof: {}", root.display());
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_choice(&store, EmbedderChoice::None).unwrap().engine;
        let receipt = engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "readonly lifetime witness".into(),
                source_id: SourceId::new("test:s30-worker-edges").unwrap(),
                logical_id: Some("witness".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        assert_eq!(receipt.cursor, 1);
        let connection =
            Connection::open_with_flags(store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        engine.slice30_observe_projection_workers_for_test(true).unwrap();
        (root, engine, connection)
    }

    #[test]
    fn disabled_collection_retains_ended_and_already_entered_lifetimes() {
        let (root, engine, connection) = fixture();
        let origin = Instant::now();
        assert!(matches!(
            original_error_with_lifetime(&engine, &connection, || {}),
            Err(rusqlite::Error::InvalidColumnType(..))
        ));
        engine.slice30_observe_projection_workers_for_test(false).unwrap();
        let ended = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        engine.slice30_observe_projection_workers_for_test(true).unwrap();
        assert!(matches!(
            original_error_with_lifetime(&engine, &connection, || {
                engine.slice30_observe_projection_workers_for_test(false).unwrap();
            }),
            Err(rusqlite::Error::InvalidColumnType(..))
        ));
        let entered = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        drop(connection);
        engine.close().unwrap();
        std::fs::write(
            root.join("toggles.json"),
            serde_json::to_vec_pretty(&json!({"ended":ended,"entered":entered})).unwrap(),
        )
        .unwrap();
        assert_eq!(
            [
                ended["records"].as_array().unwrap().len(),
                entered["records"].as_array().unwrap().len()
            ],
            [1, 1],
            "disable must retain both ended and already entered observations"
        );
        assert_eq!(ended["dropped"], 0);
        assert_eq!(entered["dropped"], 0);
    }

    #[test]
    fn actual_fixed_bound_and_cumulative_loss_survive_toggles_and_drains() {
        let (root, engine, connection) = fixture();
        let origin = Instant::now();
        for _ in 0..32769 {
            assert!(matches!(
                original_error_with_lifetime(&engine, &connection, || {}),
                Err(rusqlite::Error::InvalidColumnType(..))
            ));
        }
        engine.slice30_observe_projection_workers_for_test(false).unwrap();
        let first = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        engine.slice30_observe_projection_workers_for_test(true).unwrap();
        let second = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        engine.slice30_observe_projection_workers_for_test(false).unwrap();
        let third = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        drop(connection);
        engine.close().unwrap();
        std::fs::write(
            root.join("loss.json"),
            serde_json::to_vec_pretty(&json!({"first":first,"second":second,"third":third}))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(
            first["dropped"], 1,
            "missing actual cumulative loss after32769 ended helper observations"
        );
        assert_eq!(first["capacity"], 32768);
        assert_eq!(first["records"].as_array().unwrap().len(), 32768);
        for report in [second, third] {
            assert_eq!(report["dropped"], 1);
            assert_eq!(report["capacity"], 32768);
            assert!(report["records"].as_array().unwrap().is_empty());
        }
    }

    #[test]
    fn origin_after_actual_events_reports_unavailable_not_zero() {
        let (root, engine, connection) = fixture();
        assert!(matches!(
            original_error_with_lifetime(&engine, &connection, || {}),
            Err(rusqlite::Error::InvalidColumnType(..))
        ));
        let future = Instant::now() + std::time::Duration::from_secs(1);
        let report = engine.take_slice30_projection_measurements_for_test(future).unwrap();
        drop(connection);
        engine.close().unwrap();
        std::fs::write(root.join("clock.json"), serde_json::to_vec_pretty(&report).unwrap())
            .unwrap();
        let row = report["records"].as_array().unwrap().first().cloned().unwrap_or(Value::Null);
        assert!(
            row["clock_issue"].as_str().is_some(),
            "missing unavailable-clock observation after actual prior event"
        );
        assert_eq!(report["records"].as_array().unwrap().len(), 1);
        assert!(row["start_ns"].is_null());
        assert!(row["end_ns"].is_null());
    }
}
