use super::*;

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
/// 0.8.2 Slice E2 fix-1 [P2]: returns `Err` when any passage carries a non-finite
/// score (NaN / ±inf), mirroring the malformed-passage loud-fail contract.
/// Callers (pyo3 `rerank` binding, tests) must handle `Result`.
/// (`#[must_use]` removed: `Result` is already `#[must_use]`.)
pub fn rerank_passages(
    query: &str,
    passages: Vec<(u64, String, f64)>,
    rerank_depth: usize,
    alpha: f64,
    pool_n: usize,
) -> Result<Vec<(u64, f64, Option<f64>)>, String> {
    // [P2] guard: reject non-finite scores before they reach normalization/sort.
    // A NaN or ±inf score would produce NaN blended scores and an unstable sort
    // order — surface the error early as the typed WriteValidationError at the
    // pyo3 boundary (mirroring the malformed-passage loud-fail contract).
    for (id, _, score) in &passages {
        if !score.is_finite() {
            return Err(format!(
                "rerank: non-finite score for passage id={id}: {score} \
                 (NaN/\u{00b1}inf must not reach the normalization/sort step)"
            ));
        }
    }
    // Empty input is an unconditional identity path. Return after validating
    // the supplied passage records but before resolving device policy or
    // loading model state.
    if passages.is_empty() {
        return Ok(vec![]);
    }
    // Slice 71: a forced CUDA policy is a request to run CE inference on that
    // device, never permission to silently return CPU CE scores. Resolve before
    // loading weights so malformed/forced policy errors are observable without
    // a model download. Depth zero remains the no-model/no-device identity path.
    #[cfg(feature = "default-reranker")]
    if rerank_depth > 0 {
        fathomdb_embedder::resolve_default_reranker_device_from_env()
            .map_err(|error| format!("reranker device policy: {error}"))?;
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
        })
        .collect();
    // 0.8.5 — project `(id, score, ce_score)` so the binding can surface the CE
    // score per candidate; `ce_score` is `None` for the identity / out-of-pool path.
    // The projected id is the caller's ordinal (the engine-internal `write_cursor`).
    Ok(try_rerank_fused(query, hits, rerank_depth, alpha, pool_n)
        .map_err(|error| format!("reranker device policy: {error}"))?
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
fn ce_rerank(
    _query: &str,
    hits: &[SearchHit], // FIX-1: borrow, not move — caller retains ownership for soft-fallback
    _rerank_depth: usize, // 0.8.5: pool sizing moved to `pool_n`; depth gate stays in `rerank_fused`.
    alpha: f64,
    pool_n: usize,
) -> Result<Option<Vec<SearchHit>>, RerankerDevicePolicyError> {
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

    // Try to get the loaded model. Returns None when weights are absent.
    let Some(model) = CandleCrossEncoder::try_get_loaded()? else {
        return Ok(None);
    };

    // 0.8.5 (D4) — the reranked pool is the top `pool_n` (caller resolves the
    // `unwrap_or(rerank_depth)` default at the binding), clamped to the hit count.
    let n = pool_n.min(hits.len());
    let top = &hits[..n]; // no split_at_mut needed; borrow slices directly
    let rest = &hits[n..];

    // --- RRF min-max normalization over the top-N pool ---
    let rrf_min = top.iter().map(|h| h.score).fold(f64::INFINITY, f64::min);
    let rrf_max = top.iter().map(|h| h.score).fold(f64::NEG_INFINITY, f64::max);
    let rrf_span = rrf_max - rrf_min;

    // Batched CE scoring: ONE forward over the whole top-N pool instead of N
    // per-pair forwards. The ranking math below (RRF min-max norm, sigmoid,
    // ALPHA blend, sort) is byte-unchanged — only the scoring is batched.
    let bodies: Vec<&str> = top.iter().map(|h| h.body.as_str()).collect();
    #[cfg(feature = "slice72-test-hooks")]
    let raw_logits = slice72_test_hooks::with_ce_forward(|| model.score_batch(_query, &bodies))?;
    #[cfg(not(feature = "slice72-test-hooks"))]
    let raw_logits = model.score_batch(_query, &bodies)?;

    let mut scored: Vec<(f64, SearchHit)> = top
        .iter()
        .zip(raw_logits)
        .map(|(h, raw_logit)| {
            let rrf_norm = if rrf_span > 0.0 { (h.score - rrf_min) / rrf_span } else { 1.0 };
            // Sigmoid for CE normalization: 1/(1+exp(-x)).
            let ce_norm = 1.0 / (1.0 + (-raw_logit).exp());
            // 0.8.5 — α is the caller-supplied (clamped) blend weight; default 0.3
            // reproduces the pre-slice `const ALPHA = 0.3` blend exactly.
            let blended = alpha * ce_norm + (1.0 - alpha) * rrf_norm;
            // 0.8.5 (D1) — expose the per-candidate CE score on in-pool hits.
            let mut hit = h.clone();
            hit.ce_score = Some(ce_norm);
            (blended, hit)
        })
        .collect();

    // Sort top-N by blended score descending (stable within ties by original order).
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut result: Vec<SearchHit> = scored
        .into_iter()
        .map(|(score, mut h)| {
            h.score = score;
            h
        })
        .collect();

    // Append hits beyond rerank_depth in their original RRF order.
    result.extend_from_slice(rest);
    Ok(Some(result))
}

/// 0.8.1 Slice 10 (R1) / 0.8.2 Slice E1 — TinyBERT-L-2 cross-encoder;
/// its CPU or CUDA backend is selected by the reranker device policy.
///
/// Thin engine-side handle over the embedder crate's `CandleTinyBertReranker`
/// (Candle BERT stack + `tokenizers`, pinned `cross-encoder/ms-marco-TinyBERT-
/// L2-v2`). The model is loaded once, process-wide, the first time
/// `rerank_depth > 0` reaches the CE path (lazy init via the `OnceLock` below);
/// on cache miss that first load fetches the ~17 MB weights over the network
/// (sha256-verified). When the weights are absent and the network is
/// unavailable, the load fails and `try_get_loaded()` returns `None` so the
/// caller soft-falls-back to RRF order — it never panics.
///
/// Footprint: this whole type compiles ONLY under `default-reranker`. With the
/// feature off the CE path compiles away and `rerank_fused` is always identity.
/// With the feature on, `rerank_depth == 0` short-circuits in `rerank_fused`
/// BEFORE this is ever touched, so depth-0 stays byte-identical and no-network.
/// Similarly, an empty hit set short-circuits in `ce_rerank` before the singleton
/// is consulted (fix-1 [P2]).
#[cfg(feature = "default-reranker")]
struct CandleCrossEncoder {
    inner: &'static fathomdb_embedder::CandleTinyBertReranker,
}

/// Process-wide lazily-initialized reranker. `None` once initialization has
/// been attempted and failed (no weights + no network) — memoized so a failed
/// load is not retried on every query.
#[cfg(feature = "default-reranker")]
fn reranker_singleton(
) -> Result<Option<&'static fathomdb_embedder::CandleTinyBertReranker>, RerankerDevicePolicyError> {
    enum Singleton {
        Loaded(Box<fathomdb_embedder::CandleTinyBertReranker>),
        Unavailable,
        DevicePolicy(RerankerDevicePolicyError),
    }
    static CELL: std::sync::OnceLock<Singleton> = std::sync::OnceLock::new();
    match CELL.get_or_init(|| match fathomdb_embedder::CandleTinyBertReranker::try_load() {
        Ok(model) => Singleton::Loaded(Box::new(model)),
        Err(fathomdb_embedder::RerankerLoadError::DevicePolicy(error)) => {
            Singleton::DevicePolicy(error)
        }
        Err(_) => Singleton::Unavailable,
    }) {
        Singleton::Loaded(model) => Ok(Some(model)),
        Singleton::Unavailable => Ok(None),
        Singleton::DevicePolicy(error) => Err(error.clone()),
    }
}

#[cfg(feature = "default-reranker")]
impl CandleCrossEncoder {
    /// Returns a model handle if the reranker is (or can be) loaded, `None`
    /// otherwise. The first call drives the lazy load (cache probe → gated
    /// download); subsequent calls reuse the memoized result.
    fn try_get_loaded() -> Result<Option<Self>, RerankerDevicePolicyError> {
        Ok(reranker_singleton()?.map(|inner| Self { inner }))
    }

    /// Score a (query, passage) pair. Returns the raw cross-encoder logit, or
    /// `0.0` (a neutral logit → sigmoid 0.5) if the forward pass errors, so a
    /// single bad pair degrades to a neutral CE contribution rather than
    /// panicking in the reader thread.
    fn score(&self, query: &str, passage: &str) -> f64 {
        self.inner.score(query, passage).map(f64::from).unwrap_or(0.0)
    }

    /// Batched [`score`](Self::score): score every `(query, passage_i)` pair in a
    /// single forward pass. Returns one logit per passage in input order, each
    /// honoring the same neutral-`0.0`-on-error contract as [`score`](Self::score).
    ///
    /// Fallback: if the batched forward errors as a whole (e.g. an OOM or a
    /// tokenize failure on one pair surfaces as a batch `Err`), we DO NOT
    /// neutralize the entire pool — we fall back to per-pair [`score`](Self::score),
    /// so a single bad pair degrades only its own element to a neutral logit while
    /// the rest keep their real scores. Empty input → empty output (no forward).
    fn score_batch(
        &self,
        query: &str,
        passages: &[&str],
    ) -> Result<Vec<f64>, RerankerDevicePolicyError> {
        match self.inner.score_batch(query, passages) {
            Ok(logits) => Ok(logits.into_iter().map(f64::from).collect()),
            Err(_) if self.inner.forced_cuda_runtime_error().is_some() => {
                Err(self.inner.forced_cuda_runtime_error().expect("checked above"))
            }
            Err(_) => Ok(passages.iter().map(|p| self.score(query, p)).collect()),
        }
    }
}
