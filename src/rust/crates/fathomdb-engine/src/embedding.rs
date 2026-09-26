use super::*;

/// PR-9 — ADR-0.6.0-embedder-protocol **Invariant 5**: run one `embed()`
/// under a per-call deadline. A hung (non-panicking) embed would otherwise
/// park a projection worker forever — the EU-5f `catch_unwind` only catches
/// *panics*. On timeout we return `RuntimeEmbedderError::Timeout`, which the
/// caller's existing retry/failure path already handles.
///
/// Cancellation follows Invariant 5 exactly: the embed runs on a detached
/// thread that is allowed to *finish + discard* its result — never aborted
/// mid-call (there is no safe thread-cancel API). The caller (the projection
/// worker) holds `embed_serialize` across this call, but DROPS it the moment
/// this returns — including on timeout — so the abandoned detached thread
/// runs lock-free and a hung embed can neither hold the serialization guard
/// forever nor deadlock the pool. (The commit happens later, outside this
/// call, under the separate `commit_gate`.)
///
/// Panic-transparent: if `embed()` panics, the panic payload is captured on
/// the watchdog thread and resumed on the worker thread, so the existing
/// batch-level `catch_unwind` records `ProjectionPanic` exactly as before.
///
/// `live` counts embed threads currently alive: incremented before the spawn
/// and decremented by the thread when it finishes (even if its result was
/// abandoned on timeout). The caller reads it to bound the abandoned-thread
/// leak via the circuit breaker.
pub(crate) fn embed_with_watchdog(
    embedder: &Arc<dyn Embedder>,
    body: &str,
    timeout: Duration,
    live: &Arc<AtomicU64>,
) -> Result<Vec<f32>, RuntimeEmbedderError> {
    let (tx, rx) = mpsc::channel();
    let embedder = Arc::clone(embedder);
    let body = body.to_string();
    // Count this embed thread as live before spawning; the thread decrements
    // when it finishes, whether or not its result is still wanted.
    live.fetch_add(1, Ordering::Relaxed);
    let live_thread = Arc::clone(live);
    thread::spawn(move || {
        let outcome =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| embedder.embed(&body)));
        // The receiver may already be gone (this call timed out): an async
        // channel send never blocks, and a send to a dropped receiver is a
        // no-op error we deliberately ignore — the result is discarded.
        let _ = tx.send(outcome);
        live_thread.fetch_sub(1, Ordering::Relaxed);
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(result)) => result,
        Ok(Err(panic_payload)) => std::panic::resume_unwind(panic_payload),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(RuntimeEmbedderError::Timeout),
        // The watchdog thread dropped its sender without sending — should not
        // happen (panics are captured above), but treat as a failed embed so
        // the retry/failure path engages rather than silently succeeding.
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(RuntimeEmbedderError::Failed {
            message: "embed watchdog thread dropped its result channel".to_string(),
        }),
    }
}

/// Batch sibling of [`embed_with_watchdog`]: run ONE `embed_batch` on a detached,
/// timeout-bounded thread. Same Invariant-5 cancellation contract (the thread is
/// allowed to finish + discard on timeout, never aborted mid-call), same
/// panic-transparency (a panic is resumed on the caller so the worker's batch-level
/// `catch_unwind` records `ProjectionPanic`), same `live` accounting (one batch
/// thread = one live embed, bounding the abandoned-thread leak via the breaker).
pub(crate) fn embed_batch_with_watchdog(
    embedder: &Arc<dyn Embedder>,
    bodies: &[String],
    timeout: Duration,
    live: &Arc<AtomicU64>,
) -> Result<Vec<Vec<f32>>, RuntimeEmbedderError> {
    let (tx, rx) = mpsc::channel();
    let embedder = Arc::clone(embedder);
    let bodies = bodies.to_vec();
    live.fetch_add(1, Ordering::Relaxed);
    let live_thread = Arc::clone(live);
    thread::spawn(move || {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let refs: Vec<&str> = bodies.iter().map(String::as_str).collect();
            embedder.embed_batch(&refs)
        }));
        let _ = tx.send(outcome);
        live_thread.fetch_sub(1, Ordering::Relaxed);
    });
    match rx.recv_timeout(timeout) {
        Ok(Ok(result)) => result,
        Ok(Err(panic_payload)) => std::panic::resume_unwind(panic_payload),
        Err(mpsc::RecvTimeoutError::Timeout) => Err(RuntimeEmbedderError::Timeout),
        Err(mpsc::RecvTimeoutError::Disconnected) => Err(RuntimeEmbedderError::Failed {
            message: "embed batch watchdog thread dropped its result channel".to_string(),
        }),
    }
}

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
    /// query-embedding path — a single, direct [`Embedder::embed`] call. The
    /// per-`embed()` watchdog/circuit-breaker guards only the bulk
    /// projection/write path (many embeds, fault isolation), not single
    /// read-side embeds, so a direct call is consistent with how a query is
    /// embedded. Callers get vectors under the engine's *pinned* embedder
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
