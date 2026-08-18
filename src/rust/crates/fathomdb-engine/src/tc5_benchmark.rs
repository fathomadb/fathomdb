//! Narrow TC-5 benchmark-only vector-stage interface.
//!
//! It is feature-gated because it deliberately exposes an internal row-key
//! result to the companion executable, which hashes it before producing any
//! external result artifact. It does not call the ordinary search pipeline.

use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use rusqlite::{params, Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

/// Immutable subset selection derived from the qualified benchmark manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VectorStageScope {
    kind: String,
    selection_digest: String,
}

impl VectorStageScope {
    /// Constructs the only currently supported manifest-derived selection.
    #[must_use]
    pub fn kind(kind: impl Into<String>) -> Self {
        let kind = kind.into();
        let selection_digest = digest_text(&format!("kind:{kind}"));
        Self { kind, selection_digest }
    }

    /// Canonical digest of the fixed selection, safe to attest externally.
    #[must_use]
    pub fn selection_digest(&self) -> &str {
        &self.selection_digest
    }
}

/// Immutable request carried only to the dedicated reader-worker route.
#[derive(Clone, Debug)]
pub struct VectorStageRequest {
    /// Query vector created once by the benchmark-selected embedder.
    pub query_vector: Vec<f32>,
    /// Manifest-allowed bit-KNN candidate count.
    pub candidate_k: usize,
    /// Manifest-allowed exact-f32 result count.
    pub top_k: usize,
    /// Manifest-derived immutable selection.
    pub scope: VectorStageScope,
    /// Manifest-pinned number of vectors in that selection.
    pub expected_vector_rows: usize,
    /// Per-request observer proving the dedicated reader-worker route was used.
    pub route_observer: RouteObserver,
}

/// Stable rejection reasons for benchmark execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VectorStageError {
    /// The manifest-derived request is internally inconsistent.
    InvalidRequest,
    /// The immutable selected population did not match its manifest pin.
    SelectionDrift { expected: usize, observed: usize },
    /// The reader worker was unavailable.
    Closing,
    /// SQLite could not execute a direct vector-stage statement.
    Storage,
    /// Candidate or top-k cardinality was incomplete.
    IncompleteResults,
    /// The request-scoped observer did not observe exactly the direct route.
    ProhibitedRoute,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedRoute {
    VectorStage,
    Search,
    Fts,
    Fusion,
    Graph,
    #[cfg_attr(not(feature = "default-reranker"), allow(dead_code))]
    CrossEncoder,
}

thread_local! {
    static ACTIVE_ROUTE_TRAPS: RefCell<Option<RouteObserver>> = const { RefCell::new(None) };
}

/// Request-scoped, trap-backed route observer for the TC-5 direct path.
#[derive(Clone, Debug, Default)]
pub struct RouteObserver {
    observed: Arc<Mutex<Vec<ObservedRoute>>>,
}

impl RouteObserver {
    /// Creates an empty observer for one vector-stage request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    fn observe_vector_stage(&self) -> Result<(), VectorStageError> {
        self.observe(ObservedRoute::VectorStage)
    }

    fn observe(&self, route: ObservedRoute) -> Result<(), VectorStageError> {
        self.observed.lock().map_err(|_| VectorStageError::ProhibitedRoute)?.push(route);
        Ok(())
    }

    /// Activates the process-local forbidden-route traps for this reader request.
    fn activate_for_request(&self) -> Result<RouteTrapGuard, VectorStageError> {
        ACTIVE_ROUTE_TRAPS.with(|active| {
            let mut active = active.borrow_mut();
            if active.is_some() {
                return Err(VectorStageError::ProhibitedRoute);
            }
            *active = Some(self.clone());
            Ok(RouteTrapGuard)
        })
    }

    fn attestation(&self) -> Result<RouteAttestation, VectorStageError> {
        let observed = self.observed.lock().map_err(|_| VectorStageError::ProhibitedRoute)?;
        let mut routes = RouteAttestation::default();
        for route in observed.iter().copied() {
            match route {
                ObservedRoute::VectorStage => routes.vector_stage += 1,
                ObservedRoute::Search => routes.search += 1,
                ObservedRoute::Fts => routes.fts += 1,
                ObservedRoute::Fusion => routes.fusion += 1,
                ObservedRoute::Graph => routes.graph += 1,
                ObservedRoute::CrossEncoder => routes.cross_encoder += 1,
            }
        }
        if routes.vector_stage != 1
            || routes.search != 0
            || routes.fts != 0
            || routes.fusion != 0
            || routes.graph != 0
            || routes.cross_encoder != 0
        {
            return Err(VectorStageError::ProhibitedRoute);
        }
        Ok(routes)
    }
}

struct RouteTrapGuard;

impl Drop for RouteTrapGuard {
    fn drop(&mut self) {
        ACTIVE_ROUTE_TRAPS.with(|active| *active.borrow_mut() = None);
    }
}

fn record_forbidden_route(route: ObservedRoute) {
    ACTIVE_ROUTE_TRAPS.with(|active| {
        if let Some(observer) = active.borrow().as_ref() {
            let _ = observer.observe(route);
        }
    });
}

/// Records the actual ordinary-search seam if a TC-5 request is active.
pub(crate) fn record_search_route() {
    record_forbidden_route(ObservedRoute::Search);
}

/// Records the actual FTS seam if a TC-5 request is active.
pub(crate) fn record_fts_route() {
    record_forbidden_route(ObservedRoute::Fts);
}

/// Records the actual fusion seam if a TC-5 request is active.
pub(crate) fn record_fusion_route() {
    record_forbidden_route(ObservedRoute::Fusion);
}

/// Records the actual graph seam if a TC-5 request is active.
pub(crate) fn record_graph_route() {
    record_forbidden_route(ObservedRoute::Graph);
}

/// Records the actual cross-encoder seam if a TC-5 request is active.
#[cfg_attr(not(feature = "default-reranker"), allow(dead_code))]
pub(crate) fn record_cross_encoder_route() {
    record_forbidden_route(ObservedRoute::CrossEncoder);
}

/// Route counters make the direct-path boundary inspectable in tests.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RouteAttestation {
    /// Direct vector-stage worker route was entered exactly once.
    pub vector_stage: u8,
    /// Ordinary hybrid search is structurally unreachable from this request.
    pub search: u8,
    /// FTS is structurally unreachable from this request.
    pub fts: u8,
    /// Fusion is structurally unreachable from this request.
    pub fusion: u8,
    /// Graph expansion is structurally unreachable from this request.
    pub graph: u8,
    /// Cross-encoder reranking is structurally unreachable from this request.
    pub cross_encoder: u8,
}

impl RouteAttestation {
    /// Identifies the request-scoped trap-backed observer that produced this evidence.
    #[must_use]
    pub fn observation_source(&self) -> &'static str {
        "request_scoped_trap"
    }
}

/// Materialized, internal vector-stage data for the benchmark executable.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorStageResult {
    /// Bit-KNN candidate row keys, in bit-distance then row-key order.
    pub candidates: Vec<u64>,
    /// Exact-f32 rerank row keys, in f32-distance then row-key order.
    pub rerank: Vec<u64>,
    /// Exact-f32 full-selection truth row keys in the same deterministic order.
    pub ground_truth: Vec<u64>,
    /// Candidate count before top-k truncation.
    pub candidate_count: usize,
    /// Number of vectors in the immutable selection snapshot.
    pub selected_vector_rows: usize,
    /// Safe selection identity to carry into the result envelope.
    pub selection_digest: String,
    /// Candidate bit-KNN duration only.
    pub candidate_elapsed_ns: u128,
    /// Exact-f32 candidate rerank duration only.
    pub rerank_elapsed_ns: u128,
    /// Full-selection exact-f32 truth duration only.
    pub ground_truth_elapsed_ns: u128,
    /// Candidate SQL execution authority.
    pub candidate_execution: &'static str,
    /// Rerank SQL execution authority.
    pub rerank_execution: &'static str,
    /// Direct-path observer evidence.
    pub routes: RouteAttestation,
}

pub(crate) fn read_vector_stage_in_tx(
    reader: &mut Connection,
    request: VectorStageRequest,
) -> Result<VectorStageResult, VectorStageError> {
    if request.query_vector.is_empty()
        || request.top_k == 0
        || request.candidate_k < request.top_k
        || request.candidate_k > request.expected_vector_rows
    {
        return Err(VectorStageError::InvalidRequest);
    }
    let _route_traps = request.route_observer.activate_for_request()?;
    request.route_observer.observe_vector_stage()?;
    let query = serde_json::to_string(&request.query_vector)
        .map_err(|_| VectorStageError::InvalidRequest)?;
    let tx = reader
        .transaction_with_behavior(TransactionBehavior::Deferred)
        .map_err(|_| VectorStageError::Storage)?;
    let observed: usize = tx
        .query_row(
            "SELECT COUNT(*) FROM vector_default WHERE kind = ?1",
            [&request.scope.kind],
            |row| row.get(0),
        )
        .map_err(|_| VectorStageError::Storage)?;
    if observed != request.expected_vector_rows {
        return Err(VectorStageError::SelectionDrift {
            expected: request.expected_vector_rows,
            observed,
        });
    }

    let candidate_started = Instant::now();
    let mut candidate_stmt = tx
        .prepare(
            "WITH bit_candidates AS (\
               SELECT rowid, distance FROM vector_default \
               WHERE embedding_bin MATCH vec_quantize_binary(vec_f32(?1)) AND kind = ?2 \
               ORDER BY distance LIMIT ?3\
             ) SELECT rowid FROM bit_candidates ORDER BY distance, rowid",
        )
        .map_err(|_| VectorStageError::Storage)?;
    let candidates: Vec<u64> = candidate_stmt
        .query_map(params![query, request.scope.kind, request.candidate_k as i64], |row| {
            row.get::<_, i64>(0).map(|id| id as u64)
        })
        .map_err(|_| VectorStageError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| VectorStageError::Storage)?;
    let candidate_elapsed_ns = candidate_started.elapsed().as_nanos();
    if candidates.len() != request.candidate_k {
        return Err(VectorStageError::IncompleteResults);
    }
    drop(candidate_stmt);

    let rerank_started = Instant::now();
    let list = candidates.iter().map(u64::to_string).collect::<Vec<_>>().join(",");
    let rerank_sql = format!(
        "SELECT rowid FROM vector_default WHERE rowid IN ({list}) \
         ORDER BY vec_distance_l2(embedding, vec_f32(?1)), rowid LIMIT ?2"
    );
    let rerank: Vec<u64> = tx
        .prepare(&rerank_sql)
        .map_err(|_| VectorStageError::Storage)?
        .query_map(params![query, request.top_k as i64], |row| {
            row.get::<_, i64>(0).map(|id| id as u64)
        })
        .map_err(|_| VectorStageError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| VectorStageError::Storage)?;
    let rerank_elapsed_ns = rerank_started.elapsed().as_nanos();
    if rerank.len() != request.top_k {
        return Err(VectorStageError::IncompleteResults);
    }

    let truth_started = Instant::now();
    let ground_truth: Vec<u64> = tx
        .prepare(
            "SELECT rowid FROM vector_default WHERE kind = ?1 \
             ORDER BY vec_distance_l2(embedding, vec_f32(?2)), rowid LIMIT ?3",
        )
        .map_err(|_| VectorStageError::Storage)?
        .query_map(params![request.scope.kind, query, request.top_k as i64], |row| {
            row.get::<_, i64>(0).map(|id| id as u64)
        })
        .map_err(|_| VectorStageError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| VectorStageError::Storage)?;
    let ground_truth_elapsed_ns = truth_started.elapsed().as_nanos();
    if ground_truth.len() != request.top_k {
        return Err(VectorStageError::IncompleteResults);
    }
    tx.commit().map_err(|_| VectorStageError::Storage)?;
    Ok(VectorStageResult {
        candidate_count: candidates.len(),
        candidates,
        rerank,
        ground_truth,
        selected_vector_rows: observed,
        selection_digest: request.scope.selection_digest,
        candidate_elapsed_ns,
        rerank_elapsed_ns,
        ground_truth_elapsed_ns,
        candidate_execution: "cpu/sqlite-vec",
        rerank_execution: "cpu/sqlite-vec",
        routes: request.route_observer.attestation()?,
    })
}

fn digest_text(text: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(text.as_bytes());
    digest.finalize().iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod route_trap_tests {
    use super::*;

    #[test]
    fn every_forbidden_route_trap_invalidates_a_tc5_attestation() {
        let traps = [
            record_search_route,
            record_fts_route,
            record_fusion_route,
            record_graph_route,
            record_cross_encoder_route,
        ];
        for trap in traps {
            let observer = RouteObserver::new();
            let _request = observer.activate_for_request().unwrap();
            observer.observe_vector_stage().unwrap();
            trap();
            assert_eq!(observer.attestation(), Err(VectorStageError::ProhibitedRoute));
        }
    }
}
