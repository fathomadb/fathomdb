use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::{Engine, EngineError};

const CAPACITY: usize = 32_768;
static NEXT_ENGINE: AtomicU64 = AtomicU64::new(1);

pub(super) struct Collector {
    engine_id: u64,
    enabled: AtomicBool,
    next_call: AtomicU64,
    records: Mutex<Vec<Value>>,
    dropped: AtomicU64,
}

impl Collector {
    pub(super) fn new() -> Self {
        Self {
            engine_id: NEXT_ENGINE.fetch_add(1, Ordering::Relaxed),
            enabled: AtomicBool::new(false),
            next_call: AtomicU64::new(1),
            records: Mutex::new(Vec::new()),
            dropped: AtomicU64::new(0),
        }
    }

    pub(super) fn begin(self: &Arc<Self>, worker: usize) -> Option<Completion> {
        self.enabled.load(Ordering::Relaxed).then(|| Completion {
            collector: Arc::clone(self),
            call_id: self.next_call.fetch_add(1, Ordering::Relaxed),
            worker,
            snapshot: None,
            predicates: serde_json::Map::new(),
            blocks: serde_json::Map::new(),
            outcome: None,
            error_class: None,
            stamp_attempted: false,
            active_blocks: BTreeMap::new(),
        })
    }
}

pub(super) struct Completion {
    collector: Arc<Collector>,
    call_id: u64,
    worker: usize,
    snapshot: Option<Value>,
    predicates: serde_json::Map<String, Value>,
    blocks: serde_json::Map<String, Value>,
    outcome: Option<&'static str>,
    error_class: Option<&'static str>,
    stamp_attempted: bool,
    // Only the fixed vector/text call sites populate this map.
    active_blocks: BTreeMap<&'static str, std::time::Instant>,
}

impl Completion {
    pub(super) fn begin_block(&mut self, name: &'static str) -> std::time::Instant {
        let started = std::time::Instant::now();
        self.active_blocks.insert(name, started);
        started
    }

    pub(super) fn finish_outcome<T>(
        &mut self,
        result: &Result<T, crate::search::SearchReaderError>,
    ) {
        use crate::search::SearchReaderError;
        self.outcome = Some(if result.is_ok() { "ok" } else { "error" });
        self.error_class = result.as_ref().err().map(|error| match error {
            SearchReaderError::Sqlite(_) => "Sqlite",
            SearchReaderError::Evidence(_) => "Evidence",
            SearchReaderError::InvalidFilter(_) => "InvalidFilter",
            SearchReaderError::RerankerDevicePolicy(_) => "RerankerDevicePolicy",
            SearchReaderError::FrozenRead(_) => "FrozenRead",
            SearchReaderError::WriteValidation => "WriteValidation",
            SearchReaderError::InvalidArgument(_) => "InvalidArgument",
        });
        if result.is_err() {
            let error_class = self.error_class;
            for (name, started) in std::mem::take(&mut self.active_blocks) {
                self.blocks.entry(name.to_owned()).or_insert_with(|| json!({
                    "invoked": true,
                    "elapsed_ns": u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX),
                    "rows": null, "reason": "entered_error", "error_class": error_class,
                    "degraded": null,
                }));
            }
            for name in [
                "has_source_dependencies",
                "vector_arm_requires_fallback",
                "all_nodes_directly_eligible",
            ] {
                self.predicates.entry(name.to_owned()).or_insert_with(|| json!({
                    "invoked": false, "value": null, "elapsed_ns": null, "reason": "not_reached",
                }));
            }
            for name in ["vector", "text"] {
                self.blocks.entry(name.to_owned()).or_insert_with(|| {
                    json!({
                        "invoked": false, "rows": null, "elapsed_ns": null, "reason": "not_reached",
                        "degraded": null,
                    })
                });
            }
        }
    }

    pub(super) fn complete_block(
        &mut self,
        name: &'static str,
        elapsed: std::time::Duration,
        rows: usize,
    ) {
        self.active_blocks.remove(name);
        let mut record = json!({
            "invoked": true,
            "elapsed_ns": u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX),
            "rows": rows,
        });
        if name == "vector" {
            record["degraded"] = json!(false);
        }
        self.blocks.insert(name.into(), record);
    }

    pub(super) fn skipped_block(
        &mut self,
        name: &'static str,
        reason: &'static str,
        degraded: bool,
    ) {
        self.blocks.insert(
            name.into(),
            json!({
                "invoked": false, "elapsed_ns": null, "rows": null,
                "reason": reason, "degraded": degraded,
            }),
        );
    }

    pub(super) fn skipped_predicate(&mut self, name: &'static str, reason: &'static str) {
        self.predicates.insert(
            name.into(),
            json!({
                "invoked": false, "value": null, "elapsed_ns": null, "reason": reason,
            }),
        );
    }

    pub(super) fn attach_sql(&mut self, name: &'static str, sql: Value) {
        if let Some(span) = self.predicates.get_mut(name) {
            span["sql"] = sql;
        }
    }

    pub(super) fn stamp(&mut self, connection: &rusqlite::Connection, effective_at: i64) {
        self.stamp_attempted = true;
        let started = std::time::Instant::now();
        let generation = crate::frozen_read::load_visibility_generation(connection);
        let cursor = crate::projection_commit::load_projection_cursor(connection);
        let observer_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
        if let (Ok(generation), Ok(cursor)) = (generation, cursor) {
            self.snapshot = Some(json!({
                "event": "SnapshotStamp",
                "generation": generation,
                "projection_cursor": cursor,
                "effective_at": effective_at,
                "observer_ns": observer_ns,
            }));
        }
    }
}

impl Drop for Completion {
    fn drop(&mut self) {
        // Observation cannot change the product's return or unwind path.
        let mut records = self.collector.records.lock().unwrap_or_else(|error| error.into_inner());
        if records.len() < CAPACITY {
            records.push(json!({
                "event": "QueryComplete",
                "engine_id": self.collector.engine_id,
                "call_id": self.call_id,
                "worker": self.worker,
                "snapshot": self.snapshot,
                "predicates": self.predicates,
                "blocks": self.blocks,
                "outcome": self.outcome,
                "error_class": self.error_class,
                "snapshot_available": self.snapshot.is_some(),
                "snapshot_unavailable_reason": if self.snapshot.is_some() {
                    None
                } else if self.stamp_attempted {
                    Some("observer_read_failed")
                } else {
                    Some("not_reached")
                },
            }));
        } else {
            self.collector.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub(super) fn measure_bool<E>(
    observation: &mut Option<Completion>,
    name: &'static str,
    operation: impl FnOnce() -> Result<bool, E>,
) -> Result<bool, E> {
    let Some(observation) = observation.as_mut() else {
        return operation();
    };
    let started = std::time::Instant::now();
    let result = operation();
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    observation.predicates.insert(
        name.into(),
        json!({"invoked": true, "value": result.as_ref().ok(), "elapsed_ns": elapsed_ns}),
    );
    result
}

impl Engine {
    /// Enable or disable private query observations on this Engine only.
    ///
    /// Existing records remain available to drain. A query samples the flag at
    /// reader entry; disabling does not cancel an already observed call.
    /// Returns `EngineError::Closing` after admission closes. This non-default
    /// measurement feature has no SDK or production configuration counterpart.
    #[doc(hidden)]
    pub fn slice30_observe_queries_for_test(&self, enabled: bool) -> Result<(), EngineError> {
        let _operation = self.ensure_open()?;
        self.wal_attribution.measurement.enabled.store(enabled, Ordering::Relaxed);
        Ok(())
    }

    /// Report the fixed capacity and cumulative records dropped by this Engine.
    ///
    /// The object has `capacity` and `dropped` integer fields. Drops
    /// accumulate for this Engine's lifetime: enable/disable and record drains
    /// do not reset them; a newly opened Engine starts at zero. This read is
    /// not atomic with a separate record drain; conservation uses quiescent
    /// calls, and reports compare cumulative differences rather than summing
    /// repeated readings. Returns `Closing` when admission closes.
    #[doc(hidden)]
    pub fn slice30_measurement_stats_for_test(&self) -> Result<Value, EngineError> {
        let _operation = self.ensure_open()?;
        Ok(json!({
            "capacity": CAPACITY,
            "dropped": self.wal_attribution.measurement.dropped.load(Ordering::Relaxed),
        }))
    }

    /// Drain this Engine's private completed query records in completion order.
    ///
    /// Records contain engine/call IDs and reader worker index. At most 32768
    /// records are retained between drains. Records include same-reader snapshot,
    /// actual predicate timing and vector/text block observations when available.
    /// Cumulative drops are returned separately by the stats getter. The reader
    /// result's outcome/category is reported without changing that result; an API
    /// failure before reader dispatch has no new query record.
    /// Returns `Closing` when admission closes or `Storage` on a poisoned buffer.
    #[doc(hidden)]
    pub fn take_slice30_measurements_for_test(&self) -> Result<Vec<Value>, EngineError> {
        let _operation = self.ensure_open()?;
        let mut records =
            self.wal_attribution.measurement.records.lock().map_err(|_| EngineError::Storage)?;
        Ok(std::mem::take(&mut *records))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EmbedderChoice, InitialState, PreparedWrite, SourceId};
    use proptest::prelude::*;
    use tempfile::TempDir;

    fn readonly_sql_error_after_block_entry(block: &'static str) -> Value {
        let root = TempDir::new().unwrap();
        let path = root.path().join("helper.sqlite");
        let engine = Engine::open_with_choice(&path, EmbedderChoice::None).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "alphaonly evidence".into(),
                source_id: SourceId::new("test:slice30-block-error").unwrap(),
                logical_id: Some("alpha".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        engine.slice30_observe_queries_for_test(true).unwrap();
        let connection = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let mut observation = engine.wal_attribution.measurement.begin(0).unwrap();
        if block == "text" {
            let started = observation.begin_block("vector");
            let count: i64 = connection
                .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0))
                .unwrap();
            assert_eq!(count, 1);
            observation.complete_block("vector", started.elapsed(), 1);
        }
        let _started = observation.begin_block(block);
        let result: Result<(), crate::search::SearchReaderError> = connection
            .query_row("SELECT body FROM canonical_nodes WHERE logical_id='alpha' AND superseded_at IS NULL", [], |row| row.get::<_, i64>(0))
            .map(|_| ())
            .map_err(crate::search::SearchReaderError::Sqlite);
        assert!(matches!(
            &result,
            Err(crate::search::SearchReaderError::Sqlite(rusqlite::Error::InvalidColumnType(..)))
        ));
        observation.finish_outcome(&result);
        assert!(matches!(
            &result,
            Err(crate::search::SearchReaderError::Sqlite(rusqlite::Error::InvalidColumnType(..)))
        ));
        drop(observation);
        let records = engine.take_slice30_measurements_for_test().unwrap();
        assert_eq!(records.len(), 1);
        let record = records.into_iter().next().unwrap();
        assert_eq!(record["outcome"], "error");
        assert_eq!(record["error_class"], "Sqlite");
        if block == "vector" {
            assert_eq!(record["blocks"]["text"]["invoked"], false);
            assert_eq!(record["blocks"]["text"]["reason"], "not_reached");
        } else {
            let completed = &record["blocks"]["vector"];
            assert_eq!(completed["invoked"], true);
            assert_eq!(completed["rows"], 1);
            assert!(completed["elapsed_ns"].as_u64().unwrap() > 0);
            assert!(completed["reason"].is_null());
        }
        drop(connection);
        engine.close().unwrap();
        record
    }

    fn entered_error(record: &Value, block: &str) {
        let span = &record["blocks"][block];
        assert_eq!(span["invoked"], true, "entered SQL error was mislabelled NOT_REACHED");
        assert_eq!(span["reason"], "entered_error");
        assert!(span["elapsed_ns"].as_u64().unwrap() > 0);
        assert!(span["rows"].is_null());
    }

    #[test]
    fn vector_entry_sql_error_remains_entered_and_text_unreached() {
        entered_error(&readonly_sql_error_after_block_entry("vector"), "vector");
    }

    #[test]
    fn text_entry_sql_error_remains_entered_and_completed_vector_survives() {
        entered_error(&readonly_sql_error_after_block_entry("text"), "text");
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 8, max_shrink_iters: 32, failure_persistence: None, ..ProptestConfig::default() })]
        #[test]
        fn entered_error_projection_covers_both_fixed_block_labels(text in any::<bool>()) {
            let block = if text { "text" } else { "vector" };
            let record = readonly_sql_error_after_block_entry(block);
            prop_assert_eq!(record["blocks"][block]["invoked"].as_bool(), Some(true), "entered SQL error lost its entry state");
            prop_assert_eq!(record["blocks"][block]["reason"].as_str(), Some("entered_error"));
            prop_assert!(record["blocks"][block]["rows"].is_null());
        }
    }
}
