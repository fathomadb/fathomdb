use super::*;
use crate::search_types::{ArmDegradation, DegradationReason, SoftFallbackBranch};

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
    /// `ready`, `processing`, or `blocked`.
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
        remediations: vec!["configure_default_embedder", "submit_non_embedding_input"],
        documentation_url: "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED",
    }
}

pub(crate) fn map_runtime_embedder_error(err: RuntimeEmbedderError) -> EngineError {
    match err {
        RuntimeEmbedderError::CudaPoolExhausted { ordinal, max_size_bytes, message } => {
            EngineError::CudaPoolExhausted { ordinal, max_size_bytes, message }
        }
        RuntimeEmbedderError::CudaContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        } => EngineError::CudaContextLost {
            recorded_context_id,
            current_context_id,
            driver_error,
            operation,
        },
        RuntimeEmbedderError::CudaPrivateBuildRefused { ordinal, message } => {
            EngineError::CudaPrivateBuildRefused { ordinal, message }
        }
        RuntimeEmbedderError::Failed { .. } | RuntimeEmbedderError::Timeout => {
            EngineError::Embedder
        }
        // `EmbedderError` is `#[non_exhaustive]`; a kind added after this
        // release keeps the untyped embedder class.
        _ => EngineError::Embedder,
    }
}

/// The query vector for a search. A failed query embedding drops the vector
/// arm (`Ok(None)`, the sparse fallback), except actual engine shutdown and CUDA
/// pool kinds, which are raised typed. Bounded fallback causes are recorded in `degradations`
/// (see [`query_vector_or_degradation`]). A provider panic resumes.
pub(crate) fn search_query_vector(
    dispatcher: &EmbedDispatcher,
    text: &str,
    degradations: &mut Vec<ArmDegradation>,
) -> Result<Option<Vec<f32>>, EngineError> {
    match dispatch_embed_vector(dispatcher, text) {
        Err(DispatchError::Closing) => Err(EngineError::Closing),
        Err(DispatchError::Provider(
            error @ (RuntimeEmbedderError::CudaPoolExhausted { .. }
            | RuntimeEmbedderError::CudaContextLost { .. }
            | RuntimeEmbedderError::CudaPrivateBuildRefused { .. }),
        )) => Err(map_runtime_embedder_error(error)),
        embedded => Ok(query_vector_or_degradation(embedded, degradations)),
    }
}

pub(crate) fn dispatch_embed_vector(
    dispatcher: &EmbedDispatcher,
    text: &str,
) -> Result<Vec<f32>, DispatchError> {
    match dispatcher.submit_text(text.to_owned())?.wait()? {
        EmbedOutput::One(vector) => Ok(vector),
        EmbedOutput::Batch(_) | EmbedOutput::Projection(_) => {
            unreachable!("single embed request returned another output kind")
        }
    }
}

/// The query vector, or `None` with the reason recorded when the embedding was
/// refused for a reason the search reports instead of failing on. Errors other
/// than missing configuration, a dispatcher deadline or queue overload keep
/// the unlabeled sparse fallback; a provider panic resumes on the caller.
pub(crate) fn query_vector_or_degradation(
    embedded: Result<Vec<f32>, DispatchError>,
    degradations: &mut Vec<ArmDegradation>,
) -> Option<Vec<f32>> {
    match embedded {
        Ok(vector) => Some(vector),
        Err(DispatchError::Panic(payload)) => std::panic::resume_unwind(payload),
        Err(DispatchError::NotConfigured) => {
            degradations.push(ArmDegradation::new(
                SoftFallbackBranch::Vector,
                DegradationReason::EmbedderAbsent,
            ));
            None
        }
        Err(DispatchError::StartedTimeout) => {
            degradations.push(ArmDegradation::new(
                SoftFallbackBranch::Vector,
                DegradationReason::EmbedderTimeout,
            ));
            None
        }
        Err(DispatchError::Saturated | DispatchError::QueuedExpired) => {
            degradations.push(ArmDegradation::new(
                SoftFallbackBranch::Vector,
                DegradationReason::EmbedderOverloaded,
            ));
            None
        }
        Err(_) => None,
    }
}

impl Engine {
    pub(crate) fn usable_dense_runtime(&self) -> bool {
        self.runtime_embedder.is_some()
    }

    /// 0.7.2 PR-2b — NON-test observation seam. Drains and returns every
    /// `EmbedderEvent` queued since the last drain (mean pin, manual mean
    /// recompute). Production callers use
    /// this to observe the synchronous recompute work; events are queued
    /// only AFTER the recompute transaction is durable, so a rolled-back
    /// recompute never surfaces. Mirrors the at-open
    /// `OpenReport.embedder_events` channel for the steady-state path.
    pub fn drain_embedder_events(&self) -> Result<Vec<EmbedderEvent>, EngineError> {
        let _operation = self.ensure_open()?;
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
    /// [`EngineError::Embedder`] for a started provider error or timeout. A
    /// provider failure of a CUDA pool kind keeps its typed variant
    /// ([`EngineError::CudaPoolExhausted`], [`EngineError::CudaContextLost`],
    /// [`EngineError::CudaPrivateBuildRefused`]).
    pub fn embed_text(&self, text: &str) -> Result<Vec<f32>, EngineError> {
        let _operation = self.ensure_open()?;
        dispatch_embed_vector(&self.embed_dispatch, text).map_err(|error| match error {
            DispatchError::NotConfigured => EngineError::EmbedderNotConfigured,
            DispatchError::Saturated | DispatchError::QueuedExpired => EngineError::Overloaded,
            DispatchError::Closing | DispatchError::Cancelled => EngineError::Closing,
            DispatchError::Provider(error) => map_runtime_embedder_error(error),
            DispatchError::StartedTimeout | DispatchError::InvalidOutput => EngineError::Embedder,
            DispatchError::Panic(payload) => std::panic::resume_unwind(payload),
        })
    }
}

impl Engine {
    /// EU-5b test seam — drain MeanVecPinned events queued by the
    /// projection-commit pin transaction since the last drain. Production
    /// callers consume these via `OpenReport.embedder_events`; this seam
    /// exists so the EU-5b RED test can observe the live emission.
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn drain_mean_centering_events_for_test(&self) -> Result<Vec<EmbedderEvent>, EngineError> {
        let _operation = self.ensure_open()?;
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

#[cfg(test)]
mod closing_tests {
    use super::*;
    use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{mpsc, Arc, Mutex};
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    const HELD_QUERY: &str = "slice70-held-existing-helper-query";

    #[derive(Debug)]
    struct HeldQueryProvider {
        armed: AtomicBool,
        reached: mpsc::Sender<()>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl Embedder for HeldQueryProvider {
        fn identity(&self) -> EmbedderIdentity {
            EmbedderIdentity::new("s70-actual-close", "r1", 8)
        }
        fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
            if text == HELD_QUERY && self.armed.swap(false, Ordering::SeqCst) {
                let _ = self.reached.send(());
                // A lost test actor cannot leave the provider blocked indefinitely.
                let _ = self.release.lock().unwrap().recv_timeout(Duration::from_secs(5));
            }
            Ok(vec![1.0; 8])
        }
    }

    struct CloseActors {
        release: Option<mpsc::Sender<()>>,
        actors: Vec<JoinHandle<()>>,
    }

    impl CloseActors {
        fn release_and_join(&mut self) -> bool {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
            let mut joined = true;
            for actor in self.actors.drain(..) {
                joined &= actor.join().is_ok();
            }
            joined
        }
    }

    impl Drop for CloseActors {
        fn drop(&mut self) {
            let _ = self.release_and_join();
        }
    }

    #[test]
    fn query_helper_returns_closing_when_its_engine_closes() {
        let dir = tempfile::tempdir().unwrap();
        let (reached_tx, reached_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let provider = Arc::new(HeldQueryProvider {
            armed: AtomicBool::new(false),
            reached: reached_tx,
            release: Mutex::new(release_rx),
        });
        let engine = Arc::new(
            Engine::open_with_test_choice_and_config_for_test(
                dir.path().join("helper-close.sqlite"),
                TestEmbedderChoice::Caller(provider.clone()),
                EngineConfig {
                    embedder_pool_size: Some(1),
                    embedder_call_timeout_ms: Some(5_000),
                    ..EngineConfig::default()
                },
            )
            .unwrap()
            .engine,
        );
        // Install cleanup before arming the provider or starting either actor.
        let mut cleanup = CloseActors { release: Some(release_tx), actors: Vec::new() };
        provider.armed.store(true, Ordering::SeqCst);
        let dispatch = Arc::clone(&engine.embed_dispatch);
        let (helper_tx, helper_rx) = mpsc::channel();
        cleanup.actors.push(thread::spawn(move || {
            let mut degradations = Vec::new();
            let outcome = search_query_vector(&dispatch, HELD_QUERY, &mut degradations);
            let _ = helper_tx.send((outcome, degradations));
        }));
        let reached = reached_rx.recv_timeout(Duration::from_secs(2));
        let close_engine = Arc::clone(&engine);
        let (close_tx, close_rx) = mpsc::channel();
        cleanup.actors.push(thread::spawn(move || {
            let _ = close_tx.send(close_engine.close());
        }));
        let helper_outcome = helper_rx.recv_timeout(Duration::from_secs(2));
        let close_before_release = close_rx.try_recv();
        let close_was_pending = matches!(close_before_release, Err(mpsc::TryRecvError::Empty));
        let joined = cleanup.release_and_join();
        let close_outcome = match close_before_release {
            Ok(result) => Some(result),
            Err(mpsc::TryRecvError::Empty) => close_rx.recv_timeout(Duration::from_secs(2)).ok(),
            Err(mpsc::TryRecvError::Disconnected) => None,
        };
        assert!(joined, "both helper and actual close actors must join");
        assert!(reached.is_ok(), "the helper request must reach the held provider");
        assert!(close_was_pending, "outer close must await the held provider independently");
        assert!(matches!(close_outcome, Some(Ok(()))), "close after release: {close_outcome:?}");
        let (outcome, degradations) =
            helper_outcome.expect("existing helper must wake before its deadline");
        assert!(degradations.is_empty());
        assert!(
            matches!(outcome, Err(EngineError::Closing)),
            "existing helper outcome: {outcome:?}"
        );
    }
}
