use super::*;

// ===== rerank (0.8.2 Slice E2) ========================================
//
// Standalone CE rerank over a caller-supplied passage list — NOT engine-bound.
// Slice 5's `fused_rerank` comparator must CE-rerank its OWN in-harness
// fused(bm25+dense) pool with the identical cross-encoder, but the engine's
// `search()` only reranks its own capped text pool. This thin wrapper marshals
// `[{"id": int, "body": str, "score": float}]` into `(id, body, score)` tuples,
// calls the pure engine helper `rerank_passages`, and returns the reranked
// order as `[{"id": int, "score": float}]` (input score is the harness's fused
// RRF score; the output score is the CE-blended score). Identity contract:
// `rerank_depth == 0` OR an empty list returns the input order with input scores
// (no model load, no network — feature-off the whole CE path is compiled away).
// Never panics: malformed passages raise the typed `WriteValidationError`; the
// pure helper is `catch_unwind`-wrapped (mirroring `call_engine`) so any
// escaping panic surfaces as a `PanicException`, never an abort.

/// Extract a required non-negative integer `id` from a passage dict.
pub(super) fn dict_u64_required(d: &Bound<'_, PyDict>, key: &str) -> PyResult<u64> {
    let v = dict_get(d, key)?.filter(|v| !v.is_none()).ok_or_else(|| {
        WriteValidationError::new_err(format!("passage missing required field {key:?}"))
    })?;
    v.extract::<u64>().map_err(|_| {
        WriteValidationError::new_err(format!(
            "passage field {key:?} must be a non-negative integer"
        ))
    })
}

/// Extract a required finite float `score` from a passage dict.
pub(super) fn dict_f64_required(d: &Bound<'_, PyDict>, key: &str) -> PyResult<f64> {
    let v = dict_get(d, key)?.filter(|v| !v.is_none()).ok_or_else(|| {
        WriteValidationError::new_err(format!("passage missing required field {key:?}"))
    })?;
    v.extract::<f64>().map_err(|_| {
        WriteValidationError::new_err(format!("passage field {key:?} must be a number"))
    })
}

#[pyfunction]
#[pyo3(signature = (query, passages, rerank_depth, alpha=None, pool_n=None))]
pub(super) fn rerank(
    py: Python<'_>,
    query: &str,
    passages: &Bound<'_, PyList>,
    rerank_depth: usize,
    // 0.8.5 (EXP-0) — CE-blend weight (default 0.3, clamped to [0,1] in the engine)
    // and reranked-pool size (default = rerank_depth). Omitting both reproduces the
    // pre-slice α=0.3 blend; `alpha=1.0, pool_n=10` is the measured-parity config.
    alpha: Option<f64>,
    pool_n: Option<usize>,
) -> PyResult<Vec<Py<PyDict>>> {
    validate_ffi_string_py(query)?;

    // Marshal the passage dicts into `(id, body, score)` tuples. `body` rides the
    // same FFI string gate as the write path (rejects embedded NUL / lone
    // surrogate as the typed WriteValidationError).
    let tuples: Vec<(u64, String, f64)> = passages
        .iter()
        .map(|item| {
            let dict = item
                .cast::<PyDict>()
                .map_err(|_| WriteValidationError::new_err("passage must be a dict"))?;
            let id = dict_u64_required(dict, "id")?;
            let body = dict_str_required(dict, "body")?;
            let score = dict_f64_required(dict, "score")?;
            Ok((id, body, score))
        })
        .collect::<PyResult<_>>()?;

    let query = query.to_string();
    // 0.8.5 (D4): resolve the binding-side defaults — α=0.3, pool_n=rerank_depth.
    let alpha = alpha.unwrap_or(0.3);
    let pool_n = pool_n.unwrap_or(rerank_depth);
    // The helper is pure CPU (no engine handle); it may perform a one-time gated
    // model load on a cold cache, so release the GIL for the duration.
    // `catch_unwind` + `AssertUnwindSafe` mirror `call_engine` so the never-panic
    // contract holds even though the helper returns a Result channel.
    // E2 fix-1 [P2]: `rust_rerank_passages` now returns `Result<Vec<…>, String>`;
    // the inner `Err` (non-finite score) surfaces as `WriteValidationError`.
    let reranked = py
        .detach(|| {
            catch_unwind(AssertUnwindSafe(move || {
                rust_rerank_passages(&query, tuples, rerank_depth, alpha, pool_n)
            }))
        })
        // outer Result: catch_unwind — any panic → PanicException (hard invariant).
        .map_err(|_| PanicException::new_err("rerank panic (see logs)"))?
        // inner Result: validation error (non-finite score) → WriteValidationError.
        .map_err(WriteValidationError::new_err)?;

    reranked
        .into_iter()
        .map(|(id, score, ce_score)| {
            let d = PyDict::new(py);
            d.set_item("id", id)?;
            d.set_item("score", score)?;
            // 0.8.5 — additive CE score; None (Python `None`) outside the reranked pool.
            d.set_item("ce_score", ce_score)?;
            Ok(d.unbind())
        })
        .collect()
}

// ===== CLS batch embedder (V-3 dense-encoder GPU path) ================
//
// Exposes candle `embed_batch` with `Pooling::Cls` to Python. The stock
// `PyEngine::embed()` uses the engine's default `Pooling::Mean`; the V-3
// A0/A3 dense stack (m1_baseline.BGEEncoder) is CLS-pooled + L2-normalized, so
// routing that harness through `embed()` would silently switch Mean↔CLS and
// break comparability to V-1. This binding loads the SAME pinned bge-small
// weights the numpy path loads, pins `Pooling::Cls`, and honors
// `FATHOMDB_EMBED_DEVICE` (unset means `auto`; `cuda:N` under the `embed-cuda`
// feature is forced). One padded `(B, L)` forward → the same per-row vectors as B single
// `embed()` calls (parity-locked in the embedder crate's tests). Additive: it
// does NOT change `embed()`'s default pooling. This also closes the standing
// "Python embed cannot select CLS pooling" exposure gap.

/// Memoize the result of `init` in `cell`, caching **only on success**.
///
/// fix-1 finding 2: the previous CLS-embedder singleton stored
/// `Option<CandleBgeEmbedder>` and initialized it with `init().ok()`, so a
/// *transient* load failure (cache miss + no network) was cached as `None`
/// forever and every later call — even after connectivity returned — reported
/// the same "unavailable" error and never retried. This helper instead returns
/// the real error on failure and leaves `cell` empty, so the next call retries;
/// a value is stored only when init succeeds. On a lost init race the loser's
/// value is dropped and the winner's `'static` ref is returned.
#[allow(dead_code)] // used under `default-embedder`; exercised directly by unit tests
pub(super) fn get_or_try_init<T, E>(
    cell: &'static std::sync::OnceLock<T>,
    init: impl FnOnce() -> Result<T, E>,
) -> Result<&'static T, E> {
    if let Some(v) = cell.get() {
        return Ok(v);
    }
    let v = init()?;
    Ok(cell.get_or_init(|| v))
}

/// Process-wide, lazily-initialized CLS-pooled default embedder. Loaded on
/// first use and memoized **on success only** (via [`get_or_try_init`]): a
/// failed load surfaces the real [`EmbedderLoadError`] and is NOT cached, so a
/// later call retries. Unlike the engine's `reranker_singleton` (which caches a
/// `None`), this preserves the ability to recover once weights become reachable.
///
/// [`EmbedderLoadError`]: fathomdb_embedder::loader::EmbedderLoadError
#[cfg(feature = "default-embedder")]
pub(super) fn cls_embedder_singleton() -> Result<
    &'static fathomdb_embedder::CandleBgeEmbedder,
    fathomdb_embedder::loader::EmbedderLoadError,
> {
    static CELL: std::sync::OnceLock<fathomdb_embedder::CandleBgeEmbedder> =
        std::sync::OnceLock::new();
    get_or_try_init(&CELL, || {
        Ok(fathomdb_embedder::CandleBgeEmbedder::new()?
            .with_pooling(fathomdb_embedder::Pooling::Cls))
    })
}

/// Embed many texts with the pinned default bge-small weights using **CLS
/// pooling** + L2-normalization, honoring `FATHOMDB_EMBED_DEVICE`. Returns one
/// `list[float]` per input, in the same order. Distinct from `Engine.embed()`,
/// which uses the engine's default Mean pooling. Requires a wheel built with
/// `default-embedder` (or `embed-cuda`); otherwise raises
/// `EmbedderNotConfiguredError`.
#[pyfunction]
pub(super) fn embed_batch_cls(py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
    for t in &texts {
        validate_ffi_string_py(t)?;
    }
    embed_batch_cls_impl(py, texts)
}

#[cfg(feature = "default-embedder")]
pub(super) fn embed_batch_cls_impl(py: Python<'_>, texts: Vec<String>) -> PyResult<Vec<Vec<f32>>> {
    use fathomdb_embedder_api::Embedder;
    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let embedder = cls_embedder_singleton().map_err(|e| {
        // fix-1 finding 2: surface the REAL loader error (e.g. checksum
        // mismatch, cache I/O, dimension drift) instead of flattening every
        // failure to a generic "cache miss + no network" string.
        EmbedderNotConfiguredError::new_err(format!("default embedder weights unavailable: {e}"))
    })?;
    // Release the GIL for the (pure CPU/GPU compute) forward pass, and wrap in
    // `catch_unwind` so the never-panic FFI contract holds even though the
    // embedder returns a Result channel (mirrors `rerank`).
    let result = py
        .detach(|| {
            catch_unwind(AssertUnwindSafe(|| {
                let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
                embedder.embed_batch(&refs)
            }))
        })
        .map_err(|_| PanicException::new_err("embed_batch_cls panic (see logs)"))?;
    result.map_err(|e| match e {
        // 0.8.28 pool study only (ruling 15): see `engine_error_to_py`.
        #[cfg(feature = "tegra-pool-experiment")]
        fathomdb_embedder_api::EmbedderError::CudaPoolExhausted {
            ordinal,
            max_size_bytes,
            message,
        } => {
            let exc = EmbedderError::new_err(format!("embed_batch_cls: {message}"));
            let value = exc.value(py);
            let _ = value.setattr("kind", "cuda_pool_exhausted");
            let _ = value.setattr("ordinal", ordinal);
            let _ = value.setattr("max_size_bytes", max_size_bytes);
            exc
        }
        e => EmbedderError::new_err(format!("embed_batch_cls: {e:?}")),
    })
}

#[cfg(not(feature = "default-embedder"))]
pub(super) fn embed_batch_cls_impl(
    _py: Python<'_>,
    _texts: Vec<String>,
) -> PyResult<Vec<Vec<f32>>> {
    Err(EmbedderNotConfiguredError::new_err(
        "embed_batch_cls requires a wheel built with the `default-embedder` \
         (or `embed-cuda`) feature",
    ))
}
