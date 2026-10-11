use super::*;
use rusqlite::OptionalExtension;

/// G9 — Reciprocal Rank Fusion constant. IR-C (2026-06-10b,
/// `performance-output-and-compare.md`) found the standard `k≈60` slightly too
/// high: the recall gain is concentrated at the top of the list, where a lower
/// `k` sharpens rank-1/2 contributions. `k=30` is the validated operating point
/// (`k10 > k30 > k60 > k100` on the sweep, `30` the conservative middle).
/// Fusion is on **rank**, never raw score.
pub const RRF_K: f64 = 30.0;

/// G9 / IR-C — per-branch RRF weights. The sweep's optimum is strongly
/// **text-dominant** (`text:vector ≈ 3:1`): the lexical (BM25) arm carries
/// exact-fact recall and the dense arm, over-weighted, is a net drag on
/// exploratory recall (`performance-output-and-compare.md`, 2026-06-10b/e). A
/// branch contributes `weight / (RRF_K + rank)`.
pub const RRF_WEIGHT_VECTOR: f64 = 1.0;

pub const RRF_WEIGHT_TEXT: f64 = 3.0;

/// R3 (Slice 30) — graph arm RRF weight. Conservative starting value (equal to
/// `RRF_WEIGHT_VECTOR`). Without R2 per-class delta data the graph arm weight
/// cannot be calibrated; 1.0 is the minimum non-zero contribution. The graph
/// arm surfaces newly-reachable nodes from BFS traversal; it is not meant to
/// override the primary text/vector signals. Revisable after R2 data arrives.
/// See `dev/design/slice-30-design.md` §Q2.
pub const RRF_WEIGHT_GRAPH: f64 = 1.0;

/// G12-recency — additive recency weight. Must satisfy two constraints:
/// 1. Small enough to never override a clear RRF signal: a gap of > RECENCY_WEIGHT
///    between two hits' RRF scores means the stronger RRF hit always wins.
/// 2. Large enough to break exact ties: any hit with a higher `write_cursor` (more
///    recent) gets RECENCY_WEIGHT × 1.0 > 0 nudge and wins a tied comparison.
///
/// Value 0.002 satisfies the near-tie-nudge contract with respect to the
/// committed test (`recency_does_not_override_a_clear_rrf_signal`):
/// the test's RRF gap is 0.01, which is larger than 0.002, so recency
/// never overrides it. Note: this value is larger than the minimum
/// vector-only rank-step at deep ranks (~0.00101 for adjacent ranks near
/// the bottom), so recency can flip a single-rank vector difference at
/// deep ranks — by design, recency is a near-tie nudge, and "near-tie"
/// is scoped to the test gap (0.01), not to every possible rank step.
///
/// 0.8.1 Slice 10 fix: the previous value `0.5/RRF_K ≈ 0.01667` violated
/// the test gap constraint (it exceeded 0.01). Lowered to 0.002.
pub const RECENCY_WEIGHT: f64 = 0.002;

/// G9 — fuse the vector and text branches with Reciprocal Rank Fusion.
///
/// Delegates to [`fuse_three_arms`] with an empty graph arm. The two-arm
/// contract is preserved: `fuse_rrf(v, t)` == `fuse_three_arms(v, t, vec![])`.
/// All existing callers are unaffected.
///
/// See [`fuse_three_arms`] for the full RRF formula documentation.
#[doc(hidden)]
#[must_use]
pub fn fuse_rrf(vector_hits: Vec<SearchHit>, text_hits: Vec<SearchHit>) -> Vec<SearchHit> {
    fuse_three_arms(vector_hits, text_hits, vec![])
}

/// R3 (Slice 30) — fuse vector, text, and graph arms with Reciprocal Rank Fusion.
///
/// Each branch contributes `weight / (RRF_K + rank)` (1-based rank within that
/// branch; `weight` = [`RRF_WEIGHT_VECTOR`] / [`RRF_WEIGHT_TEXT`] /
/// [`RRF_WEIGHT_GRAPH`], text-dominant per IR-C), accumulated **keyed on
/// `SearchHit.body`**, so a body surfaced by multiple branches accumulates all
/// terms (agreement boosts it). The fused value is written into `SearchHit.score`.
/// A body in multiple branches surfaces **once** with the **vector** branch's
/// identity (vector-first), then graph arm identity for non-vector hits, then
/// text. Output is sorted by score descending, then vector-first, then insertion
/// order — a pure, deterministic function of the three input lists.
///
/// With an empty `graph_hits` (`vec![]`), the output is byte-identical to the
/// pre-Slice-30 two-arm `fuse_rrf`. This is the backward-compatibility contract.
///
/// This is the **unconditional** new ranking (HITL Q3 — no `fusion_mode` knob,
/// no legacy path). Graph arm is opt-in via `use_graph_arm=true`.
#[doc(hidden)]
#[must_use]
pub fn fuse_three_arms(
    vector_hits: Vec<SearchHit>,
    text_hits: Vec<SearchHit>,
    graph_hits: Vec<SearchHit>,
) -> Vec<SearchHit> {
    #[cfg(feature = "tc5-benchmark")]
    tc5_benchmark::record_fusion_route();
    struct Entry {
        hit: SearchHit,
        score: f64,
        in_vector: bool,
        order: usize,
    }
    enum BodySlot {
        One(usize),
        Collisions(Vec<usize>),
    }
    let mut entries: Vec<Entry> = Vec::new();
    let mut entry_by_hash: HashMap<u64, BodySlot> = HashMap::new();
    let mut accumulate = |hit: SearchHit, rank0: usize, in_vector: bool, weight: f64| {
        let contrib = weight / (RRF_K + (rank0 as f64 + 1.0));
        let body_hash = entry_by_hash.hasher().hash_one(&hit.body);
        let existing = entry_by_hash.get(&body_hash).and_then(|slot| match slot {
            BodySlot::One(index) => (entries[*index].hit.body == hit.body).then_some(*index),
            BodySlot::Collisions(indices) => {
                indices.iter().copied().find(|index| entries[*index].hit.body == hit.body)
            }
        });
        if let Some(existing) = existing {
            entries[existing].score += contrib;
            return;
        }
        let order = entries.len();
        match entry_by_hash.entry(body_hash) {
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(BodySlot::One(order));
            }
            std::collections::hash_map::Entry::Occupied(mut slot) => {
                let body_slot = slot.get_mut();
                match body_slot {
                    BodySlot::One(previous) => {
                        *body_slot = BodySlot::Collisions(vec![*previous, order]);
                    }
                    BodySlot::Collisions(indices) => indices.push(order),
                }
            }
        }
        entries.push(Entry { hit, score: contrib, in_vector, order });
    };
    for (rank0, hit) in vector_hits.into_iter().enumerate() {
        accumulate(hit, rank0, true, RRF_WEIGHT_VECTOR);
    }
    for (rank0, hit) in text_hits.into_iter().enumerate() {
        accumulate(hit, rank0, false, RRF_WEIGHT_TEXT);
    }
    for (rank0, hit) in graph_hits.into_iter().enumerate() {
        // Graph arm: vector-first=false (never overrides an existing vector hit's
        // representative identity; only new bodies from the graph arm get GraphArm
        // as their branch identity). The in_vector=false ensures graph arm hits
        // never sort ahead of vector hits on exact score ties.
        accumulate(hit, rank0, false, RRF_WEIGHT_GRAPH);
    }
    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            // vector-first on equal score (true sorts before false).
            .then_with(|| b.in_vector.cmp(&a.in_vector))
            .then_with(|| a.order.cmp(&b.order))
    });
    entries
        .into_iter()
        .map(|mut e| {
            e.hit.score = e.score;
            e.hit
        })
        .collect()
}

/// G12-recency — reweight fused hits toward the more recent (higher
/// `write_cursor`/`id`) AFTER bit-KNN (never a vec0 predicate). Gated by the
/// caller's dedicated recency flag; `enabled=false` is a no-op (pure RRF).
#[doc(hidden)]
#[must_use]
pub fn apply_recency_reweight(hits: Vec<SearchHit>, enabled: bool) -> Vec<SearchHit> {
    if !enabled || hits.len() < 2 {
        return hits;
    }
    let min_id = hits.iter().map(|h| h.write_cursor).min().unwrap_or(0);
    let max_id = hits.iter().map(|h| h.write_cursor).max().unwrap_or(0);
    if max_id == min_id {
        return hits;
    }
    let span = (max_id - min_id) as f64;
    let mut reweighted: Vec<SearchHit> = hits
        .into_iter()
        .map(|mut h| {
            let norm = (h.write_cursor - min_id) as f64 / span;
            h.score += RECENCY_WEIGHT * norm;
            h
        })
        .collect();
    // Stable sort preserves the fused order on exact ties.
    reweighted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    reweighted
}

/// 0.8.16 Slice 5 / F9 — OFF-by-default importance/confidence reweight, applied
/// to the fused hits AFTER bit-KNN + RRF (mirrors [`apply_recency_reweight`]).
///
/// Multiplicative-on-fused (ADR-0.8.16 §2.2, HITL-SIGNED 2026-07-08): a hit's
/// score is scaled by node `importance` (`importance_by_id`) × edge `confidence`
/// (`confidence_by_id`), each keyed by the hit's interim id (`write_cursor`). A
/// missing key = `NULL` = never assigned = graceful-absent ⇒ neutral (1.0), the
/// OPP-12 Q6a graceful-absent state. Node hits carry `importance`; graph/edge hits
/// carry `confidence`; the two id-spaces never collide (cursors are globally
/// unique), so each hit gets exactly one non-neutral factor.
///
/// **R-F9-4 graceful-neutral identity:** when `enabled` but *no* hit has a
/// non-neutral factor (every importance/confidence absent), the input is returned
/// **unchanged** — byte-identical to the `enabled == false` result (no re-sort),
/// so declaring the mechanism never perturbs an all-absent corpus.
#[must_use]
pub fn apply_importance_reweight(
    hits: Vec<SearchHit>,
    importance_by_id: &HashMap<u64, f64>,
    confidence_by_id: &HashMap<u64, f64>,
    enabled: bool,
) -> Vec<SearchHit> {
    if !enabled {
        return hits;
    }
    // Graceful-neutral fast path (R-F9-4): if nothing is weighted, do not touch
    // order or scores — identical to the reweight-OFF result.
    let any_weighted = hits.iter().any(|h| {
        importance_by_id.contains_key(&h.write_cursor)
            || confidence_by_id.contains_key(&h.write_cursor)
    });
    if !any_weighted {
        return hits;
    }
    let mut reweighted: Vec<SearchHit> = hits
        .into_iter()
        .map(|mut h| {
            let importance = importance_by_id.get(&h.write_cursor).copied().unwrap_or(1.0);
            let confidence = confidence_by_id.get(&h.write_cursor).copied().unwrap_or(1.0);
            h.score *= importance * confidence;
            h
        })
        .collect();
    // Stable sort preserves the fused order on exact ties.
    reweighted.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    reweighted
}

/// 0.8.16 Slice 5 / F9 — build the per-hit importance/confidence weight maps for
/// the candidate `hits` from the durable columns (`canonical_nodes.importance`,
/// `canonical_edges.confidence`). Only NON-NULL values are inserted; an absent
/// value stays out of the map (graceful-absent ⇒ neutral in
/// [`apply_importance_reweight`]). Current-schema statement and row errors
/// propagate so a failed lookup cannot masquerade as an absent value.
pub(crate) fn build_importance_confidence_maps(
    tx: &rusqlite::Connection,
    hits: &[SearchHit],
) -> rusqlite::Result<(HashMap<u64, f64>, HashMap<u64, f64>)> {
    let mut importance_by_id: HashMap<u64, f64> = HashMap::new();
    let mut confidence_by_id: HashMap<u64, f64> = HashMap::new();
    let mut importance_stmt =
        tx.prepare("SELECT importance FROM canonical_nodes WHERE write_cursor = ?1 LIMIT 1")?;
    for h in hits {
        let value: Option<Option<f64>> =
            importance_stmt.query_row([h.write_cursor], |row| row.get(0)).optional()?;
        if let Some(Some(value)) = value {
            importance_by_id.insert(h.write_cursor, value);
        }
    }
    let mut confidence_stmt = tx.prepare(
        "SELECT confidence FROM canonical_edges \
         WHERE write_cursor = ?1 AND superseded_at IS NULL LIMIT 1",
    )?;
    for h in hits {
        let value: Option<Option<f64>> =
            confidence_stmt.query_row([h.write_cursor], |row| row.get(0)).optional()?;
        if let Some(Some(value)) = value {
            confidence_by_id.insert(h.write_cursor, value);
        }
    }
    Ok((importance_by_id, confidence_by_id))
}
