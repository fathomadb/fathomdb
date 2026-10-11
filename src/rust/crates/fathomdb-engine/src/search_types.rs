use crate::errors::EngineError;
use crate::identity::IdSpace;

/// Soft-fallback signal carried on hybrid `search` results.
///
/// Per `dev/design/retrieval.md` § Soft-fallback signal, this record is
/// present only when one non-essential branch could not contribute. Total
/// request failure is not expressed via this carrier.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SoftFallback {
    pub branch: SoftFallbackBranch,
}

/// Which retrieval branch produced a hit (or could not contribute).
///
/// `Vector` = ANN vector branch (node bodies); `Text` = node-body FTS branch;
/// `TextEdge` = edge-body hit (FTS via `search_index_edges` OR vector-projected
/// edge facts — both produce the same kind="edge_fact" row shape and share the
/// same downstream handling in `search_expand_in_tx`). `Vector`/`Text` also
/// used as soft-fallback signal when the respective branch is empty.
/// `GraphArm` = R3 (Slice 30) BFS-reachable node from the temporal fact-edge
/// graph arm. Owned by `dev/design/retrieval.md`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoftFallbackBranch {
    Vector,
    Text,
    /// G11 (Slice 15) — edge-body hit from `search_index_edges` FTS or from
    /// `vector_default` edge-fact projection. `kind = "edge_fact"` in both cases.
    TextEdge,
    /// R3 (Slice 30) — BFS-reachable node from the temporal fact-edge graph arm.
    /// Only present when `use_graph_arm = true`. Nodes in the graph arm were NOT
    /// in the initial vector/text fused result (newly-reached nodes only).
    GraphArm,
}

/// A single structured search hit (G1 / AC-057a-clean).
///
/// Both retrieval branches emit this shape. `id` is a typed [`IdSpace`] — the
/// **permanent** caller-facing identity since C-2 (0.8.19 / TC-8), NOT the
/// interim `write_cursor: u64` the pre-0.8.19 releases carried and NOT an
/// interim carrier awaiting a later swap. The positional `write_cursor` field
/// below survives as engine-internal book-keeping and the SDK bindings do not
/// surface it. See the field docs on [`SearchHit::id`] / [`IdSpace`].
/// `score` is the **G9 RRF-fused** relevance (`Σ 1/(RRF_K + rank)` over the
/// branches that surfaced this body; higher = more relevant), optionally
/// recency-reweighted when the dedicated recency flag is on. Raw `vec_distance_l2`
/// and `bm25()` are fused on **rank**, never compared raw (they are not
/// comparable). `branch` tags which retrieval branch produced the representative
/// hit (vector-first when a body is surfaced by both).
///
/// `source_id` (G0 Phase-2 / BLOCK-2; generalised by TC-31 in 0.8.20 Slice 10a)
/// carries the source-document provenance of a hit — the identifier
/// [`Engine::erase_source`] consumes. It is populated on **every** hit path:
/// - **Node hits** (text/BM25F, vector, and the pre-step-12 legacy text
///   fallback) carry the **node's own** `canonical_nodes.source_id`.
/// - **Edge hits** (edge-FTS from `search_index_edges`, and edge-fact hits
///   hydrated by the vector arm) carry the **edge's own**
///   `canonical_edges.source_id`.
/// - **GraphArm** hits carry the **traversed edge's** `source_id` (the session
///   the fact-edge was extracted from) — unchanged by TC-31 — enabling
///   `doc_id_of` to resolve a graph-reached entity back to a gold session id.
///
/// Before TC-31 only the GraphArm branch populated this, which left
/// `erase_source` shipping with its argument unreachable from a text or vector
/// hit (0.8.19 also stopped surfacing `write_cursor` to the SDKs, removing the
/// only fallback route). It stays `Option<String>`: a row written before 0.8.20,
/// or a GOVERNED row deliberately spared by the step-21 backfill under the TC-11
/// pin, legitimately carries NULL at rest and must read back as `None` rather
/// than a fabricated value.
///
/// The field never participates in ranking, so result order and scores are
/// unaffected.
/// Derives `Clone, Debug, PartialEq` but **not `Eq`** — `score: f64` forbids
/// total equality.
#[derive(Clone, Debug, PartialEq)]
pub struct SearchHit {
    /// C-2 (0.8.19 / TC-8) — the typed, non-null, id-space-total hit id
    /// ([`IdSpace`]). Was the interim `write_cursor: u64` in prior releases; now
    /// carries the cross-session-stable key: `value` is the BARE (prefix-stripped)
    /// id, and [`to_prefixed`](IdSpace::to_prefixed) (== `{prefix}{value}`)
    /// reproduces the pre-swap `stable_id` byte-for-byte (the real-gold-keying
    /// no-op). Governed hits are `l:`, doc-seeded hits `h:`, synthetic
    /// passages `p:`. This is the caller-facing identity; the positional
    /// `write_cursor` below is engine-internal book-keeping.
    pub id: IdSpace,
    /// Engine-internal positional cursor (the value `id` carried before the C-2
    /// swap). Reassigned on every re-projection/re-ingest — NOT cross-session
    /// stable, NOT the caller-facing id. Retained because the engine still needs
    /// a positional cursor for its own book-keeping (vector rowid mapping, the
    /// `state='active'` filter lookups, RRF recency/importance reweight keys,
    /// telemetry `result_ids` keying, `search_expand` re-resolution). The SDK
    /// bindings do NOT surface it.
    pub write_cursor: u64,
    pub kind: String,
    pub body: String,
    pub score: f64,
    pub branch: SoftFallbackBranch,
    pub source_id: Option<String>,
    /// 0.8.5 (EXP-0) — per-candidate cross-encoder score `ce_norm =
    /// sigmoid(ce_logit) ∈ [0,1]`. `Some` ONLY for hits inside the reranked pool
    /// (the top `pool_n` when the CE model is loaded); `None` for the unreranked
    /// remainder, the `rerank_depth == 0` identity path, an empty list, and the
    /// no-CE-model soft-fallback. Additive + nullable: it never participates in
    /// ranking, so default-path ordering/scores stay byte-stable.
    pub ce_score: Option<f64>,
}

/// G0 Phase-2 (E0a / BLOCK-1) — graph-arm frontier instrumentation. A
/// **side-channel** meter (deliberately NOT a `SearchResult`/`SearchHit` field —
/// byte stability) that proves whether the graph arm seeds a non-empty frontier.
/// Under the current doc-seeded path the frontier is empty (doc nodes carry
/// `logical_id = NULL`), so `seeds_resolved == 0` and `resolved_seed_rate == 0.0`
/// — this meter is the measurement that proves it (and, post-C1, the 0→>0 flip).
///
/// `resolved_seed_rate = seeds_resolved / seeds_considered`, with `0/0 → 0.0`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GraphFrontierStats {
    /// Hits inspected as seed candidates (the `take(SEED_N)` window, skipping TextEdge).
    pub seeds_considered: u32,
    /// Seed candidates that resolved to an active `logical_id` (pushed onto the frontier).
    pub seeds_resolved: u32,
    /// Whether the BFS frontier was non-empty after seeding.
    pub frontier_nonempty: bool,
    /// Number of graph-arm `SearchHit`s emitted (reachable, not already in the two-arm result).
    pub graph_candidates_emitted: u32,
    /// Whether a further eligible graph candidate was suppressed by the fixed frontier cap.
    pub bound_reached: bool,
}

/// Hybrid `search` result. `results` carries structured [`SearchHit`]s in
/// vector-first, dedup-on-body order. Derives `Clone, Debug, PartialEq` but
/// **not `Eq`** — each hit carries a `score: f64`.
#[derive(Clone, Debug, PartialEq)]
// 0.8.8 EXP-OBS (field-set ratification): non_exhaustive so future additive fields
// (e.g. the deferred QueryTrace.timings_ms, Q3) are non-breaking. All construction
// is in-crate (engine + tests); external crates read fields only.
#[non_exhaustive]
pub struct SearchResult {
    pub projection_cursor: u64,
    pub soft_fallback: Option<SoftFallback>,
    pub results: Vec<SearchHit>,
    /// 0.8.8 EXP-OBS (Slice 5) — opt-in retrieval explanation **sidecar**.
    /// `Some` only when an ordinary or frozen retrieval requests explanation;
    /// `None` for every default (`explain=false`) search, so `results` and
    /// `projection_cursor` stay
    /// byte-identical to the pre-0.8.8 shape (R-OBS-2 zero-cost contract,
    /// HITL-ratified sidecar carrier — see
    /// `dev/design/0.8.8-explain-and-telemetry-adr.md` §A.2). Field-set is
    /// PROPOSED/ratification-pending; additive inside `Explanation` so later
    /// amendments do not reshape `SearchResult`/`SearchHit`.
    pub explanation: Option<Explanation>,
}

/// 0.8.8 EXP-OBS (Slice 5) — the opt-in retrieval explanation payload returned
/// behind `search_explained` (the `explain=true` surface). Built from the
/// engine's OWN fusion/rerank machinery (`fuse_three_arms` per-arm ranks,
/// `ce_rerank` blend components) — no parallel machinery (R-OBS-3). Carries a
/// query-level [`QueryTrace`] plus a per-hit breakdown parallel to (and in the
/// same order as) `SearchResult.results`.
///
/// Derives `Clone, Debug, PartialEq` but **not `Eq`** — scores are `f64`.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive] // 0.8.8 field-set ratification — additive-safe sidecar
pub struct Explanation {
    pub trace: QueryTrace,
    pub per_hit: Vec<PerHitExplain>,
    /// Engine-minted content-free identity shared with telemetry when enabled.
    pub correlation_id: String,
}

/// Whether a returned hit used only its representative healthy origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralInclusionStateV1 {
    Included,
    Degraded,
}

/// Representative physical origin of a returned hit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralProjectionOriginV1 {
    SynchronousBodyFts,
    CurrentDenseGeneration,
    GraphTraversal,
}

/// Dependency-registry state observable without exposing dependency identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralDependencyStateV1 {
    NotApplicable,
    NotRegistered,
    Registered,
}

/// Class-correct lifecycle state of a returned hit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralLifecycleStateV1 {
    NodePending,
    NodeActive,
    NodeDeleted,
    EdgeValid,
}

/// Closed reason why an included hit used a degraded retrieval route.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StructuralDegradationCodeV1 {
    SoftFallbackText,
    SoftFallbackTextEdge,
    ProjectionLegacyUnverified,
    ProjectionBlocked,
    ProjectionDeferred,
    GraphBoundReached,
}

/// Content-free structural classification for one returned explained hit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StructuralInclusionV1 {
    pub schema_version: u32,
    pub inclusion_state: StructuralInclusionStateV1,
    pub projection_origin: StructuralProjectionOriginV1,
    pub dependency_state: StructuralDependencyStateV1,
    pub lifecycle_state: StructuralLifecycleStateV1,
    pub degradation_codes: Vec<StructuralDegradationCodeV1>,
}

/// 0.8.8 EXP-OBS (Slice 5) — query-level retrieval trace. Reuses the existing
/// `search_reranked` knobs + the active embedder identity; timings are coarse
/// per-stage wall-clock (monotonic) captured only on the explain path.
#[derive(Clone, Debug, PartialEq)]
// 0.8.8 field-set ratification — HARD: leaf absorbs the deferred `timings_ms` (Q3)
// and any future trace field without a contract break.
#[non_exhaustive]
pub struct QueryTrace {
    /// Query LENGTH only (chars) — never the query text (privacy; ADR §C).
    pub query_chars: u32,
    /// Caller-requested final result limit (`final_limit`).
    pub k: u32,
    pub rerank_depth: u32,
    pub pool_n: u32,
    pub alpha: f64,
    pub use_graph_arm: bool,
    /// Recency reweight (the dedicated G12 flag) was applied.
    pub recency: bool,
    /// Active embedder identity `name@revision` (+ dim), or empty when none.
    pub embedder_id: String,
    /// The CE cross-encoder actually reranked the pool (model loaded + depth>0).
    pub ce_active: bool,
    /// Per-arm input hit counts (pre-fusion).
    pub vector_hits: u32,
    pub text_hits: u32,
    pub graph_hits: u32,
    /// Edge-FTS candidates rejected only because an attribute predicate is
    /// node-scoped. Present on the opt-in explanation so this deliberate
    /// filtering never looks like an absent corpus.
    pub dropped_edge_hits: u32,
}

/// 0.8.8 EXP-OBS (Slice 5) — per-hit provenance + score breakdown. One entry per
/// returned `SearchHit`, same order. `*_rank` is the 0-based rank the hit's body
/// held in that arm's pre-fusion list (`None` = absent from that arm).
///
/// Derives `Clone, Debug, PartialEq` but **not `Eq`** — scores are `f64`.
#[derive(Clone, Debug, PartialEq)]
// 0.8.8 field-set ratification — HARD: leaf absorbs future arms / score components.
#[non_exhaustive]
pub struct PerHitExplain {
    /// The hit's engine-internal positional `write_cursor` (the pre-C-2
    /// `SearchHit.id`). Post-0.8.19 the caller-facing `SearchHit.id` is a typed
    /// [`IdSpace`]; this field keeps carrying the positional cursor so the explain
    /// sidecar cross-references the telemetry `result_ids` space. Correlate a
    /// `PerHitExplain` to its `SearchHit` by position (both lists are 1:1, same
    /// order).
    pub id: u64,
    /// Winning arm after RRF dedup (vector-first), == `SearchHit.branch`.
    pub arm: SoftFallbackBranch,
    pub vector_rank: Option<u32>,
    pub text_rank: Option<u32>,
    pub graph_rank: Option<u32>,
    /// Raw RRF fused score AFTER recency reweight, BEFORE CE blend (the value
    /// `ce_rerank` normalizes). Faithful to the engine computation — downstream
    /// may normalize. (ADR §A.4 Q1: raw exposed; normalization deferred.)
    pub fused_score: f64,
    /// In-pool cross-encoder score `sigmoid(ce_logit) ∈ [0,1]`, == the returned
    /// `SearchHit.ce_score`; `None` outside the reranked pool / no-CE path.
    pub ce_score: Option<f64>,
    /// Final blended score, == the returned `SearchHit.score`.
    pub blended: f64,
    /// 0.8.16 Slice 5 / F9 — the node `importance` scalar applied to this hit's
    /// fused contribution when the importance reweight is ON, else the raw stored
    /// value. `None` = never assigned (graceful-absent, ranks NEUTRAL). Additive
    /// (`#[non_exhaustive]` leaf absorbs the new score component).
    pub importance: Option<f64>,
    /// 0.8.16 Slice 5 / F9 — the edge `confidence` scalar applied to this hit's
    /// graph-arm contribution when the importance reweight is ON, else the raw
    /// stored value. `None` for node hits / edges without a confidence
    /// (graceful-absent, ranks NEUTRAL).
    pub confidence: Option<f64>,
    /// Content-free inclusion classification from the same search snapshot.
    pub structural: StructuralInclusionV1,
}

/// F5 (0.8.14 Slice 10) — per-field BM25F weights for the `search_index_v2`
/// multi-column FTS index. One weight per indexed field
/// (`kind`/`body`/`status`), applied as the field's contribution multiplier in
/// the BM25F weighted-term-frequency accumulation.
///
/// The default is uniform (`1.0` each) — the "unweighted" baseline the R-F5-1
/// acceptance test contrasts against. Boosting a field (e.g. `kind`) makes a
/// match in that field outrank a same-strength match in a lower-weighted field.
/// Engine-internal for 0.8.14: there is NO public Py/TS SDK surface for these
/// tunables this release (cross-binding parity is a Slice-40/X1 concern).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bm25fFieldWeights {
    pub kind: f64,
    pub body: f64,
    pub status: f64,
}

/// F5 (0.8.14 Slice 10) — the compiled BM25F query plan for the fielded lexical
/// arm (`ADR-0.8.1` §3.2 `BM25fQueryPlan`). Carries the tunable per-field
/// `weights` and the tunable length-normalization `b` (and the term-saturation
/// `k1`).
///
/// NOTE on `b`: SQLite FTS5's built-in `bm25()` auxiliary function pins its
/// internal `k1`/`b` and exposes ONLY per-column weights — it cannot express a
/// tunable `b`. So the score is computed in-engine (a textbook BM25F over the
/// FTS5-recalled candidates) rather than delegated to the built-in `bm25()`:
/// that is what makes `b` (and `k1`) genuinely tunable here, not a dead
/// parameter. The `search_index_v2` FTS5 index is still load-bearing — it does
/// the candidate recall (`MATCH`) that the scorer then ranks.
///
/// Defaults match Robertson/SQLite BM25 (`b = 0.75`, `k1 = 1.2`) with uniform
/// field weights. Engine-internal for 0.8.14 (no SDK surface).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bm25fQueryPlan {
    pub weights: Bm25fFieldWeights,
    pub b: f64,
    pub k1: f64,
}

// 0.7.0 Pack 2 (ADR-0.7.0-vector-binary-quant § 2; handoff § 2.2):
// bit-KNN candidate-set size for the two-phase read path. Tuned with
// the recall@10 floor in tests/perf_gates.rs::ac_013b_recall_at_10_floor.
//
// Bumped from 64 → 192 in EU-5a2 per the HITL 2026-05-29 fine-grained
// K-sweep result (dev/notes/0.7.1-default-embedder-research.md §5.4):
// K=192 sits above the recall-plateau knee for the default embedder.
// Public-visible so the EU-5a2 machinery test can assert the value.
pub const TOP_K_BIT_CANDIDATES: usize = 192;

/// Historical name for the public default ranked-result count (10).
///
/// Vector candidate fanout is separately at least the caller-requested result
/// limit and `TOP_K_BIT_CANDIDATES`; the test seam may raise that fanout without
/// changing visible result cardinality. There is no environment-variable
/// override on the hot path.
pub const SEARCH_RERANK_LIMIT: usize = 10;

/// Default number of ranked hits returned by public retrieval APIs.
pub const DEFAULT_SEARCH_RESULT_LIMIT: usize = SEARCH_RERANK_LIMIT;

/// Largest ranked-hit count a public retrieval request may select.
pub const MAX_SEARCH_RESULT_LIMIT: usize = 100;

pub(crate) fn validate_search_result_limit(limit: usize) -> Result<usize, EngineError> {
    if !(1..=MAX_SEARCH_RESULT_LIMIT).contains(&limit) {
        return Err(EngineError::InvalidArgument {
            msg: format!(
                "search result limit must be an integer in 1..={MAX_SEARCH_RESULT_LIMIT}; got {limit}"
            ),
        });
    }
    Ok(limit)
}

impl GraphFrontierStats {
    /// `seeds_resolved / seeds_considered`, defined as `0.0` when nothing was considered.
    pub fn resolved_seed_rate(&self) -> f64 {
        if self.seeds_considered == 0 {
            0.0
        } else {
            f64::from(self.seeds_resolved) / f64::from(self.seeds_considered)
        }
    }
}

impl Default for Bm25fFieldWeights {
    fn default() -> Self {
        Self { kind: 1.0, body: 1.0, status: 1.0 }
    }
}

impl Default for Bm25fQueryPlan {
    fn default() -> Self {
        Self { weights: Bm25fFieldWeights::default(), b: 0.75, k1: 1.2 }
    }
}
