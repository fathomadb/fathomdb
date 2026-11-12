use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(test)]
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;

use serde_json::{json, Value};

use crate::{Engine, EngineError};

pub(crate) struct Collector {
    enabled: AtomicBool,
    buffer: Mutex<Buffer>,
}

struct Buffer {
    capacity: usize,
    records: Vec<Value>,
    dropped: u64,
}

impl Collector {
    pub(crate) fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            buffer: Mutex::new(Buffer {
                capacity: super::CAPACITY,
                records: Vec::new(),
                dropped: 0,
            }),
        }
    }
}

#[cfg(test)]
fn begin(_collector: Arc<Collector>, _worker: usize, _cursors: Vec<(String, u64)>) -> Instant {
    Instant::now()
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
        _origin: Instant,
    ) -> Result<Value, EngineError> {
        let _operation = self.ensure_open()?;
        let mut buffer = self
            .wal_attribution
            .measurement
            .projection
            .buffer
            .lock()
            .map_err(|_| EngineError::Storage)?;
        Ok(json!({
            "records": std::mem::take(&mut buffer.records),
            "capacity": buffer.capacity,
            "dropped": buffer.dropped,
        }))
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
