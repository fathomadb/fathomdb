use super::*;

// ===== CLS batch embedder =============================================

/// Process-wide, lazily-initialized CLS-pooled default embedder. The value is
/// memoized only after a successful weight load, so a transient cache/network
/// failure remains retryable on the next JavaScript call.
#[cfg(feature = "default-embedder")]
pub(crate) fn cls_embedder_singleton() -> std::result::Result<
    &'static fathomdb_embedder::CandleBgeEmbedder,
    fathomdb_embedder::loader::EmbedderLoadError,
> {
    static CELL: std::sync::OnceLock<fathomdb_embedder::CandleBgeEmbedder> =
        std::sync::OnceLock::new();
    if let Some(embedder) = CELL.get() {
        return Ok(embedder);
    }
    let embedder =
        fathomdb_embedder::CandleBgeEmbedder::new()?.with_pooling(fathomdb_embedder::Pooling::Cls);
    Ok(CELL.get_or_init(|| embedder))
}

/// Embed a batch with the pinned default BGE-small model using CLS pooling.
///
/// This is the TypeScript peer of Python's module-level `embed_batch_cls`.
/// It is deliberately distinct from [`Engine::embed`], which uses the
/// engine's Mean-pooling read path. Work runs on a blocking worker so the Node
/// event loop remains available while model loading or the forward pass runs.
#[napi(js_name = "embedBatchCls")]
pub async fn embed_batch_cls(texts: Vec<String>) -> Result<Vec<Vec<f64>>> {
    for text in &texts {
        validate_ffi_string_napi(text)?;
    }
    embed_batch_cls_impl(texts).await
}

/// One caller-supplied passage accepted by standalone reranking.
#[napi(object)]
pub struct RerankPassageInput {
    pub id: i64,
    pub body: String,
    pub score: f64,
}

/// One standalone reranking result in final order.
#[napi(object)]
pub struct RerankResult {
    pub id: i64,
    pub score: f64,
    pub ce_score: Option<f64>,
}

/// Rerank an arbitrary caller-supplied passage pool with the engine CE helper.
///
/// `rerank_depth == 0` or an empty input is a model-free identity path. Builds
/// without the default reranker retain that identity behavior for every depth.
#[napi(js_name = "rerank")]
pub async fn rerank(
    query: String,
    passages: Vec<RerankPassageInput>,
    rerank_depth: u32,
    alpha: Option<f64>,
    pool_n: Option<u32>,
) -> Result<Vec<RerankResult>> {
    validate_ffi_string_napi(&query)?;
    if !alpha.unwrap_or(0.3).is_finite() {
        return Err(typed_error(CODE_WRITE_VALIDATION, "alpha must be finite", JsonValue::Null));
    }
    let mut tuples = Vec::with_capacity(passages.len());
    for passage in passages {
        if passage.id < 0 {
            return Err(typed_error(
                CODE_WRITE_VALIDATION,
                "passage id must be a non-negative integer",
                JsonValue::Null,
            ));
        }
        validate_ffi_string_napi(&passage.body)?;
        if !passage.score.is_finite() {
            return Err(typed_error(
                CODE_WRITE_VALIDATION,
                "passage score must be finite",
                JsonValue::Null,
            ));
        }
        tuples.push((passage.id as u64, passage.body, passage.score));
    }
    let depth = rerank_depth as usize;
    let alpha = alpha.unwrap_or(0.3);
    let pool_n = pool_n.map(|value| value as usize).unwrap_or(depth);
    let joined = tokio::task::spawn_blocking(move || {
        catch_unwind(AssertUnwindSafe(|| {
            rust_rerank_passages(&query, tuples, depth, alpha, pool_n)
        }))
    })
    .await;
    match joined {
        Ok(Ok(Ok(values))) => values
            .into_iter()
            .map(|(id, score, ce_score)| {
                let id = i64::try_from(id).map_err(|_| {
                    typed_error(
                        CODE_WRITE_VALIDATION,
                        "rerank result id exceeds the TypeScript integer boundary",
                        JsonValue::Null,
                    )
                })?;
                Ok(RerankResult { id, score, ce_score })
            })
            .collect(),
        Ok(Ok(Err(error))) => Err(typed_error(CODE_WRITE_VALIDATION, error, JsonValue::Null)),
        Ok(Err(_panic)) => Err(panic_error()),
        Err(join_error) => Err(typed_error(
            CODE_PANIC,
            format!("spawn_blocking join error: {join_error}"),
            JsonValue::Null,
        )),
    }
}

#[cfg(feature = "default-embedder")]
pub(crate) async fn embed_batch_cls_impl(texts: Vec<String>) -> Result<Vec<Vec<f64>>> {
    use fathomdb_embedder_api::Embedder;

    if texts.is_empty() {
        return Ok(Vec::new());
    }
    let join_result = tokio::task::spawn_blocking(move || {
        catch_unwind(AssertUnwindSafe(|| {
            let embedder = cls_embedder_singleton().map_err(|err| {
                typed_error(
                    CODE_EMBEDDER_NOT_CONFIGURED,
                    format!("default embedder weights unavailable: {err}"),
                    JsonValue::Null,
                )
            })?;
            let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
            embedder.embed_batch(&refs).map_err(|err| {
                typed_error(CODE_EMBEDDER, format!("embed_batch_cls: {err:?}"), JsonValue::Null)
            })
        }))
    })
    .await;
    match join_result {
        Ok(Ok(Ok(vectors))) => Ok(vectors
            .into_iter()
            .map(|vector| vector.into_iter().map(f64::from).collect())
            .collect()),
        Ok(Ok(Err(err))) => Err(err),
        Ok(Err(_panic)) => Err(panic_error()),
        Err(join_err) => Err(typed_error(
            CODE_PANIC,
            format!("spawn_blocking join error: {join_err}"),
            JsonValue::Null,
        )),
    }
}

#[cfg(not(feature = "default-embedder"))]
pub(crate) async fn embed_batch_cls_impl(_texts: Vec<String>) -> Result<Vec<Vec<f64>>> {
    Err(typed_error(
        CODE_EMBEDDER_NOT_CONFIGURED,
        "embedBatchCls requires a build with the `default-embedder` feature",
        JsonValue::Null,
    ))
}
