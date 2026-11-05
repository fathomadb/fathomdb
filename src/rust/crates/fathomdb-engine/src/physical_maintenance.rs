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
    pub(crate) metrics: Arc<physical_metrics::PhysicalMetrics>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

pub(crate) struct MaintenanceTurn<'a> {
    shared: &'a Shared,
    _turn: MutexGuard<'a, ()>,
}

impl Drop for MaintenanceTurn<'_> {
    fn drop(&mut self) {
        let _predicate = self.shared.stop.lock().unwrap_or_else(|p| p.into_inner());
        self.shared.priority.store(false, Ordering::Release);
        self.shared.wake.notify_all();
    }
}

impl PhysicalMaintenance {
    pub(crate) fn writer_turn(&self) -> Result<MutexGuard<'_, ()>, EngineError> {
        loop {
            let mut stop = self.shared.stop.lock().map_err(|_| EngineError::Storage)?;
            while self.shared.priority.load(Ordering::Acquire) && !*stop {
                physical_erasure::faultpoint("before_writer_priority_wait")?;
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
        let metrics = Arc::clone(&self.metrics);
        let connection = Arc::clone(&engine.connection);
        let service = Arc::clone(&engine.physical_service_gate);
        let telemetry = Arc::clone(&engine.telemetry);
        let projections = Arc::clone(&engine.projection_runtime);
        let readers = engine.reader_pool.physical_admission();
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
                    let active = match connection.try_lock() {
                        Ok(guard) => guard.as_ref().is_some_and(|c| {
                            connection_runtime::internal_debt_probe(|| {
                                physical_erasure::active(c).unwrap_or(true)
                            })
                        }),
                        // Contention cannot prove there is no debt. Assert the
                        // maintenance turn before waiting so ordinary writers
                        // cannot hide pending work at every poll.
                        Err(_) => true,
                    };
                    if !active {
                        continue;
                    }
                    shared.priority.store(true, Ordering::Release);
                    let turn = shared.turn.lock().unwrap_or_else(|p| p.into_inner());
                    let priority = MaintenanceTurn { shared: &shared, _turn: turn };
                    let mut service_guard = None;
                    let result = (|| {
                        let waiting = metrics.timer("maintenance_writer_wait");
                        let guard = physical_metrics::writer(&metrics, &connection, false)?;
                        drop(waiting);
                        let holding = metrics.timer("maintenance_writer_hold");
                        let c = guard.as_ref().ok_or(EngineError::Closing)?;
                        let active =
                            connection_runtime::internal_debt_probe(|| physical_erasure::active(c))
                                .inspect_err(|error| {
                                    // Before service admission, only this primary guard
                                    // prevents attributing a blocker to a later request.
                                    let _ = physical_erasure::record_blocker(c, error);
                                })?;
                        if !active {
                            return Ok(false);
                        }
                        // Foreground erasure owns service before primary. Never
                        // wait on its service guard while holding primary.
                        let Ok(ownership) = service.try_lock() else {
                            return Ok(false);
                        };
                        service_guard = Some(ownership);
                        metrics.begin_lifecycle();
                        let ids = physical_erasure::pending_ids(c)?;
                        dependency_closure::validate_physical_closures(c, &ids)?;
                        physical_erasure::validate_pending(c, &ids)?;
                        let graft = metrics.timer("deferred_graft");
                        if has_provider && physical_erasure::redacted_ids(c)?.is_none() {
                            boot_graft_declared_vector_backfill(c)
                                .map_err(|_| EngineError::Storage)?;
                        }
                        drop(graft);
                        let sanitation = metrics.timer("sanitation");
                        let done = physical_erasure::sanitize_unit(c)?;
                        drop(sanitation);
                        if !done {
                            return Ok(true);
                        }
                        drop(holding);
                        drop(guard);
                        let drain = metrics.timer("reader_drain");
                        let _reader_pause = metrics.timer("reader_pause");
                        let _readers = readers.pause()?;
                        drop(drain);
                        let guard = physical_metrics::writer(&metrics, &connection, true)?;
                        let c = guard.as_ref().ok_or(EngineError::Closing)?;
                        erasure::discharge_pending_redactions_on(c, &telemetry)?;
                        physical_erasure::prepare_redacted(c, &ids)?;
                        let checkpoint_timer = metrics.timer("checkpoint");
                        checkpoint(c)?;
                        drop(checkpoint_timer);
                        physical_erasure::finalize(c, &ids)?;
                        metrics.complete_lifecycle();
                        projections.notify_new_work();
                        Ok(true)
                    })();
                    if service_guard.is_some() {
                        if let Err(error) = &result {
                            if let Ok(guard) = connection.lock() {
                                if let Some(c) = guard.as_ref() {
                                    let _ = physical_erasure::record_blocker(c, error);
                                }
                            }
                        }
                    }
                    drop(priority);
                    drop(service_guard);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_debt_probes_do_not_dispatch_subscriber_profiles() {
        #[derive(Default)]
        struct Profiles(AtomicUsize);
        impl lifecycle::Subscriber for Profiles {
            fn on_event(&self, _: &lifecycle::Event) {}
            fn on_profile(&self, _: &lifecycle::ProfileRecord) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open(directory.path().join("idle-profiles.sqlite")).unwrap().engine;
        engine.set_profiling(true).unwrap();
        let profiles = Arc::new(Profiles::default());
        let subscription = engine.subscribe(profiles.clone());
        std::thread::sleep(Duration::from_millis(350));
        let idle = profiles.0.load(Ordering::SeqCst);
        engine.read_get("absent", &Default::default()).unwrap();
        let foreground = profiles.0.load(Ordering::SeqCst);
        engine.erase_source("absent").unwrap();
        let cleanup = profiles.0.load(Ordering::SeqCst);
        drop(subscription);
        engine.close().unwrap();
        assert_eq!(idle, 0, "idle scheduler probes must not dispatch subscriber callbacks");
        assert!(foreground > idle, "foreground profiling remains observable");
        assert!(cleanup > foreground, "actual cleanup profiling remains observable");
    }

    #[test]
    fn nested_probe_error_restores_subscriber_profiling() {
        #[derive(Default)]
        struct Profiles(AtomicUsize);
        impl lifecycle::Subscriber for Profiles {
            fn on_event(&self, _: &lifecycle::Event) {}
            fn on_profile(&self, _: &lifecycle::ProfileRecord) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open(directory.path().join("probe-error.sqlite")).unwrap().engine;
        engine.physical_maintenance.stop();
        engine.set_profiling(true).unwrap();
        let profiles = Arc::new(Profiles::default());
        let subscription = engine.subscribe(profiles.clone());
        let guard = engine.connection.lock().unwrap();
        let c = guard.as_ref().unwrap();
        let result: Result<(), rusqlite::Error> = connection_runtime::internal_debt_probe(|| {
            let nested: Result<i64, _> = connection_runtime::internal_debt_probe(|| {
                c.query_row("SELECT missing_column FROM missing_probe_table", [], |r| r.get(0))
            });
            assert!(nested.is_err());
            let _: i64 = c.query_row("SELECT 1", [], |r| r.get(0)).unwrap();
            assert_eq!(profiles.0.load(Ordering::SeqCst), 0);
            nested.map(|_| ())
        });
        assert!(result.is_err());
        let _: i64 = c.query_row("SELECT 2", [], |r| r.get(0)).unwrap();
        assert_eq!(profiles.0.load(Ordering::SeqCst), 1);
        drop(guard);
        drop(subscription);
        engine.close().unwrap();
    }

    #[test]
    fn synchronous_blocker_is_durable_without_a_worker_retry() {
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open(directory.path().join("blocker.sqlite")).unwrap().engine;
        engine.physical_maintenance.stop();
        crate::test_hooks::physical_erasure_hooks::arm(
            "after_sanitation_unit",
            Box::new(|| {
                Err(EngineError::ErasureIncomplete {
                    stage: "fixture_blocker".into(),
                    detail: "injected".into(),
                })
            }),
        );
        assert!(engine.erase_source("target").is_err());
        let guard = engine.connection.lock().unwrap();
        let blocker: Option<String> = guard.as_ref().unwrap().query_row(
            "SELECT json_extract(value,'$.blocker') FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'", [], |r| r.get(0)).unwrap();
        drop(guard);
        engine.close().unwrap();
        assert_eq!(blocker.as_deref(), Some("fixture_blocker"));
    }

    #[test]
    fn contended_primary_cannot_hide_pending_work_from_priority() {
        let directory = tempfile::tempdir().unwrap();
        let engine = Engine::open(directory.path().join("priority.sqlite")).unwrap().engine;
        let guard = engine.connection.lock().unwrap();
        let deadline = Instant::now() + Duration::from_millis(350);
        while !engine.physical_maintenance.shared.priority.load(Ordering::Acquire)
            && Instant::now() < deadline
        {
            std::thread::sleep(Duration::from_millis(5));
        }
        let asserted = engine.physical_maintenance.shared.priority.load(Ordering::Acquire);
        drop(guard);
        engine.close().unwrap();
        assert!(asserted, "a busy primary must not hide potentially owed work from priority");
    }

    #[cfg(feature = "test-hooks")]
    #[test]
    fn debt_free_primary_contention_does_not_reserve_physical_service() {
        let directory = tempfile::tempdir().unwrap();
        let engine =
            Arc::new(Engine::open(directory.path().join("probe-admission.sqlite")).unwrap().engine);
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "private worker admission marker".into(),
                source_id: SourceId::new("target-source").unwrap(),
                logical_id: Some("target".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        engine.drain(1_000).unwrap();
        let primary = engine.connection.lock().unwrap();
        let debt_free = matches!(physical_erasure::active(primary.as_ref().unwrap()), Ok(false));
        let deadline = Instant::now() + Duration::from_secs(2);
        while !engine.physical_maintenance.shared.priority.load(Ordering::Acquire)
            && Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        let priority = engine.physical_maintenance.shared.priority.load(Ordering::Acquire);
        let service_available = engine.physical_service_gate.try_lock().is_ok();
        let (admitted_tx, admitted_rx) = mpsc::channel();
        engine.arm_erasure_before_primary_lock_hook_for_test(Box::new(move || {
            let _ = admitted_tx.send(());
        }));
        let foreground = Arc::clone(&engine);
        let erase = std::thread::spawn(move || foreground.erase_source("target-source"));
        let admitted = admitted_rx.recv_timeout(Duration::from_secs(2)).is_ok();
        // Neither a failed rendezvous nor the expected RED may strand an actor.
        drop(primary);
        let erased = erase.join();
        let closed = engine.close();
        assert!(debt_free, "the held primary must have no committed physical debt");
        assert!(priority, "a contended primary must still publish maintenance priority");
        assert!(
            service_available,
            "speculative debt probing occupied the service before checking the held primary"
        );
        assert!(admitted, "the foreground erase must be admitted before primary release");
        assert_eq!(erased.unwrap().unwrap().nodes_excised, 1);
        closed.unwrap();
    }

    #[test]
    fn ordinary_writer_cannot_miss_the_maintenance_priority_release() {
        let maintenance = Arc::new(PhysicalMaintenance::default());
        maintenance.shared.priority.store(true, Ordering::Release);
        let (ready_tx, ready_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let (released_tx, released_rx) = mpsc::channel();
        let owner = Arc::clone(&maintenance);
        let releaser = std::thread::spawn(move || {
            let turn = owner.shared.turn.lock().unwrap();
            let priority = MaintenanceTurn { shared: &owner.shared, _turn: turn };
            ready_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            drop(priority);
            released_tx.send(()).unwrap();
        });
        ready_rx.recv().unwrap();
        let (observed_tx, observed_rx) = mpsc::channel();
        let (wait_tx, wait_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let waiter = Arc::clone(&maintenance);
        let writer = std::thread::spawn(move || {
            crate::test_hooks::physical_erasure_hooks::arm(
                "before_writer_priority_wait",
                Box::new(move || {
                    observed_tx.send(()).unwrap();
                    wait_rx.recv().unwrap();
                    Ok(())
                }),
            );
            let result = waiter.writer_turn().is_ok();
            done_tx.send(result).unwrap();
        });
        observed_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        release_tx.send(()).unwrap();
        let _ = released_rx.recv_timeout(Duration::from_millis(100));
        wait_tx.send(()).unwrap();
        let progressed = done_rx.recv_timeout(Duration::from_millis(200)).unwrap_or(false);
        // Cleanup releases a failed waiter without providing the progress witness.
        maintenance.stop();
        releaser.join().unwrap();
        writer.join().unwrap();
        assert!(progressed, "writer must observe priority release without another service turn or close notification");
    }
}
