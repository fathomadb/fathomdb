use super::*;

/// Slice 72's private trusted-runner rendezvous. This is compiled only by the
/// non-shipped characterization feature and exists solely to delimit real CE
/// forwards against a test wrapper around the real BGE embedder.
#[cfg(feature = "slice72-test-hooks")]
#[doc(hidden)]
pub mod slice72_test_hooks {
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{Arc, Condvar, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    const RENDEZVOUS_TIMEOUT: Duration = Duration::from_secs(15);
    static CE_RENDEZVOUS: Mutex<Option<Arc<ForwardRendezvous>>> = Mutex::new(None);

    #[derive(Default)]
    struct Arrival {
        bge_ready: bool,
        ce_ready: bool,
    }

    /// Actual-forward timestamps captured by a Slice 72 rendezvous.
    #[derive(Clone, Copy, Debug)]
    pub struct ForwardIntervals {
        bge_start_ns: u64,
        bge_end_ns: u64,
        ce_start_ns: u64,
        ce_end_ns: u64,
        timed_out: bool,
    }

    /// Result of a deterministic rendezvous fixture. It preserves the old
    /// overlap predicate while making attempted reuse observable to tests.
    pub struct ForwardRun {
        result: Result<ForwardIntervals, &'static str>,
        active_overlap_sample_timestamp: Option<u64>,
    }

    impl ForwardRun {
        #[must_use]
        pub fn overlaps(&self) -> bool {
            self.result.as_ref().is_ok_and(|intervals| intervals.overlaps())
        }

        #[must_use]
        pub fn is_err(&self) -> bool {
            self.result.is_err()
        }

        /// The host timestamp captured while both deterministic fixture
        /// forwards remained active.
        #[must_use]
        pub fn active_overlap_sample_timestamp(&self) -> Option<u64> {
            self.active_overlap_sample_timestamp
        }
    }

    impl ForwardIntervals {
        /// True only when the actual BGE and CE forward intervals overlap.
        #[must_use]
        pub fn overlaps(self) -> bool {
            !self.timed_out
                && self.bge_start_ns < self.ce_end_ns
                && self.ce_start_ns < self.bge_end_ns
        }
    }

    /// One bounded, single-use rendezvous between a test BGE wrapper and the
    /// CE `score_batch` forward boundary.
    pub struct ForwardRendezvous {
        started: Instant,
        arrival: Mutex<Arrival>,
        arrived: Condvar,
        timed_out: AtomicBool,
        bge_start_ns: AtomicU64,
        bge_end_ns: AtomicU64,
        ce_start_ns: AtomicU64,
        ce_end_ns: AtomicU64,
        bge_active: AtomicBool,
        ce_active: AtomicBool,
        bge_claimed: AtomicBool,
        ce_claimed: AtomicBool,
        capture_count: AtomicU64,
        fixture_claimed: AtomicBool,
    }

    impl ForwardRendezvous {
        /// Creates an unarmed rendezvous. It becomes visible to CE only through
        /// [`install_ce_forward_rendezvous`].
        #[must_use]
        pub fn new() -> Arc<Self> {
            Self::new_for_run(Instant::now())
        }

        /// Creates an unarmed rendezvous on a trusted runner's receipt clock.
        ///
        /// The hardware harness samples before model warm-up, so its later
        /// real-forward timestamps must retain that same origin rather than
        /// restarting at rendezvous construction.
        #[must_use]
        pub fn new_for_run(started: Instant) -> Arc<Self> {
            Arc::new(Self {
                started,
                arrival: Mutex::new(Arrival::default()),
                arrived: Condvar::new(),
                timed_out: AtomicBool::new(false),
                bge_start_ns: AtomicU64::new(0),
                bge_end_ns: AtomicU64::new(0),
                ce_start_ns: AtomicU64::new(0),
                ce_end_ns: AtomicU64::new(0),
                bge_active: AtomicBool::new(false),
                ce_active: AtomicBool::new(false),
                bge_claimed: AtomicBool::new(false),
                ce_claimed: AtomicBool::new(false),
                capture_count: AtomicU64::new(0),
                fixture_claimed: AtomicBool::new(false),
            })
        }

        fn stamp_ns(&self) -> u64 {
            u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX)
        }

        fn meet(&self, bge: bool) {
            let mut arrival = self.arrival.lock().expect("Slice 72 rendezvous lock");
            if bge {
                arrival.bge_ready = true;
            } else {
                arrival.ce_ready = true;
            }
            self.arrived.notify_all();
            let wait = self.arrived.wait_timeout_while(arrival, RENDEZVOUS_TIMEOUT, |state| {
                !(state.bge_ready && state.ce_ready)
            });
            match wait {
                Ok((_, timeout)) if timeout.timed_out() => {
                    self.timed_out.store(true, Ordering::Release);
                }
                Err(_) => self.timed_out.store(true, Ordering::Release),
                _ => {}
            }
        }

        /// Runs an actual BGE forward immediately after the two-party
        /// rendezvous and records only the actual-forward interval.
        pub fn run_bge_forward<T>(&self, operation: impl FnOnce() -> T) -> T {
            if self
                .bge_claimed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return operation();
            }
            self.meet(true);
            self.bge_start_ns.store(self.stamp_ns(), Ordering::Release);
            self.bge_active.store(true, Ordering::Release);
            let result = operation();
            self.bge_active.store(false, Ordering::Release);
            self.bge_end_ns.store(self.stamp_ns(), Ordering::Release);
            self.finish_capture();
            result
        }

        fn run_ce_forward<T>(&self, operation: impl FnOnce() -> T) -> T {
            if self
                .ce_claimed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return operation();
            }
            self.meet(false);
            self.ce_start_ns.store(self.stamp_ns(), Ordering::Release);
            self.ce_active.store(true, Ordering::Release);
            let result = operation();
            self.ce_active.store(false, Ordering::Release);
            self.ce_end_ns.store(self.stamp_ns(), Ordering::Release);
            self.finish_capture();
            result
        }

        fn finish_capture(&self) {
            if self.bge_end_ns.load(Ordering::Acquire) != 0
                && self.ce_end_ns.load(Ordering::Acquire) != 0
            {
                let _ =
                    self.capture_count.compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire);
            }
        }

        /// Number of complete BGE/CE forward interval captures. It can only be
        /// zero or one for a rendezvous instance.
        #[must_use]
        pub fn capture_count(&self) -> u64 {
            self.capture_count.load(Ordering::Acquire)
        }

        /// Captures a monotonic timestamp only after both claimed real forwards
        /// are active. The returned value is inside both recorded intervals.
        #[must_use]
        pub fn active_overlap_sample_timestamp(&self) -> Option<u64> {
            if self.bge_active.load(Ordering::Acquire) && self.ce_active.load(Ordering::Acquire) {
                return Some(self.stamp_ns());
            }
            None
        }

        /// Verifies that a telemetry timestamp belongs to the one actual
        /// captured BGE/CE overlap interval.
        #[must_use]
        pub fn timestamp_is_within_captured_overlap(&self, timestamp_ns: u64) -> bool {
            let intervals = self.intervals();
            self.capture_count() == 1
                && intervals.overlaps()
                && timestamp_ns >= intervals.bge_start_ns
                && timestamp_ns <= intervals.bge_end_ns
                && timestamp_ns >= intervals.ce_start_ns
                && timestamp_ns <= intervals.ce_end_ns
        }

        /// Waits until both real forward calls are active so a test-only sensor
        /// sample can begin during their overlap window.
        pub fn wait_for_active_overlap(&self, timeout: Duration) -> bool {
            let deadline = Instant::now() + timeout;
            while Instant::now() < deadline {
                if self.bge_active.load(Ordering::Acquire) && self.ce_active.load(Ordering::Acquire)
                {
                    return true;
                }
                thread::yield_now();
            }
            false
        }

        /// Returns the actual-forward timestamps; callers must require
        /// [`ForwardIntervals::overlaps`] before claiming concurrent execution.
        #[must_use]
        pub fn intervals(&self) -> ForwardIntervals {
            ForwardIntervals {
                bge_start_ns: self.bge_start_ns.load(Ordering::Acquire),
                bge_end_ns: self.bge_end_ns.load(Ordering::Acquire),
                ce_start_ns: self.ce_start_ns.load(Ordering::Acquire),
                ce_end_ns: self.ce_end_ns.load(Ordering::Acquire),
                timed_out: self.timed_out.load(Ordering::Acquire),
            }
        }

        /// Deterministic mechanism fixture for the test contract. Real
        /// characterization uses [`run_bge_forward`](Self::run_bge_forward) and
        /// the CE boundary, not this fixture.
        #[must_use]
        pub fn run_contract_fixture(self: &Arc<Self>) -> ForwardRun {
            if self
                .fixture_claimed
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                return ForwardRun {
                    result: Err("Slice 72 rendezvous is single-use"),
                    active_overlap_sample_timestamp: None,
                };
            }
            let bge = Arc::clone(self);
            let ce = Arc::clone(self);
            let bge_thread = thread::spawn(move || {
                bge.run_bge_forward(|| thread::sleep(Duration::from_millis(1)))
            });
            let ce_thread = thread::spawn(move || {
                ce.run_ce_forward(|| thread::sleep(Duration::from_millis(1)))
            });
            let active_overlap_sample_timestamp = self
                .wait_for_active_overlap(RENDEZVOUS_TIMEOUT)
                .then(|| self.active_overlap_sample_timestamp())
                .flatten();
            bge_thread.join().expect("BGE contract fixture joins");
            ce_thread.join().expect("CE contract fixture joins");
            ForwardRun { result: Ok(self.intervals()), active_overlap_sample_timestamp }
        }
    }

    /// Installs a rendezvous for one concurrent operation and removes it when
    /// the returned guard drops. Warm-up must happen before installation.
    pub fn install_ce_forward_rendezvous(
        rendezvous: Arc<ForwardRendezvous>,
    ) -> Result<InstalledForwardRendezvous, &'static str> {
        let mut active = CE_RENDEZVOUS.lock().map_err(|_| "Slice 72 hook lock poisoned")?;
        if active.is_some() {
            return Err("Slice 72 CE forward hook is already installed");
        }
        *active = Some(rendezvous.clone());
        Ok(InstalledForwardRendezvous { rendezvous })
    }

    /// RAII guard for one installed CE-forward rendezvous.
    pub struct InstalledForwardRendezvous {
        rendezvous: Arc<ForwardRendezvous>,
    }

    impl Drop for InstalledForwardRendezvous {
        fn drop(&mut self) {
            if let Ok(mut active) = CE_RENDEZVOUS.lock() {
                if active.as_ref().is_some_and(|current| Arc::ptr_eq(current, &self.rendezvous)) {
                    *active = None;
                }
            }
        }
    }

    /// Wraps the real CE forward only while a trusted-runner test has armed its
    /// rendezvous. Normal builds compile this module out entirely.
    pub fn with_ce_forward<T>(operation: impl FnOnce() -> T) -> T {
        let rendezvous = CE_RENDEZVOUS.lock().ok().and_then(|active| active.clone());
        match rendezvous {
            Some(rendezvous) => rendezvous.run_ce_forward(operation),
            None => operation(),
        }
    }

    #[cfg(test)]
    #[test]
    fn contract_fixture_records_actual_forward_overlap() {
        assert!(ForwardRendezvous::new().run_contract_fixture().overlaps());
    }
}

#[cfg(debug_assertions)]
pub(crate) const PROJECTION_TRANSACTION_TEST_PAUSE_RELEASE_TIMEOUT: Duration =
    Duration::from_secs(30);

/// Failure to observe an armed projection-worker transaction pause.
#[cfg(debug_assertions)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[doc(hidden)]
pub enum ProjectionWorkerPauseReadyError {
    /// The worker did not reach the armed pause before the supplied deadline.
    Timeout(Duration),
    /// The worker dropped its readiness sender without reaching the pause.
    WorkerDisconnected,
}

#[cfg(debug_assertions)]
impl Display for ProjectionWorkerPauseReadyError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout(timeout) => write!(
                f,
                "projection worker did not reach transaction pause within {}ms",
                timeout.as_millis()
            ),
            Self::WorkerDisconnected => {
                f.write_str("projection worker transaction pause disconnected before readiness")
            }
        }
    }
}

#[cfg(debug_assertions)]
impl Error for ProjectionWorkerPauseReadyError {}

/// Cancellation-safe handle for the test-only projection transaction pause.
#[cfg(debug_assertions)]
#[derive(Debug)]
#[doc(hidden)]
pub struct ProjectionWorkerTransactionPauseForTest {
    pub(crate) ready: Option<Receiver<()>>,
    pub(crate) release: Option<mpsc::Sender<()>>,
    pub(crate) ready_observed: bool,
}

#[cfg(debug_assertions)]
impl ProjectionWorkerTransactionPauseForTest {
    /// Wait at most `timeout` for the worker to acquire its WAL transaction.
    /// A timeout or disconnected worker cancels the pause before returning.
    pub fn wait_ready(&mut self, timeout: Duration) -> Result<(), ProjectionWorkerPauseReadyError> {
        if self.ready_observed {
            return Ok(());
        }
        let outcome = self
            .ready
            .as_ref()
            .ok_or(ProjectionWorkerPauseReadyError::WorkerDisconnected)?
            .recv_timeout(timeout);
        match outcome {
            Ok(()) => {
                self.ready.take();
                self.ready_observed = true;
                Ok(())
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                self.ready.take();
                self.release();
                Err(ProjectionWorkerPauseReadyError::Timeout(timeout))
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                self.ready.take();
                self.release();
                Err(ProjectionWorkerPauseReadyError::WorkerDisconnected)
            }
        }
    }

    /// Release the paused worker. Repeated calls are harmless.
    pub fn release(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
    }
}

#[cfg(debug_assertions)]
impl Drop for ProjectionWorkerTransactionPauseForTest {
    fn drop(&mut self) {
        self.release();
    }
}

/// 0.8.20 keystone closeout fix-3 — a test-only rendezvous hook fired at the TOP
/// of [`read_search_in_tx`], BEFORE the reader opens its deferred transaction.
///
/// It exists ONLY to make the validate/execute TOCTOU race deterministic: a test
/// arms a closure that parks the reader worker here (after the caller-side search
/// setup, before the reader pins its snapshot), performs a concurrent
/// `configure_projections` DROP of a `filterable` attribute on the writer
/// connection, then releases the reader. The reader then pins a snapshot that
/// INCLUDES the drop — exactly the window that used to yield an opaque `no such
/// column` `Storage` error and now yields a typed `InvalidFilter`. Kept OFF the
/// governed surface (`_for_test`), mirroring the sanctioned
/// `set_vector_stage_only_for_test` seam pattern. Disarmed by default: a single
/// `Relaxed` atomic load per search (same class as the four hot-path atomics
/// already read here), fires at most once (the closure is `take`n), and is a
/// no-op in production because nothing ever arms it.
pub(crate) mod reader_search_hook {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    static ARMED: AtomicBool = AtomicBool::new(false);
    #[allow(clippy::type_complexity)]
    static HOOK: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

    pub(crate) fn arm(hook: Box<dyn Fn() + Send>) {
        *HOOK.lock().expect("reader-search hook mutex") = Some(hook);
        ARMED.store(true, Ordering::SeqCst);
    }

    pub(crate) fn clear() {
        ARMED.store(false, Ordering::SeqCst);
        *HOOK.lock().expect("reader-search hook mutex") = None;
    }

    /// Fire the armed hook exactly ONCE, then disarm. Cheap early-out when
    /// disarmed (the production and common-test path).
    pub(crate) fn fire() {
        if !ARMED.load(Ordering::SeqCst) {
            return;
        }
        // Disarm first so a re-entrant / second reader never re-fires.
        ARMED.store(false, Ordering::SeqCst);
        let hook = HOOK.lock().expect("reader-search hook mutex").take();
        if let Some(hook) = hook {
            hook();
        }
    }
}

/// 0.8.20 keystone closeout fix-3 — arm the [`reader_search_hook`] (test-only).
/// See that module's docs. `#[doc(hidden)]`, `_for_test`; never re-exported from
/// the `fathomdb` facade.
#[doc(hidden)]
pub fn arm_reader_search_hook_for_test(hook: Box<dyn Fn() + Send>) {
    reader_search_hook::arm(hook);
}

/// 0.8.20 keystone closeout fix-3 — disarm the [`reader_search_hook`] (test-only).
#[doc(hidden)]
pub fn clear_reader_search_hook_for_test() {
    reader_search_hook::clear();
}

/// Slice 35 — one-shot test rendezvous after a frozen reader has authenticated
/// its context and validated the pinned SQLite snapshot.
///
/// This hook proves that a visibility mutation committed after validation
/// cannot alter the already-linearized operation. It is never armed by
/// production code and remains outside the governed API surface.
pub(crate) mod frozen_after_validation_hook {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    static ARMED: AtomicBool = AtomicBool::new(false);
    #[allow(clippy::type_complexity)]
    static HOOK: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

    pub(crate) fn arm(hook: Box<dyn Fn() + Send>) {
        *HOOK.lock().expect("frozen after-validation hook mutex") = Some(hook);
        ARMED.store(true, Ordering::SeqCst);
    }

    pub(crate) fn fire() {
        if !ARMED.swap(false, Ordering::SeqCst) {
            return;
        }
        if let Some(hook) = HOOK.lock().expect("frozen after-validation hook mutex").take() {
            hook();
        }
    }
}

/// Arm the Slice-35 post-validation rendezvous for one frozen read.
#[doc(hidden)]
pub fn arm_frozen_after_validation_hook_for_test(hook: Box<dyn Fn() + Send>) {
    frozen_after_validation_hook::arm(hook);
}

/// Arm the same post-validation rendezvous specifically for one frozen page.
#[doc(hidden)]
pub fn arm_page_after_validation_hook_for_test(hook: Box<dyn Fn() + Send>) {
    frozen_after_validation_hook::arm(hook);
}

// Slice 50 one-shot rendezvous for the exact evidence transaction seams. Both
// hooks are dormant on production paths and fire only when explicitly armed by
// an integration test.
pub(crate) mod evidence_linearization_hooks {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    struct OneShotHook {
        armed: AtomicBool,
        hook: Mutex<Option<Box<dyn Fn() + Send>>>,
    }

    impl OneShotHook {
        const fn new() -> Self {
            Self { armed: AtomicBool::new(false), hook: Mutex::new(None) }
        }

        fn arm(&self, hook: Box<dyn Fn() + Send>) {
            *self.hook.lock().expect("evidence linearization hook mutex") = Some(hook);
            self.armed.store(true, Ordering::SeqCst);
        }

        fn fire(&self) {
            if !self.armed.swap(false, Ordering::SeqCst) {
                return;
            }
            if let Some(hook) = self.hook.lock().expect("evidence linearization hook mutex").take()
            {
                hook();
            }
        }
    }

    static BEFORE_SIDECAR: OneShotHook = OneShotHook::new();
    static BEFORE_RESOLVE_RETURN: OneShotHook = OneShotHook::new();

    pub(crate) fn arm_before_sidecar(hook: Box<dyn Fn() + Send>) {
        BEFORE_SIDECAR.arm(hook);
    }

    pub(crate) fn fire_before_sidecar() {
        BEFORE_SIDECAR.fire();
    }

    pub(crate) fn arm_before_resolve_return(hook: Box<dyn Fn() + Send>) {
        BEFORE_RESOLVE_RETURN.arm(hook);
    }

    pub(crate) fn fire_before_resolve_return() {
        BEFORE_RESOLVE_RETURN.fire();
    }
}

/// Arm the Slice-50 post-ranking, pre-sidecar rendezvous for one evidence search.
#[doc(hidden)]
pub fn arm_evidence_before_sidecar_hook_for_test(hook: Box<dyn Fn() + Send>) {
    evidence_linearization_hooks::arm_before_sidecar(hook);
}

/// Arm the Slice-50 post-resolution, pre-return rendezvous for one evidence read.
#[doc(hidden)]
pub fn arm_evidence_before_resolve_return_hook_for_test(hook: Box<dyn Fn() + Send>) {
    evidence_linearization_hooks::arm_before_resolve_return(hook);
}

#[cfg(feature = "test-hooks")]
#[doc(hidden)]
impl Engine {
    /// Arm one graph-evidence resolver rendezvous on this engine only.
    pub fn arm_graph_evidence_before_resolve_return_hook_for_test(
        &self,
        hook: Box<dyn Fn() + Send>,
    ) {
        *self
            .graph_evidence_before_resolve_return_hook
            .lock()
            .expect("graph evidence resolver hook mutex") = Some(hook);
    }

    /// Arm one pre-primary-lock erasure rendezvous on this engine only.
    pub fn arm_erasure_before_primary_lock_hook_for_test(&self, hook: Box<dyn Fn() + Send>) {
        *self.erasure_before_primary_lock_hook.lock().expect("engine erasure hook mutex") =
            Some(hook);
    }
}

#[cfg(feature = "test-hooks")]
pub(crate) mod slice15_erasure_lock_hook {
    use std::sync::Mutex;

    static HOOK: Mutex<Option<Box<dyn Fn() + Send>>> = Mutex::new(None);

    pub(crate) fn arm(hook: Box<dyn Fn() + Send>) {
        *HOOK.lock().expect("Slice 15 erasure lock hook mutex") = Some(hook);
    }

    pub(crate) fn fire() {
        if let Some(hook) = HOOK.lock().expect("Slice 15 erasure lock hook mutex").take() {
            hook();
        }
    }
}

/// Arm a one-shot Slice 15 rendezvous immediately before erasure attempts the
/// primary connection lock.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn arm_erasure_before_primary_lock_hook_for_test(hook: Box<dyn Fn() + Send>) {
    slice15_erasure_lock_hook::arm(hook);
}

#[cfg(feature = "test-hooks")]
pub(crate) mod explanation_finalization_hooks {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    type ArmedHook = (std::thread::ThreadId, Box<dyn Fn() + Send>);

    struct OneShotHook {
        armed: AtomicBool,
        hook: Mutex<Option<ArmedHook>>,
    }

    impl OneShotHook {
        const fn new() -> Self {
            Self { armed: AtomicBool::new(false), hook: Mutex::new(None) }
        }

        fn arm(&self, hook: Box<dyn Fn() + Send>) {
            *self.hook.lock().expect("explanation finalization hook mutex") =
                Some((std::thread::current().id(), hook));
            self.armed.store(true, Ordering::SeqCst);
        }

        fn fire(&self) {
            if !self.armed.load(Ordering::SeqCst) {
                return;
            }
            let mut guard = self.hook.lock().expect("explanation finalization hook mutex");
            if guard.as_ref().is_none_or(|(thread_id, _)| *thread_id != std::thread::current().id())
            {
                return;
            }
            self.armed.store(false, Ordering::SeqCst);
            if let Some((_, hook)) = guard.take() {
                drop(guard);
                hook();
            }
        }
    }

    static BEFORE_TELEMETRY_LOCK: OneShotHook = OneShotHook::new();
    static AFTER_TELEMETRY_LOCK: OneShotHook = OneShotHook::new();

    pub(crate) fn arm_before(hook: Box<dyn Fn() + Send>) {
        BEFORE_TELEMETRY_LOCK.arm(hook);
    }

    pub(crate) fn arm_after(hook: Box<dyn Fn() + Send>) {
        AFTER_TELEMETRY_LOCK.arm(hook);
    }

    pub(crate) fn fire_before() {
        BEFORE_TELEMETRY_LOCK.fire();
    }

    pub(crate) fn fire_after() {
        AFTER_TELEMETRY_LOCK.fire();
    }
}

/// Arm a one-shot test rendezvous immediately before explanation finalization locks telemetry.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn arm_explanation_before_telemetry_lock_hook_for_test(hook: Box<dyn Fn() + Send>) {
    explanation_finalization_hooks::arm_before(hook);
}

/// Arm a one-shot test rendezvous after explanation finalization releases the telemetry lock.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
pub fn arm_explanation_after_telemetry_lock_hook_for_test(hook: Box<dyn Fn() + Send>) {
    explanation_finalization_hooks::arm_after(hook);
}
