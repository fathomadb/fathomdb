use super::*;
use rusqlite::OptionalExtension;
use std::collections::HashSet;

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

/// Snapshot-local ownership supplied explicitly to the fusion helpers.
///
/// This classification is separate from public IdSpace and retrieval branch.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum FusionOwnership {
    /// A canonical node at the enclosed hit's write cursor.
    CanonicalNode,
    /// A canonical edge at the enclosed hit's write cursor.
    CanonicalEdge,
    /// An explicitly ownerless node-body FTS row at its actual SQLite rowid.
    OwnerlessNodeFts { rowid: i64 },
}

/// A retrieval hit together with source-derived snapshot-local ownership.
///
/// Callers classify ownership at hydration; branch, body and IdSpace cannot
/// establish it. The carrier is never serialized into a SearchResult.
#[doc(hidden)]
#[derive(Clone, Debug, PartialEq)]
pub struct FusionHit {
    /// The representative public hit fields, including the positional cursor.
    pub hit: SearchHit,
    /// The actual canonical owner or explicitly ownerless FTS locator.
    pub ownership: FusionOwnership,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum RecordKey {
    CanonicalNode(u64),
    CanonicalEdge(u64),
    OwnerlessNodeFts(i64),
}

impl FusionHit {
    pub(crate) fn key(&self) -> RecordKey {
        self.ownership.record_key(self.write_cursor)
    }
}

impl FusionOwnership {
    pub(crate) fn record_key(self, cursor: u64) -> RecordKey {
        match self {
            Self::CanonicalNode => RecordKey::CanonicalNode(cursor),
            Self::CanonicalEdge => RecordKey::CanonicalEdge(cursor),
            Self::OwnerlessNodeFts { rowid } => RecordKey::OwnerlessNodeFts(rowid),
        }
    }
}

impl std::ops::Deref for FusionHit {
    type Target = SearchHit;

    fn deref(&self) -> &Self::Target {
        &self.hit
    }
}

impl std::ops::DerefMut for FusionHit {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.hit
    }
}

pub(crate) trait RankedHit: Clone {
    fn hit(&self) -> &SearchHit;
    fn hit_mut(&mut self) -> &mut SearchHit;
}

impl RankedHit for SearchHit {
    fn hit(&self) -> &SearchHit {
        self
    }
    fn hit_mut(&mut self) -> &mut SearchHit {
        self
    }
}

impl RankedHit for FusionHit {
    fn hit(&self) -> &SearchHit {
        &self.hit
    }
    fn hit_mut(&mut self) -> &mut SearchHit {
        &mut self.hit
    }
}

pub(crate) trait FusionRecord {
    fn record_key(&self) -> RecordKey;
    fn bodies_conflict(&self, other: &Self) -> bool;
    fn set_fused_score(&mut self, score: f64);
}

impl FusionRecord for FusionHit {
    fn record_key(&self) -> RecordKey {
        self.key()
    }
    fn bodies_conflict(&self, other: &Self) -> bool {
        self.body != other.body
    }
    fn set_fused_score(&mut self, score: f64) {
        self.score = score;
    }
}

/// Fuse explicitly owned vector and text hits with Reciprocal Rank Fusion.
///
/// Delegates to fuse_three_arms with an empty graph arm. Ownership and original
/// rank positions are retained. Conflicting bodies for one record key return
/// EngineError::InvalidArgument; no database access or identity inference occurs.
#[doc(hidden)]
pub fn fuse_rrf(
    vector_hits: Vec<FusionHit>,
    text_hits: Vec<FusionHit>,
) -> Result<Vec<FusionHit>, EngineError> {
    fuse_three_arms(vector_hits, text_hits, vec![])
}

/// Fuse explicitly owned vector, text and graph hits at their original ranks.
///
/// Each record receives one weight/(RRF_K+rank) term per arm at its first
/// positional rank. Repeats do not renumber later hits. Accumulation order is
/// vector, text, graph, which also selects the first representative. Sort by
/// score descending, vector-input membership, then stable insertion order.
/// Existing nested graph fusion remains the caller's choice of input lists.
/// Conflicting bodies for one key return EngineError::InvalidArgument; kind
/// and source remain representative-owned arm fields and may legitimately differ.
#[doc(hidden)]
pub fn fuse_three_arms(
    vector_hits: Vec<FusionHit>,
    text_hits: Vec<FusionHit>,
    graph_hits: Vec<FusionHit>,
) -> Result<Vec<FusionHit>, EngineError> {
    fuse_records(vector_hits, text_hits, graph_hits)
}

pub(crate) fn fuse_records<H: FusionRecord>(
    vector_hits: Vec<H>,
    text_hits: Vec<H>,
    graph_hits: Vec<H>,
) -> Result<Vec<H>, EngineError> {
    #[cfg(feature = "tc5-benchmark")]
    tc5_benchmark::record_fusion_route();
    struct Entry<H> {
        hit: H,
        score: f64,
        in_vector: bool,
        order: usize,
    }
    let mut entries: Vec<Entry<H>> = Vec::new();
    let mut by_record: HashMap<RecordKey, usize> = HashMap::new();
    for (arm, weight, in_vector) in [
        (vector_hits, RRF_WEIGHT_VECTOR, true),
        (text_hits, RRF_WEIGHT_TEXT, false),
        (graph_hits, RRF_WEIGHT_GRAPH, false),
    ] {
        let mut credited = HashSet::new();
        for (rank0, hit) in arm.into_iter().enumerate() {
            let key = hit.record_key();
            let contribution = weight / (RRF_K + (rank0 as f64 + 1.0));
            if let Some(&index) = by_record.get(&key) {
                if entries[index].hit.bodies_conflict(&hit) {
                    return Err(EngineError::InvalidArgument {
                        msg: "conflicting bodies for one retrieval record".into(),
                    });
                }
                if credited.insert(key) {
                    entries[index].score += contribution;
                }
            } else {
                let order = entries.len();
                by_record.insert(key, order);
                credited.insert(key);
                entries.push(Entry { hit, score: contribution, in_vector, order });
            }
        }
    }
    entries.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| b.in_vector.cmp(&a.in_vector))
            .then_with(|| a.order.cmp(&b.order))
    });
    Ok(entries
        .into_iter()
        .map(|mut entry| {
            entry.hit.set_fused_score(entry.score);
            entry.hit
        })
        .collect())
}

/// G12-recency — reweight fused hits toward the more recent (higher
/// `write_cursor`/`id`) AFTER bit-KNN (never a vec0 predicate). Gated by the
/// caller's dedicated recency flag; `enabled=false` is a no-op (pure RRF).
#[doc(hidden)]
#[must_use]
pub fn apply_recency_reweight(hits: Vec<SearchHit>, enabled: bool) -> Vec<SearchHit> {
    apply_recency_reweight_ranked(hits, enabled)
}

pub(crate) fn apply_recency_reweight_ranked<H: RankedHit>(hits: Vec<H>, enabled: bool) -> Vec<H> {
    if !enabled || hits.len() < 2 {
        return hits;
    }
    let min_id = hits.iter().map(|h| h.hit().write_cursor).min().unwrap_or(0);
    let max_id = hits.iter().map(|h| h.hit().write_cursor).max().unwrap_or(0);
    if max_id == min_id {
        return hits;
    }
    let span = (max_id - min_id) as f64;
    let mut reweighted: Vec<H> = hits
        .into_iter()
        .map(|mut h| {
            let norm = (h.hit().write_cursor - min_id) as f64 / span;
            h.hit_mut().score += RECENCY_WEIGHT * norm;
            h
        })
        .collect();
    // Stable sort preserves the fused order on exact ties.
    reweighted.sort_by(|a, b| {
        b.hit().score.partial_cmp(&a.hit().score).unwrap_or(std::cmp::Ordering::Equal)
    });
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
    apply_importance_by_key(
        hits,
        importance_by_id,
        confidence_by_id,
        |hit| hit.write_cursor,
        enabled,
    )
}

pub(crate) fn apply_record_importance_reweight(
    hits: Vec<FusionHit>,
    importance: &HashMap<RecordKey, f64>,
    confidence: &HashMap<RecordKey, f64>,
    enabled: bool,
) -> Vec<FusionHit> {
    apply_importance_by_key(hits, importance, confidence, FusionHit::key, enabled)
}

fn apply_importance_by_key<H: RankedHit, K: Eq + std::hash::Hash>(
    hits: Vec<H>,
    importance: &HashMap<K, f64>,
    confidence: &HashMap<K, f64>,
    key: impl Fn(&H) -> K,
    enabled: bool,
) -> Vec<H> {
    if !enabled
        || !hits
            .iter()
            .any(|hit| importance.contains_key(&key(hit)) || confidence.contains_key(&key(hit)))
    {
        return hits;
    }
    let mut hits = hits;
    for hit in &mut hits {
        let key = key(hit);
        hit.hit_mut().score *= importance.get(&key).copied().unwrap_or(1.0)
            * confidence.get(&key).copied().unwrap_or(1.0);
    }
    hits.sort_by(|a, b| {
        b.hit().score.partial_cmp(&a.hit().score).unwrap_or(std::cmp::Ordering::Equal)
    });
    hits
}

/// 0.8.16 Slice 5 / F9 — build the per-hit importance/confidence weight maps for
/// the candidate `hits` from the durable columns (`canonical_nodes.importance`,
/// `canonical_edges.confidence`). Only NON-NULL values are inserted; an absent
/// value stays out of the map (graceful-absent ⇒ neutral in
/// [`apply_importance_reweight`]). Current-schema statement and row errors
/// propagate so a failed lookup cannot masquerade as an absent value.
pub(crate) fn build_importance_confidence_maps(
    tx: &rusqlite::Connection,
    hits: &[FusionHit],
) -> rusqlite::Result<(HashMap<RecordKey, f64>, HashMap<RecordKey, f64>)> {
    let mut importance = HashMap::new();
    let mut confidence = HashMap::new();
    let mut node =
        tx.prepare("SELECT importance FROM canonical_nodes WHERE write_cursor=?1 LIMIT 1")?;
    let mut edge = tx.prepare("SELECT confidence FROM canonical_edges WHERE write_cursor=?1 AND superseded_at IS NULL LIMIT 1")?;
    for hit in hits {
        let (statement, map) = match hit.ownership {
            FusionOwnership::CanonicalNode => (&mut node, &mut importance),
            FusionOwnership::CanonicalEdge => (&mut edge, &mut confidence),
            FusionOwnership::OwnerlessNodeFts { .. } => continue,
        };
        let value: Option<Option<f64>> =
            statement.query_row([hit.write_cursor], |row| row.get(0)).optional()?;
        if let Some(Some(value)) = value {
            map.insert(hit.key(), value);
        }
    }
    Ok((importance, confidence))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn owned(ownership: FusionOwnership, body: &str) -> FusionHit {
        FusionHit {
            ownership,
            hit: SearchHit {
                id: IdSpace::content("shared"),
                write_cursor: 7,
                kind: "doc".into(),
                body: body.into(),
                score: 0.0,
                branch: SoftFallbackBranch::Text,
                source_id: None,
                ce_score: None,
                properties: None,
            },
        }
    }

    #[test]
    fn explicit_record_namespaces_do_not_merge_equal_cursors_or_bodies() {
        let fused = fuse_rrf(
            vec![
                owned(FusionOwnership::CanonicalNode, "same"),
                owned(FusionOwnership::CanonicalEdge, "same"),
                owned(FusionOwnership::OwnerlessNodeFts { rowid: 7 }, "same"),
            ],
            vec![],
        )
        .unwrap();
        assert_eq!(
            fused.iter().map(FusionHit::key).collect::<Vec<_>>(),
            [
                RecordKey::CanonicalNode(7),
                RecordKey::CanonicalEdge(7),
                RecordKey::OwnerlessNodeFts(7),
            ]
        );
        for (rank, hit) in fused.iter().enumerate() {
            assert_eq!(hit.score.to_bits(), (1.0_f64 / (31.0 + rank as f64)).to_bits());
        }
    }

    #[test]
    fn same_key_conflicting_bodies_refuse_but_representative_kind_and_source_can_differ() {
        let first = owned(FusionOwnership::CanonicalEdge, "same");
        let mut second = first.clone();
        second.kind = "link".into();
        second.source_id = Some("representative-other-arm".into());
        let fused = fuse_rrf(vec![first.clone()], vec![second]).unwrap();
        assert_eq!(fused.len(), 1);
        assert_eq!(fused[0].kind, first.kind);
        assert_eq!(fused[0].source_id, first.source_id);
        let conflict =
            fuse_rrf(vec![first], vec![owned(FusionOwnership::CanonicalEdge, "different")]);
        assert!(matches!(conflict, Err(EngineError::InvalidArgument { .. })));
    }

    #[test]
    fn overflow_chunking_fusion_tripwire() {
        assert_eq!(RRF_K, 30.0, "Re-evaluate dev/design/overflow-chunking-reevaluation.md: RRF_K");
        assert_eq!(
            RRF_WEIGHT_TEXT, 3.0,
            "Re-evaluate dev/design/overflow-chunking-reevaluation.md: RRF_WEIGHT_TEXT"
        );
        assert_eq!(
            RRF_WEIGHT_VECTOR, 1.0,
            "Re-evaluate dev/design/overflow-chunking-reevaluation.md: RRF_WEIGHT_VECTOR"
        );
    }
}
