use super::*;

pub(crate) fn map_runtime_embedder_error(err: RuntimeEmbedderError) -> EngineError {
    match err {
        RuntimeEmbedderError::Failed { .. } | RuntimeEmbedderError::Timeout => {
            EngineError::Embedder
        }
    }
}

pub(crate) fn dispatch_embed_vector(
    dispatcher: &EmbedDispatcher,
    text: &str,
) -> Result<Vec<f32>, DispatchError> {
    match dispatcher.submit_text(text.to_owned())?.wait()? {
        EmbedOutput::One(vector) => Ok(vector),
        EmbedOutput::Batch(_) => unreachable!("single embed request returned a batch"),
    }
}

impl Engine {
    /// Embed arbitrary text with the engine's configured runtime embedder,
    /// returning the raw (un-centered) vector.
    ///
    /// This is the read-path embed primitive: it mirrors the search
    /// query-embedding path through the bounded provider dispatcher. Callers
    /// get vectors under the engine's *pinned* embedder
    /// identity (`fathomdb-bge-small-en-v1.5` by default) rather than a
    /// parallel, possibly-divergent embedder.
    ///
    /// Returns [`EngineError::EmbedderNotConfigured`] without a provider,
    /// [`EngineError::Overloaded`] when admission or queue wait expires before
    /// service, [`EngineError::Closing`] on close cancellation, and
    /// [`EngineError::Embedder`] for a started provider error or timeout.
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, EngineError> {
        self.ensure_open()?;
        dispatch_embed_vector(&self.embed_dispatch, text).map_err(|error| match error {
            DispatchError::NotConfigured => EngineError::EmbedderNotConfigured,
            DispatchError::Saturated | DispatchError::QueuedExpired => EngineError::Overloaded,
            DispatchError::Closing | DispatchError::Cancelled => EngineError::Closing,
            DispatchError::StartedTimeout
            | DispatchError::Provider(_)
            | DispatchError::InvalidOutput => EngineError::Embedder,
            DispatchError::Panic(payload) => std::panic::resume_unwind(payload),
        })
    }
}
