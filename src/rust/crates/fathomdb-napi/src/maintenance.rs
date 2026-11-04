use super::*;

/// Native owner retained by every spawned blocking maintenance call.
#[napi]
pub struct ErasureMaintenance {
    pub(crate) inner: Arc<Mutex<fathomdb_engine::ErasureMaintenance>>,
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
