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
