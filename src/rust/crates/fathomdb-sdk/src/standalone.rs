//! Package-root operations that need no open engine: passage reranking and
//! CLS-pooled batch embedding.

use crate::error::{Error, ErrorKind, Result};
use crate::guard;
use crate::options::RerankOptions;

/// One caller-supplied passage for [`rerank`].
#[derive(Clone, Debug, PartialEq)]
pub struct RerankPassage {
    pub id: u64,
    pub body: String,
    /// The caller's first-stage score; must be finite.
    pub score: f64,
}

/// One reranked passage. `ce_score` is `None` on the identity path.
#[derive(Clone, Debug, PartialEq)]
pub struct RerankResult {
    pub id: u64,
    pub score: f64,
    pub ce_score: Option<f64>,
}

/// Rerank caller passages with the engine's cross-encoder blend.
///
/// `rerank_depth == 0`, empty input, or a build without `default-reranker`
/// returns the passages in input order (identity path).
///
/// # Errors
/// `InvalidArgument` for a non-finite `alpha`; `WriteValidation` for a
/// non-finite score or an embedded NUL; `RerankerDevicePolicy` when the
/// reranker device policy cannot be honored; a CUDA pool kind
/// (`CudaPoolExhausted`, `CudaContextLost`, `CudaPrivateBuildRefused`, with
/// [`Error::cuda_details`]) when the cross-encoder fails with one.
pub fn rerank(
    query: &str,
    passages: &[RerankPassage],
    rerank_depth: usize,
    options: RerankOptions,
) -> Result<Vec<RerankResult>> {
    let alpha = guard::alpha(options.alpha)?;
    guard::text(query)?;
    let mut tuples = Vec::with_capacity(passages.len());
    for passage in passages {
        guard::text(&passage.body)?;
        if !passage.score.is_finite() {
            return Err(Error::sdk(ErrorKind::WriteValidation, "passage score must be finite"));
        }
        tuples.push((passage.id, passage.body.clone(), passage.score));
    }
    let pool_n = options.pool_n.unwrap_or(rerank_depth);
    let reranked = fathomdb_engine::rerank_passages(query, tuples, rerank_depth, alpha, pool_n)
        .map_err(rerank_error)?;
    Ok(reranked
        .into_iter()
        .map(|(id, score, ce_score)| RerankResult { id, score, ce_score })
        .collect())
}

fn rerank_error(error: fathomdb_engine::RerankPassagesError) -> Error {
    match error {
        fathomdb_engine::RerankPassagesError::WriteValidation { message } => {
            Error::sdk(ErrorKind::WriteValidation, message)
        }
        fathomdb_engine::RerankPassagesError::Reranker(error) => {
            Error::from(fathomdb_engine::EngineError::from(error))
        }
    }
}

#[cfg_attr(not(feature = "default-embedder"), allow(dead_code))]
fn cls_forward_error(error: fathomdb_embedder_api::EmbedderError) -> Error {
    Error::embedder(&error, "embed_batch_cls")
}

/// Embed `texts` with the pinned default BGE weights using CLS pooling and
/// L2 normalization; one vector per input, in order. Distinct from
/// [`crate::Engine::embed`], which is mean-pooled.
///
/// # Errors
/// `EmbedderNotConfigured` without the `default-embedder` feature (for every
/// input, including empty) or when the weights cannot be loaded; a CUDA pool
/// kind (with [`Error::cuda_details`]) when loading or embedding fails with
/// one; `Embedder` when embedding otherwise fails; `WriteValidation` for an
/// embedded NUL.
pub fn embed_batch_cls(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    guard::texts(texts)?;
    cls::embed(texts)
}

#[cfg(feature = "default-embedder")]
mod cls {
    use std::sync::OnceLock;

    use fathomdb_embedder::loader::EmbedderLoadError;
    use fathomdb_embedder::{CandleBgeEmbedder, Pooling};
    use fathomdb_embedder_api::Embedder;

    use crate::error::{CudaErrorDetails, Error, ErrorKind, Result};

    static EMBEDDER: OnceLock<CandleBgeEmbedder> = OnceLock::new();

    /// A CUDA pool kind keeps its typed class; any other load failure means
    /// the default weights are unavailable.
    pub(super) fn load_error(error: EmbedderLoadError) -> Error {
        match error {
            EmbedderLoadError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
                Error::Cuda(CudaErrorDetails::PoolExhausted { ordinal, max_size_bytes, message })
            }
            EmbedderLoadError::CudaContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            } => Error::Cuda(CudaErrorDetails::ContextLost {
                recorded_context_id,
                current_context_id,
                driver_error,
                operation,
            }),
            EmbedderLoadError::CudaPrivateBuildRefused { ordinal, message } => {
                Error::Cuda(CudaErrorDetails::PrivateBuildRefused { ordinal, message })
            }
            other => Error::sdk(
                ErrorKind::EmbedderNotConfigured,
                format!("default embedder weights unavailable: {other}"),
            ),
        }
    }

    // Cached on success only, so a failed weight load is retried next call.
    fn embedder() -> Result<&'static CandleBgeEmbedder> {
        if let Some(embedder) = EMBEDDER.get() {
            return Ok(embedder);
        }
        let loaded = CandleBgeEmbedder::new().map_err(load_error)?.with_pooling(Pooling::Cls);
        Ok(EMBEDDER.get_or_init(|| loaded))
    }

    pub(super) fn embed(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        embedder()?.embed_batch(texts).map_err(super::cls_forward_error)
    }
}

#[cfg(not(feature = "default-embedder"))]
mod cls {
    use crate::error::{Error, ErrorKind, Result};

    pub(super) fn embed(_texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Err(Error::sdk(
            ErrorKind::EmbedderNotConfigured,
            "embed_batch_cls requires the `default-embedder` feature",
        ))
    }
}

#[cfg(test)]
mod tests {
    use fathomdb_embedder::RerankerDevicePolicyError;
    use fathomdb_embedder_api::EmbedderError;
    use fathomdb_engine::RerankPassagesError;

    use super::*;
    use crate::error::CudaErrorDetails;

    #[test]
    fn rerank_failures_keep_their_typed_kinds() {
        let error = rerank_error(RerankPassagesError::Reranker(
            RerankerDevicePolicyError::CudaPoolExhausted {
                ordinal: 0,
                max_size_bytes: 1,
                message: "oom".to_owned(),
            },
        ));
        assert_eq!(error.kind(), ErrorKind::CudaPoolExhausted);
        assert_eq!(
            error.cuda_details(),
            Some(&CudaErrorDetails::PoolExhausted {
                ordinal: 0,
                max_size_bytes: 1,
                message: "oom".to_owned(),
            })
        );
        let error =
            rerank_error(RerankPassagesError::Reranker(RerankerDevicePolicyError::Resolution(
                fathomdb_embedder::RerankerDeviceResolutionError::CudaNotCompiled { ordinal: 0 },
            )));
        assert_eq!(error.kind(), ErrorKind::RerankerDevicePolicy);
        let error =
            rerank_error(RerankPassagesError::WriteValidation { message: "non-finite".into() });
        assert_eq!(error.kind(), ErrorKind::WriteValidation);
        assert!(error.to_string().contains("non-finite"));
    }

    #[test]
    fn embed_batch_cls_forward_failures_keep_their_typed_kinds() {
        let error = cls_forward_error(EmbedderError::CudaContextLost {
            recorded_context_id: 1,
            current_context_id: None,
            driver_error: "d".to_owned(),
            operation: "o".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::CudaContextLost);
        let error = cls_forward_error(EmbedderError::CudaPrivateBuildRefused {
            ordinal: 0,
            message: "m".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::CudaPrivateBuildRefused);
        let error = cls_forward_error(EmbedderError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 1,
            message: "m".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::CudaPoolExhausted);
        let error = cls_forward_error(EmbedderError::Failed { message: "boom".to_owned() });
        assert_eq!(error.kind(), ErrorKind::Embedder);
        assert!(error.to_string().contains("boom"));
    }

    #[cfg(feature = "default-embedder")]
    #[test]
    fn embed_batch_cls_load_failures_keep_their_typed_kinds() {
        use fathomdb_embedder::loader::EmbedderLoadError;
        let error = cls::load_error(EmbedderLoadError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 1,
            message: "m".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::CudaPoolExhausted);
        let error = cls::load_error(EmbedderLoadError::CudaContextLost {
            recorded_context_id: 1,
            current_context_id: None,
            driver_error: "d".to_owned(),
            operation: "o".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::CudaContextLost);
        let error = cls::load_error(EmbedderLoadError::CudaPrivateBuildRefused {
            ordinal: 0,
            message: "m".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::CudaPrivateBuildRefused);
        let error = cls::load_error(EmbedderLoadError::DeviceInitialization {
            message: "no device".to_owned(),
        });
        assert_eq!(error.kind(), ErrorKind::EmbedderNotConfigured);
    }
}
