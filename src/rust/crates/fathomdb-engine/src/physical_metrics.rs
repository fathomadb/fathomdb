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

pub(crate) struct PhysicalAttempt<'a>(&'a Engine);

impl Engine {
    pub(crate) fn physical_attempt(&self) -> PhysicalAttempt<'_> {
        self.physical_maintenance.metrics.begin_lifecycle();
        PhysicalAttempt(self)
    }
}

impl Drop for PhysicalAttempt<'_> {
    fn drop(&mut self) {
        #[cfg(any(test, feature = "test-hooks"))]
        if self.0.physical_maintenance.metrics.enabled.load(Ordering::Relaxed) {
            if let Ok(guard) = self.0.connection.lock() {
                if guard.as_ref().is_none_or(|c| matches!(physical_erasure::active(c), Ok(false))) {
                    self.0
                        .physical_maintenance
                        .metrics
                        .started
                        .lock()
                        .unwrap_or_else(|p| p.into_inner())
                        .take();
                }
            }
        }
        #[cfg(not(any(test, feature = "test-hooks")))]
        let _ = self.0;
    }
}

pub(crate) struct PhysicalWriter<'a> {
    _phase: Option<PhysicalTimer>,
    _holding: PhysicalTimer,
    _scope: MetricScope,
    connection: std::sync::MutexGuard<'a, Option<Connection>>,
}

impl std::ops::Deref for PhysicalWriter<'_> {
    type Target = Option<Connection>;
    fn deref(&self) -> &Self::Target {
        &self.connection
    }
}
impl std::ops::DerefMut for PhysicalWriter<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.connection
    }
}

pub(crate) fn writer<'a>(
    metrics: &Arc<PhysicalMetrics>,
    connection: &'a Mutex<Option<Connection>>,
    finalizing: bool,
) -> Result<PhysicalWriter<'a>, EngineError> {
    let waiting = metrics.timer("physical_writer_wait");
    let phase_wait = finalizing.then(|| metrics.timer("finalization_writer_wait"));
    let connection = connection.lock().map_err(|_| EngineError::Storage)?;
    drop(phase_wait);
    drop(waiting);
    crate::frozen_read::require_clean_visibility(connection.as_ref().ok_or(EngineError::Closing)?)?;
    Ok(PhysicalWriter {
        _scope: scope(metrics),
        _phase: finalizing.then(|| metrics.timer("finalization_writer_hold")),
        _holding: metrics.timer("physical_writer_hold"),
        connection,
    })
}

impl Engine {
    pub(crate) fn physical_writer(
        &self,
        finalizing: bool,
    ) -> Result<PhysicalWriter<'_>, EngineError> {
        writer(&self.physical_maintenance.metrics, &self.connection, finalizing)
    }
}

#[cfg(any(test, feature = "test-hooks"))]
thread_local! {
    static CURRENT: std::cell::RefCell<Option<Arc<PhysicalMetrics>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) struct MetricScope {
    #[cfg(any(test, feature = "test-hooks"))]
    previous: Option<Arc<PhysicalMetrics>>,
}

pub(crate) fn scope(metrics: &Arc<PhysicalMetrics>) -> MetricScope {
    #[cfg(any(test, feature = "test-hooks"))]
    {
        MetricScope { previous: CURRENT.with(|slot| slot.replace(Some(Arc::clone(metrics)))) }
    }
    #[cfg(not(any(test, feature = "test-hooks")))]
    {
        let _ = metrics;
        MetricScope {}
    }
}

impl Drop for MetricScope {
    fn drop(&mut self) {
        #[cfg(any(test, feature = "test-hooks"))]
        CURRENT.with(|slot| {
            slot.replace(self.previous.take());
        });
    }
}

pub(crate) fn stage(name: &'static str) -> PhysicalTimer {
    #[cfg(any(test, feature = "test-hooks"))]
    {
        CURRENT.with(|slot| {
            slot.borrow()
                .as_ref()
                .map_or(PhysicalTimer { sample: None }, |metrics| metrics.timer(name))
        })
    }
    #[cfg(not(any(test, feature = "test-hooks")))]
    {
        let _ = name;
        PhysicalTimer {}
    }
}

#[cfg(test)]
mod slice84_tests {
    use super::*;

    #[test]
    fn admission_profile_records_actual_nested_work_and_restores_scope() {
        let c = crate::vector_storage::slice70_mapping_fixture(false);
        let metrics = Arc::new(PhysicalMetrics::default());
        metrics.enabled.store(true, Ordering::Relaxed);
        {
            let _scope = scope(&metrics);
            crate::physical_erasure::envelope(&c).unwrap();
        }
        crate::physical_erasure::envelope(&c).unwrap();
        let stages = metrics.stages.lock().unwrap();
        for name in [
            "envelope",
            "envelope_pages",
            "envelope_bodies",
            "envelope_vectors",
            "envelope_metadata",
        ] {
            assert_eq!(stages.get(name).map(|row| row.0), Some(1), "missing/escaped stage {name}");
        }
    }
}
