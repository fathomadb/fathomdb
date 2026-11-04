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
    /// Release temporary ownership; durable physical debt remains owed.
    fn close(&self, py: Python<'_>) -> PyResult<String> {
        let handle = Arc::clone(&self.inner);
        call_engine(py, move || {
            let mut guard = handle.lock().map_err(|_| RustEngineError::Storage)?;
            guard.close().map(service_state)
        })
    }
}
