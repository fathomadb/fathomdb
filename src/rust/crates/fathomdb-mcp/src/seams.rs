//! Adapter-only test seams (feature `test-seams`): hold a store's open or
//! its engine calls on the blocking thread, or panic inside a tool handler.
//! They live in the adapter's bridge and need no engine hook (MR-003.1,
//! MR-033.1, MR-002.2).

use std::collections::BTreeSet;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

#[derive(Default)]
struct State {
    open_blocked: BTreeSet<String>,
    calls_blocked: BTreeSet<String>,
    panicking_tools: BTreeSet<String>,
    panicking_calls: BTreeSet<String>,
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

    /// Release every held open and call.
    pub fn release_all(&self) {
        let mut state = self.state();
        state.open_blocked.clear();
        state.calls_blocked.clear();
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

    pub(crate) fn before_open(&self, store: &str) {
        self.wait_while(|state| state.open_blocked.contains(store));
    }

    pub(crate) fn before_call(&self, store: &str) {
        self.wait_while(|state| state.calls_blocked.contains(store));
        if self.state().panicking_calls.contains(store) {
            panic!("test seam: forced panic in an engine call on {store}");
        }
    }

    pub(crate) fn tool_panics(&self, tool: &str) -> bool {
        self.state().panicking_tools.contains(tool)
    }
}
