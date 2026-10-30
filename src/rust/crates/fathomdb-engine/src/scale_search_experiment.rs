//! Experiment-only alternatives for the hybrid search costs that grow with
//! the store (`dev/experiments/ac013-scale-search/`). Nothing here is active
//! unless `FATHOMDB_PERF_EXPERIMENTS` is set; each knob is read once per
//! process.
//!
//! - `FATHOMDB_PERF_ELIGIBILITY=cache`: the two direct-eligibility `EXISTS`
//!   scans (`dependency_closure::all_nodes_directly_eligible` and the
//!   direct-state half of `vector_arm_requires_fallback`) are answered from a
//!   process-wide cache keyed by the database path, the snapshot's
//!   `_fathomdb_read_visibility_state` (generation, nonce), which triggers
//!   bump on every insert, update and delete of `canonical_nodes`,
//!   `canonical_edges` and `_fathomdb_vector_rows` (the only tables the scans
//!   read), and the view flags. An answer is cached only when it cannot
//!   depend on the instant: the view includes out-of-window rows, or the
//!   snapshot has no node validity window and no edge `t_invalid`.
//! - `FATHOMDB_PERF_FTS_ROWID_CURSOR=1`: body-FTS rows (`search_index`) are
//!   inserted with `rowid = write_cursor` (the projector path only; a
//!   prototype for new stores).

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use rusqlite::{Connection, OptionalExtension};

fn knob(name: &str) -> Option<String> {
    std::env::var_os("FATHOMDB_PERF_EXPERIMENTS")?;
    std::env::var(name).ok().filter(|v| !v.is_empty())
}

pub(crate) fn eligibility_cache_enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| knob("FATHOMDB_PERF_ELIGIBILITY").as_deref() == Some("cache"))
}

pub(crate) fn fts_rowid_is_cursor() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| knob("FATHOMDB_PERF_FTS_ROWID_CURSOR").is_some())
}

/// Which direct-eligibility scan an answer belongs to.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) enum EligibilityScan {
    AllNodesDirectlyEligible,
    VectorDirectStateUnsafe,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
struct Key {
    path: String,
    generation: i64,
    nonce: String,
    scan: EligibilityScan,
    flags: (bool, bool, bool),
}

static CACHE: Mutex<Option<HashMap<Key, bool>>> = Mutex::new(None);
type Snapshot = (String, i64, String);
static TIME_BOUNDS: Mutex<Option<HashMap<Snapshot, bool>>> = Mutex::new(None);
static HITS: AtomicU64 = AtomicU64::new(0);
static MISSES: AtomicU64 = AtomicU64::new(0);
const CACHE_CAPACITY: usize = 256;

/// `(hits, misses)` of the eligibility cache since process start.
#[doc(hidden)]
#[cfg_attr(not(feature = "test-hooks"), allow(dead_code))]
pub fn eligibility_cache_stats_for_test() -> (u64, u64) {
    (HITS.load(Ordering::SeqCst), MISSES.load(Ordering::SeqCst))
}

fn snapshot_generation(connection: &Connection) -> Option<(i64, String)> {
    connection
        .prepare_cached(
            "SELECT generation, state_nonce FROM _fathomdb_read_visibility_state \
             WHERE singleton=1",
        )
        .ok()?
        .query_row([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
        .optional()
        .ok()
        .flatten()
}

fn snapshot_has_time_bounds(connection: &Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM canonical_nodes \
                       WHERE valid_from IS NOT NULL OR valid_until IS NOT NULL) \
             OR EXISTS(SELECT 1 FROM canonical_edges WHERE t_invalid IS NOT NULL)",
        [],
        |row| row.get(0),
    )
}

/// `compute()` under the eligibility cache. `connection` must be the reader
/// transaction's connection, so the generation and the scan share one
/// snapshot.
pub(crate) fn cached_eligibility(
    connection: &Connection,
    scan: EligibilityScan,
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    compute: impl FnOnce() -> rusqlite::Result<bool>,
) -> rusqlite::Result<bool> {
    if !eligibility_cache_enabled() {
        return compute();
    }
    let (Some(path), Some((generation, nonce))) =
        (connection.path().map(str::to_string), snapshot_generation(connection))
    else {
        return compute();
    };
    let key = Key {
        path,
        generation,
        nonce,
        scan,
        flags: (include_superseded, include_inactive, include_out_of_window),
    };
    if let Some(answer) =
        CACHE.lock().ok().and_then(|cache| cache.as_ref().and_then(|c| c.get(&key).copied()))
    {
        HITS.fetch_add(1, Ordering::SeqCst);
        return Ok(answer);
    }
    MISSES.fetch_add(1, Ordering::SeqCst);
    let answer = compute()?;
    let snapshot = (key.path.clone(), key.generation, key.nonce.clone());
    let known = TIME_BOUNDS
        .lock()
        .ok()
        .and_then(|bounds| bounds.as_ref().and_then(|b| b.get(&snapshot).copied()));
    let time_bounds = match known {
        Some(bounds) => bounds,
        None if include_out_of_window => false,
        None => {
            let bounds = snapshot_has_time_bounds(connection)?;
            if let Ok(mut map) = TIME_BOUNDS.lock() {
                let map = map.get_or_insert_with(HashMap::new);
                if map.len() >= CACHE_CAPACITY {
                    map.clear();
                }
                map.insert(snapshot, bounds);
            }
            bounds
        }
    };
    if include_out_of_window || !time_bounds {
        if let Ok(mut cache) = CACHE.lock() {
            let map = cache.get_or_insert_with(HashMap::new);
            if map.len() >= CACHE_CAPACITY {
                map.clear();
            }
            map.insert(key, answer);
        }
    }
    Ok(answer)
}
