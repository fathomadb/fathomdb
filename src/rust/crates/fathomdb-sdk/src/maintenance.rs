/// Exclusive ownership of one Engine's physical-erasure maintenance window.
/// Dropping it releases temporary ownership, never durable owed cleanup.
pub struct ErasureMaintenance {
    pub(crate) inner: fathomdb_engine::ErasureMaintenance,
}

impl ErasureMaintenance {
    /// Release temporary admission gates and report remaining service state.
    /// This is not a physical-completion result.
    pub fn close(&mut self) -> crate::Result<fathomdb_engine::ErasureServiceState> {
        Ok(self.inner.close()?)
    }
}
