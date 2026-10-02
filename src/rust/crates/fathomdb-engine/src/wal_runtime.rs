use super::*;

/// Test-only live-connection audit. Each long-lived Engine connection acquires
/// one registration after its actual SQLite handle exists and drops it when the
/// handle leaves service; an incomplete registry is a diagnostic failure, never
/// evidence of an external WAL holder.
#[cfg(any(test, feature = "test-hooks"))]
#[derive(Default)]
pub(crate) struct ManagedConnectionRegistry {
    pub(crate) live: Mutex<BTreeSet<(WalAttributionRole, usize)>>,
    pub(crate) opens: Mutex<BTreeMap<ManagedConnectionCategory, usize>>,
    #[cfg(test)]
    pub(crate) runtime_probe_lifecycle: Mutex<RuntimeProbeLifecycle>,
}

#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum ManagedConnectionCategory {
    Writer,
    ReaderWorker,
    ProjectionDispatcher,
    ProjectionWorker,
    RuntimeProbe,
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) struct ManagedConnectionRegistration {
    registry: Arc<ManagedConnectionRegistry>,
    role: WalAttributionRole,
    index: usize,
}

/// Private lifecycle facts for an independent diagnostic probe. A probe is
/// useful only after its native SQLite connection has actually been dropped;
/// an implicit drop is deliberately retained as incomplete rather than being
/// mistaken for an external holder.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RuntimeProbeLifecycle {
    pub(crate) live: usize,
    pub(crate) actual_drops: usize,
    pub(crate) incomplete_drops: usize,
}

#[cfg(test)]
pub(crate) struct RuntimeProbeRegistration {
    registry: Arc<ManagedConnectionRegistry>,
    acknowledged: bool,
}

#[cfg(test)]
pub(crate) struct RuntimeProbeConnection {
    connection: Option<Connection>,
    registration: RuntimeProbeRegistration,
}

#[cfg(any(test, feature = "test-hooks"))]
impl ManagedConnectionRegistry {
    pub(crate) fn register(
        self: &Arc<Self>,
        role: WalAttributionRole,
        index: usize,
    ) -> ManagedConnectionRegistration {
        let inserted = self.live.lock().expect("managed connection registry").insert((role, index));
        assert!(inserted, "duplicate managed connection registration for {}:{index}", role.name());
        ManagedConnectionRegistration { registry: Arc::clone(self), role, index }
    }

    pub(crate) fn exact_live(&self, worker_count: usize) -> bool {
        let expected = native_state_expected_roles(worker_count);
        self.live.lock().map(|live| *live == expected).unwrap_or(false)
    }

    pub(crate) fn record_open(&self, category: ManagedConnectionCategory) {
        let mut opens = self.opens.lock().expect("managed connection open audit");
        *opens.entry(category).or_default() += 1;
    }

    pub(crate) fn creation_counts(&self) -> Option<(usize, usize, usize, usize, usize)> {
        let opens = self.opens.lock().ok()?;
        Some((
            *opens.get(&ManagedConnectionCategory::Writer).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::ReaderWorker).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::ProjectionDispatcher).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::ProjectionWorker).unwrap_or(&0),
            *opens.get(&ManagedConnectionCategory::RuntimeProbe).unwrap_or(&0),
        ))
    }

    #[cfg(test)]
    fn register_runtime_probe(self: &Arc<Self>) -> RuntimeProbeRegistration {
        let mut lifecycle = self.runtime_probe_lifecycle.lock().expect("runtime probe lifecycle");
        lifecycle.live += 1;
        RuntimeProbeRegistration { registry: Arc::clone(self), acknowledged: false }
    }
}

#[cfg(any(test, feature = "test-hooks"))]
impl Drop for ManagedConnectionRegistration {
    fn drop(&mut self) {
        if let Ok(mut live) = self.registry.live.lock() {
            live.remove(&(self.role, self.index));
        }
    }
}

#[cfg(test)]
impl RuntimeProbeRegistration {
    fn acknowledge_actual_drop(&mut self) -> RuntimeProbeLifecycle {
        let mut lifecycle =
            self.registry.runtime_probe_lifecycle.lock().expect("runtime probe lifecycle");
        assert!(lifecycle.live > 0, "runtime probe acknowledgement without a live probe");
        lifecycle.live -= 1;
        lifecycle.actual_drops += 1;
        self.acknowledged = true;
        *lifecycle
    }
}

#[cfg(test)]
impl Drop for RuntimeProbeRegistration {
    fn drop(&mut self) {
        if !self.acknowledged {
            if let Ok(mut lifecycle) = self.registry.runtime_probe_lifecycle.lock() {
                lifecycle.live = lifecycle.live.saturating_sub(1);
                lifecycle.incomplete_drops += 1;
            }
        }
    }
}

#[cfg(test)]
impl RuntimeProbeConnection {
    pub(crate) fn open(
        path: &Path,
        registry: &Arc<ManagedConnectionRegistry>,
    ) -> rusqlite::Result<Self> {
        let connection =
            open_managed_connection(path, ManagedConnectionCategory::RuntimeProbe, registry)?;
        Ok(Self { connection: Some(connection), registration: registry.register_runtime_probe() })
    }

    pub(crate) fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("runtime probe connection remains live")
    }

    pub(crate) fn close_and_acknowledge(mut self) -> rusqlite::Result<RuntimeProbeLifecycle> {
        self.connection
            .take()
            .expect("runtime probe connection remains live")
            .close()
            .map_err(|(_, error)| error)?;
        Ok(self.registration.acknowledge_actual_drop())
    }
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) struct RuntimeConnectionInventoryRequest {
    pub(crate) pending: BTreeSet<(WalAttributionRole, usize)>,
    pub(crate) respond: SyncSender<(WalAttributionRole, usize, bool)>,
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) struct RuntimeNativeStateRequest {
    pub(crate) pending: BTreeSet<(WalAttributionRole, usize)>,
    pub(crate) respond: SyncSender<NativeConnectionStateFact>,
}

/// Typed outcome of [`Engine::truncate_wal`]. `Done` matches SQLite's
/// `busy = 0` return from `PRAGMA wal_checkpoint(TRUNCATE)`; any other
/// value surfaces as `Busy`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TruncateWalStatus {
    Done,
    Busy,
}

/// Result of [`Engine::truncate_wal`] or `recover_truncate_wal`. Carries the
/// three counters returned by `PRAGMA wal_checkpoint(TRUNCATE)`: `busy`,
/// `log_frames`, `checkpointed_frames`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TruncateWalReport {
    pub status: TruncateWalStatus,
    pub busy: u32,
    pub log_frames: u32,
    pub checkpointed_frames: u32,
    /// True only when the locked pre-probe found a malformed WAL and SQLite
    /// subsequently completed the truncate checkpoint.
    pub discarded_corrupt_wal: bool,
}

impl Engine {
    /// Read the exact live engine-owned SQLite handle inventory without opening a probe.
    #[cfg(feature = "test-hooks")]
    pub fn d27_connection_inventory_for_test(&self) -> Result<String, EngineError> {
        self.ensure_open()?;
        let workers = self.resolved_config.scheduler_runtime_threads;
        if !self.managed_connections.exact_live(workers) {
            return Err(EngineError::Storage);
        }
        let live = self.managed_connections.live.lock().map_err(|_| EngineError::Storage)?;
        let count = |role| live.iter().filter(|(member, _)| *member == role).count();
        let writer = count(WalAttributionRole::Writer);
        let readers = count(WalAttributionRole::ReaderWorker);
        let dispatcher = count(WalAttributionRole::ProjectionDispatcher);
        let workers = count(WalAttributionRole::ProjectionWorker);
        Ok(format!("live=writer:{writer},readers:{readers},dispatcher:{dispatcher},workers:{workers},probes:0"))
    }

    #[allow(dead_code)]
    pub(crate) fn wal_attribution_snapshot(&self) -> WalAttributionSnapshot {
        self.wal_attribution.snapshot()
    }

    #[allow(dead_code)]
    pub(crate) fn wal_attribution_checkpoints_for_test(&self) -> Vec<WalCheckpointRecord> {
        self.wal_attribution.checkpoints()
    }

    // `test` added alongside `debug_assertions`/`test-hooks` so the crate's own
    // `--release --tests` lib-test build (cfg(test) true, debug_assertions
    // false) can still see this seam; it stays absent from any shipped build.
    #[cfg(any(test, debug_assertions, feature = "test-hooks"))]
    #[allow(dead_code)]
    #[doc(hidden)]
    pub fn pause_reader_after_wal_snapshot_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>) {
        let snapshot_ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        self.reader_pool
            .hold_worker_zero_wal_snapshot(Arc::clone(&snapshot_ready), Arc::clone(&release));
        (snapshot_ready, release)
    }

    /// Pause worker zero on a real snapshot with cancellation-safe bounded
    /// release. Dropping the returned sender releases the worker immediately.
    #[cfg(debug_assertions)]
    #[doc(hidden)]
    pub fn pause_reader_with_timeout_for_test(&self) -> (Receiver<usize>, SyncSender<()>) {
        let (snapshot_ready_tx, snapshot_ready_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel(1);
        self.reader_pool.hold_worker_zero_wal_snapshot_bounded(snapshot_ready_tx, release_rx);
        (snapshot_ready_rx, release_tx)
    }

    #[cfg(test)]
    pub(crate) fn post_commit_ack_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>, Arc<Barrier>) {
        let snapshot_ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let committed = Arc::new(Barrier::new(2));
        self.reader_pool.hold_worker_zero_wal_snapshot_with_commit_ack(
            Arc::clone(&snapshot_ready),
            Arc::clone(&release),
            Arc::clone(&committed),
        );
        (snapshot_ready, release, committed)
    }

    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn arm_next_reader_snapshot_pause_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>, Arc<Mutex<Option<String>>>) {
        self.wal_attribution.arm_reader_snapshot_pause()
    }

    /// Arm a private completion rendezvous for the next public reader request.
    ///
    /// The ready barrier fires only after the helper-local SQLite transaction
    /// and statements have dropped, the attribution collector is idle, and
    /// before the materialized response is delivered. Available only to tests
    /// and disposable `test-hooks` artifacts.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn arm_next_reader_completion_pause_for_test(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>, Arc<AtomicBool>) {
        self.wal_attribution.arm_reader_completion_pause()
    }

    /// Private Slice 65 test rendezvous. Unlike the snapshot hook this is
    /// deliberately unavailable to test-hook artifacts: it verifies only the
    /// collector's internal response-handoff boundary.
    #[cfg(test)]
    pub(crate) fn pause_next_reader_handoff_for_test(&self) -> (Arc<Barrier>, Arc<Barrier>) {
        self.wal_attribution.arm_reader_handoff_pause()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn wal_attribution_checkpoint_records_for_test(
        &self,
    ) -> Vec<(usize, bool, String, Vec<String>)> {
        self.wal_attribution_checkpoints_for_test()
            .into_iter()
            .map(|record| {
                (
                    record.attempt,
                    record.busy,
                    record.classification.to_string(),
                    record
                        .active_roles
                        .iter()
                        .map(|(role, index)| format!("{}:{index}", role.name()))
                        .collect(),
                )
            })
            .collect()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn wal_attribution_idle_for_test(&self) -> bool {
        self.wal_attribution_snapshot().no_owned_snapshot
    }

    /// Arm private observation for the next erasure checkpoint sequence. The
    /// observer only reads connection state around the already-required
    /// checkpoint call; it never issues a diagnostic SQLite statement.
    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn arm_actual_checkpoint_observation_for_test(&self, control: &'static str) {
        *self.actual_checkpoint_observations.lock().expect("actual checkpoint observation") =
            Some(ActualCheckpointObserver { control, records: Vec::new() });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn actual_checkpoint_observation_for_test(
        &self,
        phase: &'static str,
        ordinal: usize,
        checkpoint_begin_overlap: bool,
        elapsed: Option<Duration>,
        report: Option<TruncateWalReport>,
    ) {
        let mut observations =
            self.actual_checkpoint_observations.lock().expect("actual checkpoint observation");
        let Some(observer) = observations.as_mut() else {
            return;
        };
        let writer_autocommit = self
            .connection
            .lock()
            .ok()
            .and_then(|connection| connection.as_ref().map(Connection::is_autocommit))
            .unwrap_or(false);
        let expected_runtime_probes = match observer.control {
            "direct_rust" => 0,
            "python_serial" => 0,
            _ => 2,
        };
        let direct_inventory =
            self.actual_checkpoint_direct_inventory_for_test(expected_runtime_probes);
        let collector_roles = self
            .wal_attribution_snapshot()
            .active_roles
            .into_iter()
            .map(|(role, index)| format!("{}:{index}", role.name()))
            .collect();
        observer.records.push(ActualCheckpointObservation {
            control: observer.control,
            phase,
            ordinal,
            writer_autocommit,
            direct_inventory,
            collector_roles,
            checkpoint_begin_overlap,
            elapsed,
            report,
        });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn actual_checkpoint_direct_inventory_for_test(
        &self,
        expected_runtime_probes: usize,
    ) -> String {
        let worker_count = self.projection_runtime.shared.worker_count;
        let registry_complete = self.managed_connections.exact_live(worker_count);
        let creation = self.managed_connections.creation_counts();
        let writer_autocommit = self
            .connection
            .lock()
            .ok()
            .and_then(|connection| connection.as_ref().map(Connection::is_autocommit))
            .unwrap_or(false);
        let readers = self.reader_pool.wal_connection_inventory_for_test();
        let runtime = self.projection_runtime.report_runtime_connection_inventory_for_test();
        let reader_autocommit =
            readers.len() == READER_POOL_SIZE && readers.iter().all(|value| *value);
        let runtime_autocommit = runtime.as_ref().is_ok_and(|entries| {
            let expected = projection_runtime::projection_wal_roles(worker_count);
            entries.iter().map(|(role, index, _)| (*role, *index)).collect::<BTreeSet<_>>()
                == expected
                && entries.iter().all(|(_, _, autocommit)| *autocommit)
        });
        let dispatcher_autocommit = runtime.as_ref().is_ok_and(|entries| {
            entries
                .iter()
                .find(|(role, index, _)| {
                    *role == WalAttributionRole::ProjectionDispatcher && *index == 0
                })
                .is_some_and(|(_, _, autocommit)| *autocommit)
        });
        let workers_autocommit = runtime.as_ref().is_ok_and(|entries| {
            entries
                .iter()
                .filter(|(role, _, _)| *role == WalAttributionRole::ProjectionWorker)
                .count()
                == worker_count
                && entries
                    .iter()
                    .filter(|(role, _, _)| *role == WalAttributionRole::ProjectionWorker)
                    .all(|(_, _, autocommit)| *autocommit)
        });
        let creation_text = creation.map_or_else(
            || "unknown".to_string(),
            |(writer, readers, dispatcher, workers, probes)| {
                format!("writer:{writer},readers:{readers},dispatcher:{dispatcher},workers:{workers},probes:{probes}")
            },
        );
        let complete = registry_complete
            && creation == Some((1, READER_POOL_SIZE, 1, worker_count, expected_runtime_probes))
            && writer_autocommit
            && reader_autocommit
            && runtime_autocommit;
        format!(
            "roles=writer:0,readers:0-7,dispatcher:0,workers:0-{};writer={};readers={};dispatcher={};workers={};registry={};creation={};complete={}",
            worker_count - 1,
            if writer_autocommit { "autocommit" } else { "not_autocommit" },
            if reader_autocommit { "autocommit" } else { "not_autocommit" },
            if dispatcher_autocommit { "autocommit" } else { "not_autocommit" },
            if workers_autocommit { format!("{worker_count}-autocommit") } else { "not_autocommit".to_string() },
            if registry_complete { "complete" } else { "incomplete" },
            creation_text,
            u8::from(complete),
        )
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn take_actual_checkpoint_observations_for_test(&self) -> Vec<String> {
        let records = self
            .actual_checkpoint_observations
            .lock()
            .expect("actual checkpoint observation")
            .take()
            .map_or_else(Vec::new, |observer| observer.records);
        records
            .into_iter()
            .map(|record| {
                let collector_roles = if record.collector_roles.is_empty() {
                    "idle".to_string()
                } else {
                    record.collector_roles.join(",")
                };
                let timing = record.elapsed.map_or_else(String::new, |elapsed| {
                    format!(" elapsed_ms={}", elapsed.as_millis())
                });
                let outcome = record.report.map_or_else(String::new, |report| {
                    format!(
                        " busy={} log_frames={} checkpointed_frames={}",
                        u8::from(report.busy != 0), report.log_frames, report.checkpointed_frames,
                    )
                });
                format!(
                    "control={} phase={} ordinal={} writer_autocommit={} direct_inventory={} collector_roles={} checkpoint_begin_overlap={}{}{}",
                    record.control,
                    record.phase,
                    record.ordinal,
                    u8::from(record.writer_autocommit),
                    record.direct_inventory,
                    collector_roles,
                    u8::from(record.checkpoint_begin_overlap),
                    timing,
                    outcome,
                )
            })
            .collect()
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn arm_python_serial_actual_checkpoint_observation_for_test(&self) {
        self.arm_actual_checkpoint_observation_for_test("python_serial");
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn drain_actual_checkpoint_observations_for_test(&self) -> Vec<String> {
        self.take_actual_checkpoint_observations_for_test()
    }

    /// Arm direct native-state observation around the existing Slice 65
    /// test-hook sampler. It is deliberately separate from the real-erasure
    /// observer so the normal serial path gains no connection inspection.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn arm_binding_native_state_observation_for_test(&self) {
        *self.binding_native_state_observations.lock().expect("binding native state observation") =
            Some(BindingNativeStateObserver { records: Vec::new() });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn binding_native_state_observation_for_test(
        &self,
        phase: &'static str,
        ordinal: usize,
    ) {
        let mut observations = self
            .binding_native_state_observations
            .lock()
            .expect("binding native state observation");
        let Some(observer) = observations.as_mut() else {
            return;
        };
        observer.records.push(BindingNativeStateObservation {
            phase,
            ordinal,
            inventory: self.native_state_inventory_for_test(),
        });
    }

    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn drain_binding_native_state_observations_for_test(&self) -> Vec<String> {
        self.binding_native_state_observations
            .lock()
            .expect("binding native state observation")
            .take()
            .map_or_else(Vec::new, |observer| observer.records)
            .into_iter()
            .map(|record| {
                format!(
                    "control=binding_sampler phase={} ordinal={} {}",
                    record.phase,
                    record.ordinal,
                    native_state_inventory_text(&record.inventory),
                )
            })
            .collect()
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(crate) fn native_state_inventory_for_test(&self) -> NativeStateInventory {
        let worker_count = self.projection_runtime.shared.worker_count;
        let mut facts = Vec::with_capacity(1 + READER_POOL_SIZE + 1 + worker_count);
        let writer = match self.connection.lock() {
            Ok(connection) => connection.as_ref().map_or_else(
                || {
                    unavailable_native_connection_state_for_test(
                        WalAttributionRole::Writer,
                        0,
                        NativeStateReply::Error("writer_unavailable"),
                    )
                },
                |connection| {
                    native_connection_state_for_test(connection, WalAttributionRole::Writer, 0)
                },
            ),
            Err(_) => unavailable_native_connection_state_for_test(
                WalAttributionRole::Writer,
                0,
                NativeStateReply::Error("writer_lock"),
            ),
        };
        facts.push(writer);
        facts.extend(self.reader_pool.wal_native_state_inventory_for_test());
        match self.projection_runtime.report_runtime_native_state_inventory_for_test() {
            Ok(runtime) => facts.extend(runtime),
            Err(reason) => {
                facts.push(unavailable_native_connection_state_for_test(
                    WalAttributionRole::ProjectionDispatcher,
                    0,
                    NativeStateReply::Error(reason),
                ));
                facts.extend((0..worker_count).map(|index| {
                    unavailable_native_connection_state_for_test(
                        WalAttributionRole::ProjectionWorker,
                        index,
                        NativeStateReply::Error(reason),
                    )
                }));
            }
        }
        facts.sort_by_key(|fact| (fact.role, fact.index));
        let expected = native_state_expected_roles(worker_count);
        let actual = facts.iter().map(|fact| (fact.role, fact.index)).collect::<BTreeSet<_>>();
        let unique = facts.len() == actual.len();
        let managed = self.managed_connections.exact_live(worker_count);
        let received_and_idle = facts.iter().all(|fact| {
            matches!(fact.reply, NativeStateReply::Received)
                && fact.autocommit == Some(true)
                && fact.transaction == NativeTransactionState::None
                && fact.busy_statement == Some(false)
        });
        let (complete, reason) = if !managed {
            (false, "registry_mismatch")
        } else if actual != expected || !unique {
            (false, "role_mismatch")
        } else if !received_and_idle {
            (false, "native_state_not_idle")
        } else {
            (true, "complete")
        };
        NativeStateInventory { facts, complete, reason }
    }

    /// Return exact direct native state for the private installed-binding
    /// diagnostic. Any missing, duplicate, timed-out, errored, or non-idle
    /// role fails closed; callers cannot upgrade a checkpoint classification.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn binding_native_state_inventory_for_test(&self) -> Result<String, EngineError> {
        self.ensure_open()?;
        let inventory = self.native_state_inventory_for_test();
        if !inventory.complete {
            return Err(EngineError::Storage);
        }
        Ok(native_state_inventory_text(&inventory))
    }

    /// Return direct, complete managed-connection facts for the private Slice
    /// 65 installed-binding diagnostic. This is intentionally unavailable from
    /// ordinary builds and never enters an SDK error or diagnostic surface.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn binding_connection_inventory_for_test(&self) -> Result<String, EngineError> {
        self.ensure_open()?;
        let deadline = Instant::now() + Duration::from_secs(2);
        let worker_count = self.projection_runtime.shared.worker_count;
        while !self.managed_connections.exact_live(worker_count) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        if !self.managed_connections.exact_live(worker_count) {
            return Err(EngineError::Storage);
        }
        let creation = self.managed_connections.creation_counts().ok_or(EngineError::Storage)?;
        if creation != (1, READER_POOL_SIZE, 1, worker_count, 0) {
            return Err(EngineError::Storage);
        }
        let writer_autocommit = self
            .connection
            .lock()
            .map_err(|_| EngineError::Storage)?
            .as_ref()
            .is_some_and(Connection::is_autocommit);
        if !writer_autocommit {
            return Err(EngineError::Storage);
        }
        let readers = self.reader_pool.wal_connection_inventory_for_test();
        if readers.len() != READER_POOL_SIZE || readers.into_iter().any(|autocommit| !autocommit) {
            return Err(EngineError::Storage);
        }
        let runtime = self
            .projection_runtime
            .report_runtime_connection_inventory_for_test()
            .map_err(|_| EngineError::Storage)?;
        let expected = projection_runtime::projection_wal_roles(worker_count);
        let actual =
            runtime.iter().map(|(role, index, _)| (*role, *index)).collect::<BTreeSet<_>>();
        if actual != expected || runtime.iter().any(|(_, _, autocommit)| !autocommit) {
            return Err(EngineError::Storage);
        }
        let snapshot = self.wal_attribution_snapshot();
        if !snapshot.no_owned_snapshot
            || snapshot.roles.iter().any(|role| role.active || role.phase != "idle")
        {
            return Err(EngineError::Storage);
        }
        Ok(format!(
            "roles=writer:0,readers:0-7,dispatcher:0,workers:0-{};writer=autocommit;readers=8-autocommit;dispatcher=autocommit;workers={worker_count}-autocommit;creation=writer:{},readers:{},dispatcher:{},workers:{},probes:{}",
            worker_count - 1,
            creation.0, creation.1, creation.2, creation.3, creation.4,
        ))
    }

    /// Run one bounded, test-only checkpoint sampler without performing any
    /// erasure work. The returned records are private diagnostic observations,
    /// not a retry or reclassification of an erasure outcome.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn checkpoint_at_rest_for_test(&self) -> Result<Vec<(bool, u32, u32)>, EngineError> {
        self.ensure_open()?;
        let mut reports = Vec::new();
        for attempt in 0..ERASURE_WAL_TRUNCATE_ATTEMPTS {
            self.wal_attribution.set(WalAttributionRole::Writer, 0, true, "checkpoint_start");
            let overlap = self.wal_attribution.checkpoint_begin();
            self.binding_native_state_observation_for_test("before", (attempt + 1) as usize);
            let started = Instant::now();
            let report = self.wal_checkpoint_truncate_once(false)?;
            self.binding_native_state_observation_for_test("after", (attempt + 1) as usize);
            self.wal_attribution.checkpoint_end();
            self.wal_attribution.set(WalAttributionRole::Writer, 0, false, "idle");
            let snapshot = self.wal_attribution_snapshot();
            let classification = self.wal_attribution.classification(&snapshot, overlap);
            self.wal_attribution.checkpoint_event(
                (attempt + 1) as usize,
                started.elapsed(),
                &report,
                classification,
                snapshot.active_roles,
            );
            reports.push((report.busy != 0, report.log_frames, report.checkpointed_frames));
            if report.busy == 0 {
                break;
            }
            if attempt + 1 < ERASURE_WAL_TRUNCATE_ATTEMPTS {
                thread::sleep(Duration::from_millis(ERASURE_WAL_TRUNCATE_BACKOFF_MS));
            }
        }
        Ok(reports)
    }

    /// Take one native Rusqlite checkpoint sample for the disposable Slice 65
    /// child probe. This opens and drops exactly one independent connection;
    /// it neither opens an Engine nor retries an erasure result.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn native_raw_wal_checkpoint_for_test(path: &str) -> Result<(bool, u32, u32), EngineError> {
        let registry = Arc::new(ManagedConnectionRegistry::default());
        let connection = open_managed_connection(
            Path::new(path),
            ManagedConnectionCategory::RuntimeProbe,
            &registry,
        )
        .map_err(|_| EngineError::Storage)?;
        connection.execute_batch("PRAGMA busy_timeout = 0").map_err(|_| EngineError::Storage)?;
        connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get::<_, i32>(0)? != 0, row.get(1)?, row.get(2)?))
            })
            .map_err(|_| EngineError::Storage)
    }

    /// Recover `--truncate-wal` seam. Runs
    /// `PRAGMA wal_checkpoint(TRUNCATE)` and returns the three counters
    /// SQLite reports. `status = Busy` when SQLite signalled a blocked
    /// checkpoint (`busy != 0`); the WAL may still be partially
    /// checkpointed in that case.
    #[cfg(feature = "operator")]
    pub fn truncate_wal(&self) -> Result<TruncateWalReport, EngineError> {
        self.ensure_open()?;
        // The operator verb keeps SQLite's own busy handler: `recover
        // --truncate-wal` is an explicit, foreground operator act, so waiting out
        // a transient reader is the helpful behaviour.
        self.wal_checkpoint_truncate_once(true)
    }

    /// One `PRAGMA wal_checkpoint(TRUNCATE)` on the writer connection.
    ///
    /// NOT operator-gated: the erasure verbs (`purge` is a default-feature verb)
    /// need it too, and a `#[cfg(feature = "operator")]` helper would break the
    /// default build. Acquires the connection mutex, so callers must NOT already
    /// hold it — every erasure verb calls this AFTER its transaction has
    /// committed and the guard has been dropped.
    ///
    /// `honor_busy_timeout = false` suppresses SQLite's busy handler for the
    /// duration of the checkpoint. rusqlite installs a **5 s** default
    /// `busy_timeout`, so a blocked checkpoint sits for 5 s before reporting
    /// `busy` — under the erasure verbs' bounded retry that compounds to a ~25 s
    /// stall on a verb that is supposed to fail fast. The erasure path therefore
    /// takes the immediate `busy` answer and runs its OWN short backoff; the
    /// prior value is restored before returning, on every path.
    pub(crate) fn wal_checkpoint_truncate_once(
        &self,
        honor_busy_timeout: bool,
    ) -> Result<TruncateWalReport, EngineError> {
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;

        let restore_timeout_ms: Option<i64> = if honor_busy_timeout {
            None
        } else {
            let previous: i64 = connection
                .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                .map_err(|_| EngineError::Storage)?;
            connection.busy_timeout(Duration::ZERO).map_err(|_| EngineError::Storage)?;
            Some(previous)
        };

        let checkpoint: rusqlite::Result<(i64, i64, i64)> =
            connection.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            });

        if let Some(previous) = restore_timeout_ms {
            let previous = u64::try_from(previous.max(0)).unwrap_or(0);
            connection
                .busy_timeout(Duration::from_millis(previous))
                .map_err(|_| EngineError::Storage)?;
        }

        let (busy, log_frames, checkpointed_frames) =
            checkpoint.map_err(|_| EngineError::Storage)?;
        let status = if busy == 0 { TruncateWalStatus::Done } else { TruncateWalStatus::Busy };
        Ok(TruncateWalReport {
            status,
            busy: busy.max(0) as u32,
            log_frames: log_frames.max(0) as u32,
            checkpointed_frames: checkpointed_frames.max(0) as u32,
            discarded_corrupt_wal: false,
        })
    }
}
