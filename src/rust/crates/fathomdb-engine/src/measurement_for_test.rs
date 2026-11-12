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
        })
    }
}

pub(super) struct Completion {
    collector: Arc<Collector>,
    call_id: u64,
    worker: usize,
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
            }));
        } else {
            self.collector.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
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

    /// Drain this Engine's private completed query records in completion order.
    ///
    /// Records contain engine/call IDs and reader worker index. At most 32768
    /// records are retained between drains. This first completion transport does
    /// not yet expose snapshot, predicate, error disposition or drop metadata.
    /// Returns `Closing` when admission closes or `Storage` on a poisoned buffer.
    #[doc(hidden)]
    pub fn take_slice30_measurements_for_test(&self) -> Result<Vec<Value>, EngineError> {
        let _operation = self.ensure_open()?;
        let mut records =
            self.wal_attribution.measurement.records.lock().map_err(|_| EngineError::Storage)?;
        Ok(std::mem::take(&mut *records))
    }
}
