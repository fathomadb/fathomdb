use super::*;
use std::sync::Mutex;

/// Native owner retained for the lifetime of blocking maintenance calls.
#[pyclass(name = "ErasureMaintenance")]
pub(super) struct PyErasureMaintenance {
    pub(super) inner: Arc<Mutex<fathomdb_engine::ErasureMaintenance>>,
}

pub(super) fn service_state(value: fathomdb_engine::ErasureServiceState) -> String {
    use fathomdb_engine::ErasureServiceState::*;
    match value {
        NormalWithQueuedRestrictions => "normal_with_queued_restrictions",
        MaintenanceOwned => "maintenance_owned",
        ReadsAllowedWritesFenced => "reads_allowed_writes_fenced",
        ClosedOrUnavailable => "closed_or_unavailable",
    }
    .into()
}

#[pymethods]
impl PyErasureMaintenance {
    /// Heavy non-destructive admission inside the exclusive window.
    fn queue_physical_erasure(
        &self,
        py: Python<'_>,
        selector_json: String,
        key: String,
    ) -> PyResult<String> {
        let selector = serde_json::from_str(&selector_json)
            .map_err(|_| WriteValidationError::new_err("invalid erasure selector"))?;
        let handle = Arc::clone(&self.inner);
        let result = call_engine(py, move || {
            let guard = handle.try_lock().map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => {
                    RustEngineError::Maintenance(fathomdb_engine::MaintenanceError::MaintenanceBusy)
                }
                std::sync::TryLockError::Poisoned(_) => RustEngineError::Storage,
            })?;
            guard.queue_physical_erasure(selector, &key)
        })?;
        serde_json::to_string(&result)
            .map_err(|_| StorageError::new_err("erasure result serialization failed"))
    }

    /// Release temporary ownership; durable physical debt remains owed.
    fn close(&self, py: Python<'_>) -> PyResult<String> {
        let handle = Arc::clone(&self.inner);
        call_engine(py, move || {
            let mut guard = handle.lock().map_err(|_| RustEngineError::Storage)?;
            guard.close().map(service_state)
        })
    }
}
