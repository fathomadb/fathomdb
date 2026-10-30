//! Drain phase counters (experiment tooling, `dev/experiments/ac013-seed-drain/`).
//!
//! Process-global wall-time and count accumulators for the projection
//! runtime's drain phases: the dispatcher's pending-job scan, each worker
//! batch's embed, `commit_gate` wait and commit, and `Engine::drain`'s
//! pending-work probe on the engine connection. Only compiled with
//! `test-hooks`; always on there (a few atomics per scan or batch).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

/// A snapshot of the drain phase counters.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct DrainPhaseTrace {
    /// Dispatcher scans (`next_pending_projection_jobs` calls).
    pub dispatcher_scans: u64,
    pub dispatcher_scan_ns: u64,
    /// Jobs the dispatcher enqueued.
    pub dispatcher_jobs: u64,
    /// Candidate rows the scans read, including ones skipped as in flight.
    pub dispatcher_rows_seen: u64,
    pub dispatcher_inflight_skipped: u64,
    /// Candidate-page queries the scans ran.
    pub dispatcher_pages: u64,
    /// Worker commit batches and the jobs in them.
    pub worker_batches: u64,
    pub worker_jobs: u64,
    pub worker_embed_ns: u64,
    pub worker_gate_wait_ns: u64,
    /// From holding `commit_gate` to the end of the commit.
    pub worker_commit_ns: u64,
    /// `Engine::drain` pending-work probes, their time, and how many found work.
    pub drain_probes: u64,
    pub drain_probe_ns: u64,
    pub drain_probe_pending: u64,
}

macro_rules! counters {
    ($($name:ident),* $(,)?) => {
        struct Counters { $($name: AtomicU64,)* }
        static COUNTERS: Counters = Counters { $($name: AtomicU64::new(0),)* };
        /// Read every counter without clearing.
        #[doc(hidden)]
        pub fn drain_phase_trace_for_test() -> DrainPhaseTrace {
            DrainPhaseTrace { $($name: COUNTERS.$name.load(Ordering::Relaxed),)* }
        }
        /// Zero every counter.
        #[doc(hidden)]
        pub fn reset_drain_phase_trace_for_test() {
            $(COUNTERS.$name.store(0, Ordering::Relaxed);)*
        }
    };
}

counters!(
    dispatcher_scans,
    dispatcher_scan_ns,
    dispatcher_jobs,
    dispatcher_rows_seen,
    dispatcher_inflight_skipped,
    dispatcher_pages,
    worker_batches,
    worker_jobs,
    worker_embed_ns,
    worker_gate_wait_ns,
    worker_commit_ns,
    drain_probes,
    drain_probe_ns,
    drain_probe_pending,
);

fn elapsed_ns(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

pub(crate) fn dispatcher_scan(started: Instant, jobs: usize) {
    COUNTERS.dispatcher_scans.fetch_add(1, Ordering::Relaxed);
    COUNTERS.dispatcher_scan_ns.fetch_add(elapsed_ns(started), Ordering::Relaxed);
    COUNTERS.dispatcher_jobs.fetch_add(jobs as u64, Ordering::Relaxed);
}

pub(crate) fn dispatcher_page(rows: usize) {
    COUNTERS.dispatcher_pages.fetch_add(1, Ordering::Relaxed);
    COUNTERS.dispatcher_rows_seen.fetch_add(rows as u64, Ordering::Relaxed);
}

pub(crate) fn dispatcher_inflight_skip() {
    COUNTERS.dispatcher_inflight_skipped.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn worker_embed(started: Instant, jobs: usize) {
    COUNTERS.worker_batches.fetch_add(1, Ordering::Relaxed);
    COUNTERS.worker_jobs.fetch_add(jobs as u64, Ordering::Relaxed);
    COUNTERS.worker_embed_ns.fetch_add(elapsed_ns(started), Ordering::Relaxed);
}

pub(crate) fn worker_gate_wait(started: Instant) {
    COUNTERS.worker_gate_wait_ns.fetch_add(elapsed_ns(started), Ordering::Relaxed);
}

pub(crate) fn worker_commit(started: Instant) {
    COUNTERS.worker_commit_ns.fetch_add(elapsed_ns(started), Ordering::Relaxed);
}

pub(crate) fn drain_probe(started: Instant, pending: Option<bool>) {
    if pending.is_none() {
        return;
    }
    COUNTERS.drain_probes.fetch_add(1, Ordering::Relaxed);
    COUNTERS.drain_probe_ns.fetch_add(elapsed_ns(started), Ordering::Relaxed);
    if pending == Some(true) {
        COUNTERS.drain_probe_pending.fetch_add(1, Ordering::Relaxed);
    }
}
