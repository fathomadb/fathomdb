//! Private, opt-in D27 observation carried by existing engine workers.

use serde::Serialize;
use std::cell::RefCell;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Instant;

/// Measured work that caused a provider dispatch.
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum Owner {
    /// A foreground search or direct embed operation.
    Foreground { operation_sequence: usize },
    /// One batch or per-job projection request.
    Projection { projection_cursors: Vec<u64> },
}

thread_local! { static OWNER: RefCell<Option<Owner>> = const { RefCell::new(None) }; }

pub(crate) fn current_owner() -> Option<Owner> {
    OWNER.with(|owner| owner.borrow().clone())
}

pub(crate) fn with_owner<R>(owner: Option<Owner>, work: impl FnOnce() -> R) -> R {
    struct Restore(Option<Owner>);
    impl Drop for Restore {
        fn drop(&mut self) {
            OWNER.with(|slot| *slot.borrow_mut() = self.0.take());
        }
    }
    let previous = OWNER.with(|slot| slot.replace(owner));
    let _restore = Restore(previous);
    work()
}

/// Engine-owned lifecycle of one admitted provider request.
#[derive(Clone, Debug, Serialize)]
pub struct DispatchEvent {
    /// This record is emitted by the engine dispatch worker.
    pub source: &'static str,
    /// Unique request identity within this engine observation.
    pub request_id: u64,
    /// Measured foreground sequence or canonical projection cursor set.
    pub owner: Option<Owner>,
    /// Monotonic admission instant relative to the measurement origin.
    pub admitted_ns: u128,
    /// Whether the request entered the bounded queue.
    pub queued: bool,
    /// Monotonic worker-start instant, absent if the request never started.
    pub started_ns: Option<u128>,
    /// Monotonic terminal instant, present when the request finished.
    pub terminal_ns: Option<u128>,
}

/// Resolved worker topology from the engine open.
#[derive(Clone, Debug, Serialize)]
pub struct ConfigurationObservation {
    /// This fact is read from the engine's resolved configuration.
    pub source: &'static str,
    /// Projection orchestration threads.
    pub scheduler_runtime_threads: usize,
    /// Provider dispatch workers.
    pub embedder_pool_size: usize,
}

/// Engine projection admission high-water observation.
#[derive(Clone, Debug, Serialize)]
pub struct ProjectionAdmissionObservation {
    /// This fact is updated by the projection dispatcher.
    pub source: &'static str,
    /// Maximum concurrent active plus queued projection rows.
    pub active_plus_queued_high_water: usize,
}

/// Test-only snapshot of the resolved runtime and observed work.
#[derive(Clone, Debug, Serialize)]
pub struct D27Observation {
    /// Resolved configuration from the engine open.
    pub configuration_observation: ConfigurationObservation,
    /// Maximum projection admission observed by the engine.
    pub projection_admission_observation: ProjectionAdmissionObservation,
    /// Admitted provider requests recorded by the engine.
    pub embed_dispatch_events: Vec<DispatchEvent>,
}

pub(crate) struct Collector {
    origin: Instant,
    high_water: AtomicUsize,
    events: Mutex<Vec<DispatchEvent>>,
}

impl Collector {
    pub(crate) fn new(origin: Instant) -> Self {
        Self { origin, high_water: AtomicUsize::new(0), events: Mutex::new(Vec::new()) }
    }

    pub(crate) fn now_ns(&self) -> u128 {
        self.origin.elapsed().as_nanos()
    }

    pub(crate) fn admit(&self, owner: Option<Owner>) -> u64 {
        let mut events = self.events.lock().unwrap_or_else(|e| e.into_inner());
        let id = events.len() as u64 + 1;
        let event = DispatchEvent {
            source: "engine",
            request_id: id,
            owner,
            admitted_ns: self.now_ns(),
            queued: false,
            started_ns: None,
            terminal_ns: None,
        };
        events.push(event);
        id
    }

    pub(crate) fn start(&self, id: u64) {
        let now = self.now_ns();
        if let Some(event) =
            self.events.lock().unwrap_or_else(|e| e.into_inner()).get_mut((id - 1) as usize)
        {
            event.started_ns = Some(now);
        }
    }

    pub(crate) fn queued(&self, id: u64) {
        if let Some(event) =
            self.events.lock().unwrap_or_else(|e| e.into_inner()).get_mut((id - 1) as usize)
        {
            event.queued = true;
        }
    }

    pub(crate) fn terminal(&self, id: u64) {
        let now = self.now_ns();
        if let Some(event) =
            self.events.lock().unwrap_or_else(|e| e.into_inner()).get_mut((id - 1) as usize)
        {
            if event.terminal_ns.is_none() {
                event.terminal_ns = Some(now);
            }
        }
    }

    pub(crate) fn projection_admitted(&self, count: usize) {
        self.high_water.fetch_max(count, Ordering::Relaxed);
    }

    pub(crate) fn snapshot(&self, scheduler: usize, embedder: usize) -> D27Observation {
        D27Observation {
            configuration_observation: ConfigurationObservation {
                source: "engine",
                scheduler_runtime_threads: scheduler,
                embedder_pool_size: embedder,
            },
            projection_admission_observation: ProjectionAdmissionObservation {
                source: "engine",
                active_plus_queued_high_water: self.high_water.load(Ordering::Relaxed),
            },
            embed_dispatch_events: self.events.lock().unwrap_or_else(|e| e.into_inner()).clone(),
        }
    }
}
