//! Per-connection session: the default store and, per `(store, scope_key)`,
//! the last 256 continuations, response ids, frozen read contexts, and last
//! observed visibility state (interface §5, `AC32-MCP-SAFETY-04`). Handles are
//! short server-side keys, not encoded state, so a forged or cross-scope handle
//! simply misses (MR-041.5).

use std::collections::{BTreeMap, VecDeque};
use std::sync::{Mutex, MutexGuard};

use fathomdb_sdk::{FrozenReadContextV1, VisibilityStateV1};

use crate::config::SearchMode;
use crate::output::DriftLabel;
use crate::shape::CanonicalSearch;

/// Handles kept per `(store, scope_key)` and kind; the least recently used is dropped.
pub(crate) const HANDLES_PER_STORE: usize = 256;

/// The state a continuation re-runs.
#[derive(Clone, Debug)]
pub(crate) struct Continuation {
    pub(crate) epoch: u64,
    pub(crate) args: CanonicalSearch,
    pub(crate) offset: usize,
    pub(crate) validity_instant: i64,
}

/// One frozen read context minted or reused in this session.
#[derive(Clone, Debug)]
pub(crate) struct FrozenEntry {
    pub(crate) epoch: u64,
    pub(crate) context: FrozenReadContextV1,
    pub(crate) minted_state: VisibilityStateV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResponseCursor {
    Unique(u64),
    Ambiguous,
}

impl ResponseCursor {
    fn include(&mut self, cursor: u64) {
        if matches!(self, Self::Unique(existing) if *existing != cursor) {
            *self = Self::Ambiguous;
        }
    }

    pub(crate) fn cursor(self) -> Option<u64> {
        match self {
            Self::Unique(cursor) => Some(cursor),
            Self::Ambiguous => None,
        }
    }
}

pub(crate) fn fold_response_refs(
    entries: impl IntoIterator<Item = (String, u64)>,
) -> BTreeMap<String, ResponseCursor> {
    let mut refs = BTreeMap::new();
    for (reference, cursor) in entries {
        refs.entry(reference)
            .and_modify(|value: &mut ResponseCursor| value.include(cursor))
            .or_insert(ResponseCursor::Unique(cursor));
    }
    refs
}

/// What feedback needs to credit a response (MR-024).
#[derive(Clone, Debug)]
pub(crate) struct ResponseContext {
    pub(crate) epoch: u64,
    pub(crate) mode: SearchMode,
    /// The telemetry query id of a captured hybrid search.
    pub(crate) query_id: Option<String>,
    /// One cursor or absorbing ambiguity per returned reference, never last-wins.
    pub(crate) refs: BTreeMap<String, ResponseCursor>,
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
    frozen_contexts: Lru<FrozenEntry>,
    last_state: Option<VisibilityStateV1>,
}

impl Tables {
    fn new() -> Self {
        Self {
            continuations: Lru::new(),
            responses: Lru::new(),
            frozen_contexts: Lru::new(),
            last_state: None,
        }
    }
}

/// Where a response id was found.
pub(crate) enum ResponseLookup {
    Found(ResponseContext),
    OtherStore,
    Missing,
}

type TableKey = (String, String);

pub(crate) struct Session {
    default_store: Mutex<Option<String>>,
    tables: Mutex<BTreeMap<TableKey, Tables>>,
}

impl Session {
    pub(crate) fn new(default_store: Option<String>) -> Self {
        Self { default_store: Mutex::new(default_store), tables: Mutex::new(BTreeMap::new()) }
    }

    fn tables(&self) -> MutexGuard<'_, BTreeMap<TableKey, Tables>> {
        self.tables.lock().unwrap_or_else(|poison| poison.into_inner())
    }

    pub(crate) fn default_store(&self) -> Option<String> {
        self.default_store.lock().unwrap_or_else(|poison| poison.into_inner()).clone()
    }

    pub(crate) fn set_default_store(&self, store: &str) {
        *self.default_store.lock().unwrap_or_else(|poison| poison.into_inner()) =
            Some(store.to_string());
    }

    pub(crate) fn put_continuation(
        &self,
        store: &str,
        scope_key: &str,
        handle: String,
        value: Continuation,
    ) {
        self.tables()
            .entry((store.to_string(), scope_key.to_string()))
            .or_insert_with(Tables::new)
            .continuations
            .insert(handle, value);
    }

    pub(crate) fn continuation(
        &self,
        store: &str,
        scope_key: &str,
        handle: &str,
    ) -> Option<Continuation> {
        self.tables()
            .get_mut(&(store.to_string(), scope_key.to_string()))?
            .continuations
            .get(handle)
    }

    pub(crate) fn put_frozen_context(
        &self,
        store: &str,
        scope_key: &str,
        handle: String,
        entry: FrozenEntry,
    ) {
        self.tables()
            .entry((store.to_string(), scope_key.to_string()))
            .or_insert_with(Tables::new)
            .frozen_contexts
            .insert(handle, entry);
    }

    pub(crate) fn frozen_context(
        &self,
        store: &str,
        scope_key: &str,
        handle: &str,
    ) -> Option<FrozenEntry> {
        self.tables()
            .get_mut(&(store.to_string(), scope_key.to_string()))?
            .frozen_contexts
            .get(handle)
    }

    pub(crate) fn put_response(
        &self,
        store: &str,
        scope_key: &str,
        handle: String,
        value: ResponseContext,
    ) {
        self.tables()
            .entry((store.to_string(), scope_key.to_string()))
            .or_insert_with(Tables::new)
            .responses
            .insert(handle, value);
    }

    pub(crate) fn response(&self, store: &str, scope_key: &str, handle: &str) -> ResponseLookup {
        let mut tables = self.tables();
        if let Some(found) = tables
            .get_mut(&(store.to_string(), scope_key.to_string()))
            .and_then(|tables| tables.responses.get(handle))
        {
            return ResponseLookup::Found(found);
        }
        // Only check other stores within the SAME scope_key so cross-scope
        // response handles never disclose existence (`AC32-MCP-SAFETY-04`).
        if tables
            .iter()
            .any(|((_, sk), tables)| sk == scope_key && tables.responses.contains(handle))
        {
            ResponseLookup::OtherStore
        } else {
            ResponseLookup::Missing
        }
    }

    /// Record the latest visibility snapshot for `(store, scope_key)` and
    /// classify drift relative to the previous observation in this session.
    pub(crate) fn observe_state(
        &self,
        store: &str,
        scope_key: &str,
        current: &VisibilityStateV1,
    ) -> DriftLabel {
        let mut tables = self.tables();
        let entry =
            tables.entry((store.to_string(), scope_key.to_string())).or_insert_with(Tables::new);
        let drift = match &entry.last_state {
            None => DriftLabel::Unchanged,
            Some(prev) => classify_drift(prev, current),
        };
        entry.last_state = Some(current.clone());
        drift
    }
}

pub(crate) fn classify_drift(prev: &VisibilityStateV1, current: &VisibilityStateV1) -> DriftLabel {
    if prev.database_id == current.database_id
        && prev.generation == current.generation
        && prev.nonce == current.nonce
        && prev.canonical_write_boundary == current.canonical_write_boundary
        && prev.projection_cursor == current.projection_cursor
    {
        return DriftLabel::Unchanged;
    }
    if prev.database_id != current.database_id
        || prev.canonical_write_boundary != current.canonical_write_boundary
    {
        return DriftLabel::Canonical;
    }
    if prev.projection_cursor != current.projection_cursor {
        return DriftLabel::Index;
    }
    DriftLabel::Canonical
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_same_cursor_stays_unique_and_distinct_cursors_stay_ambiguous() {
        let unique = fold_response_refs([("same".into(), 7), ("same".into(), 7)]);
        assert_eq!(unique["same"], ResponseCursor::Unique(7));
        let mixed = fold_response_refs([
            ("same".into(), 7),
            ("other".into(), 9),
            ("same".into(), 8),
            ("same".into(), 7),
        ]);
        assert_eq!(mixed.len(), 2);
        assert_eq!(mixed["same"], ResponseCursor::Ambiguous);
        assert_eq!(mixed["same"].cursor(), None);
        assert_eq!(mixed["other"].cursor(), Some(9));
    }
}
