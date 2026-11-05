use super::*;

/// Native owner retained by every spawned blocking maintenance call.
#[napi]
pub struct ErasureMaintenance {
    pub(crate) inner: Arc<Mutex<fathomdb_engine::ErasureMaintenance>>,
    #[cfg(feature = "test-hooks")]
    pub(crate) pause: Arc<Mutex<Option<ErasurePauseState>>>,
}

pub(crate) fn service_state(value: fathomdb_engine::ErasureServiceState) -> String {
    use fathomdb_engine::ErasureServiceState::*;
    match value {
        NormalWithQueuedRestrictions => "normal_with_queued_restrictions",
        MaintenanceOwned => "maintenance_owned",
        ReadsAllowedWritesFenced => "reads_allowed_writes_fenced",
        ClosedOrUnavailable => "closed_or_unavailable",
    }
    .into()
}

#[napi]
impl ErasureMaintenance {
    /// Heavy non-destructive admission inside the exclusive window.
    #[napi]
    pub async fn queue_physical_erasure(
        &self,
        selector: JsonValue,
        key: String,
    ) -> Result<JsonValue> {
        let selector = serde_json::from_value(selector)
            .map_err(|_| engine_error_to_napi(RustEngineError::WriteValidation))?;
        let handle = Arc::clone(&self.inner);
        let result = call_engine(move || {
            let guard = handle.try_lock().map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => {
                    RustEngineError::Maintenance(fathomdb_engine::MaintenanceError::MaintenanceBusy)
                }
                std::sync::TryLockError::Poisoned(_) => RustEngineError::Storage,
            })?;
            guard.queue_physical_erasure(selector, &key)
        })
        .await?;
        serde_json::to_value(result).map_err(|_| engine_error_to_napi(RustEngineError::Storage))
    }

    /// Attempt exact physical completion; a lost Promise never cancels native work.
    #[napi]
    pub async fn complete_physical_erasure(&self, ticket: String) -> Result<JsonValue> {
        let handle = Arc::clone(&self.inner);
        #[cfg(feature = "test-hooks")]
        let pause = Arc::clone(&self.pause);
        let result = call_engine(move || {
            let guard = handle.try_lock().map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => {
                    RustEngineError::Maintenance(fathomdb_engine::MaintenanceError::MaintenanceBusy)
                }
                std::sync::TryLockError::Poisoned(_) => RustEngineError::Storage,
            })?;
            #[cfg(feature = "test-hooks")]
            install_erasure_pause(&pause)?;
            guard.complete_physical_erasure(&ticket)
        })
        .await?;
        serde_json::to_value(result).map_err(|_| engine_error_to_napi(RustEngineError::Storage))
    }

    /// Reattach only a recorded authenticated original; never creates a sink.
    #[napi]
    pub async fn reattach_original_erasure_sink(&self, ticket: String, path: String) -> Result<()> {
        validate_ffi_string_napi(&path)?;
        let handle = Arc::clone(&self.inner);
        call_engine(move || {
            let guard = handle.try_lock().map_err(|error| match error {
                std::sync::TryLockError::WouldBlock => {
                    RustEngineError::Maintenance(fathomdb_engine::MaintenanceError::MaintenanceBusy)
                }
                std::sync::TryLockError::Poisoned(_) => RustEngineError::Storage,
            })?;
            guard.reattach_original_erasure_sink(&ticket, &path)
        })
        .await
    }

    /// Release temporary ownership; durable physical debt remains owed.
    #[napi]
    pub async fn close(&self) -> Result<String> {
        let handle = Arc::clone(&self.inner);
        call_engine(move || {
            let mut guard = handle.lock().map_err(|_| RustEngineError::Storage)?;
            guard.close().map(service_state)
        })
        .await
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
#[napi]
/// Development-only bounded rendezvous for actual in-flight cleanup ownership.
pub struct ErasurePause {
    ready: Arc<Mutex<std::sync::mpsc::Receiver<()>>>,
    release: Mutex<Option<std::sync::mpsc::Sender<()>>>,
}
#[cfg(feature = "test-hooks")]
#[napi]
impl ErasurePause {
    #[napi]
    /// Wait at most ten seconds for actual post-delete native entry.
    pub async fn wait_entered(&self) -> Result<()> {
        let ready = Arc::clone(&self.ready);
        call_engine(move || {
            ready
                .lock()
                .map_err(|_| RustEngineError::Storage)?
                .recv_timeout(std::time::Duration::from_secs(10))
                .map_err(|_| RustEngineError::Storage)
        })
        .await
    }
    #[napi]
    /// Release a paused native turn; repeated release is harmless.
    pub fn release(&self) {
        if let Some(sender) = self.release.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = sender.send(());
        }
    }
}

#[cfg(feature = "test-hooks")]
#[napi]
impl ErasureMaintenance {
    #[napi(js_name = "_armErasurePauseForTest")]
    /// Arm a one-shot pause installed on the actual blocking-operation thread.
    pub fn arm_erasure_pause_for_test(&self) -> Result<ErasurePause> {
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let mut pause =
            self.pause.lock().map_err(|_| engine_error_to_napi(RustEngineError::Storage))?;
        if pause.is_some() {
            return Err(engine_error_to_napi(RustEngineError::Storage));
        }
        *pause = Some((ready_tx, release_rx));
        Ok(ErasurePause {
            ready: Arc::new(Mutex::new(ready_rx)),
            release: Mutex::new(Some(release_tx)),
        })
    }
}
