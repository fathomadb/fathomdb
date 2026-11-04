/// Exclusive ownership of one Engine's physical-erasure maintenance window.
/// Dropping it releases temporary ownership, never durable owed cleanup.
pub struct ErasureMaintenance {
    pub(crate) inner: fathomdb_engine::ErasureMaintenance,
}

impl ErasureMaintenance {
    /// Perform heavy non-destructive admission under exclusive ownership.
    /// Returns the same retained request for an exact key/selector retry.
    pub fn queue_physical_erasure(
        &self,
        selector: crate::PhysicalErasureSelector,
        key: &str,
    ) -> crate::Result<crate::QueueResult> {
        Ok(self.inner.queue_physical_erasure(selector, key)?)
    }

    /// Attempt exact physical completion for this ticket. Complete requires
    /// actual registered-copy/sink/WAL discharge; Incomplete retains owed work.
    pub fn complete_physical_erasure(&self, ticket: &str) -> crate::Result<crate::CleanupResult> {
        Ok(self.inner.complete_physical_erasure(ticket)?)
    }

    /// Release temporary admission gates and report remaining service state.
    /// This is not a physical-completion result.
    pub fn close(&mut self) -> crate::Result<fathomdb_engine::ErasureServiceState> {
        Ok(self.inner.close()?)
    }
}
