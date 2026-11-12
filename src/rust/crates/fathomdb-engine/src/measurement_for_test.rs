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
}

impl Completion {
    pub(super) fn complete_block(
        &mut self,
        name: &'static str,
        elapsed: std::time::Duration,
        rows: usize,
    ) {
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
    /// Cumulative drops are returned separately by the stats getter; final query
    /// error disposition is not yet supplied.
    /// Returns `Closing` when admission closes or `Storage` on a poisoned buffer.
    #[doc(hidden)]
    pub fn take_slice30_measurements_for_test(&self) -> Result<Vec<Value>, EngineError> {
        let _operation = self.ensure_open()?;
        let mut records =
            self.wal_attribution.measurement.records.lock().map_err(|_| EngineError::Storage)?;
        Ok(std::mem::take(&mut *records))
    }
}
