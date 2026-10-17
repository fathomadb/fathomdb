//! Per-connection session: the default store and, per store, the last 256
//! continuations and response ids (interface §5). Handles are short
//! server-side keys, not encoded state, so a forged or edited handle simply
//! misses (MR-041.5).

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard};

use crate::config::SearchMode;
use crate::shape::CanonicalSearch;

/// Handles kept per store and kind; the least recently used is dropped.
pub(crate) const HANDLES_PER_STORE: usize = 256;

/// The state a continuation re-runs.
#[derive(Clone, Debug)]
pub(crate) struct Continuation {
    pub(crate) epoch: u64,
    pub(crate) args: CanonicalSearch,
    pub(crate) offset: usize,
    pub(crate) validity_instant: i64,
}

/// What feedback needs to credit a response (MR-024).
#[derive(Clone, Debug)]
pub(crate) struct ResponseContext {
    pub(crate) epoch: u64,
    pub(crate) mode: SearchMode,
    /// The telemetry query id of a captured hybrid search.
    pub(crate) query_id: Option<String>,
    /// Reference to engine write cursor, for `record_feedback`.
    pub(crate) refs: BTreeMap<String, u64>,
}

struct Lru<V> {
    entries: VecDeque<(String, V)>,
}

impl<V: Clone> Lru<V> {
    fn new() -> Self {
        Self { entries: VecDeque::new() }
    }

    fn insert(&mut self, key: String, value: V) {
        self.entries.retain(|(existing, _)| *existing != key);
        self.entries.push_back((key, value));
        while self.entries.len() > HANDLES_PER_STORE {
            self.entries.pop_front();
        }
    }

    fn get(&mut self, key: &str) -> Option<V> {
        let index = self.entries.iter().position(|(existing, _)| existing == key)?;
        let entry = self.entries.remove(index)?;
        let value = entry.1.clone();
        self.entries.push_back(entry);
        Some(value)
    }

    fn contains(&self, key: &str) -> bool {
        self.entries.iter().any(|(existing, _)| existing == key)
    }
}

struct Tables {
    continuations: Lru<Continuation>,
    responses: Lru<ResponseContext>,
}

impl Tables {
    fn new() -> Self {
        Self { continuations: Lru::new(), responses: Lru::new() }
    }
}

/// Where a response id was found.
pub(crate) enum ResponseLookup {
    Found(ResponseContext),
    OtherStore,
    Missing,
}

pub(crate) struct Session {
    default_store: Mutex<Option<String>>,
    tables: Mutex<BTreeMap<String, Tables>>,
}

impl Session {
    pub(crate) fn new(default_store: Option<String>) -> Self {
        Self { default_store: Mutex::new(default_store), tables: Mutex::new(BTreeMap::new()) }
    }

    fn tables(&self) -> MutexGuard<'_, BTreeMap<String, Tables>> {
        self.tables.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    pub(crate) fn default_store(&self) -> Option<String> {
        self.default_store.lock().unwrap_or_else(|poison| poison.into_inner()).clone()
    }

    pub(crate) fn set_default_store(&self, store: &str) {
        *self.default_store.lock().unwrap_or_else(|poison| poison.into_inner()) =
            Some(store.to_string());
    }

    pub(crate) fn put_continuation(&self, store: &str, handle: String, value: Continuation) {
        self.tables()
            .entry(store.to_string())
            .or_insert_with(Tables::new)
            .continuations
            .insert(handle, value);
    }

    pub(crate) fn continuation(&self, store: &str, handle: &str) -> Option<Continuation> {
        self.tables().get_mut(store)?.continuations.get(handle)
    }

    pub(crate) fn put_response(&self, store: &str, handle: String, value: ResponseContext) {
        self.tables()
            .entry(store.to_string())
            .or_insert_with(Tables::new)
            .responses
            .insert(handle, value);
    }

    pub(crate) fn response(&self, store: &str, handle: &str) -> ResponseLookup {
        let mut tables = self.tables();
        if let Some(found) = tables.get_mut(store).and_then(|tables| tables.responses.get(handle)) {
            return ResponseLookup::Found(found);
        }
        if tables.values().any(|tables| tables.responses.contains(handle)) {
            ResponseLookup::OtherStore
        } else {
            ResponseLookup::Missing
        }
    }
}
