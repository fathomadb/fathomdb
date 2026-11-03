use super::*;

#[derive(Default)]
pub(crate) struct PhysicalMetrics {
    #[cfg(any(test, feature = "test-hooks"))]
    enabled: AtomicBool,
    #[cfg(any(test, feature = "test-hooks"))]
    stages: Mutex<BTreeMap<&'static str, (u64, u64, u64)>>,
    #[cfg(any(test, feature = "test-hooks"))]
    started: Mutex<Option<Instant>>,
}

pub(crate) struct PhysicalTimer {
    #[cfg(any(test, feature = "test-hooks"))]
    sample: Option<(Arc<PhysicalMetrics>, &'static str, Instant)>,
}

impl PhysicalMetrics {
    pub(crate) fn timer(self: &Arc<Self>, stage: &'static str) -> PhysicalTimer {
        #[cfg(any(test, feature = "test-hooks"))]
        {
            PhysicalTimer {
                sample: self
                    .enabled
                    .load(Ordering::Relaxed)
                    .then(|| (Arc::clone(self), stage, Instant::now())),
            }
        }
        #[cfg(not(any(test, feature = "test-hooks")))]
        {
            let _ = stage;
            PhysicalTimer {}
        }
    }

    pub(crate) fn begin_lifecycle(&self) {
        #[cfg(any(test, feature = "test-hooks"))]
        if self.enabled.load(Ordering::Relaxed) {
            self.started.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(Instant::now);
        }
    }

    pub(crate) fn complete_lifecycle(&self) {
        #[cfg(any(test, feature = "test-hooks"))]
        if let Some(start) = self.started.lock().unwrap_or_else(|p| p.into_inner()).take() {
            self.record("completed_lifecycle", start.elapsed());
        }
    }

    #[cfg(any(test, feature = "test-hooks"))]
    fn record(&self, stage: &'static str, elapsed: Duration) {
        let ns = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        let mut stages = self.stages.lock().unwrap_or_else(|p| p.into_inner());
        let entry = stages.entry(stage).or_default();
        entry.0 = entry.0.saturating_add(1);
        entry.1 = entry.1.saturating_add(ns);
        entry.2 = entry.2.max(ns);
    }
}

impl Drop for PhysicalTimer {
    fn drop(&mut self) {
        #[cfg(any(test, feature = "test-hooks"))]
        if let Some((metrics, stage, start)) = &self.sample {
            metrics.record(stage, start.elapsed());
        }
    }
}

#[cfg(feature = "test-hooks")]
impl Engine {
    /// Enable opt-in actual physical-cleanup stage counters for qualification.
    #[doc(hidden)]
    pub fn enable_physical_cleanup_metrics_for_test(&self) {
        self.physical_maintenance.metrics.enabled.store(true, Ordering::Relaxed);
    }

    /// Return overlapping stage count, total and maximum nanoseconds; no latency guarantee.
    #[doc(hidden)]
    pub fn physical_cleanup_metrics_for_test(&self) -> serde_json::Value {
        let pending = self
            .physical_maintenance
            .metrics
            .started
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .is_some();
        let stages =
            self.physical_maintenance.metrics.stages.lock().unwrap_or_else(|p| p.into_inner());
        let mut values: serde_json::Map<String, serde_json::Value> = stages
            .iter()
            .map(|(name, (count, total, max))| {
                (name.to_string(), serde_json::json!({"count":count,"total_ns":total,"max_ns":max}))
            })
            .collect();
        values.insert("_unfinished_lifecycle".into(), serde_json::json!(pending));
        serde_json::Value::Object(values)
    }
}
