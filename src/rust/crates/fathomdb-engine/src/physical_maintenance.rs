use super::*;
use std::sync::{Condvar, MutexGuard};

#[derive(Default)]
struct Shared {
    stop: Mutex<bool>,
    wake: Condvar,
    priority: AtomicBool,
    turn: Mutex<()>,
}

#[derive(Default)]
pub(crate) struct PhysicalMaintenance {
    shared: Arc<Shared>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

pub(crate) struct MaintenanceTurn<'a> {
    shared: &'a Shared,
    _turn: MutexGuard<'a, ()>,
}

impl Drop for MaintenanceTurn<'_> {
    fn drop(&mut self) {
        self.shared.priority.store(false, Ordering::Release);
        self.shared.wake.notify_all();
    }
}

impl PhysicalMaintenance {
    pub(crate) fn writer_turn(&self) -> Result<MutexGuard<'_, ()>, EngineError> {
        loop {
            let mut stop = self.shared.stop.lock().map_err(|_| EngineError::Storage)?;
            while self.shared.priority.load(Ordering::Acquire) && !*stop {
                stop = self.shared.wake.wait(stop).map_err(|_| EngineError::Storage)?;
            }
            if *stop {
                return Err(EngineError::Closing);
            }
            drop(stop);
            let turn = self.shared.turn.lock().map_err(|_| EngineError::Storage)?;
            if !self.shared.priority.load(Ordering::Acquire) {
                return Ok(turn);
            }
        }
    }

    pub(crate) fn start(&self, engine: &Engine) -> Result<(), EngineError> {
        let shared = Arc::clone(&self.shared);
        let connection = Arc::clone(&engine.connection);
        let service = Arc::clone(&engine.physical_service_gate);
        let telemetry = Arc::clone(&engine.telemetry);
        let projections = Arc::clone(&engine.projection_runtime);
        let has_provider = engine.runtime_embedder.is_some();
        let handle = std::thread::Builder::new()
            .name("fathomdb-physical-cleanup".into())
            .spawn(move || {
                let mut delay = Duration::from_millis(100);
                loop {
                    let stop = shared.stop.lock().unwrap_or_else(|p| p.into_inner());
                    let (stop, _) = shared
                        .wake
                        .wait_timeout_while(stop, delay, |stop| !*stop)
                        .unwrap_or_else(|p| p.into_inner());
                    if *stop {
                        break;
                    }
                    drop(stop);
                    let Ok(_service) = service.try_lock() else {
                        continue;
                    };
                    shared.priority.store(true, Ordering::Release);
                    let turn = shared.turn.lock().unwrap_or_else(|p| p.into_inner());
                    let priority = MaintenanceTurn { shared: &shared, _turn: turn };
                    let result = (|| {
                        let guard = connection.lock().map_err(|_| EngineError::Storage)?;
                        let c = guard.as_ref().ok_or(EngineError::Closing)?;
                        if !physical_erasure::active(c)? {
                            return Ok(false);
                        }
                        let ids = physical_erasure::pending_ids(c)?;
                        dependency_closure::validate_physical_closures(c, &ids)?;
                        physical_erasure::validate_pending(c, &ids)?;
                        if has_provider && physical_erasure::redacted_ids(c)?.is_none() {
                            boot_graft_declared_vector_backfill(c)
                                .map_err(|_| EngineError::Storage)?;
                        }
                        if !physical_erasure::sanitize_unit(c)? {
                            return Ok(true);
                        }
                        erasure::discharge_pending_redactions_on(c, &telemetry)?;
                        physical_erasure::prepare_redacted(c, &ids)?;
                        checkpoint(c)?;
                        physical_erasure::finalize(c, &ids)?;
                        projections.notify_new_work();
                        Ok(true)
                    })();
                    if let Err(error) = &result {
                        if let Ok(guard) = connection.lock() {
                            if let Some(c) = guard.as_ref() {
                                let _ = physical_erasure::record_blocker(c, error);
                            }
                        }
                    }
                    drop(priority);
                    delay = if matches!(result, Ok(true)) {
                        Duration::from_millis(5)
                    } else {
                        Duration::from_millis(100)
                    };
                }
            })
            .map_err(|_| EngineError::Scheduler)?;
        *self.worker.lock().map_err(|_| EngineError::Storage)? = Some(handle);
        Ok(())
    }

    pub(crate) fn stop(&self) {
        *self.shared.stop.lock().unwrap_or_else(|p| p.into_inner()) = true;
        self.shared.wake.notify_all();
        if let Some(worker) = self.worker.lock().unwrap_or_else(|p| p.into_inner()).take() {
            let _ = worker.join();
        }
    }
}

fn checkpoint(c: &Connection) -> Result<(), EngineError> {
    let previous: u64 =
        c.query_row("PRAGMA busy_timeout", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
    c.busy_timeout(Duration::ZERO).map_err(|_| EngineError::Storage)?;
    let result = c.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| r.get::<_, i64>(0));
    c.busy_timeout(Duration::from_millis(previous)).map_err(|_| EngineError::Storage)?;
    if result.map_err(|_| EngineError::Storage)? != 0 {
        return Err(EngineError::ErasureIncomplete {
            stage: "wal_checkpoint".into(),
            detail: "a reader pins physical cleanup; maintenance will retry".into(),
        });
    }
    Ok(())
}
