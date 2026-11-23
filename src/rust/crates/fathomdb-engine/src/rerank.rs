use super::*;
#[cfg(any(feature = "default-reranker", test, feature = "test-hooks"))]
use crate::fusion::RecordKey;
use crate::fusion::{FusionHit, RankedHit};
#[cfg(any(feature = "default-reranker", test, feature = "test-hooks"))]
use std::collections::HashMap;

/// Per-query cross-encoder score cache (`sigmoid(ce_logit)`) shared between
/// the `FusedTopK` / `Both` seed-ranking pass and the final rerank pass so
/// two-arm hits are never scored twice by the cross-encoder.
#[derive(Default)]
pub(crate) struct CeScoreCache {
    #[cfg(any(feature = "default-reranker", test, feature = "test-hooks"))]
    scores: HashMap<RecordKey, f64>,
    #[cfg(feature = "default-reranker")]
    model_absent: bool,
}

#[cfg(any(test, feature = "test-hooks"))]
#[doc(hidden)]
pub type TestCeScorerFn = std::sync::Arc<dyn Fn(&str, &[&str]) -> Vec<f32> + Send + Sync>;

#[cfg(any(test, feature = "test-hooks"))]
static TEST_CE_SCORERS: std::sync::OnceLock<std::sync::Mutex<HashMap<String, TestCeScorerFn>>> =
    std::sync::OnceLock::new();

#[cfg(any(test, feature = "test-hooks"))]
fn test_ce_scorers() -> &'static std::sync::Mutex<HashMap<String, TestCeScorerFn>> {
    TEST_CE_SCORERS.get_or_init(|| std::sync::Mutex::new(HashMap::new()))
}

/// RAII guard that uninstalls a query-scoped test cross-encoder scorer on drop.
#[cfg(any(test, feature = "test-hooks"))]
#[doc(hidden)]
pub struct TestCeScorerGuard {
    query: String,
}

#[cfg(any(test, feature = "test-hooks"))]
impl Drop for TestCeScorerGuard {
    fn drop(&mut self) {
        if let Ok(mut map) = test_ce_scorers().lock() {
            map.remove(&self.query);
        }
    }
}

/// Install a deterministic query-scoped cross-encoder scorer for tests.
/// Only queries matching `query` exactly invoke `scorer`; other concurrent
/// tests are completely unaffected.
#[cfg(any(test, feature = "test-hooks"))]
#[doc(hidden)]
pub fn install_test_ce_scorer_for_test(
    query: impl Into<String>,
    scorer: TestCeScorerFn,
) -> TestCeScorerGuard {
    let query = query.into();
    test_ce_scorers().lock().expect("test ce scorer lock").insert(query.clone(), scorer);
    TestCeScorerGuard { query }
}

#[cfg(any(test, feature = "test-hooks"))]
struct ClosureCeScorer(TestCeScorerFn);

#[cfg(any(test, feature = "test-hooks"))]
impl CeScorer for ClosureCeScorer {
    fn score_pair(
        &self,
        query: &str,
        passage: &str,
    ) -> Result<f32, Option<RerankerDevicePolicyError>> {
        Ok((self.0)(query, &[passage]).into_iter().next().unwrap_or(0.0))
    }

    fn score_pairs(
        &self,
        query: &str,
        passages: &[&str],
    ) -> Result<Vec<f32>, Option<RerankerDevicePolicyError>> {
        Ok((self.0)(query, passages))
    }

    fn forced_cuda_runtime_error(&self) -> Option<RerankerDevicePolicyError> {
        None
    }
}

/// 0.8.1 Slice 10 (R1) — CE rerank seam.
///
/// `rerank_depth = 0` (or model absent / `default-reranker` feature off): returns
/// `hits` **unchanged** — byte-identical to the old identity stub. This is the
/// soft-fallback contract.
///
/// `rerank_depth > 0` with the `default-reranker` feature on and the model
/// loaded: scores the top-`rerank_depth` (query, passage) pairs with the
/// TinyBERT-L-2 cross-encoder, blends CE score with the RRF score using the
/// formula from the design memo (Decision 5), re-sorts the top-N, and appends
/// the remainder in their original RRF order.
///
/// Score-blend (Decision 5): `α × sigmoid(ce_logit) + (1−α) × rrf_score_normalized`
/// where both CE and RRF scores are normalized to [0,1] over the reranked pool.
///
/// 0.8.5 (EXP-0): `alpha` (clamped to `[0,1]`) and `pool_n` (the reranked-pool
/// size, clamped to `hits.len()`) are caller-supplied. The defaults
/// `alpha = 0.3, pool_n = rerank_depth` reproduce the pre-slice blend exactly.
/// `rerank_depth == 0` remains the identity gate regardless of `pool_n`.
///
/// This is the rerank hook, **not** the dropped `fusion_mode` knob.
#[doc(hidden)]
#[must_use]
pub fn rerank_fused(
    _query: &str,
    hits: Vec<SearchHit>,
    rerank_depth: usize,
    alpha: f64,
    pool_n: usize,
) -> Vec<SearchHit> {
    try_rerank_fused(_query, hits.clone(), rerank_depth, alpha, pool_n).unwrap_or(hits)
}

/// Fallible form of [`rerank_fused`] used by normal engine requests.
///
/// The legacy helper keeps the historic soft-fallback signature for pure ranking
/// callers. Production search and standalone rerank use this path so a forced
/// CUDA runtime failure remains observable instead of becoming an RRF result.
#[doc(hidden)]
pub fn try_rerank_fused(
    _query: &str,
    hits: Vec<SearchHit>,
    rerank_depth: usize,
    alpha: f64,
    pool_n: usize,
) -> Result<Vec<SearchHit>, RerankerDevicePolicyError> {
    try_rerank_ranked(_query, hits, rerank_depth, alpha, pool_n)
}

pub(crate) fn try_rerank_ranked<H: RankedHit>(
    _query: &str,
    hits: Vec<H>,
    rerank_depth: usize,
    alpha: f64,
    pool_n: usize,
) -> Result<Vec<H>, RerankerDevicePolicyError> {
    // Soft-fallback: depth=0 → identity (byte-identical to old stub). NOTE this
    // early gate is independent of `pool_n`: `rerank_depth == 0, pool_n = 10`
    // does NOT rerank (0.8.5 D4).
    if rerank_depth == 0 {
        return Ok(hits);
    }

    // Feature-gated CE inference. In the default build (no feature) this block
    // compiles away and `hits` is returned unchanged regardless of `rerank_depth`.
    // FIX-1: pass `&hits` (borrow) so `hits` remains owned for the soft-fallback path.
    #[cfg(feature = "default-reranker")]
    {
        if let Some(reranked) = ce_rerank(_query, &hits, rerank_depth, alpha, pool_n)? {
            return Ok(reranked);
        }
    }

    // 0.8.5: the bindings/default callers pass `alpha = 0.3, pool_n = rerank_depth`;
    // referenced here so the no-feature build does not warn on unused params.
    #[cfg(not(feature = "default-reranker"))]
    let _ = (alpha, pool_n);

    // Model absent (feature off, weights not loaded, or CE returned None) →
    // soft-fallback: return input unchanged.
    Ok(hits)
}

/// Rerank `hits` while caching per-`RecordKey` normalized CE scores (`sigmoid(logit)`)
/// in `cache`. Hits already present in `cache` reuse their cached CE score without
/// invoking the cross-encoder again.
pub(crate) fn try_rerank_ranked_with_cache(
    query: &str,
    hits: Vec<FusionHit>,
    rerank_depth: usize,
    alpha: f64,
    pool_n: usize,
    cache: &mut CeScoreCache,
) -> Result<Vec<FusionHit>, RerankerDevicePolicyError> {
    if rerank_depth == 0 || hits.is_empty() {
        return Ok(hits);
    }
    let n = pool_n.min(hits.len());
    if n == 0 {
        return Ok(hits);
    }

    #[cfg(any(test, feature = "test-hooks"))]
    if let Some(scorer_fn) = test_ce_scorers().lock().ok().and_then(|map| map.get(query).cloned()) {
        let scorer = ClosureCeScorer(scorer_fn);
        return ce_rerank_with_scorer_and_cache(&scorer, query, &hits, alpha, pool_n, cache);
    }

    #[cfg(feature = "default-reranker")]
    {
        if cache.model_absent {
            return Ok(hits);
        }
        if let Some(reranked) = ce_rerank_cached(query, &hits, alpha, pool_n, cache)? {
            return Ok(reranked);
        }
        cache.model_absent = true;
    }

    #[cfg(not(feature = "default-reranker"))]
    let _ = (query, alpha, pool_n, cache);

    Ok(hits)
}

/// Why [`rerank_passages`] refused a request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RerankPassagesError {
    /// A caller-supplied passage was invalid (a non-finite score).
    WriteValidation { message: String },
    /// The cross-encoder refused: its device policy could not be honored, or
    /// inference failed with a CUDA pool kind. The typed kind is kept.
    Reranker(RerankerDevicePolicyError),
}

impl std::fmt::Display for RerankPassagesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WriteValidation { message } => f.write_str(message),
            Self::Reranker(error) => write!(f, "reranker: {error}"),
        }
    }
}

impl std::error::Error for RerankPassagesError {}

/// `WriteValidation` maps to [`EngineError::WriteValidation`]; a reranker
/// refusal maps through `From<RerankerDevicePolicyError>`, so the three CUDA
/// pool kinds keep their typed variants.
impl From<RerankPassagesError> for EngineError {
    fn from(error: RerankPassagesError) -> Self {
        match error {
            RerankPassagesError::WriteValidation { .. } => Self::WriteValidation,
            RerankPassagesError::Reranker(error) => Self::from(error),
        }
    }
}

/// 0.8.2 Slice E2 — standalone CE rerank of a caller-supplied passage list.
///
/// The pure, testable core that the `fathomdb.rerank` pyo3 binding is a thin
/// wrapper over. Slice 5's `fused_rerank` comparator must CE-rerank its OWN
/// in-harness fused(bm25+dense) pool — a pool the engine's `search()` never
/// constructs — so the CE has to be reachable over an arbitrary passage list,
/// not just the engine's capped text-only pool. This adapts `(id, body, score)`
/// passages into `SearchHit`s (`kind = "passage"`, `branch = Vector`,
/// `source_id = None`; only `body` and `score` feed the blend), runs them
/// through [`rerank_fused`], and projects back to `(id, score, ce_score)` in the reranked
/// order.
///
/// Contract (inherited verbatim from `rerank_fused`): `rerank_depth == 0` OR an
/// empty list returns the input order WITH the input scores, byte-identical — no
/// model load, no network. With `--features default-reranker` and
/// `rerank_depth > 0` the CE blends the top-`depth` and may reorder; with the
/// feature off the CE path compiles away and this is always identity.
///
/// # Errors
///
/// [`RerankPassagesError::WriteValidation`] when any passage carries a
/// non-finite score (NaN / ±inf). [`RerankPassagesError::Reranker`] when the
/// reranker device policy cannot be honored or the cross-encoder fails with a
/// CUDA pool kind (pool exhaustion, context loss, private-build refusal);
/// those keep their typed kinds and are never flattened to a string.
pub fn rerank_passages(
    query: &str,
    passages: Vec<(u64, String, f64)>,
    rerank_depth: usize,
    alpha: f64,
    pool_n: usize,
) -> Result<Vec<(u64, f64, Option<f64>)>, RerankPassagesError> {
    // [P2] guard: reject non-finite scores before they reach normalization/sort.
    // A NaN or ±inf score would produce NaN blended scores and an unstable sort
    // order — surface the error early as the typed WriteValidationError at the
    // pyo3 boundary (mirroring the malformed-passage loud-fail contract).
    for (id, _, score) in &passages {
        if !score.is_finite() {
            return Err(RerankPassagesError::WriteValidation {
                message: format!(
                    "rerank: non-finite score for passage id={id}: {score} \
                     (NaN/\u{00b1}inf must not reach the normalization/sort step)"
                ),
            });
        }
    }
    // Empty input is an unconditional identity path. Return after validating
    // the supplied passage records but before resolving device policy or
    // loading model state.
    if passages.is_empty() {
        return Ok(vec![]);
    }
    #[cfg(feature = "default-reranker")]
    let _module_lease = if rerank_depth > 0 {
        Some(
            fathomdb_embedder::module_models::admit_reranker()
                .map_err(RerankPassagesError::Reranker)?,
        )
    } else {
        None
    };
    // Slice 71: a forced CUDA policy is a request to run CE inference on that
    // device, never permission to silently return CPU CE scores. Resolve before
    // loading weights so malformed/forced policy errors are observable without
    // a model download. Depth zero remains the no-model/no-device identity path.
    #[cfg(feature = "default-reranker")]
    if rerank_depth > 0 {
        fathomdb_embedder::resolve_default_reranker_device_from_env()
            .map_err(RerankPassagesError::Reranker)?;
    }
    let hits: Vec<SearchHit> = passages
        .into_iter()
        .map(|(id, body, score)| SearchHit {
            // C-2: synthetic passages carry no canonical identity — mint the
            // `Passage` (`p:`) id from the caller-supplied ordinal. The ordinal
            // is ALSO kept as the engine-internal positional cursor so the
            // projection below returns it byte-unchanged.
            id: IdSpace::passage(id.to_string()),
            write_cursor: id,
            kind: "passage".to_string(),
            body,
            score,
            branch: SoftFallbackBranch::Vector,
            source_id: None,
            ce_score: None,
            properties: None,
        })
        .collect();
    // 0.8.5 — project `(id, score, ce_score)` so the binding can surface the CE
    // score per candidate; `ce_score` is `None` for the identity / out-of-pool path.
    // The projected id is the caller's ordinal (the engine-internal `write_cursor`).
    Ok(try_rerank_fused(query, hits, rerank_depth, alpha, pool_n)
        .map_err(RerankPassagesError::Reranker)?
        .into_iter()
        .map(|h| (h.write_cursor, h.score, h.ce_score))
        .collect())
}

/// 0.8.1 Slice 10 — score-blend reranking when CE model is loaded.
///
/// Returns `Some(reranked)` if the model is available, `None` otherwise
/// (caller then applies the soft-fallback).
///
/// Design memo Decision 5:
/// - CE normalized = sigmoid(raw_logit) ∈ [0,1]
/// - RRF normalized = min-max of `hit.score` over the top-K pool
/// - `final_score = 0.3 × ce_norm + 0.7 × rrf_norm`
/// - Hits beyond `rerank_depth` keep their original RRF scores and order.
#[cfg(feature = "default-reranker")]
fn ce_rerank<H: RankedHit>(
    _query: &str,
    hits: &[H], // FIX-1: borrow, not move — caller retains ownership for soft-fallback
    _rerank_depth: usize, // 0.8.5: pool sizing moved to `pool_n`; depth gate stays in `rerank_fused`.
    alpha: f64,
    pool_n: usize,
) -> Result<Option<Vec<H>>, RerankerDevicePolicyError> {
    #[cfg(feature = "tc5-benchmark")]
    tc5_benchmark::record_cross_encoder_route();
    // 0.8.5 (D3) — clamp α to [0,1] silently here so EVERY path (engine search,
    // `rerank_passages`, the bindings) is covered by one clamp, matching the
    // existing `pool_n.min(len)` clamp idiom.
    // codex §9 P2-1: `f64::clamp(NaN)` returns NaN (clamp does NOT map NaN into
    // range) — a non-finite α would then make every blended score NaN and destroy
    // the ranking. The high-level SDKs reject non-finite α, but the low-level
    // `rerank()` / direct-Rust callers don't, so fall back to the documented
    // default α=0.3 here for any non-finite input.
    let alpha = if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.3 };
    // fix-1 [P2]: short-circuit before touching the singleton when there is
    // nothing to rerank — avoids loading/downloading the ~17 MB model for an
    // empty result set and prevents memoizing a transient load failure.
    if hits.is_empty() {
        return Ok(Some(vec![]));
    }

    // 0.8.5 (D4) — the reranked pool is the top `pool_n` (caller resolves the
    // `unwrap_or(rerank_depth)` default at the binding), clamped to the hit count.
    let n = pool_n.min(hits.len());
    let top = &hits[..n]; // no split_at_mut needed; borrow slices directly

    // Batched CE scoring: ONE forward over the whole top-N pool instead of N
    // per-pair forwards. The ranking math below (RRF min-max norm, sigmoid,
    // ALPHA blend, sort) is byte-unchanged — only the scoring is batched.
    let bodies: Vec<&str> = top.iter().map(|h| h.hit().body.as_str()).collect();
    #[cfg(feature = "slice72-test-hooks")]
    let raw_logits = fathomdb_embedder::module_models::with_reranker(|model| {
        slice72_test_hooks::with_ce_forward(|| score_pool(model, _query, &bodies))
    })?;
    #[cfg(not(feature = "slice72-test-hooks"))]
    let raw_logits = fathomdb_embedder::module_models::with_reranker(|model| {
        score_pool(model, _query, &bodies)
    })?;

    Ok(raw_logits.map(|raw_logits| blend_ce_ranked(hits, raw_logits, alpha, pool_n)))
}

#[cfg(feature = "default-reranker")]
fn ce_rerank_cached(
    query: &str,
    hits: &[FusionHit],
    alpha: f64,
    pool_n: usize,
    cache: &mut CeScoreCache,
) -> Result<Option<Vec<FusionHit>>, RerankerDevicePolicyError> {
    #[cfg(feature = "tc5-benchmark")]
    tc5_benchmark::record_cross_encoder_route();
    let alpha = if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.3 };
    if hits.is_empty() {
        return Ok(Some(vec![]));
    }
    let n = pool_n.min(hits.len());
    let top = &hits[..n];

    let mut uncached_indices: Vec<usize> = Vec::new();
    let mut uncached_bodies: Vec<&str> = Vec::new();
    for (idx, h) in top.iter().enumerate() {
        if !cache.scores.contains_key(&h.key()) {
            uncached_indices.push(idx);
            uncached_bodies.push(h.hit().body.as_str());
        }
    }

    if !uncached_bodies.is_empty() {
        #[cfg(feature = "slice72-test-hooks")]
        let raw_logits = fathomdb_embedder::module_models::with_reranker(|model| {
            slice72_test_hooks::with_ce_forward(|| score_pool(model, query, &uncached_bodies))
        })?;
        #[cfg(not(feature = "slice72-test-hooks"))]
        let raw_logits = fathomdb_embedder::module_models::with_reranker(|model| {
            score_pool(model, query, &uncached_bodies)
        })?;
        let Some(raw_logits) = raw_logits else {
            return Ok(None);
        };
        for (idx, raw_logit) in uncached_indices.into_iter().zip(raw_logits) {
            let ce_norm = 1.0 / (1.0 + (-raw_logit).exp());
            cache.scores.insert(top[idx].key(), ce_norm);
        }
    }

    let ce_norms: Vec<f64> =
        top.iter().map(|h| cache.scores.get(&h.key()).copied().unwrap_or(0.5)).collect();
    Ok(Some(blend_ce_norms(hits, &ce_norms, alpha, pool_n)))
}

#[cfg(any(test, feature = "test-hooks"))]
fn ce_rerank_with_scorer_and_cache<S: CeScorer + ?Sized>(
    scorer: &S,
    query: &str,
    hits: &[FusionHit],
    alpha: f64,
    pool_n: usize,
    cache: &mut CeScoreCache,
) -> Result<Vec<FusionHit>, RerankerDevicePolicyError> {
    let alpha = if alpha.is_finite() { alpha.clamp(0.0, 1.0) } else { 0.3 };
    if hits.is_empty() {
        return Ok(vec![]);
    }
    let n = pool_n.min(hits.len());
    let top = &hits[..n];

    let mut uncached_indices: Vec<usize> = Vec::new();
    let mut uncached_bodies: Vec<&str> = Vec::new();
    for (idx, h) in top.iter().enumerate() {
        if !cache.scores.contains_key(&h.key()) {
            uncached_indices.push(idx);
            uncached_bodies.push(h.hit().body.as_str());
        }
    }

    if !uncached_bodies.is_empty() {
        let raw_logits = score_pool(scorer, query, &uncached_bodies)?;
        for (idx, raw_logit) in uncached_indices.into_iter().zip(raw_logits) {
            let ce_norm = 1.0 / (1.0 + (-raw_logit).exp());
            cache.scores.insert(top[idx].key(), ce_norm);
        }
    }

    let ce_norms: Vec<f64> =
        top.iter().map(|h| cache.scores.get(&h.key()).copied().unwrap_or(0.5)).collect();
    Ok(blend_ce_norms(hits, &ce_norms, alpha, pool_n))
}

#[cfg(any(feature = "default-reranker", test))]
fn blend_ce_ranked<H: RankedHit>(
    hits: &[H],
    raw_logits: Vec<f64>,
    alpha: f64,
    pool_n: usize,
) -> Vec<H> {
    let ce_norms: Vec<f64> =
        raw_logits.into_iter().map(|raw_logit| 1.0 / (1.0 + (-raw_logit).exp())).collect();
    blend_ce_norms(hits, &ce_norms, alpha, pool_n)
}

#[cfg(any(feature = "default-reranker", test, feature = "test-hooks"))]
fn blend_ce_norms<H: RankedHit>(hits: &[H], ce_norms: &[f64], alpha: f64, pool_n: usize) -> Vec<H> {
    let n = pool_n.min(hits.len());
    let top = &hits[..n];
    let rest = &hits[n..];
    // --- RRF min-max normalization over the top-N pool ---
    let rrf_min = top.iter().map(|h| h.hit().score).fold(f64::INFINITY, f64::min);
    let rrf_max = top.iter().map(|h| h.hit().score).fold(f64::NEG_INFINITY, f64::max);
    let rrf_span = rrf_max - rrf_min;

    let mut scored: Vec<(f64, H)> = top
        .iter()
        .zip(ce_norms.iter().copied())
        .map(|(h, ce_norm)| {
            let rrf_norm = if rrf_span > 0.0 { (h.hit().score - rrf_min) / rrf_span } else { 1.0 };
            // 0.8.5 — α is the caller-supplied (clamped) blend weight; default 0.3
            // reproduces the pre-slice `const ALPHA = 0.3` blend exactly.
            let blended = alpha * ce_norm + (1.0 - alpha) * rrf_norm;
            // 0.8.5 (D1) — expose the per-candidate CE score on in-pool hits.
            let mut hit = h.clone();
            hit.hit_mut().ce_score = Some(ce_norm);
            (blended, hit)
        })
        .collect();

    // Sort top-N by blended score descending (stable within ties by original order).
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut result: Vec<H> = scored
        .into_iter()
        .map(|(score, mut h)| {
            h.hit_mut().score = score;
            h
        })
        .collect();

    // Append hits beyond rerank_depth in their original RRF order.
    result.extend_from_slice(rest);
    result
}

/// One cross-encoder scorer. `Err(Some(error))` is a CUDA pool kind that must
/// propagate under every device policy; `Err(None)` is any other failure,
/// which keeps the neutral-score fallback (or the forced-policy refusal).
#[cfg(any(feature = "default-reranker", test, feature = "test-hooks"))]
trait CeScorer {
    fn score_pair(
        &self,
        query: &str,
        passage: &str,
    ) -> Result<f32, Option<RerankerDevicePolicyError>>;

    fn score_pairs(
        &self,
        query: &str,
        passages: &[&str],
    ) -> Result<Vec<f32>, Option<RerankerDevicePolicyError>>;

    fn forced_cuda_runtime_error(&self) -> Option<RerankerDevicePolicyError>;
}

/// Score every `(query, passage_i)` pair, in input order.
///
/// A batch failure is classified first. A CUDA pool kind propagates whatever
/// the policy (F-1). Any other batch failure is the forced policy's refusal
/// when CUDA is forced, and otherwise falls back to per-pair scoring, where a
/// failed pair scores a neutral `0.0` unless it, too, is a CUDA pool kind.
#[cfg(any(feature = "default-reranker", test, feature = "test-hooks"))]
fn score_pool<S: CeScorer + ?Sized>(
    scorer: &S,
    query: &str,
    passages: &[&str],
) -> Result<Vec<f64>, RerankerDevicePolicyError> {
    match scorer.score_pairs(query, passages) {
        Ok(logits) => Ok(logits.into_iter().map(f64::from).collect()),
        Err(Some(error)) => Err(error),
        Err(None) => {
            if let Some(error) = scorer.forced_cuda_runtime_error() {
                return Err(error);
            }
            passages
                .iter()
                .map(|passage| match scorer.score_pair(query, passage) {
                    Ok(logit) => Ok(f64::from(logit)),
                    Err(Some(error)) => Err(error),
                    Err(None) => Ok(0.0),
                })
                .collect()
        }
    }
}

#[cfg(feature = "default-reranker")]
impl CeScorer for fathomdb_embedder::CandleTinyBertReranker {
    fn score_pair(
        &self,
        query: &str,
        passage: &str,
    ) -> Result<f32, Option<RerankerDevicePolicyError>> {
        self.score(query, passage).map_err(|error| self.cuda_pool_error(&error))
    }

    fn score_pairs(
        &self,
        query: &str,
        passages: &[&str],
    ) -> Result<Vec<f32>, Option<RerankerDevicePolicyError>> {
        self.score_batch(query, passages).map_err(|error| self.cuda_pool_error(&error))
    }

    fn forced_cuda_runtime_error(&self) -> Option<RerankerDevicePolicyError> {
        fathomdb_embedder::CandleTinyBertReranker::forced_cuda_runtime_error(self)
    }
}

#[cfg(test)]
mod tests;
