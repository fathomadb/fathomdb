use super::*;
use std::sync::Mutex;

/// Native owner retained for the lifetime of blocking maintenance calls.
#[pyclass(name = "ErasureMaintenance")]
pub(super) struct PyErasureMaintenance {
    pub(super) inner: Arc<Mutex<fathomdb_engine::ErasureMaintenance>>,
    #[cfg(feature = "test-hooks")]
    pub(super) pause: Arc<Mutex<Option<ErasurePauseState>>>,
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
    #[cfg(feature = "test-hooks")]
    fn _arm_erasure_pause_for_test(&self) -> PyResult<PyErasurePause> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let mut pause =
            self.pause.lock().map_err(|_| PyValueError::new_err("erasure pause unavailable"))?;
        if pause.is_some() {
            return Err(PyValueError::new_err("erasure pause already armed"));
        }
        *pause = Some((ready_tx, release_rx));
        Ok(PyErasurePause { ready: Mutex::new(ready_rx), release: Mutex::new(Some(release_tx)) })
    }

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

    /// Attempt exact physical completion; caller drop never cancels native work.
    fn complete_physical_erasure(&self, py: Python<'_>, ticket: String) -> PyResult<String> {
        let handle = Arc::clone(&self.inner);
        #[cfg(feature = "test-hooks")]
        let pause = Arc::clone(&self.pause);
        let result = call_engine(py, move || {
            let guard = handle.try_lock().map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => {
                    RustEngineError::Maintenance(fathomdb_engine::MaintenanceError::MaintenanceBusy)
                }
                std::sync::TryLockError::Poisoned(_) => RustEngineError::Storage,
            })?;
            #[cfg(feature = "test-hooks")]
            install_erasure_pause(&pause)?;
            guard.complete_physical_erasure(&ticket)
        })?;
        serde_json::to_string(&result)
            .map_err(|_| StorageError::new_err("erasure completion serialization failed"))
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

#[cfg(feature = "test-hooks")]
pub(crate) type ErasurePauseState = (std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>);

#[cfg(feature = "test-hooks")]
fn install_erasure_pause(
    pause: &Mutex<Option<ErasurePauseState>>,
) -> std::result::Result<(), RustEngineError> {
    if let Some((ready, release)) = pause.lock().map_err(|_| RustEngineError::Storage)?.take() {
        fathomdb_engine::arm_physical_erasure_hook_for_test(
            "after_delete_commit",
            Box::new(move || {
                ready.send(()).map_err(|_| RustEngineError::Storage)?;
                release
                    .recv_timeout(std::time::Duration::from_secs(15))
                    .map_err(|_| RustEngineError::Storage)
            }),
        );
    }
    Ok(())
}

#[cfg(feature = "test-hooks")]
#[pyclass(name = "_ErasurePause")]
pub(super) struct PyErasurePause {
    ready: Mutex<std::sync::mpsc::Receiver<()>>,
    release: Mutex<Option<std::sync::mpsc::Sender<()>>>,
}
#[cfg(feature = "test-hooks")]
#[pymethods]
impl PyErasurePause {
    fn wait_entered(&self, py: Python<'_>) -> PyResult<()> {
        py.detach(|| {
            self.ready
                .lock()
                .map_err(|_| ())?
                .recv_timeout(std::time::Duration::from_secs(10))
                .map_err(|_| ())
        })
        .map_err(|_| PyValueError::new_err("erasure pause was not reached"))
    }
    fn release(&self) {
        if let Some(sender) = self.release.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = sender.send(());
        }
    }
}
