//! **FathomDB embedder API** — the semver-stable trait contract for embedder
//! plugins.
//!
//! This crate is the seam between FathomDB and whatever produces vectors. It
//! contains a trait, an identity struct and an error type, and nothing else: it
//! deliberately pulls in no model runtime, so implementing it costs you no
//! dependency on ours.
//!
//! Implement `Embedder` to plug your own model in. `EmbedderIdentity`
//! (`name` + `revision` + `dimension`) is the contract that keeps stored
//! vectors interpretable: the engine records it on first configure and refuses
//! to reopen a database under a different identity, because vector identity
//! belongs to the embedder and silently re-embedding under a new model would
//! corrupt retrieval.
//!
//! # Versioned independently (Axis E)
//!
//! This crate does **not** move in lockstep with the rest of the workspace.
//! Bumping a FathomDB binding does not force a bump here, which is what
//! decouples embedder-protocol stability from FathomDB's release cadence. Treat
//! any change to the trait as a breaking change for every implementor.
//!
//! For the built-in implementation, see the `fathomdb-embedder` crate; to use
//! FathomDB itself, see the `fathomdb` facade crate.

pub type Vector = Vec<f32>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbedderIdentity {
    pub name: String,
    pub revision: String,
    pub dimension: u32,
}

impl EmbedderIdentity {
    #[must_use]
    pub fn new(name: impl Into<String>, revision: impl Into<String>, dimension: u32) -> Self {
        Self { name: name.into(), revision: revision.into(), dimension }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EmbedderError {
    Failed { message: String },
    Timeout,
}

pub trait Embedder: Send + Sync {
    fn identity(&self) -> EmbedderIdentity;

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError>;

    /// Embed many inputs in one call. The default implementation loops [`embed`],
    /// so every backend works unchanged; backends with a true batched forward
    /// (e.g. the candle GPU path) override this to amortize per-call overhead and
    /// saturate the device (minutes -> seconds on a full-corpus embed).
    ///
    /// Contract: `embed_batch` MUST be numerically equivalent (within float
    /// tolerance) to calling [`embed`] on each input, so a caller can switch to
    /// batching WITHOUT changing the vectors written to an index. Locked by a
    /// parity test in the default-embedder crate.
    ///
    /// [`embed`]: Embedder::embed
    fn embed_batch(&self, inputs: &[&str]) -> Result<Vec<Vector>, EmbedderError> {
        inputs.iter().map(|input| self.embed(input)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::EmbedderError;

    #[test]
    fn the_cuda_pool_errors_carry_their_payloads() {
        let exhausted = EmbedderError::CudaPoolExhausted {
            ordinal: 0,
            max_size_bytes: 3 << 30,
            message: "forward: out of memory".to_owned(),
        };
        let lost = EmbedderError::CudaContextLost {
            recorded_context_id: 7,
            current_context_id: None,
            driver_error: "CUDA_ERROR_CONTEXT_IS_DESTROYED".to_owned(),
            operation: "forward".to_owned(),
        };
        let refused =
            EmbedderError::CudaPrivateBuildRefused { ordinal: 0, message: "refused".to_owned() };
        assert_ne!(exhausted, lost);
        assert_ne!(lost, refused);
        assert_eq!(exhausted.clone(), exhausted);
    }
}
