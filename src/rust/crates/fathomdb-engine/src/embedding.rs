use super::*;

/// G11 — the fixed projection kind every EDGE body is scheduled under
/// (`resolve_source_type` maps it to `source_type = 'edge_fact'` in
/// `vector_default`). Named so the fix-4 deferral can say WHICH rows it
/// deliberately leaves on the shipped terminal path (see
/// `projection_dispatcher_loop`).
pub(crate) const EDGE_FACT_KIND: &str = "edge_fact";

/// The lifecycle state of outstanding embedding work for this open session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingReadinessState {
    /// No eligible embedding work is outstanding.
    Ready,
    /// A usable embedder is processing eligible work.
    Processing,
    /// Work exists but the session is unavailable for a non-configuration reason.
    Deferred,
    /// Work exists and this session has no configured embedder.
    Blocked,
}

impl EmbeddingReadinessState {
    /// Stable lower-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Processing => "processing",
            Self::Deferred => "deferred",
            Self::Blocked => "blocked",
        }
    }
}

/// The projection operation that needs an embedder.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EmbeddingOperation {
    /// The body of a canonical graph edge projects as `edge_fact`.
    GraphEdgeBodyProjection,
    /// A caller-declared vector projection has outstanding work.
    VectorProjection,
}

impl EmbeddingOperation {
    /// Stable lower-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GraphEdgeBodyProjection => "graph_edge_body_projection",
            Self::VectorProjection => "vector_projection",
        }
    }
}

/// A typed configuration outcome shared by drain errors and readiness reports.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbedderRequired {
    /// Stable error code for all bindings.
    pub code: &'static str,
    /// The blocked embedding operation.
    pub operation: EmbeddingOperation,
    /// Stable state spelling, always `blocked` for this payload.
    pub state: EmbeddingReadinessState,
    /// Ordered, machine-readable corrective actions.
    pub remediations: Vec<&'static str>,
    /// Stable documentation address for this outcome.
    pub documentation_url: &'static str,
}

/// A Rust-owned, pure current report for embedding readiness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EmbeddingReadiness {
    /// `ready`, `processing`, `deferred`, or `blocked`.
    pub state: EmbeddingReadinessState,
    /// Whether this session can use its dense runtime now.
    pub usable_embedder: bool,
    /// Number of eligible rows awaiting embedding.
    pub pending_count: u64,
    /// Sorted projection kinds represented by the pending rows.
    pub affected_kinds: Vec<String>,
    /// Present exactly when `state` is `blocked`.
    pub blocked: Option<EmbedderRequired>,
}

pub(crate) fn embedder_required_for(affected_kinds: &[String]) -> EmbedderRequired {
    let operation = if affected_kinds.iter().any(|kind| kind == EDGE_FACT_KIND) {
        EmbeddingOperation::GraphEdgeBodyProjection
    } else {
        EmbeddingOperation::VectorProjection
    };
    EmbedderRequired {
        code: "FDB_EMBEDDER_REQUIRED",
        operation,
        state: EmbeddingReadinessState::Blocked,
        remediations: vec![
            "configure_default_embedder",
            "configure_caller_embedder",
            "submit_non_embedding_input",
        ],
        documentation_url: "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED",
    }
}

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
    pub(crate) fn usable_dense_runtime(&self) -> bool {
        usable_dense_runtime(
            self.runtime_embedder.as_deref(),
            self.dense_disabled.load(Ordering::Acquire),
        )
    }

    /// 0.7.2 PR-2b — NON-test observation seam. Drains and returns every
    /// `EmbedderEvent` queued since the last drain (mean pin, manual mean
    /// recompute). Production callers use
    /// this to observe the synchronous recompute work; events are queued
    /// only AFTER the recompute transaction is durable, so a rolled-back
    /// recompute never surfaces. Mirrors the at-open
    /// `OpenReport.embedder_events` channel for the steady-state path.
    pub fn drain_embedder_events(&self) -> Result<Vec<EmbedderEvent>, EngineError> {
        self.ensure_open()?;
        let mut events = self
            .projection_runtime
            .shared
            .pending_events
            .lock()
            .map_err(|_| EngineError::Storage)?;
        Ok(std::mem::take(&mut *events))
    }

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

impl Engine {
    /// EU-5b test seam — drain MeanVecPinned events queued by the
    /// projection-commit pin transaction since the last drain. Production
    /// callers consume these via `OpenReport.embedder_events`; this seam
    /// exists so the EU-5b RED test can observe the live emission.
    #[doc(hidden)]
    pub fn drain_mean_centering_events_for_test(&self) -> Result<Vec<EmbedderEvent>, EngineError> {
        self.ensure_open()?;
        let mut events = self
            .projection_runtime
            .shared
            .pending_events
            .lock()
            .map_err(|_| EngineError::Storage)?;
        let out = std::mem::take(&mut *events);
        Ok(out)
    }
}
