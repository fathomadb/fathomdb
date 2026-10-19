//! Package-root operations that need no open engine: passage reranking,
//! CLS-pooled batch embedding, search hints and snippet selection.

use crate::error::{Error, ErrorKind, Result};
use crate::guard;
use crate::options::RerankOptions;
use crate::{Hint, SearchFacts, SnippetOptions, SnippetV1};

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
/// non-finite score, an embedded NUL, or a reranker refusal.
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
        .map_err(|message| Error::sdk(ErrorKind::WriteValidation, message))?;
    Ok(reranked
        .into_iter()
        .map(|(id, score, ce_score)| RerankResult { id, score, ce_score })
        .collect())
}

/// Embed `texts` with the pinned default BGE weights using CLS pooling and
/// L2 normalization; one vector per input, in order. Distinct from
/// [`crate::Engine::embed`], which is mean-pooled.
///
/// # Errors
/// `EmbedderNotConfigured` without the `default-embedder` feature (for every
/// input, including empty) or when the weights cannot be loaded; `Embedder`
/// when embedding fails; `WriteValidation` for an embedded NUL.
pub fn embed_batch_cls(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
    guard::texts(texts)?;
    cls::embed(texts)
}

#[cfg(feature = "default-embedder")]
mod cls {
    use std::sync::OnceLock;

    use fathomdb_embedder::{CandleBgeEmbedder, Pooling};
    use fathomdb_embedder_api::Embedder;

    use crate::error::{Error, ErrorKind, Result};

    static EMBEDDER: OnceLock<CandleBgeEmbedder> = OnceLock::new();

    // Cached on success only, so a failed weight load is retried next call.
    fn embedder() -> Result<&'static CandleBgeEmbedder> {
        if let Some(embedder) = EMBEDDER.get() {
            return Ok(embedder);
        }
        let loaded = CandleBgeEmbedder::new()
            .map_err(|error| {
                Error::sdk(
                    ErrorKind::EmbedderNotConfigured,
                    format!("default embedder weights unavailable: {error}"),
                )
            })?
            .with_pooling(Pooling::Cls);
        Ok(EMBEDDER.get_or_init(|| loaded))
    }

    pub(super) fn embed(texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        embedder()?
            .embed_batch(texts)
            .map_err(|error| Error::sdk(ErrorKind::Embedder, format!("embed_batch_cls: {error:?}")))
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

/// Append the engine-subset hints for one search page to `out`
/// (ADR-0.8.29-agent-hints-sidecar): at most one per reason, in the order
/// `degraded`, `no_results`, `more_results`.
///
/// - `degraded` when `facts.degradations` is non-empty, naming its first
///   entry.
/// - `no_results` when `facts.returned` is zero, with `mode: Hybrid` only for
///   a text-mode search with `hybrid_available`.
/// - `more_results` when the page reached its limit and the limit is below
///   [`crate::HINT_LIMIT_CAP`], suggesting the smaller of twice the limit and
///   the cap.
///
/// Reads nothing but `facts`; never panics.
pub fn search_hints(facts: &SearchFacts<'_>, out: &mut Vec<Hint>) {
    fathomdb_engine::search_hints(facts, out);
}

/// Select the EP-17 match-centred snippet for `body` from its match `spans`
/// (byte ranges `[start, end)`) under `options`; the selector the engine
/// runs, for re-running selection under another budget.
///
/// Empty spans, spans past the end of the body and spans off `char`
/// boundaries are ignored; options outside their documented ranges are
/// clamped ([`SnippetOptions::validate`] is the rejecting check). Pure: the
/// result depends only on the arguments and the build's Unicode tables.
#[must_use]
pub fn snippet_for(body: &str, spans: &[(u32, u32)], options: &SnippetOptions) -> SnippetV1 {
    fathomdb_engine::snippet_for(body, spans, options)
}
