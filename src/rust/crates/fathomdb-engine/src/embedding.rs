use super::*;

pub(crate) fn map_runtime_embedder_error(err: RuntimeEmbedderError) -> EngineError {
    match err {
        RuntimeEmbedderError::Failed { .. } | RuntimeEmbedderError::Timeout => {
            EngineError::Embedder
        }
    }
}

impl Engine {
    /// Embed arbitrary text with the engine's configured runtime embedder,
    /// returning the raw (un-centered) vector.
    ///
    /// This is the read-path embed primitive: it mirrors the search
    /// query-embedding path — a single, direct [`Embedder::embed`] call.
    /// Foreground dispatch is integrated in a later slice batch. Callers get
    /// vectors under the engine's *pinned* embedder
    /// identity (`fathomdb-bge-small-en-v1.5` by default) rather than a
    /// parallel, possibly-divergent embedder.
    ///
    /// Returns [`EngineError::EmbedderNotConfigured`] if the engine was opened
    /// without an embedder (`use_default_embedder = false`).
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, EngineError> {
        self.ensure_open()?;
        let embedder =
            self.runtime_embedder.as_ref().cloned().ok_or(EngineError::EmbedderNotConfigured)?;
        embedder.embed(text).map_err(map_runtime_embedder_error)
    }
}
