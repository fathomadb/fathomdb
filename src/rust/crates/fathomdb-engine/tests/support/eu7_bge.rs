use fathomdb_embedder::CandleBgeEmbedder;
use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};

// Keep historical batching/truncation defaults; production dispatch owns concurrency.
pub(crate) struct Eu7Bge {
    inner: CandleBgeEmbedder,
    identity: EmbedderIdentity,
}

impl Eu7Bge {
    pub(crate) fn new(inner: CandleBgeEmbedder) -> Self {
        let identity = inner.identity();
        Self { inner, identity }
    }
}

impl Embedder for Eu7Bge {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        self.inner.embed(text)
    }
}
