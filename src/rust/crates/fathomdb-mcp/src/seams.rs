//! Adapter-only test seams (feature `test-seams`): hold a store's open or
//! its engine calls on the blocking thread, panic inside a tool handler, or
//! send a burst of progress notifications from one.
//! They live in the adapter's bridge and need no engine hook (MR-003.1,
//! MR-033.1, MR-002.2).

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

#[derive(Default)]
struct State {
    open_blocked: BTreeSet<String>,
    calls_blocked: BTreeSet<String>,
    /// Calls held by `block_calls`, per store, oldest first.
    held: BTreeMap<String, VecDeque<u64>>,
    /// Held calls let through one at a time by `release_one_call`.
    released: BTreeSet<u64>,
    next_call: u64,
    panicking_tools: BTreeSet<String>,
    panicking_calls: BTreeSet<String>,
    panicking_waiters: BTreeSet<String>,
    admitted_blocked: BTreeSet<String>,
    admitted_held: BTreeMap<String, usize>,
    progress_bursts: BTreeMap<String, u32>,
}

/// Injectable delays and faults, shared by every clone.
#[derive(Clone, Default)]
pub struct Seams {
    inner: Arc<(Mutex<State>, Condvar)>,
}

impl Seams {
    fn state(&self) -> MutexGuard<'_, State> {
        self.inner.0.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    fn wait_while(&self, blocked: impl Fn(&State) -> bool) {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap_or_else(|poison| poison.into_inner());
        while blocked(&state) {
            state = condvar.wait(state).unwrap_or_else(|poison| poison.into_inner());
        }
    }

    /// Hold every open of `store` before `Engine::open` until released.
    pub fn block_open(&self, store: &str) {
        self.state().open_blocked.insert(store.to_string());
    }

    /// Let held and future opens of `store` proceed.
    pub fn release_open(&self, store: &str) {
        self.state().open_blocked.remove(store);
        self.inner.1.notify_all();
    }

    /// Hold every engine call on `store` (on its blocking thread) until released.
    pub fn block_calls(&self, store: &str) {
        self.state().calls_blocked.insert(store.to_string());
    }

    /// Let held and future engine calls on `store` proceed.
    pub fn release_calls(&self, store: &str) {
        self.state().calls_blocked.remove(store);
        self.inner.1.notify_all();
    }

    /// Let the oldest call held on `store` proceed; later calls stay held.
    /// Returns false when no call is held.
    pub fn release_one_call(&self, store: &str) -> bool {
        let mut state = self.state();
        let Some(call) = state.held.get_mut(store).and_then(VecDeque::pop_front) else {
            return false;
        };
        state.released.insert(call);
        drop(state);
        self.inner.1.notify_all();
        true
    }

    /// How many calls on `store` are held by `block_calls` right now.
    pub fn held_calls(&self, store: &str) -> usize {
        self.state().held.get(store).map_or(0, VecDeque::len)
    }

    /// Release every held open and call, including calls held after admission.
    pub fn release_all(&self) {
        let mut state = self.state();
        state.open_blocked.clear();
        state.calls_blocked.clear();
        state.admitted_blocked.clear();
        drop(state);
        self.inner.1.notify_all();
    }

    /// Make every later call to `tool` panic inside its handler.
    pub fn panic_in_tool(&self, tool: &str) {
        self.state().panicking_tools.insert(tool.to_string());
    }

    /// Make every later engine call on `store` panic on its blocking thread.
    pub fn panic_calls(&self, store: &str) {
        self.state().panicking_calls.insert(store.to_string());
    }

    /// Undo [`Seams::panic_calls`].
    pub fn stop_panicking_calls(&self, store: &str) {
        self.state().panicking_calls.remove(store);
    }

    /// Make every later call on `store` that has to wait for an admission
    /// permit panic in its handler task, after joining the queue and before
    /// the wait.
    pub fn panic_queued_calls(&self, store: &str) {
        self.state().panicking_waiters.insert(store.to_string());
    }

    /// Undo [`Seams::panic_queued_calls`].
    pub fn stop_panicking_queued_calls(&self, store: &str) {
        self.state().panicking_waiters.remove(store);
    }

    /// Hold every call on `store` right after it gets its admission permit,
    /// before its store guard exists, until released.
    pub fn hold_admitted_calls(&self, store: &str) {
        self.state().admitted_blocked.insert(store.to_string());
    }

    /// Let calls held by [`Seams::hold_admitted_calls`] proceed.
    pub fn release_admitted_calls(&self, store: &str) {
        self.state().admitted_blocked.remove(store);
    }

    /// How many calls on `store` are held between permit and guard.
    pub fn admitted_held(&self, store: &str) -> usize {
        self.state().admitted_held.get(store).copied().unwrap_or(0)
    }

    /// Make every later call to `tool` that carries a progress token send `n`
    /// progress notifications (values 1..=n) before the tool body runs.
    pub fn burst_progress(&self, tool: &str, n: u32) {
        self.state().progress_bursts.insert(tool.to_string(), n);
    }

    pub(crate) fn before_open(&self, store: &str) {
        self.wait_while(|state| state.open_blocked.contains(store));
    }

    pub(crate) fn before_call(&self, store: &str) {
        self.hold_call(store);
        if self.state().panicking_calls.contains(store) {
            panic!("test seam: forced panic in an engine call on {store}");
        }
    }

    fn hold_call(&self, store: &str) {
        let (lock, condvar) = &*self.inner;
        let mut state = lock.lock().unwrap_or_else(|poison| poison.into_inner());
        if !state.calls_blocked.contains(store) {
            return;
        }
        let call = state.next_call;
        state.next_call += 1;
        state.held.entry(store.to_string()).or_default().push_back(call);
        while state.calls_blocked.contains(store) && !state.released.contains(&call) {
            state = condvar.wait(state).unwrap_or_else(|poison| poison.into_inner());
        }
        state.released.remove(&call);
        if let Some(held) = state.held.get_mut(store) {
            held.retain(|other| *other != call);
        }
    }

    pub(crate) fn after_queued(&self, store: &str) {
        if self.state().panicking_waiters.contains(store) {
            panic!("test seam: forced panic in a queued call on {store}");
        }
    }

    // Runs on the async handler task, so it polls instead of parking the
    // thread on the condition variable.
    pub(crate) async fn after_permit(&self, store: &str) {
        if !self.state().admitted_blocked.contains(store) {
            return;
        }
        *self.state().admitted_held.entry(store.to_string()).or_default() += 1;
        while self.state().admitted_blocked.contains(store) {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        *self.state().admitted_held.entry(store.to_string()).or_default() -= 1;
    }

    pub(crate) fn tool_panics(&self, tool: &str) -> bool {
        self.state().panicking_tools.contains(tool)
    }

    pub(crate) fn progress_burst(&self, tool: &str) -> u32 {
        self.state().progress_bursts.get(tool).copied().unwrap_or(0)
    }
}
