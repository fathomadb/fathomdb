/// Connection identities intentionally contain no path, SQL, request, or user
/// data. They are used only in the opt-in WAL diagnostic stream.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum WalAttributionRole {
    Writer,
    ReaderWorker,
    ProjectionDispatcher,
    ProjectionWorker,
}

impl WalAttributionRole {
    pub(super) const fn name(self) -> &'static str {
        match self {
            Self::Writer => "writer",
            Self::ReaderWorker => "reader_worker",
            Self::ProjectionDispatcher => "projection_dispatcher",
            Self::ProjectionWorker => "projection_worker",
        }
    }
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) fn native_connection_state_for_test(
    connection: &Connection,
    role: WalAttributionRole,
    index: usize,
) -> NativeConnectionStateFact {
    let transaction = match connection.transaction_state(Some("main")) {
        Ok(TransactionState::None) => NativeTransactionState::None,
        Ok(TransactionState::Read) => NativeTransactionState::Read,
        Ok(TransactionState::Write) => NativeTransactionState::Write,
        Ok(_) => NativeTransactionState::Unavailable,
        Err(_) => NativeTransactionState::Unavailable,
    };
    NativeConnectionStateFact {
        role,
        index,
        autocommit: Some(connection.is_autocommit()),
        transaction,
        busy_statement: Some(connection.is_busy()),
        reply: if transaction == NativeTransactionState::Unavailable {
            NativeStateReply::Error("transaction_state")
        } else {
            NativeStateReply::Received
        },
    }
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) fn unavailable_native_connection_state_for_test(
    role: WalAttributionRole,
    index: usize,
    reply: NativeStateReply,
) -> NativeConnectionStateFact {
    NativeConnectionStateFact {
        role,
        index,
        autocommit: None,
        transaction: NativeTransactionState::Unavailable,
        busy_statement: None,
        reply,
    }
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) fn native_state_role_name(role: WalAttributionRole) -> &'static str {
    match role {
        WalAttributionRole::Writer => "writer",
        WalAttributionRole::ReaderWorker => "readers",
        WalAttributionRole::ProjectionDispatcher => "dispatcher",
        WalAttributionRole::ProjectionWorker => "workers",
    }
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) fn native_state_fact_text(fact: &NativeConnectionStateFact) -> String {
    let autocommit = fact
        .autocommit
        .map_or_else(|| "unavailable".to_string(), |value| u8::from(value).to_string());
    let transaction = match fact.transaction {
        NativeTransactionState::None => "none",
        NativeTransactionState::Read => "read",
        NativeTransactionState::Write => "write",
        NativeTransactionState::Unavailable => "unavailable",
    };
    let busy = fact
        .busy_statement
        .map_or_else(|| "unavailable".to_string(), |value| u8::from(value).to_string());
    let delivery = match fact.reply {
        NativeStateReply::Received => "received=1".to_string(),
        NativeStateReply::Timeout => "timeout=1".to_string(),
        NativeStateReply::Error(reason) => format!("error={reason}"),
    };
    format!(
        "{}:{}(auto={autocommit},txn={transaction},busy={busy},{delivery})",
        native_state_role_name(fact.role),
        fact.index,
    )
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) fn native_state_expected_roles() -> BTreeSet<(WalAttributionRole, usize)> {
    BTreeSet::from([
        (WalAttributionRole::Writer, 0),
        (WalAttributionRole::ReaderWorker, 0),
        (WalAttributionRole::ReaderWorker, 1),
        (WalAttributionRole::ReaderWorker, 2),
        (WalAttributionRole::ReaderWorker, 3),
        (WalAttributionRole::ReaderWorker, 4),
        (WalAttributionRole::ReaderWorker, 5),
        (WalAttributionRole::ReaderWorker, 6),
        (WalAttributionRole::ReaderWorker, 7),
        (WalAttributionRole::ProjectionDispatcher, 0),
        (WalAttributionRole::ProjectionWorker, 0),
        (WalAttributionRole::ProjectionWorker, 1),
    ])
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) fn native_state_inventory_text(inventory: &NativeStateInventory) -> String {
    let facts = inventory.facts.iter().map(native_state_fact_text).collect::<Vec<_>>().join(",");
    format!(
        "state_inventory={} reason={} roles={facts}",
        if inventory.complete { "complete" } else { "incomplete" },
        inventory.reason,
    )
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct WalAttributionRoleSnapshot {
    pub(super) role: &'static str,
    pub(super) index: usize,
    pub(super) active: bool,
    pub(super) phase: &'static str,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct WalAttributionSnapshot {
    pub(super) roles: Vec<WalAttributionRoleSnapshot>,
    pub(super) active_roles: Vec<(WalAttributionRole, usize)>,
    pub(super) no_owned_snapshot: bool,
    pub(super) local_checkpoint_overlap: bool,
}

/// A retained, redacted record for one erasure checkpoint attempt. This is
/// private to the engine and is read only by the in-crate Slice 65 witness.
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub(super) struct WalCheckpointRecord {
    pub(super) attempt: usize,
    pub(super) busy: bool,
    pub(super) classification: &'static str,
    pub(super) active_roles: Vec<(WalAttributionRole, usize)>,
}

/// A private before/after record for the *existing* checkpoint invocation in
/// an erasure attempt. It intentionally contains no database path, SQL, or
/// caller data.
#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Debug)]
pub(super) struct ActualCheckpointObservation {
    pub(super) control: &'static str,
    pub(super) phase: &'static str,
    pub(super) ordinal: usize,
    pub(super) writer_autocommit: bool,
    pub(super) direct_inventory: String,
    pub(super) collector_roles: Vec<String>,
    pub(super) checkpoint_begin_overlap: bool,
    pub(super) elapsed: Option<Duration>,
    pub(super) report: Option<TruncateWalReport>,
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) struct ActualCheckpointObserver {
    pub(super) control: &'static str,
    pub(super) records: Vec<ActualCheckpointObservation>,
}

/// Test-only direct SQLite transaction and prepared-statement state captured
/// on the connection's owning thread. This intentionally avoids SQL and FFI:
/// rusqlite exposes the SQLite transaction and statement-busy facts directly.
#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum NativeTransactionState {
    None,
    Read,
    Write,
    Unavailable,
}

#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Debug)]
pub(super) struct NativeConnectionStateFact {
    pub(super) role: WalAttributionRole,
    pub(super) index: usize,
    pub(super) autocommit: Option<bool>,
    pub(super) transaction: NativeTransactionState,
    pub(super) busy_statement: Option<bool>,
    pub(super) reply: NativeStateReply,
}

#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Debug)]
pub(super) enum NativeStateReply {
    Received,
    Timeout,
    Error(&'static str),
}

/// Complete only when every managed role replied on its owning thread and was
/// idle. An incomplete inventory is diagnostic evidence, never a reason to
/// modify or reinterpret a checkpoint outcome.
#[cfg(any(test, feature = "test-hooks"))]
#[derive(Clone, Debug)]
pub(super) struct NativeStateInventory {
    pub(super) facts: Vec<NativeConnectionStateFact>,
    pub(super) complete: bool,
    pub(super) reason: &'static str,
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) struct BindingNativeStateObservation {
    pub(super) phase: &'static str,
    pub(super) ordinal: usize,
    pub(super) inventory: NativeStateInventory,
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) struct BindingNativeStateObserver {
    pub(super) records: Vec<BindingNativeStateObservation>,
}

#[derive(Clone, Copy)]
pub(super) struct WalAttributionRoleState {
    pub(super) active: bool,
    pub(super) phase: &'static str,
}

#[cfg(any(test, feature = "test-hooks"))]
pub(super) type ReaderSnapshotPause = (Arc<Barrier>, Arc<Barrier>, Arc<Mutex<Option<String>>>);

#[cfg(test)]
pub(super) type ReaderHandoffPause = (Arc<Barrier>, Arc<Barrier>);

#[cfg(any(test, feature = "test-hooks"))]
pub(super) type ReaderCompletionPause = (Arc<Barrier>, Arc<Barrier>, Arc<AtomicBool>);

/// Private diagnostic state. It is off unless controlled CI/test config opts in
/// via `FATHOMDB_WAL_ATTRIBUTION=1`; the off path is a single atomic read.
pub(super) struct WalAttributionCollector {
    pub(super) enabled: bool,
    pub(super) started: Instant,
    pub(super) roles: Mutex<BTreeMap<(WalAttributionRole, usize), WalAttributionRoleState>>,
    pub(super) checkpoints: Mutex<Vec<WalCheckpointRecord>>,
    pub(super) checkpoint_active: AtomicBool,
    #[cfg(any(test, feature = "test-hooks"))]
    pub(super) reader_snapshot_pause: Mutex<Option<ReaderSnapshotPause>>,
    #[cfg(test)]
    pub(super) reader_handoff_pause: Mutex<Option<ReaderHandoffPause>>,
    #[cfg(any(test, feature = "test-hooks"))]
    pub(super) reader_completion_pause: Mutex<Option<ReaderCompletionPause>>,
}

/// Marks a real SQLite transaction/snapshot interval and restores idle state on
/// every return path, including `?` propagation and panic unwinding.
pub(super) struct WalAttributionActivity {
    pub(super) collector: Arc<WalAttributionCollector>,
    pub(super) role: WalAttributionRole,
    pub(super) index: usize,
}

impl WalAttributionActivity {
    pub(super) fn begin(
        collector: Arc<WalAttributionCollector>,
        role: WalAttributionRole,
        index: usize,
        phase: &'static str,
    ) -> Self {
        collector.set(role, index, true, phase);
        Self { collector, role, index }
    }
}

impl Drop for WalAttributionActivity {
    fn drop(&mut self) {
        self.collector.set(self.role, self.index, false, "completed");
        self.collector.set(self.role, self.index, false, "idle");
    }
}

impl WalAttributionCollector {
    pub(super) fn new() -> Self {
        Self {
            enabled: cfg!(test) || std::env::var_os("FATHOMDB_WAL_ATTRIBUTION").is_some(),
            started: Instant::now(),
            roles: Mutex::new(BTreeMap::new()),
            checkpoints: Mutex::new(Vec::new()),
            checkpoint_active: AtomicBool::new(false),
            #[cfg(any(test, feature = "test-hooks"))]
            reader_snapshot_pause: Mutex::new(None),
            #[cfg(test)]
            reader_handoff_pause: Mutex::new(None),
            #[cfg(any(test, feature = "test-hooks"))]
            reader_completion_pause: Mutex::new(None),
        }
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(super) fn arm_reader_snapshot_pause(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>, Arc<Mutex<Option<String>>>) {
        let ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let native_state = Arc::new(Mutex::new(None));
        *self.reader_snapshot_pause.lock().expect("reader snapshot pause mutex") =
            Some((Arc::clone(&ready), Arc::clone(&release), Arc::clone(&native_state)));
        (ready, release, native_state)
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(super) fn fire_reader_snapshot_pause(&self, connection: &Connection, worker_idx: usize) {
        if let Some((ready, release, native_state)) =
            self.reader_snapshot_pause.lock().expect("reader snapshot pause mutex").take()
        {
            *native_state.lock().expect("reader snapshot native state") =
                Some(native_state_fact_text(&native_connection_state_for_test(
                    connection,
                    WalAttributionRole::ReaderWorker,
                    worker_idx,
                )));
            ready.wait();
            release.wait();
        }
    }

    #[cfg(test)]
    pub(super) fn arm_reader_handoff_pause(&self) -> (Arc<Barrier>, Arc<Barrier>) {
        let ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        *self.reader_handoff_pause.lock().expect("reader handoff pause mutex") =
            Some((Arc::clone(&ready), Arc::clone(&release)));
        (ready, release)
    }

    #[cfg(test)]
    pub(super) fn fire_reader_handoff_pause(&self) {
        if let Some((ready, release)) =
            self.reader_handoff_pause.lock().expect("reader handoff pause mutex").take()
        {
            ready.wait();
            release.wait();
        }
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(super) fn arm_reader_completion_pause(
        &self,
    ) -> (Arc<Barrier>, Arc<Barrier>, Arc<AtomicBool>) {
        let ready = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let reader_autocommit = Arc::new(AtomicBool::new(false));
        *self.reader_completion_pause.lock().expect("reader completion pause mutex") =
            Some((Arc::clone(&ready), Arc::clone(&release), Arc::clone(&reader_autocommit)));
        (ready, release, reader_autocommit)
    }

    #[cfg(any(test, feature = "test-hooks"))]
    pub(super) fn fire_reader_completion_pause(&self, reader_autocommit: bool) {
        if let Some((ready, release, observed_autocommit)) =
            self.reader_completion_pause.lock().expect("reader completion pause mutex").take()
        {
            observed_autocommit.store(reader_autocommit, Ordering::Release);
            ready.wait();
            release.wait();
        }
    }

    pub(super) fn register(&self, role: WalAttributionRole, index: usize) {
        if !self.enabled {
            return;
        }
        if let Ok(mut roles) = self.roles.lock() {
            roles.insert((role, index), WalAttributionRoleState { active: false, phase: "idle" });
        }
        self.emit(role, index, "opened");
    }

    pub(super) fn set(
        &self,
        role: WalAttributionRole,
        index: usize,
        active: bool,
        phase: &'static str,
    ) {
        if !self.enabled {
            return;
        }
        if let Ok(mut roles) = self.roles.lock() {
            roles.insert((role, index), WalAttributionRoleState { active, phase });
        }
        self.emit(role, index, phase);
    }

    pub(super) fn snapshot(&self) -> WalAttributionSnapshot {
        let local_checkpoint_overlap = self.checkpoint_active.load(Ordering::SeqCst);
        let role_states = self.roles.lock().map(|roles| roles.clone()).unwrap_or_default();
        let roles = role_states
            .iter()
            .map(|((role, index), state)| WalAttributionRoleSnapshot {
                role: role.name(),
                index: *index,
                active: state.active,
                phase: state.phase,
            })
            .collect();
        let active_roles: Vec<(WalAttributionRole, usize)> = role_states
            .iter()
            .filter_map(|((role, index), state)| {
                (state.active
                    && matches!(
                        role,
                        WalAttributionRole::ReaderWorker
                            | WalAttributionRole::ProjectionDispatcher
                            | WalAttributionRole::ProjectionWorker
                    ))
                .then_some((*role, *index))
            })
            .collect();
        let no_owned_snapshot = active_roles.is_empty() && !local_checkpoint_overlap;
        WalAttributionSnapshot { roles, active_roles, no_owned_snapshot, local_checkpoint_overlap }
    }

    pub(super) fn checkpoint_begin(&self) -> bool {
        if !self.enabled {
            return false;
        }
        self.checkpoint_active.swap(true, Ordering::SeqCst)
    }

    pub(super) fn checkpoint_end(&self) {
        if self.enabled {
            self.checkpoint_active.store(false, Ordering::SeqCst);
        }
    }

    pub(super) fn classification(
        &self,
        snapshot: &WalAttributionSnapshot,
        overlap: bool,
    ) -> &'static str {
        if overlap {
            "local_checkpoint_overlap"
        } else if snapshot.roles.iter().any(|role| role.active && role.role == "reader_worker") {
            "owned_reader_snapshot"
        } else if snapshot.roles.iter().any(|role| {
            role.active && matches!(role.role, "projection_dispatcher" | "projection_worker")
        }) {
            "owned_runtime_transaction"
        } else {
            "no_owned_snapshot"
        }
    }

    pub(super) fn checkpoint_event(
        &self,
        attempt: usize,
        elapsed: Duration,
        report: &TruncateWalReport,
        classification: &'static str,
        active_roles: Vec<(WalAttributionRole, usize)>,
    ) {
        if self.enabled {
            if let Ok(mut checkpoints) = self.checkpoints.lock() {
                checkpoints.push(WalCheckpointRecord {
                    attempt,
                    busy: report.status == TruncateWalStatus::Busy,
                    classification,
                    active_roles: active_roles.clone(),
                });
            }
            eprintln!(
                "slice65_wal checkpoint_attempt={} elapsed_ms={} busy={} log_frames={} checkpointed_frames={} classification={} active_roles={}",
                attempt,
                elapsed.as_millis(),
                report.busy,
                report.log_frames,
                report.checkpointed_frames,
                classification,
                format_active_wal_roles(&active_roles),
            );
        }
    }

    #[allow(dead_code)]
    pub(super) fn checkpoints(&self) -> Vec<WalCheckpointRecord> {
        self.checkpoints.lock().map(|records| records.clone()).unwrap_or_default()
    }

    pub(super) fn emit(&self, role: WalAttributionRole, index: usize, phase: &'static str) {
        eprintln!(
            "slice65_wal role={} index={} phase={} elapsed_ms={}",
            role.name(),
            index,
            phase,
            self.started.elapsed().as_millis()
        );
    }
}

pub(super) fn format_active_wal_roles(roles: &[(WalAttributionRole, usize)]) -> String {
    roles
        .iter()
        .map(|(role, index)| format!("{}:{index}", role.name()))
        .collect::<Vec<_>>()
        .join(",")
}
use crate::{TruncateWalReport, TruncateWalStatus};
#[cfg(any(test, feature = "test-hooks"))]
use rusqlite::{Connection, TransactionState};
use std::collections::BTreeMap;
#[cfg(any(test, feature = "test-hooks"))]
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(any(test, feature = "test-hooks"))]
use std::sync::Barrier;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
