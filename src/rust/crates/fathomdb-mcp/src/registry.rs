//! Store registry: names to files, the per-store lifecycle state machine
//! (`closed → opening → ready → draining → closed`), open epochs, the idle
//! clock, the open-store limit and the apply-on-open list (design §4, §5).
//!
//! One `std::sync::Mutex` guards state transitions only. It is never held
//! across an `.await` or an engine call: opens and drains run on spawned
//! tasks and blocking threads, and report back under the mutex.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::{Duration, Instant};

use fathomdb_sdk::{read, Engine, OpenOptions};
use tokio::sync::{watch, Notify, OwnedSemaphorePermit, Semaphore};

use crate::config::Config;
use crate::error::{Busy, BusyReason, ToolError};
use crate::hooks::Hooks;
use crate::names::{validate_store_name, NameError};

/// Store lifecycle phase.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Closed,
    Opening,
    Reopening,
    Ready,
    Draining,
}

impl Phase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::Opening => "opening",
            Self::Reopening => "reopening",
            Self::Ready => "ready",
            Self::Draining => "draining",
        }
    }
}

/// Facts captured by the apply-on-open list for one open epoch.
#[derive(Debug)]
pub(crate) struct OpenInfo {
    /// A telemetry sink was enabled for this epoch.
    pub(crate) telemetry: bool,
    /// The engine's dense runtime is usable (`embedding_readiness`).
    pub(crate) embedder_usable: bool,
    /// The engine belongs to an embedding host (MR-004).
    pub(crate) hosted: bool,
}

/// Fires when the client cancels the call (`notifications/cancelled`).
#[derive(Clone)]
pub(crate) struct Cancel(watch::Receiver<bool>);

impl Cancel {
    pub(crate) fn new(cancelled: watch::Receiver<bool>) -> Self {
        Self(cancelled)
    }

    /// Resolves once the call is cancelled; never, if it is not.
    async fn cancelled(&mut self) {
        if self.0.wait_for(|cancelled| *cancelled).await.is_err() {
            std::future::pending::<()>().await;
        }
    }
}

/// The tool kinds with their own service-time window (queue-admission
/// § 6.1). Search is split by mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(crate) enum ToolKind {
    SearchText,
    SearchHybrid,
    Fetch,
    Expand,
    Orient,
    /// Takes a permit but never waits and leaves no sample.
    Feedback,
}

impl ToolKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "search_text" => Some(Self::SearchText),
            "search_hybrid" => Some(Self::SearchHybrid),
            "fetch" => Some(Self::Fetch),
            "expand" => Some(Self::Expand),
            "orient" => Some(Self::Orient),
            _ => None,
        }
    }
}

/// What a call brings to admission.
pub(crate) struct Admission<'a> {
    pub(crate) kind: ToolKind,
    /// The call deadline; `None` (internal callers only) never waits.
    pub(crate) wait_until: Option<Instant>,
    /// Progress notifications while the call waits.
    pub(crate) progress: Option<&'a ProgressFn>,
    /// The client's cancel; a waiting call leaves the queue on it.
    pub(crate) cancel: Option<&'a Cancel>,
}

/// Progress callback for a call that waits on an open: `(progress, phase)`.
pub(crate) type ProgressFn = Arc<dyn Fn(f64, &'static str) + Send + Sync>;

enum Source {
    Managed { path: PathBuf, create: bool },
    Hosted,
}

struct Slot {
    source: Source,
    phase: Phase,
    epoch: u64,
    attempt: u64,
    engine: Option<Arc<Engine>>,
    info: Option<Arc<OpenInfo>>,
    load: Arc<Load>,
    last_used: LastUsed,
    last_error: Option<ToolError>,
    changed: watch::Sender<u64>,
}

impl Slot {
    fn new(source: Source, cap: usize) -> Self {
        Self {
            source,
            phase: Phase::Closed,
            epoch: 0,
            attempt: 0,
            engine: None,
            info: None,
            load: Arc::new(Load::new(cap)),
            last_used: LastUsed::default(),
            last_error: None,
            changed: watch::channel(0).0,
        }
    }

    fn managed(&self) -> bool {
        matches!(self.source, Source::Managed { .. })
    }

    fn bump(&self) {
        self.changed.send_modify(|version| *version += 1);
    }

    fn readiness(&self) -> &'static str {
        match (self.phase, &self.last_error) {
            (Phase::Closed, Some(_)) => "failed",
            (phase, _) => phase.as_str(),
        }
    }
}

/// The least deadline a call of a tool kind with no history must have left
/// to be started (queue-admission § 6.1, early-shed rule).
const NEED_FLOOR: Duration = Duration::from_millis(50);
/// Completed calls per tool kind kept in a store's service-time window.
const WINDOW_SAMPLES: usize = 128;
/// Completed calls of a kind before its window replaces the floor.
const WARM_SAMPLES: usize = 8;
/// The share of recent calls of a kind that must fit in the time a waiter
/// has left. Provisional: the run-to-completion follow-up decides whether
/// the check stays.
const NEED_QUANTILE: f64 = 0.9;

/// Admission state of one store. Kept across reopens, so waiters survive a
/// drain.
struct Load {
    cap: usize,
    /// `admission_cap` permits, handed out in request order.
    permits: Arc<Semaphore>,
    /// Calls waiting for a permit.
    queued: AtomicUsize,
    /// Abandoned calls whose engine work still holds a permit.
    abandoned: AtomicUsize,
    /// Fires when the last permit is returned.
    idle: Notify,
    /// Service times of the last `WINDOW_SAMPLES` completed calls, per tool
    /// kind, oldest first.
    windows: Mutex<BTreeMap<ToolKind, VecDeque<Duration>>>,
}

impl Load {
    fn new(cap: usize) -> Self {
        Self {
            cap,
            permits: Arc::new(Semaphore::new(cap)),
            queued: AtomicUsize::new(0),
            abandoned: AtomicUsize::new(0),
            idle: Notify::new(),
            windows: Mutex::new(BTreeMap::new()),
        }
    }

    fn windows(&self) -> MutexGuard<'_, BTreeMap<ToolKind, VecDeque<Duration>>> {
        self.windows.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn record(&self, kind: ToolKind, sample: Duration) {
        if kind != ToolKind::Feedback {
            let mut windows = self.windows();
            let window = windows.entry(kind).or_default();
            if window.len() == WINDOW_SAMPLES {
                window.pop_front();
            }
            window.push_back(sample);
        }
    }

    /// The early-shed threshold: the least time a call of `kind` must have
    /// left to be started.
    fn need(&self, kind: ToolKind) -> Duration {
        let windows = self.windows();
        let Some(window) = windows.get(&kind).filter(|window| window.len() >= WARM_SAMPLES) else {
            return NEED_FLOOR;
        };
        let mut sorted: Vec<Duration> = window.iter().copied().collect();
        sorted.sort_unstable();
        // Nearest rank: the smallest sample at or above the quantile.
        let rank = (NEED_QUANTILE * sorted.len() as f64).ceil() as usize;
        sorted[rank.clamp(1, sorted.len()) - 1]
    }

    /// Permits held. A call counts from the moment it holds a permit, before
    /// its guard exists, so idle expiry and drains never miss it.
    fn in_flight(&self) -> usize {
        self.cap - self.permits.available_permits()
    }

    /// Calls holding or waiting for a permit.
    fn busy_calls(&self) -> usize {
        self.in_flight() + self.queued.load(Ordering::SeqCst)
    }

    fn busy(&self, reason: BusyReason) -> Busy {
        let abandoned = self.abandoned.load(Ordering::SeqCst);
        Busy {
            reason,
            in_flight: self.in_flight(),
            queued: self.queued.load(Ordering::SeqCst),
            abandoned,
            // Abandoned work has no known end, so a store whose every permit
            // it holds cannot say when it frees up.
            stuck: abandoned >= self.cap,
        }
    }
}

/// An admission permit. Dropping it returns the permit and wakes drains
/// waiting for the store to go idle.
struct Permit {
    permit: Option<OwnedSemaphorePermit>,
    load: Arc<Load>,
}

impl Drop for Permit {
    fn drop(&mut self) {
        drop(self.permit.take());
        if self.load.in_flight() == 0 {
            self.load.idle.notify_waiters();
        }
    }
}

/// One call counted in `queued`. Dropping it, on every exit from the wait
/// (permit, timeout, cancel, phase change, panic), uncounts it.
struct QueueTicket<'a>(&'a Load);

impl<'a> QueueTicket<'a> {
    fn new(load: &'a Load) -> Self {
        load.queued.fetch_add(1, Ordering::SeqCst);
        Self(load)
    }
}

impl Drop for QueueTicket<'_> {
    fn drop(&mut self) {
        self.0.queued.fetch_sub(1, Ordering::SeqCst);
    }
}

/// When a store last admitted or finished a call. Shared with its guards,
/// which stamp it on release without taking the registry mutex.
#[derive(Clone)]
struct LastUsed(Arc<Mutex<Instant>>);

impl Default for LastUsed {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Instant::now())))
    }
}

impl LastUsed {
    fn touch(&self) {
        *self.0.lock().unwrap_or_else(|poison| poison.into_inner()) = Instant::now();
    }

    fn get(&self) -> Instant {
        *self.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }
}

#[derive(Default)]
struct Inner {
    slots: BTreeMap<String, Slot>,
    shutting_down: bool,
}

/// One row of `list_stores`.
pub(crate) struct Listing {
    pub(crate) name: String,
    pub(crate) open: bool,
    pub(crate) readiness: &'static str,
}

/// An admitted call on a ready store. Holding it keeps the store from going
/// idle or draining; dropping it releases the admission slot without taking
/// the registry mutex.
pub(crate) struct StoreGuard {
    pub(crate) name: String,
    pub(crate) epoch: u64,
    pub(crate) engine: Arc<Engine>,
    pub(crate) info: Arc<OpenInfo>,
    last_used: LastUsed,
    kind: ToolKind,
    /// When the permit was acquired.
    started: Instant,
    run: Arc<AtomicU8>,
    // Dropped after `drop` below has run, so the sample is in the window
    // before the next waiter gets the permit and computes its need.
    permit: Permit,
}

impl Drop for StoreGuard {
    fn drop(&mut self) {
        // Stamp before releasing the slot, so the idle clock never sees a
        // finished call as idle since its start (FIX-1 CR-5).
        self.last_used.touch();
        let load = &self.permit.load;
        match self.run.compare_exchange(RUNNING, DONE, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => load.record(self.kind, self.started.elapsed()),
            // Abandoned calls leave no sample; the window would otherwise
            // learn from work nobody waited for.
            Err(_) => {
                load.abandoned.fetch_sub(1, Ordering::SeqCst);
            }
        }
    }
}

const RUNNING: u8 = 0;
const DONE: u8 = 1;
const ABANDONED: u8 = 2;

impl StoreGuard {
    /// A handle that marks this call abandoned after the guard has moved
    /// into the blocking closure.
    pub(crate) fn abandoner(&self) -> Abandoner {
        Abandoner { run: self.run.clone(), load: self.permit.load.clone() }
    }
}

/// Marks a running call abandoned: its permit stays held until its thread
/// returns, it is counted in `abandoned`, and it leaves no sample.
pub(crate) struct Abandoner {
    run: Arc<AtomicU8>,
    load: Arc<Load>,
}

impl Abandoner {
    /// Returns false when the call had already finished.
    pub(crate) fn abandon(&self) -> bool {
        // Counted before the flag flips, so the guard's decrement can never
        // run first and wrap the counter.
        self.load.abandoned.fetch_add(1, Ordering::SeqCst);
        if self.run.compare_exchange(RUNNING, ABANDONED, Ordering::SeqCst, Ordering::SeqCst).is_ok()
        {
            return true;
        }
        self.load.abandoned.fetch_sub(1, Ordering::SeqCst);
        false
    }

    /// The store's load as a busy object with `reason`.
    pub(crate) fn busy(&self, reason: BusyReason) -> Busy {
        self.load.busy(reason)
    }
}

/// What `acquire` does after one look at the slot under the mutex.
enum Next {
    Admit { load: Arc<Load>, epoch: u64, changed: watch::Receiver<u64> },
    Wait(watch::Receiver<u64>, Phase),
}

/// How a wait for a permit ended.
enum Waited {
    Permit(OwnedSemaphorePermit),
    Expired,
    Moved,
    Cancelled,
}

/// Why a host engine could not be registered.
#[derive(Debug, Eq, PartialEq)]
pub(crate) enum RegisterError {
    BadName,
    Duplicate,
}

pub(crate) struct Registry {
    config: Arc<Config>,
    hooks: Hooks,
    inner: Mutex<Inner>,
    me: Weak<Registry>,
}

/// How long shutdown waits beyond the call deadline for stores to drain: an
/// abandoned call cannot outlive its deadline by more than this unless it
/// is stuck, and a stuck call must not hold the process open.
const SHUTDOWN_MARGIN: Duration = Duration::from_secs(5);

impl Registry {
    pub(crate) fn new(config: Arc<Config>, hooks: Hooks) -> Arc<Registry> {
        Arc::new_cyclic(|me| Registry {
            config,
            hooks,
            inner: Mutex::new(Inner::default()),
            me: me.clone(),
        })
    }

    fn cap(&self) -> usize {
        self.config.admission_cap as usize
    }

    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn this(&self) -> Arc<Registry> {
        self.me.upgrade().expect("registry is alive while its methods run")
    }

    /// Register a host-owned engine under `name` (MR-004). The registry never
    /// opens, drains or closes it.
    pub(crate) fn register_hosted(
        &self,
        name: &str,
        engine: Arc<Engine>,
    ) -> Result<(), RegisterError> {
        validate_store_name(name).map_err(|_| RegisterError::BadName)?;
        // Reserve the name before touching the engine, so a rejected
        // duplicate never re-runs apply-on-open (which would restart the
        // engine's telemetry sequence). The list itself runs outside the
        // mutex because it does file I/O.
        {
            let mut inner = self.lock();
            if inner.slots.contains_key(name) || self.config.stores.contains_key(name) {
                return Err(RegisterError::Duplicate);
            }
            let mut slot = Slot::new(Source::Hosted, self.cap());
            slot.phase = Phase::Opening;
            inner.slots.insert(name.to_string(), slot);
        }
        let info = apply_on_open(&engine, name, 1, &self.config, true);
        let mut inner = self.lock();
        let slot = inner.slots.get_mut(name).expect("reserved above");
        slot.phase = Phase::Ready;
        slot.epoch = 1;
        slot.attempt = 1;
        slot.engine = Some(engine);
        slot.info = Some(Arc::new(info));
        slot.bump();
        Ok(())
    }

    /// Validate that `name` is addressable, creating its slot on first use.
    /// The grammar is checked before any filesystem access (MR-005.3).
    pub(crate) fn ensure_slot(&self, name: &str) -> Result<(), ToolError> {
        if validate_store_name(name).is_err() {
            return Err(ToolError::bad_store_name(name));
        }
        if self.lock().slots.contains_key(name) {
            return Ok(());
        }
        match self.config.resolve(name) {
            Ok(resolution) => {
                let mut inner = self.lock();
                let cap = self.cap();
                inner.slots.entry(name.to_string()).or_insert_with(|| {
                    Slot::new(
                        Source::Managed { path: resolution.path, create: resolution.create },
                        cap,
                    )
                });
                Ok(())
            }
            Err(NameError::BadName) => Err(ToolError::bad_store_name(name)),
            Err(NameError::Unknown) => Err(ToolError::unknown_store(name, self.candidates(name))),
        }
    }

    fn all_names(&self) -> BTreeSet<String> {
        let mut names: BTreeSet<String> = self.config.addressable_names().into_iter().collect();
        let inner = self.lock();
        names.extend(
            inner.slots.iter().filter(|(_, slot)| !slot.managed()).map(|(name, _)| name.clone()),
        );
        names
    }

    fn candidates(&self, name: &str) -> Vec<String> {
        let mut scored: Vec<(usize, String)> = self
            .all_names()
            .into_iter()
            .map(|candidate| (edit_distance(name, &candidate), candidate))
            .collect();
        scored.sort();
        let threshold = (name.chars().count() / 2).max(3);
        let near: Vec<String> = scored
            .iter()
            .filter(|(distance, _)| *distance <= threshold)
            .take(5)
            .map(|(_, candidate)| candidate.clone())
            .collect();
        if near.is_empty() {
            scored.into_iter().take(5).map(|(_, candidate)| candidate).collect()
        } else {
            near
        }
    }

    /// Every addressable or registered store with its state (MR-005.2).
    pub(crate) fn listing(&self) -> Vec<Listing> {
        let names = self.all_names();
        let inner = self.lock();
        names
            .into_iter()
            .map(|name| match inner.slots.get(&name) {
                Some(slot) => Listing {
                    open: matches!(slot.phase, Phase::Ready | Phase::Draining),
                    readiness: slot.readiness(),
                    name,
                },
                None => Listing { name, open: false, readiness: "closed" },
            })
            .collect()
    }

    /// The current epoch of `name` when it is ready.
    pub(crate) fn ready_epoch(&self, name: &str) -> Option<u64> {
        let inner = self.lock();
        inner.slots.get(name).filter(|slot| slot.phase == Phase::Ready).map(|slot| slot.epoch)
    }

    /// Admit one call on `name`.
    ///
    /// A ready store admits the call when a permit is free; otherwise the
    /// call waits for one, in arrival order, until `wait_until`. A closed
    /// store starts opening (draining the least recently used idle store
    /// when the open limit is reached). A call with a deadline in
    /// `wait_until` waits for the open until then, reporting progress when
    /// `progress` is given, and returns `not_ready` with the phase at the
    /// deadline. Without a deadline (internal callers only) it never waits:
    /// it returns `busy` when no permit is free, `not_ready` at once on a
    /// store that is not ready, or the error the previous open recorded
    /// while the retry starts (MR-003.2, MR-006.6).
    pub(crate) async fn acquire(
        &self,
        name: &str,
        call: &Admission<'_>,
    ) -> Result<StoreGuard, ToolError> {
        let wait_until = call.wait_until;
        self.ensure_slot(name)?;
        let mut waited_attempt: Option<u64> = None;
        let mut ticks = 0_u32;
        loop {
            let next = {
                let mut inner = self.lock();
                if inner.shutting_down {
                    return Err(ToolError::not_ready(name, "draining"));
                }
                let slot = inner.slots.get_mut(name).expect("slot exists after ensure_slot");
                match slot.phase {
                    Phase::Ready => Next::Admit {
                        load: slot.load.clone(),
                        epoch: slot.epoch,
                        changed: slot.changed.subscribe(),
                    },
                    Phase::Closed => {
                        if let (Some(attempt), Some(error)) = (waited_attempt, &slot.last_error) {
                            if slot.attempt == attempt {
                                return Err(error.clone());
                            }
                        }
                        let recorded = slot.last_error.clone();
                        self.start_open_locked(&mut inner, name)?;
                        let slot = inner.slots.get(name).expect("slot exists");
                        if wait_until.is_none() {
                            return Err(recorded.unwrap_or_else(|| {
                                ToolError::not_ready(name, slot.phase.as_str())
                            }));
                        }
                        waited_attempt = Some(slot.attempt);
                        Next::Wait(slot.changed.subscribe(), slot.phase)
                    }
                    phase @ (Phase::Opening | Phase::Reopening | Phase::Draining) => {
                        if wait_until.is_none() {
                            return Err(ToolError::not_ready(name, phase.as_str()));
                        }
                        if phase != Phase::Draining {
                            waited_attempt = Some(slot.attempt);
                        }
                        Next::Wait(slot.changed.subscribe(), phase)
                    }
                }
            };
            let (mut changed, phase) = match next {
                Next::Admit { load, epoch, changed } => {
                    match self.admit(name, load, epoch, changed, call).await? {
                        Some(guard) => return Ok(guard),
                        None => continue,
                    }
                }
                Next::Wait(changed, phase) => (changed, phase),
            };
            let deadline = wait_until.expect("only waiting calls reach here");
            let now = Instant::now();
            if now >= deadline {
                return Err(ToolError::not_ready(name, phase.as_str()));
            }
            if let Some(progress) = call.progress {
                progress(f64::from(ticks), phase.as_str());
                ticks += 1;
            }
            let step = (deadline - now).min(Duration::from_secs(1));
            let _ = tokio::time::timeout(step, changed.changed()).await;
        }
    }

    /// Wait for a permit of a ready store. `None` means the store left
    /// `Ready` (or reopened) meanwhile, and the caller goes back to the open
    /// wait.
    async fn admit(
        &self,
        name: &str,
        load: Arc<Load>,
        epoch: u64,
        mut changed: watch::Receiver<u64>,
        call: &Admission<'_>,
    ) -> Result<Option<StoreGuard>, ToolError> {
        let refuse = |reason| ToolError::admission(name, load.busy(reason));
        for (kind, sample) in self.hooks.seeded_service_times(name) {
            if let Some(kind) = ToolKind::from_name(&kind) {
                load.record(kind, sample);
            }
        }
        let permit = match call.wait_until {
            None => load
                .permits
                .clone()
                .try_acquire_owned()
                .map_err(|_| refuse(BusyReason::Deadline))?,
            Some(deadline) => {
                // Step 2: the only test on arrival.
                if load.permits.available_permits() == 0
                    && load.queued.load(Ordering::SeqCst) >= self.config.queue_limit as usize
                {
                    return Err(refuse(BusyReason::QueueFull));
                }
                let ticket = QueueTicket::new(&load);
                self.hooks.after_queued(name);
                let waited = async {
                    // Pinned once: dropping the acquire future would lose
                    // the call's place in the queue.
                    let acquire = load.permits.clone().acquire_owned();
                    tokio::pin!(acquire);
                    // A call is started only if it has at least `need` left
                    // (step 3), so the wait ends `need` before the deadline.
                    let need = load.need(call.kind);
                    let wait_end = deadline.checked_sub(need).unwrap_or(deadline);
                    let expired = tokio::time::sleep_until(wait_end.into());
                    tokio::pin!(expired);
                    let mut cancel = call.cancel.cloned();
                    let cancelled = async {
                        match cancel.as_mut() {
                            Some(cancel) => cancel.cancelled().await,
                            None => std::future::pending().await,
                        }
                    };
                    tokio::pin!(cancelled);
                    loop {
                        tokio::select! {
                            biased;
                            permit = &mut acquire => {
                                return Waited::Permit(
                                    permit.expect("admission semaphores are never closed"),
                                );
                            }
                            () = &mut expired => return Waited::Expired,
                            () = &mut cancelled => return Waited::Cancelled,
                            _ = changed.changed() => {
                                if self.ready_epoch(name) != Some(epoch) {
                                    return Waited::Moved;
                                }
                            }
                        }
                    }
                }
                .await;
                drop(ticket);
                match waited {
                    Waited::Permit(permit) => permit,
                    Waited::Expired => return Err(refuse(BusyReason::Deadline)),
                    Waited::Moved => return Ok(None),
                    Waited::Cancelled => return Err(ToolError::cancelled()),
                }
            }
        };
        let started = Instant::now();
        let permit = Permit { permit: Some(permit), load: load.clone() };
        self.hooks.after_permit(name).await;
        let mut inner = self.lock();
        if inner.shutting_down {
            return Err(ToolError::not_ready(name, "draining"));
        }
        let slot = inner.slots.get_mut(name).expect("slot exists after ensure_slot");
        if slot.phase != Phase::Ready || slot.epoch != epoch {
            return Ok(None);
        }
        // Step 4: need is re-evaluated, since calls that completed during the
        // wait may have changed it.
        if let Some(deadline) = call.wait_until {
            if deadline.saturating_duration_since(Instant::now()) < load.need(call.kind) {
                drop(permit);
                return Err(refuse(BusyReason::Deadline));
            }
        }
        slot.last_used.touch();
        Ok(Some(StoreGuard {
            name: name.to_string(),
            epoch: slot.epoch,
            engine: slot.engine.clone().expect("ready slots hold an engine"),
            info: slot.info.clone().expect("ready slots hold open info"),
            last_used: slot.last_used.clone(),
            kind: call.kind,
            started,
            run: Arc::new(AtomicU8::new(RUNNING)),
            permit,
        }))
    }

    fn start_open_locked(&self, inner: &mut Inner, name: &str) -> Result<(), ToolError> {
        let limit = self.config.max_open_stores as usize;
        let others: Vec<(String, Phase, usize, Instant)> = inner
            .slots
            .iter()
            .filter(|(other, slot)| {
                other.as_str() != name && slot.managed() && slot.phase != Phase::Closed
            })
            .map(|(other, slot)| {
                (other.clone(), slot.phase, slot.load.busy_calls(), slot.last_used.get())
            })
            .collect();
        let mut waits = Vec::new();
        if others.len() >= limit {
            let mut to_free = others.len() + 1 - limit;
            let mut drainable: Vec<&(String, Phase, usize, Instant)> = others
                .iter()
                .filter(|(_, phase, busy_calls, _)| *phase == Phase::Ready && *busy_calls == 0)
                .collect();
            drainable.sort_by_key(|(_, _, _, last_used)| *last_used);
            let draining: Vec<&String> = others
                .iter()
                .filter(|(_, phase, _, _)| *phase == Phase::Draining)
                .map(|(other, _, _, _)| other)
                .collect();
            if draining.len() + drainable.len() < to_free {
                return Err(ToolError::busy(name));
            }
            for other in draining.into_iter().take(to_free) {
                waits.push(other.clone());
                to_free -= 1;
            }
            for (other, _, _, _) in drainable.into_iter().take(to_free) {
                self.start_drain_locked(inner, other);
                waits.push(other.clone());
            }
        }
        let registry = self.this();
        let slot = inner.slots.get_mut(name).expect("slot exists");
        let Source::Managed { path, create } = &slot.source else {
            unreachable!("hosted slots are always ready");
        };
        let (path, create) = (path.clone(), *create);
        slot.phase = if slot.epoch > 0 { Phase::Reopening } else { Phase::Opening };
        slot.attempt += 1;
        slot.bump();
        let next_epoch = slot.epoch + 1;
        let waits: Vec<(String, watch::Receiver<u64>)> = waits
            .into_iter()
            .map(|other| {
                let receiver = inner.slots[&other].changed.subscribe();
                (other, receiver)
            })
            .collect();
        let name = name.to_string();
        tokio::spawn(async move {
            for (other, mut receiver) in waits {
                while registry.phase(&other) != Some(Phase::Closed) {
                    if receiver.changed().await.is_err() {
                        break;
                    }
                }
            }
            let config = registry.config.clone();
            let hooks = registry.hooks.clone();
            let task_name = name.clone();
            let opened = tokio::task::spawn_blocking(move || {
                open_store(&task_name, &path, create, next_epoch, &config, &hooks)
            })
            .await
            .unwrap_or_else(|_| Err(ToolError::panic()));
            registry.finish_open(&name, next_epoch, opened);
        });
        Ok(())
    }

    fn phase(&self, name: &str) -> Option<Phase> {
        self.lock().slots.get(name).map(|slot| slot.phase)
    }

    fn finish_open(
        &self,
        name: &str,
        epoch: u64,
        opened: Result<(Arc<Engine>, OpenInfo), ToolError>,
    ) {
        let mut inner = self.lock();
        let shutting_down = inner.shutting_down;
        let slot = inner.slots.get_mut(name).expect("slot exists");
        match opened {
            Ok((engine, info)) => {
                slot.engine = Some(engine);
                slot.info = Some(Arc::new(info));
                slot.epoch = epoch;
                slot.phase = Phase::Ready;
                slot.last_used.touch();
                slot.last_error = None;
                slot.bump();
                if shutting_down {
                    self.start_drain_locked(&mut inner, name);
                }
            }
            Err(error) => {
                slot.phase = Phase::Closed;
                slot.last_error = Some(error);
                slot.bump();
            }
        }
    }

    fn start_drain_locked(&self, inner: &mut Inner, name: &str) {
        let slot = inner.slots.get_mut(name).expect("slot exists");
        debug_assert_eq!(slot.phase, Phase::Ready);
        slot.phase = Phase::Draining;
        slot.bump();
        let engine = slot.engine.clone();
        let load = slot.load.clone();
        let registry = self.this();
        let name = name.to_string();
        let drain_ms = self.config.call_deadline_ms;
        tokio::spawn(async move {
            wait_idle(&load).await;
            if let Some(engine) = engine {
                let store = name.clone();
                // `close` runs whether or not `drain` succeeded, so a failed
                // drain (EmbedderRequired, Scheduler) never keeps the file
                // locked.
                let _ = tokio::task::spawn_blocking(move || {
                    if let Err(error) = engine.drain(drain_ms) {
                        tracing::warn!(store = %store, error = %error, "drain before close failed; closing anyway");
                    }
                    if let Err(error) = engine.close() {
                        tracing::warn!(store = %store, error = %error, "close failed");
                    }
                })
                .await;
            }
            registry.finish_drain(&name);
        });
    }

    fn finish_drain(&self, name: &str) {
        let mut inner = self.lock();
        let slot = inner.slots.get_mut(name).expect("slot exists");
        slot.engine = None;
        slot.info = None;
        slot.phase = Phase::Closed;
        slot.bump();
        tracing::info!(store = %name, "store closed");
    }

    /// Start opening every `eager` store.
    pub(crate) fn open_eager(&self) {
        let eager: Vec<String> = self
            .config
            .stores
            .iter()
            .filter(|(_, entry)| entry.eager)
            .map(|(name, _)| name.clone())
            .collect();
        for name in eager {
            if self.ensure_slot(&name).is_err() {
                continue;
            }
            let mut inner = self.lock();
            if inner.slots[&name].phase == Phase::Closed {
                if let Err(error) = self.start_open_locked(&mut inner, &name) {
                    tracing::warn!(store = %name, code = %error.code, "eager open deferred");
                }
            }
        }
    }

    /// Drain every ready store idle for at least `idle_timeout_secs`.
    pub(crate) fn expire_idle(&self) {
        let timeout = Duration::from_secs(self.config.idle_timeout_secs);
        if timeout.is_zero() {
            return;
        }
        let mut inner = self.lock();
        if inner.shutting_down {
            return;
        }
        let expired: Vec<String> = inner
            .slots
            .iter()
            .filter(|(_, slot)| {
                slot.managed()
                    && slot.phase == Phase::Ready
                    && slot.load.busy_calls() == 0
                    && slot.last_used.get().elapsed() >= timeout
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in expired {
            tracing::info!(store = %name, "idle timeout; closing");
            self.start_drain_locked(&mut inner, &name);
        }
    }

    /// Stop admitting calls and drain every adapter-opened store. Host
    /// engines are left to their host.
    pub(crate) async fn shutdown(&self) {
        let started = Instant::now();
        loop {
            let pending = {
                let mut inner = self.lock();
                inner.shutting_down = true;
                let ready: Vec<String> = inner
                    .slots
                    .iter()
                    .filter(|(_, slot)| slot.managed() && slot.phase == Phase::Ready)
                    .map(|(name, _)| name.clone())
                    .collect();
                for name in ready {
                    self.start_drain_locked(&mut inner, &name);
                }
                inner
                    .slots
                    .values()
                    .find(|slot| slot.managed() && slot.phase != Phase::Closed)
                    .map(|slot| slot.changed.subscribe())
            };
            let Some(mut changed) = pending else { return };
            let bound = Duration::from_millis(self.config.call_deadline_ms) + SHUTDOWN_MARGIN;
            let Some(remaining) = bound.checked_sub(started.elapsed()) else {
                tracing::warn!("shutdown: stores still busy after the bound; exiting anyway");
                return;
            };
            let _ = tokio::time::timeout(remaining, changed.changed()).await;
        }
    }
}

async fn wait_idle(load: &Load) {
    loop {
        let notified = load.idle.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        if load.in_flight() == 0 {
            return;
        }
        notified.await;
    }
}

fn open_store(
    name: &str,
    path: &Path,
    create: bool,
    epoch: u64,
    config: &Config,
    hooks: &Hooks,
) -> Result<(Arc<Engine>, OpenInfo), ToolError> {
    hooks.before_open(name);
    // `Engine::open` creates a missing file, so `create: false` is enforced
    // here. The check-then-open race is accepted: the directory is
    // operator-controlled.
    if !create && !path.try_exists().unwrap_or(false) {
        let error = ToolError::missing_file(name);
        tracing::warn!(store = %name, class = "unavailable", code = %error.code, "store open failed");
        return Err(error);
    }
    let options =
        OpenOptions { use_default_embedder: config.use_default_embedder, ..OpenOptions::default() };
    let engine = Engine::open(path, options).map_err(|error| {
        let mapped = ToolError::from_open(name, &error);
        tracing::warn!(
            store = %name,
            class = "unavailable",
            code = %mapped.code,
            error = %error,
            "store open failed"
        );
        mapped
    })?;
    let info = apply_on_open(&engine, name, epoch, config, false);
    tracing::info!(store = %name, epoch, "store ready");
    Ok((Arc::new(engine), info))
}

/// The apply-on-open list (design §5): run after every successful open, first
/// or reopen, so both take the same path (MR-006.2).
fn apply_on_open(
    engine: &Engine,
    name: &str,
    epoch: u64,
    config: &Config,
    hosted: bool,
) -> OpenInfo {
    let embedder_usable = read::embedding_readiness(engine)
        .map(|readiness| readiness.usable_embedder)
        .unwrap_or(false);
    // A fresh sink file per epoch: engine query ids are `q0-<seq>` and the
    // sequence restarts on every enable, so reusing a file would repeat ids
    // (MR-024.7).
    let telemetry = match &config.telemetry_dir {
        None => false,
        Some(dir) => {
            let file = dir.join(format!("{name}.{}.{epoch}.jsonl", std::process::id()));
            let enabled = std::fs::create_dir_all(dir).is_ok()
                && file.to_str().is_some_and(|file| engine.enable_telemetry(file).is_ok());
            if !enabled {
                tracing::warn!(store = %name, sink = %file.display(), "telemetry sink could not be enabled");
            }
            enabled
        }
    };
    OpenInfo { telemetry, embedder_usable, hosted }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let right: Vec<char> = right.chars().collect();
    let mut previous: Vec<usize> = (0..=right.len()).collect();
    for (i, l) in left.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, r) in right.iter().enumerate() {
            let substitution = previous[j] + usize::from(l != *r);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[right.len()]
}
