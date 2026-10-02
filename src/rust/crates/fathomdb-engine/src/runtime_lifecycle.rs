use super::*;

impl Drop for Engine {
    fn drop(&mut self) {
        let _ = self.close();
    }
}

impl Engine {
    /// Stop admission, quiesce all SQLite owners, then drain provider workers.
    /// Active database work can extend the total duration; the provider drain
    /// uses one 30-second deadline shared by concurrent and repeated calls.
    /// Returns [`EngineError::Scheduler`] while a provider worker remains.
    pub fn close(&self) -> Result<(), EngineError> {
        self.closed.store(true, Ordering::SeqCst);
        self.embed_dispatch.close();
        let _close_guard = self.close_lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        self.projection_runtime.stop();
        // Uninstall profile callbacks before dropping the connections so
        // SQLite cannot fire one last callback against a profile context
        // whose Box is about to free. Per `dev/design/engine.md` § Close
        // path step 6, readers drain before the writer connection so
        // SQLite's last-handle checkpointer runs on the writer. Each
        // reader worker uninstalls its own callback inside
        // `reader_worker_loop` before dropping its connection, then
        // exits — `shutdown` joins those threads here.
        self.reader_pool.shutdown();
        if let Ok(mut connection) = self.connection.lock() {
            if let Some(conn) = connection.as_ref() {
                uninstall_profile_callback(conn);
            }
            connection.take();
        }
        #[cfg(any(test, feature = "test-hooks"))]
        if let Ok(mut registration) = self.writer_connection_registration.lock() {
            registration.take();
        }
        if let Ok(mut contexts) = self.profile_contexts.lock() {
            contexts.clear();
        }
        if let Ok(mut lock) = self.lock.lock() {
            lock.take();
        }
        if self.embed_dispatch.join_after_quiescence() {
            Ok(())
        } else {
            Err(EngineError::Scheduler)
        }
    }

    /// Block until in-flight writes drain or `timeout_ms` elapses.
    ///
    /// Surface owned by `dev/interfaces/rust.md` § Engine-attached
    /// instrumentation; semantics are owned by `dev/design/lifecycle.md`.
    pub fn drain(&self, timeout_ms: u64) -> Result<(), EngineError> {
        self.ensure_open()?;
        // Only a session with no configured embedder can produce the typed
        // missing-configuration result. A configured (including refused)
        // runtime goes straight to the scheduler's authoritative idle check,
        // avoiding an otherwise duplicate full pending-work scan.
        if self.runtime_embedder.is_none() {
            let readiness = self.read_embedding_readiness()?;
            if let Some(blocked) = readiness.blocked {
                return Err(EngineError::EmbedderRequired(blocked));
            }
            // The readiness read already uses the same durable pending-work
            // predicate as `wait_for_idle`. When it finds no pending row, wait
            // only for any worker finishing its post-commit bookkeeping;
            // opening a second connection for an identical database scan cannot
            // add evidence.
            if readiness.pending_count == 0 {
                return if self.projection_runtime.wait_for_workers_idle(timeout_ms) {
                    Ok(())
                } else {
                    Err(EngineError::Scheduler)
                };
            }
        }
        if self.projection_runtime.wait_for_idle(timeout_ms, || match self.connection.try_lock() {
            Ok(connection) => Some(
                connection
                    .as_ref()
                    .and_then(|connection| connection_has_pending_projection_work(connection).ok())
                    .unwrap_or(true),
            ),
            Err(std::sync::TryLockError::WouldBlock) => None,
            Err(std::sync::TryLockError::Poisoned(_)) => Some(true),
        }) {
            Ok(())
        } else {
            Err(EngineError::Scheduler)
        }
    }

    /// Wait before a metadata-only mutation that must not turn an absent
    /// embedder into an unrelated operation failure. Direct [`Self::drain`]
    /// still reports the typed Slice-30 feedback; with no configured runtime
    /// no embedding worker can be concurrently committing the durable pending
    /// rows, so registry and lifecycle metadata may be updated safely.
    pub(crate) fn drain_for_non_embedding_mutation(&self) -> Result<(), EngineError> {
        match self.drain(LIFECYCLE_DRAIN_TIMEOUT_MS) {
            Ok(()) | Err(EngineError::EmbedderRequired(_)) => Ok(()),
            Err(error) => Err(error),
        }
    }
}
