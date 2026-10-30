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
//! - `FATHOMDB_PERF_TEXT_ARM=deferred`: on the Slice 71 deferred-identity
//!   hybrid route (no filter, dependencies, relaxed view, reweighting,
//!   rerank, graph arm, explain, snippets or expansion, every node directly
//!   eligible), the text arm reads `rowid, bm25()` for every match and FTS
//!   content only for the rows that can reach the fused top `final_limit`;
//!   see [`fuse_deferred_content`]. Needs `FATHOMDB_PERF_FTS_ROWID_CURSOR`
//!   stores; any row whose rowid is not its cursor falls back to the complete
//!   path.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension};

use crate::fusion::{fuse_rrf, RRF_K, RRF_WEIGHT_TEXT, RRF_WEIGHT_VECTOR};
use crate::identity::IdSpace;
use crate::search_types::{SearchHit, SoftFallbackBranch};

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

pub(crate) fn text_arm_deferred() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| knob("FATHOMDB_PERF_TEXT_ARM").as_deref() == Some("deferred"))
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

fn record_route(_route: &str) {
    #[cfg(feature = "test-hooks")]
    crate::search::record_fts_route_for_test(_route);
}

fn text_hit(body: String, kind: String, cursor: u64, score: f64) -> SearchHit {
    SearchHit {
        id: IdSpace::content(String::new()),
        body,
        kind,
        write_cursor: cursor,
        score,
        branch: SoftFallbackBranch::Text,
        source_id: None,
        ce_score: None,
    }
}

/// The complete deferred-identity text list (`search.rs`): every match,
/// sorted by bm25 then cursor.
fn complete_text_candidates(
    tx: &Connection,
    match_expression: &str,
) -> rusqlite::Result<Vec<SearchHit>> {
    let mut statement = tx.prepare_cached(
        "SELECT body, kind, write_cursor, bm25(search_index) FROM search_index \
         WHERE search_index MATCH ?1",
    )?;
    let mut hits = statement
        .query_map([match_expression], |row| {
            Ok(text_hit(row.get(0)?, row.get(1)?, row.get::<_, i64>(2)? as u64, row.get(3)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    hits.sort_by(|l, r| {
        l.score.total_cmp(&r.score).then_with(|| l.write_cursor.cmp(&r.write_cursor))
    });
    Ok(hits)
}

/// The fused list `fuse_rrf(vector_hits, complete text list ++ edge_hits)`
/// would produce, exact in its first `final_limit` entries (the caller
/// truncates there), reading FTS content only where it can matter.
///
/// Why it is exact, with `rowid = write_cursor`:
/// - The text rank of a body is the position of its first row in
///   (bm25, cursor) order. Rows with the same body tokenize identically, so
///   they share one bm25 value; the first row of a vector hit's body is found
///   by reading bodies only inside its own bm25 tie group, up to its row.
/// - A text-only body at position i scores `3/(31+i)`, strictly falling in
///   i, so the top `final_limit` can hold only the first `final_limit` new
///   text-only bodies in rank order; the walk stops there.
/// - Scores are computed with the same expressions and addition order as
///   `fuse_three_arms`, and entries sort on the same keys.
///
/// Falls back to the complete path (returns it, records
/// `hybrid_deferred_content_fallback`) when a vector hit has no FTS row with
/// its own cursor and body, a fetched row's rowid is not its cursor, or the
/// walk meets a vector body before its computed rank.
pub(crate) fn fuse_deferred_content(
    tx: &Connection,
    match_expression: &str,
    vector_hits: Vec<SearchHit>,
    edge_hits: Vec<SearchHit>,
    final_limit: usize,
) -> rusqlite::Result<Vec<SearchHit>> {
    if let Some(fused) =
        deferred_fusion(tx, match_expression, &vector_hits, &edge_hits, final_limit)?
    {
        record_route("hybrid_deferred_content");
        return Ok(fused);
    }
    record_route("hybrid_deferred_content_fallback");
    let mut text = complete_text_candidates(tx, match_expression)?;
    text.extend(edge_hits);
    Ok(fuse_rrf(vector_hits, text))
}

fn deferred_fusion(
    tx: &Connection,
    match_expression: &str,
    vector_hits: &[SearchHit],
    edge_hits: &[SearchHit],
    final_limit: usize,
) -> rusqlite::Result<Option<Vec<SearchHit>>> {
    let mut scored: Vec<(i64, f64)> = tx
        .prepare_cached(
            "SELECT rowid, bm25(search_index) FROM search_index WHERE search_index MATCH ?1",
        )?
        .query_map([match_expression], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    scored.sort_by(|l, r| l.1.total_cmp(&r.1).then_with(|| l.0.cmp(&r.0)));
    let matched = scored.len();
    let mut content =
        tx.prepare_cached("SELECT body, kind, write_cursor FROM search_index WHERE rowid = ?1")?;
    let mut fetch = |rowid: i64| -> rusqlite::Result<Option<(String, String)>> {
        let row = content
            .query_row([rowid], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, i64>(2)?))
            })
            .optional()?;
        Ok(row.and_then(|(body, kind, cursor)| (cursor == rowid).then_some((body, kind))))
    };

    struct Entry {
        hit: SearchHit,
        score: f64,
        in_vector: bool,
        order: usize,
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut entry_of_body: HashMap<String, usize> = HashMap::new();
    for (rank0, hit) in vector_hits.iter().enumerate() {
        if entry_of_body.contains_key(&hit.body) {
            continue;
        }
        let score = RRF_WEIGHT_VECTOR / (RRF_K + (rank0 as f64 + 1.0));
        entry_of_body.insert(hit.body.clone(), entries.len());
        let order = entries.len();
        entries.push(Entry { hit: hit.clone(), score, in_vector: true, order });
    }

    let wanted: HashSet<i64> = entries.iter().map(|e| e.hit.write_cursor as i64).collect();
    let mut position: HashMap<i64, usize> = HashMap::with_capacity(wanted.len());
    for (i, (rowid, _)) in scored.iter().enumerate() {
        if wanted.contains(rowid) {
            position.insert(*rowid, i);
        }
    }
    let mut rank_of_body: HashMap<String, usize> = HashMap::new();
    for entry in &mut entries {
        let cursor = entry.hit.write_cursor as i64;
        match fetch(cursor)? {
            Some((body, _)) if body == entry.hit.body => {}
            _ => return Ok(None),
        }
        let mut rank = None;
        if let Some(&p) = position.get(&cursor) {
            let bits = scored[p].1.to_bits();
            let mut q = p;
            while q > 0 && scored[q - 1].1.to_bits() == bits {
                q -= 1;
            }
            for i in q..p {
                let Some((body, _)) = fetch(scored[i].0)? else {
                    return Ok(None);
                };
                if body == entry.hit.body {
                    rank = Some(i);
                    break;
                }
            }
            rank = rank.or(Some(p));
        } else if let Some(j) = edge_hits.iter().position(|e| e.body == entry.hit.body) {
            rank = Some(matched + j);
        }
        if let Some(rank) = rank {
            entry.score += RRF_WEIGHT_TEXT / (RRF_K + (rank as f64 + 1.0));
            rank_of_body.insert(entry.hit.body.clone(), rank);
        }
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut text_only = 0usize;
    let mut i = 0usize;
    while text_only < final_limit && i < matched + edge_hits.len() {
        let hit = if i < matched {
            let (rowid, score) = scored[i];
            let Some((body, kind)) = fetch(rowid)? else {
                return Ok(None);
            };
            text_hit(body, kind, rowid as u64, score)
        } else {
            edge_hits[i - matched].clone()
        };
        if entry_of_body.contains_key(&hit.body) {
            if !seen.contains(&hit.body) && rank_of_body.get(&hit.body) != Some(&i) {
                return Ok(None);
            }
            seen.insert(hit.body);
        } else if seen.insert(hit.body.clone()) {
            let score = RRF_WEIGHT_TEXT / (RRF_K + (i as f64 + 1.0));
            let order = entries.len();
            entries.push(Entry { hit, score, in_vector: false, order });
            text_only += 1;
        }
        i += 1;
    }

    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.in_vector.cmp(&a.in_vector))
            .then_with(|| a.order.cmp(&b.order))
    });
    Ok(Some(
        entries
            .into_iter()
            .map(|mut e| {
                e.hit.score = e.score;
                e.hit
            })
            .collect(),
    ))
}
