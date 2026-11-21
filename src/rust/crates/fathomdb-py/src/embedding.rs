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
    if !alpha.is_finite() {
        return Err(PyValueError::new_err("alpha must be finite"));
    }
    let pool_n = pool_n.unwrap_or(rerank_depth);
    // The helper is pure CPU (no engine handle); it may perform a one-time gated
    // model load on a cold cache, so release the GIL for the duration.
    // `catch_unwind` + `AssertUnwindSafe` mirror `call_engine` so the never-panic
    // contract holds even though the helper returns a Result channel.
    // The inner `Err` keeps its typed class: a non-finite score is
    // `WriteValidationError`; a reranker refusal is its own class.
    let reranked = py
        .detach(|| {
            catch_unwind(AssertUnwindSafe(move || {
                rust_rerank_passages(&query, tuples, rerank_depth, alpha, pool_n)
            }))
        })
        // outer Result: catch_unwind — any panic → PanicException (hard invariant).
        .map_err(|_| PanicException::new_err("rerank panic (see logs)"))?
        .map_err(rerank_passages_error_to_py)?;

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

/// A standalone rerank refusal: invalid input is `WriteValidationError`; a
/// reranker refusal keeps its typed class.
pub(super) fn rerank_passages_error_to_py(error: fathomdb_engine::RerankPassagesError) -> PyErr {
    match error {
        fathomdb_engine::RerankPassagesError::WriteValidation { message } => {
            WriteValidationError::new_err(message)
        }
        fathomdb_engine::RerankPassagesError::Reranker(error) => {
            reranker_device_policy_error_to_py(error)
        }
    }
}

/// A CLS forward failure: a CUDA pool kind keeps its typed class.
#[cfg_attr(not(feature = "default-embedder"), allow(dead_code))]
pub(super) fn cls_forward_error_to_py(error: fathomdb_embedder_api::EmbedderError) -> PyErr {
    embedder_api_error_to_py(error, "embed_batch_cls")
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
    let result = py
        .detach(|| {
            catch_unwind(AssertUnwindSafe(|| {
                let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
                fathomdb_embedder::module_models::embed_batch_cls(&refs)
            }))
        })
        .map_err(|_| PanicException::new_err("embed_batch_cls panic (see logs)"))?;
    result.map_err(|error| {
        use fathomdb_embedder::module_models::ModuleEmbeddingError;
        match error {
            ModuleEmbeddingError::Busy => OverloadedError::new_err("module models are busy"),
            ModuleEmbeddingError::Load(error) => cls_load_error_to_py(error),
            ModuleEmbeddingError::Forward(error) => cls_forward_error_to_py(error),
        }
    })
}

/// A CLS model-load failure: a CUDA pool kind keeps its typed class; any
/// other failure (the real loader error, e.g. checksum mismatch or cache I/O)
/// means the default weights are unavailable.
#[cfg(feature = "default-embedder")]
pub(super) fn cls_load_error_to_py(error: fathomdb_embedder::loader::EmbedderLoadError) -> PyErr {
    use fathomdb_embedder::loader::EmbedderLoadError;
    match error {
        EmbedderLoadError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
            cuda_pool_exhausted_to_py(ordinal, max_size_bytes, message)
        }
        EmbedderLoadError::CudaContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        } => cuda_context_lost_to_py(
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        ),
        EmbedderLoadError::CudaPrivateBuildRefused { ordinal, message } => {
            cuda_private_build_refused_to_py(ordinal, message)
        }
        other => EmbedderNotConfiguredError::new_err(format!(
            "default embedder weights unavailable: {other}"
        )),
    }
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

/// Release cached module CLS and reranker models without closing engines.
/// Active inference/loading/release raises OverloadedError and clears neither
/// cache. Empty/repeated release succeeds. Healthy subsequent calls may reload;
/// CUDA generation loss remains terminal until process restart.
#[pyfunction]
pub(super) fn release_models(py: Python<'_>) -> PyResult<()> {
    py.detach(|| catch_unwind(AssertUnwindSafe(fathomdb_embedder::module_models::release_models)))
        .map_err(|_| PanicException::new_err("release_models panic (see logs)"))?
        .map_err(reranker_device_policy_error_to_py)
}
