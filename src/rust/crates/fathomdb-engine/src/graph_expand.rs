use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::fmt::{Display, Formatter};
use std::sync::atomic::Ordering as AtomicOrdering;
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, AtomicU64};
#[cfg(feature = "test-hooks")]
use std::sync::Mutex;

use crate::{
    append_node_eligibility_sql, begin_attributed_reader_tx, compile_text_query, frozen_read,
    projection_generation, structural_dependency_state, Engine, EngineError, FrozenReadContextV1,
    FrozenView, IdSpace, IdSpaceKind, ProjectionGenerationOriginV1, ProjectionReadinessV1,
    ProjectionRuntimeStateV1, ReadContextV1, ReadView, SearchFilter, StructuralDependencyStateV1,
    StructuralLifecycleStateV1, TraversalDirection, WalAttributionCollector,
};
#[cfg(feature = "test-hooks")]
use crate::{dependency_closure, ClosureCauseV1};
#[cfg(feature = "test-hooks")]
use rusqlite::params;
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

const SCHEMA_VERSION: u32 = 1;

fn is_false(value: &bool) -> bool {
    !*value
}

#[cfg(feature = "test-hooks")]
static GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-hooks")]
fn observe_current_rss_peak(test_controls: &GraphExpandReaderControlsForTest) {
    let Some(peak) = test_controls.rss_peak_bytes.as_ref() else { return };
    let observed = crate::process_current_rss_bytes();
    let mut current = peak.load(AtomicOrdering::Relaxed);
    while observed > current {
        match peak.compare_exchange_weak(
            current,
            observed,
            AtomicOrdering::Relaxed,
            AtomicOrdering::Relaxed,
        ) {
            Ok(_) => return,
            Err(next) => current = next,
        }
    }
}

#[cfg(feature = "test-hooks")]
#[derive(Default)]
pub(crate) struct GraphExpandRetentionCountersForTest {
    retained_edge_batch_rows: AtomicU64,
    frontier_states: AtomicU64,
    visited_states: AtomicU64,
    candidate_targets: AtomicU64,
}

#[cfg(feature = "test-hooks")]
fn observe_retained_graph_state(
    test_controls: &GraphExpandReaderControlsForTest,
    retained_edge_batch_rows: usize,
    frontier_states: usize,
    visited_states: usize,
    candidate_targets: usize,
) {
    let Some(counters) = test_controls.retention_counters.as_ref() else { return };
    counters.retained_edge_batch_rows.fetch_max(
        u64::try_from(retained_edge_batch_rows).unwrap_or(u64::MAX),
        AtomicOrdering::Relaxed,
    );
    counters
        .frontier_states
        .fetch_max(u64::try_from(frontier_states).unwrap_or(u64::MAX), AtomicOrdering::Relaxed);
    counters
        .visited_states
        .fetch_max(u64::try_from(visited_states).unwrap_or(u64::MAX), AtomicOrdering::Relaxed);
    counters
        .candidate_targets
        .fetch_max(u64::try_from(candidate_targets).unwrap_or(u64::MAX), AtomicOrdering::Relaxed);
}

/// The source from which graph-expansion seeds were resolved.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphSeedSourceV1 {
    /// Logical nodes selected by indexed full-text ranking.
    Query,
    /// Logical nodes supplied explicitly by the caller.
    Explicit,
}

/// The database read mode used by a graph expansion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphReadModeV1 {
    /// A current reader transaction.
    Current,
    /// An authenticated frozen reader transaction.
    Frozen,
}

/// The serving projection generation's origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProjectionOriginV1 {
    /// No retrieval projection applies to explicit seeding.
    NotApplicable,
    /// A generation minted for a fresh database.
    Fresh,
    /// A legacy generation whose provenance cannot be fully verified.
    LegacyUnverified,
    /// A generation minted by projection configuration.
    Configuration,
    /// A generation minted by a rebuild.
    Rebuild,
}

/// Readiness of the projection generation observed by graph expansion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProjectionReadinessV1 {
    /// No retrieval projection applies to explicit seeding.
    NotApplicable,
    /// Projection work is complete.
    Ready,
    /// Projection work is actively processing.
    Processing,
    /// Projection work is blocked by absent runtime configuration.
    Blocked,
    /// Projection work is deferred by runtime policy.
    Deferred,
    /// Projection work completed with a hard degradation.
    Degraded,
}

/// Stable graph-expansion degradation vocabulary.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GraphExpansionDegradationCodeV1 {
    /// Query seeding deliberately used logical-node FTS without a dense arm.
    QuerySeedTextFallback,
    /// The serving projection has legacy-unverified provenance.
    ProjectionLegacyUnverified,
    /// Projection work is processing.
    ProjectionProcessing,
    /// Projection work is blocked.
    ProjectionBlocked,
    /// Projection work is deferred.
    ProjectionDeferred,
    /// Projection work is degraded.
    ProjectionDegraded,
}

/// Closed seed carrier for graph expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphSeedV1 {
    /// Resolve up to `ranked_limit` logical nodes through native FTS.
    Query { schema_version: u32, text: String, ranked_limit: u32 },
    /// Resolve caller-ordered logical identifiers.
    Explicit { schema_version: u32, logical_ids: Vec<IdSpace> },
}

/// Closed current-or-frozen read context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphReadContextV1 {
    /// Use a newly pinned current reader snapshot.
    Current { schema_version: u32, context: ReadContextV1 },
    /// Use an authenticated frozen reader snapshot.
    Frozen { schema_version: u32, context: FrozenReadContextV1 },
}

/// Versioned, bounded constrained graph-expansion request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphExpandRequestV1 {
    pub schema_version: u32,
    pub seed: GraphSeedV1,
    pub direction: TraversalDirection,
    pub edge_kinds: Vec<String>,
    pub target_kinds: Vec<String>,
    pub context: GraphReadContextV1,
    pub max_depth: u32,
    pub result_limit: u32,
    pub max_work_units: u64,
    pub include_explanation: bool,
    pub include_evidence: bool,
}

/// A logical seed resolved inside the graph-expansion snapshot.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct ResolvedGraphSeedV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub seed_ordinal: u32,
    pub query_score: Option<f64>,
}

/// Compact deterministic origin for one returned target.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphOriginV1 {
    pub schema_version: u32,
    pub seed_logical_id: String,
    pub seed_ordinal: u32,
    pub predecessor_logical_id: String,
    pub target_logical_id: String,
    pub hop_count: u32,
    pub terminal_edge_kind: String,
    pub terminal_direction: TraversalDirection,
}

/// One graph-expansion target with its compact origin.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphTargetV1 {
    pub schema_version: u32,
    pub logical_id: String,
    pub kind: String,
    pub body: String,
    pub write_cursor: u64,
    pub origin: GraphOriginV1,
}

/// Structural explanation for one target.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphTargetExplanationV1 {
    pub schema_version: u32,
    pub target_index: u32,
    pub origin: GraphOriginV1,
    pub lifecycle_state: StructuralLifecycleStateV1,
    pub dependency_state: StructuralDependencyStateV1,
}

/// Optional compact explanation sidecar for graph expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct GraphExpansionExplanationV1 {
    pub schema_version: u32,
    pub correlation_id: String,
    pub seed_source: GraphSeedSourceV1,
    pub read_mode: GraphReadModeV1,
    pub projection_generation_id: Option<String>,
    pub projection_origin: GraphProjectionOriginV1,
    pub projection_readiness: GraphProjectionReadinessV1,
    pub degradation_codes: Vec<GraphExpansionDegradationCodeV1>,
    pub per_target: Vec<GraphTargetExplanationV1>,
}

/// Complete, deterministic one-page graph-expansion result.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct GraphExpandResultV1 {
    pub schema_version: u32,
    pub seeds: Vec<ResolvedGraphSeedV1>,
    pub targets: Vec<GraphTargetV1>,
    pub complete: bool,
    pub work_units: u64,
    pub degradation_codes: Vec<GraphExpansionDegradationCodeV1>,
    pub explanation: Option<GraphExpansionExplanationV1>,
    pub evidence: Option<crate::GraphEvidenceSidecarV1>,
}

/// Closed graph-expansion refusal reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphExpansionErrorReasonV1 {
    UnsupportedSchemaVersion,
    UnknownField,
    GraphSeedInvalid,
    GraphDirectionInvalid,
    GraphEdgeKindsInvalid,
    GraphTargetKindsInvalid,
    GraphContextInvalid,
    GraphDepthInvalid,
    GraphResultLimitInvalid,
    GraphWorkLimitInvalid,
    GraphSeedUnavailable,
    GraphExpansionBoundExceeded,
    GraphProjectionUnavailable,
    GraphCorrupt,
}

impl GraphExpansionErrorReasonV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::GraphSeedInvalid => "graph_seed_invalid",
            Self::GraphDirectionInvalid => "graph_direction_invalid",
            Self::GraphEdgeKindsInvalid => "graph_edge_kinds_invalid",
            Self::GraphTargetKindsInvalid => "graph_target_kinds_invalid",
            Self::GraphContextInvalid => "graph_context_invalid",
            Self::GraphDepthInvalid => "graph_depth_invalid",
            Self::GraphResultLimitInvalid => "graph_result_limit_invalid",
            Self::GraphWorkLimitInvalid => "graph_work_limit_invalid",
            Self::GraphSeedUnavailable => "graph_seed_unavailable",
            Self::GraphExpansionBoundExceeded => "graph_expansion_bound_exceeded",
            Self::GraphProjectionUnavailable => "graph_projection_unavailable",
            Self::GraphCorrupt => "graph_corrupt",
        }
    }
}

/// Typed graph-expansion refusal with an RFC 6901 field path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphExpansionErrorV1 {
    pub schema_version: u32,
    pub reason: GraphExpansionErrorReasonV1,
    pub field_path: String,
}

impl GraphExpansionErrorV1 {
    fn new(reason: GraphExpansionErrorReasonV1, field_path: impl Into<String>) -> Self {
        Self { schema_version: SCHEMA_VERSION, reason, field_path: field_path.into() }
    }
}

impl Display for GraphExpansionErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for GraphExpansionErrorV1 {}

/// Compose graph-expansion degradation codes from the three contract axes.
#[doc(hidden)]
#[must_use]
fn graph_expansion_degradation_codes_impl(
    seed_source: GraphSeedSourceV1,
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
) -> Vec<GraphExpansionDegradationCodeV1> {
    let mut codes = Vec::new();
    if seed_source == GraphSeedSourceV1::Query {
        codes.push(GraphExpansionDegradationCodeV1::QuerySeedTextFallback);
    }
    if origin == GraphProjectionOriginV1::LegacyUnverified {
        codes.push(GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified);
    }
    match readiness {
        GraphProjectionReadinessV1::Processing => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionProcessing);
        }
        GraphProjectionReadinessV1::Blocked => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionBlocked);
        }
        GraphProjectionReadinessV1::Deferred => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionDeferred);
        }
        GraphProjectionReadinessV1::Degraded => {
            codes.push(GraphExpansionDegradationCodeV1::ProjectionDegraded);
        }
        GraphProjectionReadinessV1::NotApplicable | GraphProjectionReadinessV1::Ready => {}
    }
    codes.sort();
    codes.dedup();
    codes
}

fn graph_expansion_degradation_codes(
    seed_source: GraphSeedSourceV1,
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
) -> Vec<GraphExpansionDegradationCodeV1> {
    graph_expansion_degradation_codes_impl(seed_source, origin, readiness)
}

/// Compose graph-expansion degradation codes for test-only matrix coverage.
#[cfg(feature = "test-hooks")]
#[doc(hidden)]
#[must_use]
pub fn graph_expansion_degradation_codes_for_test(
    seed_source: GraphSeedSourceV1,
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
) -> Vec<GraphExpansionDegradationCodeV1> {
    graph_expansion_degradation_codes_impl(seed_source, origin, readiness)
}

#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Eq, PartialEq)]
enum GraphExpandRendezvousPhase {
    BeforePin,
    AfterPin,
}

#[cfg(feature = "test-hooks")]
struct GraphExpandPinRendezvous {
    phase: GraphExpandRendezvousPhase,
    timeout: std::time::Duration,
    entered_send: std::sync::mpsc::SyncSender<()>,
    entered_receive: Mutex<std::sync::mpsc::Receiver<()>>,
    release_send: std::sync::mpsc::SyncSender<()>,
    release_receive: Mutex<std::sync::mpsc::Receiver<()>>,
    released: AtomicBool,
}

/// Owned, request-scoped test rendezvous for graph-expansion transaction seams.
///
/// It is compiled only with `test-hooks`. Each request receives its own handle;
/// the worker and every owner observe one bounded release channel, so a dropped
/// test cannot strand a reader transaction.
#[cfg(feature = "test-hooks")]
#[derive(Clone)]
pub struct GraphExpandRendezvousForTest {
    inner: std::sync::Arc<GraphExpandPinRendezvous>,
}

#[cfg(feature = "test-hooks")]
impl GraphExpandRendezvousForTest {
    fn new(phase: GraphExpandRendezvousPhase, timeout: std::time::Duration) -> Self {
        let (entered_send, entered_receive) = std::sync::mpsc::sync_channel(1);
        let (release_send, release_receive) = std::sync::mpsc::sync_channel(1);
        Self {
            inner: std::sync::Arc::new(GraphExpandPinRendezvous {
                phase,
                timeout,
                entered_send,
                entered_receive: Mutex::new(entered_receive),
                release_send,
                release_receive: Mutex::new(release_receive),
                released: AtomicBool::new(false),
            }),
        }
    }

    /// Pause the owning request immediately before its SQLite transaction pins.
    #[must_use]
    pub fn before_pin(timeout: std::time::Duration) -> Self {
        Self::new(GraphExpandRendezvousPhase::BeforePin, timeout)
    }

    /// Pause the owning request immediately after its SQLite transaction pins.
    #[must_use]
    pub fn after_pin(timeout: std::time::Duration) -> Self {
        Self::new(GraphExpandRendezvousPhase::AfterPin, timeout)
    }

    /// Wait for the owning request to reach its configured transaction seam.
    pub fn wait_until_entered(&self) -> Result<(), String> {
        self.inner
            .entered_receive
            .lock()
            .expect("graph-expand rendezvous entered mutex")
            .recv_timeout(self.inner.timeout)
            .map_err(|error| format!("graph-expand rendezvous did not enter: {error}"))
    }

    /// Idempotently release or disarm the owning request.
    pub fn release(&self) {
        if !self.inner.released.swap(true, AtomicOrdering::SeqCst) {
            let _ = self.inner.release_send.try_send(());
        }
    }

    fn fire(&self, phase: GraphExpandRendezvousPhase) {
        if self.inner.phase != phase || self.inner.released.load(AtomicOrdering::SeqCst) {
            return;
        }
        if self.inner.entered_send.try_send(()).is_err() {
            self.release();
            return;
        }
        let _ = self
            .inner
            .release_receive
            .lock()
            .expect("graph-expand rendezvous release mutex")
            .recv_timeout(self.inner.timeout);
        self.release();
    }
}

#[cfg(feature = "test-hooks")]
impl Drop for GraphExpandRendezvousForTest {
    fn drop(&mut self) {
        self.release();
    }
}

/// Test-only measurement carrier for bounded graph-expansion fixtures.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphExpandMeasurementForTest {
    pub peak_rss_delta_bytes: u64,
}

/// Current process RSS evidence from one isolated graph-expansion fixture arm.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphExpandCurrentRssSampleForTest {
    pub current_rss_delta_bytes: u64,
    pub work_units: u64,
}

/// Live current-RSS peak sampled inside one isolated graph-expansion process.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphExpandIsolatedProcessRssSampleForTest {
    pub process_id: u32,
    pub peak_rss_delta_bytes: u64,
    pub work_units: u64,
    pub retained_edge_batch_rows: u64,
    pub frontier_states: u64,
    pub visited_states: u64,
    pub candidate_targets: u64,
}

/// An owned, request-scoped projection lifecycle observation for real SQLite
/// graph-expansion fixtures.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphExpandProjectionStateForTest {
    origin: GraphProjectionOriginV1,
    readiness: GraphProjectionReadinessV1,
}

/// Source projection status injected before graph-expansion's production mapping.
#[cfg(feature = "test-hooks")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GraphExpandProjectionGenerationForTest {
    origin: ProjectionGenerationOriginV1,
    readiness: ProjectionReadinessV1,
}

#[cfg(feature = "test-hooks")]
#[derive(Default)]
pub(crate) struct GraphExpandReaderControlsForTest {
    pub(crate) rendezvous: Option<GraphExpandRendezvousForTest>,
    pub(crate) projection_state: Option<GraphExpandProjectionStateForTest>,
    pub(crate) projection_generation: Option<GraphExpandProjectionGenerationForTest>,
    pub(crate) rss_peak_bytes: Option<std::sync::Arc<AtomicU64>>,
    pub(crate) retention_counters: Option<std::sync::Arc<GraphExpandRetentionCountersForTest>>,
}

#[cfg(feature = "test-hooks")]
impl GraphExpandProjectionStateForTest {
    #[must_use]
    pub fn fresh_ready() -> Self {
        Self {
            origin: GraphProjectionOriginV1::Fresh,
            readiness: GraphProjectionReadinessV1::Ready,
        }
    }

    #[must_use]
    pub fn projection_legacy_unverified_degraded() -> Self {
        Self {
            origin: GraphProjectionOriginV1::LegacyUnverified,
            readiness: GraphProjectionReadinessV1::Degraded,
        }
    }

    #[must_use]
    pub fn configuration_processing() -> Self {
        Self {
            origin: GraphProjectionOriginV1::Configuration,
            readiness: GraphProjectionReadinessV1::Processing,
        }
    }

    #[must_use]
    pub fn configuration_blocked() -> Self {
        Self {
            origin: GraphProjectionOriginV1::Configuration,
            readiness: GraphProjectionReadinessV1::Blocked,
        }
    }

    #[must_use]
    pub fn rebuild_deferred() -> Self {
        Self {
            origin: GraphProjectionOriginV1::Rebuild,
            readiness: GraphProjectionReadinessV1::Deferred,
        }
    }
}

#[cfg(feature = "test-hooks")]
impl Engine {
    #[doc(hidden)]
    pub fn measure_graph_expand_for_test(&self) -> GraphExpandMeasurementForTest {
        GraphExpandMeasurementForTest {
            peak_rss_delta_bytes: self.graph_expand_rss_delta_bytes.load(AtomicOrdering::Relaxed),
        }
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_dependency_closure_for_test(&self) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let cursor = self.next_cursor.load(AtomicOrdering::SeqCst).saturating_add(1);
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO canonical_nodes(\
                write_cursor, kind, body, source_id, logical_id, row_kind, state, reason, valid_from, valid_until\
             ) VALUES(?1, 'fact', 'derived', 'source-owner', 'derived', 'leaf', 'active', NULL, NULL, NULL)",
            [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_artifact_revisions(\
                schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness\
             ) VALUES(1, 'derived-r1', 'node', ?1, 'derived_semantic', 'complete')",
            [i64::try_from(cursor).map_err(|_| EngineError::Storage)?],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_source_links(\
                schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
                locator_kind, start_byte, end_byte, hash_algorithm, hash_digest\
             ) SELECT schema_version, 'derived-r1', 'source-owner', source_version_id, source_revision_id,\
                      locator_kind, start_byte, end_byte, hash_algorithm, hash_digest \
               FROM _fathomdb_source_links WHERE artifact_revision_id='source-r1'",
            [],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        self.next_cursor.store(cursor, AtomicOrdering::SeqCst);
        Ok(())
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_nonterminal_dependency_closure_for_test(
        &self,
    ) -> Result<(), EngineError> {
        self.ensure_open()?;
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::admit_soft_closure(
            &tx,
            "source-r1",
            ClosureCauseV1::SoftDeleted,
            self.next_cursor.load(AtomicOrdering::SeqCst),
            dependency_closure::SoftClosureMode::Proving,
        )?;
        tx.commit().map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_erasure_for_test(&self) -> Result<(), EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        connection
            .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, i64>(0))
            .map(|_| ())
            .map_err(|_| EngineError::Storage)
    }

    #[doc(hidden)]
    pub fn seed_graph_expand_projection_state_for_test(&self) -> GraphExpandProjectionStateForTest {
        GraphExpandProjectionStateForTest::fresh_ready()
    }

    #[doc(hidden)]
    pub fn graph_expand_current_rss_samples_for_test(
        request: &GraphExpandRequestV1,
        work_units: &[u64],
        unrelated_nodes: &[u64],
    ) -> Result<Vec<GraphExpandCurrentRssSampleForTest>, EngineError> {
        if work_units.len() != unrelated_nodes.len() {
            return Err(EngineError::Storage);
        }
        let sequence = GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
        let root = std::env::temp_dir()
            .join(format!("fathomdb-slice60-rss-{}-{sequence}", std::process::id()));
        std::fs::create_dir_all(&root).map_err(|_| EngineError::Storage)?;
        let mut samples = Vec::with_capacity(work_units.len());
        for (index, (&work, &unrelated)) in work_units.iter().zip(unrelated_nodes).enumerate() {
            let path = root.join(format!("arm-{index}.sqlite"));
            let opened = Engine::open(&path).map_err(|_| EngineError::Storage)?;
            opened.engine.seed_graph_expand_rss_fixture_for_test(work, unrelated)?;
            let baseline = crate::process_current_rss_bytes();
            let result = opened.engine.graph_expand(request)?;
            let observed = crate::process_current_rss_bytes().saturating_sub(baseline);
            samples.push(GraphExpandCurrentRssSampleForTest {
                current_rss_delta_bytes: observed,
                work_units: result.work_units,
            });
            opened.engine.close()?;
        }
        let _ = std::fs::remove_dir_all(root);
        Ok(samples)
    }

    #[doc(hidden)]
    pub fn graph_expand_isolated_process_rss_samples_for_test(
        request: &GraphExpandRequestV1,
        work_units: &[u64],
        unrelated_nodes: &[u64],
    ) -> Result<Vec<GraphExpandIsolatedProcessRssSampleForTest>, EngineError> {
        const CHILD_ARM: &str = "FATHOMDB_SLICE60_FIX5_RSS_CHILD_ARM";
        const SAMPLE_PREFIX: &str = "FATHOMDB_SLICE60_FIX5_RSS_SAMPLE=";

        if work_units.len() != unrelated_nodes.len() {
            return Err(EngineError::Storage);
        }
        if let Ok(index) = std::env::var(CHILD_ARM) {
            let index = index.parse::<usize>().map_err(|_| EngineError::Storage)?;
            let work = *work_units.get(index).ok_or(EngineError::Storage)?;
            let unrelated = *unrelated_nodes.get(index).ok_or(EngineError::Storage)?;
            let sample = Self::measure_isolated_process_rss_arm_for_test(request, work, unrelated)?;
            println!(
                "{SAMPLE_PREFIX}{}:{}:{}:{}:{}:{}:{}",
                sample.process_id,
                sample.peak_rss_delta_bytes,
                sample.work_units,
                sample.retained_edge_batch_rows,
                sample.frontier_states,
                sample.visited_states,
                sample.candidate_targets,
            );
            std::process::exit(0);
        }

        let executable = std::env::current_exe().map_err(|_| EngineError::Storage)?;
        let mut samples = Vec::with_capacity(work_units.len());
        for index in 0..work_units.len() {
            let output = std::process::Command::new(&executable)
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env(CHILD_ARM, index.to_string())
                .output()
                .map_err(|_| EngineError::Storage)?;
            if !output.status.success() {
                return Err(EngineError::Storage);
            }
            let stdout = String::from_utf8(output.stdout).map_err(|_| EngineError::Storage)?;
            let fields = stdout
                .lines()
                .find_map(|line| line.split_once(SAMPLE_PREFIX).map(|(_, sample)| sample))
                .ok_or(EngineError::Storage)?
                .split(':')
                .collect::<Vec<_>>();
            let [process_id, peak_rss_delta_bytes, work_units, retained_edge_batch_rows, frontier_states, visited_states, candidate_targets] =
                fields.as_slice()
            else {
                return Err(EngineError::Storage);
            };
            samples.push(GraphExpandIsolatedProcessRssSampleForTest {
                process_id: process_id.parse().map_err(|_| EngineError::Storage)?,
                peak_rss_delta_bytes: peak_rss_delta_bytes
                    .parse()
                    .map_err(|_| EngineError::Storage)?,
                work_units: work_units.parse().map_err(|_| EngineError::Storage)?,
                retained_edge_batch_rows: retained_edge_batch_rows
                    .parse()
                    .map_err(|_| EngineError::Storage)?,
                frontier_states: frontier_states.parse().map_err(|_| EngineError::Storage)?,
                visited_states: visited_states.parse().map_err(|_| EngineError::Storage)?,
                candidate_targets: candidate_targets.parse().map_err(|_| EngineError::Storage)?,
            });
        }
        Ok(samples)
    }

    fn measure_isolated_process_rss_arm_for_test(
        request: &GraphExpandRequestV1,
        work: u64,
        unrelated: u64,
    ) -> Result<GraphExpandIsolatedProcessRssSampleForTest, EngineError> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (request, work, unrelated);
            return Err(EngineError::Storage);
        }
        #[cfg(target_os = "linux")]
        {
            let sequence = GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
            let root = std::env::temp_dir()
                .join(format!("fathomdb-slice60-rss-child-{}-{sequence}", std::process::id()));
            std::fs::create_dir_all(&root).map_err(|_| EngineError::Storage)?;
            let path = root.join("arm.sqlite");
            let opened = Engine::open(&path).map_err(|_| EngineError::Storage)?;
            opened.engine.seed_graph_expand_rss_fixture_for_test(work, unrelated)?;
            let baseline = crate::process_current_rss_bytes();
            if baseline == 0 {
                return Err(EngineError::Storage);
            }
            let rss_peak = std::sync::Arc::new(AtomicU64::new(baseline));
            let retention_counters =
                std::sync::Arc::new(GraphExpandRetentionCountersForTest::default());
            let result = opened.engine.graph_expand_inner(
                request,
                GraphExpandReaderControlsForTest {
                    rss_peak_bytes: Some(std::sync::Arc::clone(&rss_peak)),
                    retention_counters: Some(std::sync::Arc::clone(&retention_counters)),
                    ..GraphExpandReaderControlsForTest::default()
                },
            )?;
            let observed = crate::process_current_rss_bytes();
            let mut peak = rss_peak.load(AtomicOrdering::Relaxed);
            while observed > peak {
                match rss_peak.compare_exchange_weak(
                    peak,
                    observed,
                    AtomicOrdering::Relaxed,
                    AtomicOrdering::Relaxed,
                ) {
                    Ok(_) => break,
                    Err(next) => peak = next,
                }
            }
            opened.engine.close()?;
            let _ = std::fs::remove_dir_all(root);
            Ok(GraphExpandIsolatedProcessRssSampleForTest {
                process_id: std::process::id(),
                peak_rss_delta_bytes: peak.saturating_sub(baseline),
                work_units: result.work_units,
                retained_edge_batch_rows: retention_counters
                    .retained_edge_batch_rows
                    .load(AtomicOrdering::Relaxed),
                frontier_states: retention_counters.frontier_states.load(AtomicOrdering::Relaxed),
                visited_states: retention_counters.visited_states.load(AtomicOrdering::Relaxed),
                candidate_targets: retention_counters
                    .candidate_targets
                    .load(AtomicOrdering::Relaxed),
            })
        }
    }

    fn seed_graph_expand_rss_fixture_for_test(
        &self,
        work: u64,
        unrelated: u64,
    ) -> Result<(), EngineError> {
        use crate::{InitialState, PreparedWrite, SourceId};

        {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            connection
                .execute_batch("PRAGMA default_cache_size=64")
                .map_err(|_| EngineError::Storage)?;
        }
        self.write(&[PreparedWrite::Node {
            logical_id: Some("root".into()),
            kind: "fact".into(),
            body: "root".into(),
            source_id: SourceId::new("slice60-rss").map_err(|_| EngineError::Storage)?,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])?;
        let mut writes = Vec::new();
        for index in 0..work {
            writes.push(PreparedWrite::Node {
                logical_id: Some(format!("target-{index}")),
                kind: "fact".into(),
                body: "target".into(),
                source_id: SourceId::new("slice60-rss").map_err(|_| EngineError::Storage)?,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            });
            writes.push(PreparedWrite::Edge {
                logical_id: Some(format!("edge-{index}")),
                kind: "link".into(),
                from: "root".into(),
                to: format!("target-{index}"),
                source_id: SourceId::new("slice60-rss").map_err(|_| EngineError::Storage)?,
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            });
        }
        if !writes.is_empty() {
            self.write(&writes)?;
        }
        if unrelated > 0 {
            let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_mut().ok_or(EngineError::Closing)?;
            let first = self.next_cursor.load(AtomicOrdering::SeqCst).saturating_add(1);
            let tx = connection
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(|_| EngineError::Storage)?;
            let mut insert = tx
                .prepare("INSERT INTO canonical_nodes(\
                    write_cursor,kind,body,source_id,logical_id,row_kind,state,reason,valid_from,valid_until\
                 ) VALUES(?1,'fact','unrelated','slice60-rss',?2,'leaf','active',NULL,NULL,NULL)")
                .map_err(|_| EngineError::Storage)?;
            for index in 0..unrelated {
                let cursor = first.saturating_add(index);
                insert
                    .execute(params![
                        i64::try_from(cursor).map_err(|_| EngineError::Storage)?,
                        format!("unrelated-{index}")
                    ])
                    .map_err(|_| EngineError::Storage)?;
            }
            drop(insert);
            tx.commit().map_err(|_| EngineError::Storage)?;
            self.next_cursor
                .store(first.saturating_add(unrelated).saturating_sub(1), AtomicOrdering::SeqCst);
        }
        Ok(())
    }
}

fn graph_error(reason: GraphExpansionErrorReasonV1, path: impl Into<String>) -> EngineError {
    GraphExpansionErrorV1::new(reason, path).into()
}

fn validate_kind_list(
    values: &[String],
    reason: GraphExpansionErrorReasonV1,
    base: &str,
) -> Result<(), EngineError> {
    if values.len() > 32 {
        return Err(graph_error(reason, base));
    }
    let mut seen = HashSet::new();
    for (index, value) in values.iter().enumerate() {
        if value.trim().is_empty() || !seen.insert(value) {
            return Err(graph_error(reason, format!("{base}/{index}")));
        }
    }
    Ok(())
}

fn validate_semantics(request: &GraphExpandRequestV1) -> Result<(), EngineError> {
    if request.schema_version != SCHEMA_VERSION {
        return Err(graph_error(
            GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ));
    }
    match &request.seed {
        GraphSeedV1::Query { schema_version, text, ranked_limit } => {
            if *schema_version != SCHEMA_VERSION {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                    "/seed/schemaVersion",
                ));
            }
            if text.trim().is_empty() {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/text",
                ));
            }
            if !(1..=25).contains(ranked_limit) {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/rankedLimit",
                ));
            }
        }
        GraphSeedV1::Explicit { schema_version, logical_ids } => {
            if *schema_version != SCHEMA_VERSION {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                    "/seed/schemaVersion",
                ));
            }
            if logical_ids.is_empty() || logical_ids.len() > 25 {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/logicalIds",
                ));
            }
            let mut seen = HashSet::new();
            for (index, id) in logical_ids.iter().enumerate() {
                if id.space != IdSpaceKind::Logical {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        format!("/seed/logicalIds/{index}/space"),
                    ));
                }
                if id.value.is_empty() || id.value.contains('\u{1e}') {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        format!("/seed/logicalIds/{index}/value"),
                    ));
                }
                if !seen.insert(&id.value) {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        format!("/seed/logicalIds/{index}"),
                    ));
                }
            }
        }
    }
    validate_kind_list(
        &request.edge_kinds,
        GraphExpansionErrorReasonV1::GraphEdgeKindsInvalid,
        "/edgeKinds",
    )?;
    validate_kind_list(
        &request.target_kinds,
        GraphExpansionErrorReasonV1::GraphTargetKindsInvalid,
        "/targetKinds",
    )?;
    if request.max_depth > 3 {
        return Err(graph_error(GraphExpansionErrorReasonV1::GraphDepthInvalid, "/maxDepth"));
    }
    if !(1..=50).contains(&request.result_limit) {
        return Err(graph_error(
            GraphExpansionErrorReasonV1::GraphResultLimitInvalid,
            "/resultLimit",
        ));
    }
    if !(1..=10_000).contains(&request.max_work_units) {
        return Err(graph_error(
            GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
            "/maxWorkUnits",
        ));
    }
    Ok(())
}

impl Engine {
    /// Expand caller-supplied or FTS-resolved logical seeds through one bounded graph walk.
    pub fn graph_expand(
        &self,
        request: &GraphExpandRequestV1,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            #[cfg(feature = "test-hooks")]
            GraphExpandReaderControlsForTest::default(),
        )
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn graph_expand_with_rendezvous_for_test(
        &self,
        request: &GraphExpandRequestV1,
        rendezvous: GraphExpandRendezvousForTest,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                rendezvous: Some(rendezvous),
                ..GraphExpandReaderControlsForTest::default()
            },
        )
    }

    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn graph_expand_with_projection_state_for_test(
        &self,
        request: &GraphExpandRequestV1,
        projection_state: GraphExpandProjectionStateForTest,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                projection_state: Some(projection_state),
                ..GraphExpandReaderControlsForTest::default()
            },
        )
    }

    #[doc(hidden)]
    #[cfg(feature = "test-hooks")]
    pub fn graph_expand_with_projection_generation_for_test(
        &self,
        request: &GraphExpandRequestV1,
        origin: ProjectionGenerationOriginV1,
        readiness: ProjectionReadinessV1,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                projection_generation: Some(GraphExpandProjectionGenerationForTest {
                    origin,
                    readiness,
                }),
                ..GraphExpandReaderControlsForTest::default()
            },
        )
    }

    pub(crate) fn graph_expand_inner(
        &self,
        request: &GraphExpandRequestV1,
        #[cfg(feature = "test-hooks")] test_controls: GraphExpandReaderControlsForTest,
    ) -> Result<GraphExpandResultV1, EngineError> {
        self.ensure_open()?;
        if request.include_evidence && matches!(request.context, GraphReadContextV1::Current { .. })
        {
            return Err(graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context"));
        }
        let frozen_binding = match &request.context {
            GraphReadContextV1::Current { schema_version, context } => {
                if *schema_version != SCHEMA_VERSION {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                        "/context/schemaVersion",
                    ));
                }
                frozen_read::validate_context(context).map_err(|_| {
                    graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")
                })?;
                None
            }
            GraphReadContextV1::Frozen { schema_version, context } => {
                if *schema_version != SCHEMA_VERSION {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                        "/context/schemaVersion",
                    ));
                }
                let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                Some(Box::new(frozen_read::authenticate(connection, context)?))
            }
        };
        let context = match &request.context {
            GraphReadContextV1::Current { context, .. } => context,
            GraphReadContextV1::Frozen { context, .. } => &context.context,
        };
        if context.view.include_superseded || context.view.include_inactive {
            return Err(graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context"));
        }
        validate_semantics(request)?;
        let projection_runtime_state = if self.runtime_embedder.is_none() {
            ProjectionRuntimeStateV1::Absent
        } else if self.dense_disabled.load(AtomicOrdering::Acquire) {
            ProjectionRuntimeStateV1::Refused
        } else {
            ProjectionRuntimeStateV1::Usable
        };
        #[cfg(feature = "test-hooks")]
        self.graph_expand_rss_baseline_bytes
            .store(crate::process_current_rss_bytes(), AtomicOrdering::Relaxed);
        let evidence_authority = match (&request.context, request.include_evidence) {
            (GraphReadContextV1::Frozen { context, .. }, true) => {
                let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                Some(crate::evidence::graph_evidence_authority(connection, context)?)
            }
            _ => None,
        };
        let (respond, receive) = std::sync::mpsc::sync_channel(1);
        self.reader_pool
            .dispatch(crate::ReaderRequest::GraphExpand(Box::new(
                crate::GraphExpandReaderRequest {
                    request: request.clone(),
                    frozen_binding,
                    projection_runtime_state,
                    evidence_authority,
                    #[cfg(feature = "test-hooks")]
                    test_controls,
                    respond,
                },
            )))
            .map_err(|_| EngineError::Closing)?;
        let mut result = receive.recv().map_err(|_| EngineError::Storage)??;
        #[cfg(feature = "test-hooks")]
        {
            let baseline = self.graph_expand_rss_baseline_bytes.load(AtomicOrdering::Relaxed);
            let observed = crate::process_current_rss_bytes().saturating_sub(baseline);
            self.graph_expand_rss_delta_bytes.store(observed, AtomicOrdering::Relaxed);
        }
        if request.include_explanation {
            let sequence = self.explanation_sequence.fetch_add(1, AtomicOrdering::Relaxed);
            let correlation_id = format!("x{:032x}-{sequence}", self.explanation_open_nonce);
            if let Some(explanation) = result.explanation.as_mut() {
                explanation.correlation_id = correlation_id;
            }
        }
        Ok(result)
    }

    /// Return endpoint-index query plans used by graph expansion.
    #[doc(hidden)]
    pub fn explain_graph_expand_for_test(
        &self,
        direction: TraversalDirection,
    ) -> Result<Vec<String>, EngineError> {
        self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let query = graph_expand_incident_query("fixture", direction, 1);
        explain_graph_expand_incident_query(connection, &query)
    }
}

#[derive(Clone)]
struct EdgeRow {
    write_cursor: u64,
    kind: String,
    terminal_direction: TraversalDirection,
    next_logical_id: String,
    logical_id: Option<String>,
    superseded_at: Option<i64>,
    t_invalid: Option<i64>,
}

struct GraphCandidate {
    target: GraphTargetV1,
    terminal_edge_cursor: u64,
}

fn load_node(
    tx: &Connection,
    logical_id: &str,
    view: FrozenView,
    filter: &SearchFilter,
) -> Result<Option<(String, String, u64)>, EngineError> {
    let mut params = vec![rusqlite::types::Value::Text(logical_id.to_string())];
    if let Some(now) = view.now_param() {
        params.push(rusqlite::types::Value::Integer(now));
    }
    let eligibility = append_node_eligibility_sql(Some(filter), "n", &mut params);
    let sql = format!(
        "SELECT n.kind,n.body,n.write_cursor FROM canonical_nodes n WHERE n.logical_id=?1{}{} ORDER BY n.write_cursor DESC LIMIT 1",
        view.node_sql("n", 2), eligibility
    );
    tx.query_row(&sql, rusqlite::params_from_iter(params.iter()), |row| {
        Ok((row.get(0)?, row.get(1)?, u64::try_from(row.get::<_, i64>(2)?).unwrap_or(0)))
    })
    .optional()
    .map_err(|_| EngineError::Storage)
}

struct GraphExpandIncidentQuery {
    sql: String,
    logical_id: String,
    limit: i64,
}

fn graph_expand_incident_sql(direction: TraversalDirection) -> String {
    let predicate = match direction {
        TraversalDirection::Outgoing => "from_id=?1",
        TraversalDirection::Incoming => "to_id=?1",
        TraversalDirection::Both => "(from_id=?1 OR to_id=?1)",
    };
    format!("SELECT write_cursor,kind,from_id,to_id,logical_id,superseded_at,t_invalid FROM canonical_edges WHERE {predicate} LIMIT ?2")
}

fn graph_expand_incident_query(
    logical_id: &str,
    direction: TraversalDirection,
    limit: u64,
) -> GraphExpandIncidentQuery {
    GraphExpandIncidentQuery {
        sql: graph_expand_incident_sql(direction),
        logical_id: logical_id.to_string(),
        limit: i64::try_from(limit).unwrap_or(i64::MAX),
    }
}

fn explain_graph_expand_incident_query(
    connection: &Connection,
    query: &GraphExpandIncidentQuery,
) -> Result<Vec<String>, EngineError> {
    let sql = &query.sql;
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(rusqlite::params![query.logical_id, query.limit], |row| row.get(3))
        .map_err(|_| EngineError::Storage)?;
    rows.collect::<rusqlite::Result<Vec<String>>>().map_err(|_| EngineError::Storage)
}

fn load_incident_edges(
    tx: &Connection,
    logical_id: &str,
    direction: TraversalDirection,
    limit: u64,
) -> Result<Vec<EdgeRow>, EngineError> {
    let query = graph_expand_incident_query(logical_id, direction, limit);
    let mut statement = tx.prepare(&query.sql).map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(rusqlite::params![query.logical_id, query.limit], |row| {
            let from_id: String = row.get(2)?;
            let to_id: String = row.get(3)?;
            let (terminal_direction, next_logical_id) = if from_id == logical_id {
                (TraversalDirection::Outgoing, to_id)
            } else {
                (TraversalDirection::Incoming, from_id)
            };
            Ok(EdgeRow {
                write_cursor: u64::try_from(row.get::<_, i64>(0)?).unwrap_or(0),
                kind: row.get(1)?,
                terminal_direction,
                next_logical_id,
                logical_id: row.get(4)?,
                superseded_at: row.get(5)?,
                t_invalid: row.get(6)?,
            })
        })
        .map_err(|_| EngineError::Storage)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)
}

fn direction_and_next(edge: &EdgeRow) -> (TraversalDirection, &str) {
    (edge.terminal_direction, &edge.next_logical_id)
}

fn origin_cmp(left: &GraphOriginV1, right: &GraphOriginV1) -> Ordering {
    let direction_rank = |value| match value {
        TraversalDirection::Outgoing => 0_u8,
        TraversalDirection::Incoming => 1,
        TraversalDirection::Both => 2,
    };
    (
        left.hop_count,
        left.seed_ordinal,
        left.predecessor_logical_id.as_bytes(),
        direction_rank(left.terminal_direction),
        left.terminal_edge_kind.as_bytes(),
        left.target_logical_id.as_bytes(),
    )
        .cmp(&(
            right.hop_count,
            right.seed_ordinal,
            right.predecessor_logical_id.as_bytes(),
            direction_rank(right.terminal_direction),
            right.terminal_edge_kind.as_bytes(),
            right.target_logical_id.as_bytes(),
        ))
}

fn map_projection_origin(origin: ProjectionGenerationOriginV1) -> GraphProjectionOriginV1 {
    match origin {
        ProjectionGenerationOriginV1::Fresh => GraphProjectionOriginV1::Fresh,
        ProjectionGenerationOriginV1::LegacyUnverified => GraphProjectionOriginV1::LegacyUnverified,
        ProjectionGenerationOriginV1::Configuration => GraphProjectionOriginV1::Configuration,
        ProjectionGenerationOriginV1::Rebuild => GraphProjectionOriginV1::Rebuild,
    }
}

fn map_projection_readiness(readiness: ProjectionReadinessV1) -> GraphProjectionReadinessV1 {
    match readiness {
        ProjectionReadinessV1::Ready => GraphProjectionReadinessV1::Ready,
        ProjectionReadinessV1::Processing => GraphProjectionReadinessV1::Processing,
        ProjectionReadinessV1::Blocked => GraphProjectionReadinessV1::Blocked,
        ProjectionReadinessV1::Deferred => GraphProjectionReadinessV1::Deferred,
        ProjectionReadinessV1::Degraded => GraphProjectionReadinessV1::Degraded,
    }
}

fn resolve_seeds(
    tx: &Connection,
    request: &GraphExpandRequestV1,
    view: FrozenView,
    filter: &SearchFilter,
) -> Result<Vec<ResolvedGraphSeedV1>, EngineError> {
    match &request.seed {
        GraphSeedV1::Explicit { logical_ids, .. } => logical_ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                if load_node(tx, &id.value, view, filter)?.is_none() {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::GraphSeedUnavailable,
                        format!("/seed/logicalIds/{index}"),
                    ));
                }
                Ok(ResolvedGraphSeedV1 {
                    schema_version: 1,
                    logical_id: id.value.clone(),
                    seed_ordinal: u32::try_from(index).unwrap_or(u32::MAX),
                    query_score: None,
                })
            })
            .collect(),
        GraphSeedV1::Query { text, ranked_limit, .. } => {
            let compiled = compile_text_query(text);
            let mut params = vec![rusqlite::types::Value::Text(compiled.match_expression)];
            if let Some(now) = view.now_param() {
                params.push(rusqlite::types::Value::Integer(now));
            }
            let eligibility = append_node_eligibility_sql(Some(filter), "cn", &mut params);
            params.push(rusqlite::types::Value::Integer(i64::from(*ranked_limit)));
            let limit_index = params.len();
            let sql = format!(
                "SELECT cn.logical_id,bm25(search_index) FROM search_index JOIN canonical_nodes cn ON cn.write_cursor=search_index.write_cursor WHERE search_index MATCH ?1 AND cn.logical_id IS NOT NULL{}{}{} ORDER BY bm25(search_index),search_index.write_cursor LIMIT ?{limit_index}",
                view.node_sql("cn", 2), eligibility, ""
            );
            let mut statement = tx.prepare(&sql).map_err(|_| {
                graph_error(GraphExpansionErrorReasonV1::GraphProjectionUnavailable, "/projection")
            })?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok((row.get::<_, String>(0)?, -row.get::<_, f64>(1)?))
                })
                .map_err(|_| {
                    graph_error(
                        GraphExpansionErrorReasonV1::GraphProjectionUnavailable,
                        "/projection",
                    )
                })?;
            let mut seen = HashSet::new();
            let mut seeds = Vec::new();
            for row in rows {
                let (logical_id, score) = row.map_err(|_| EngineError::Storage)?;
                if seen.insert(logical_id.clone()) {
                    seeds.push(ResolvedGraphSeedV1 {
                        schema_version: 1,
                        logical_id,
                        seed_ordinal: u32::try_from(seeds.len()).unwrap_or(u32::MAX),
                        query_score: Some(score),
                    });
                }
            }
            Ok(seeds)
        }
    }
}

pub(crate) fn read_graph_expand_in_tx(
    reader: &mut Connection,
    request: &GraphExpandRequestV1,
    frozen_binding: Option<&frozen_read::FrozenReadBinding>,
    projection_runtime_state: ProjectionRuntimeStateV1,
    evidence_authority: Option<&crate::evidence::GraphEvidenceAuthority>,
    #[cfg(feature = "test-hooks")] test_controls: &GraphExpandReaderControlsForTest,
    attribution: &std::sync::Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<GraphExpandResultV1, EngineError> {
    #[cfg(feature = "test-hooks")]
    if let Some(rendezvous) = test_controls.rendezvous.as_ref() {
        rendezvous.fire(GraphExpandRendezvousPhase::BeforePin);
    }
    let tx = begin_attributed_reader_tx(reader, attribution, worker_idx)
        .map_err(|_| EngineError::Storage)?;
    #[cfg(feature = "test-hooks")]
    observe_current_rss_peak(test_controls);
    if let Some(binding) = frozen_binding {
        frozen_read::validate_snapshot(&tx, binding)?;
    } else {
        tx.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, i64>(0))
            .map_err(|_| EngineError::Storage)?;
    }
    #[cfg(feature = "test-hooks")]
    if let Some(rendezvous) = test_controls.rendezvous.as_ref() {
        rendezvous.fire(GraphExpandRendezvousPhase::AfterPin);
    }

    let (context, read_mode) = match &request.context {
        GraphReadContextV1::Current { context, .. } => (context, GraphReadModeV1::Current),
        GraphReadContextV1::Frozen { context, .. } => (&context.context, GraphReadModeV1::Frozen),
    };
    validate_filter_attributes_on_snapshot(&tx, &context.eligibility).map_err(
        |error| match error {
            crate::SearchReaderError::InvalidFilter(_) => {
                graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")
            }
            _ => EngineError::Storage,
        },
    )?;
    let view = context.view.freeze();
    let seeds = resolve_seeds(&tx, request, view, &context.eligibility)?;
    let seed_source = match request.seed {
        GraphSeedV1::Query { .. } => GraphSeedSourceV1::Query,
        GraphSeedV1::Explicit { .. } => GraphSeedSourceV1::Explicit,
    };
    let (generation_id, projection_origin, projection_readiness) = if seed_source
        == GraphSeedSourceV1::Query
    {
        let status = projection_generation::status_in_snapshot(
            &tx,
            projection_runtime_state,
            view.edge_now(),
            crate::load_next_cursor(&tx),
        )?;
        {
            let (source_origin, source_readiness) = {
                #[cfg(feature = "test-hooks")]
                if let Some(state) = test_controls.projection_generation {
                    (state.origin, state.readiness)
                } else {
                    (status.origin, status.readiness)
                }
                #[cfg(not(feature = "test-hooks"))]
                (status.origin, status.readiness)
            };
            let (origin, readiness) = {
                #[cfg(feature = "test-hooks")]
                if let Some(state) = test_controls.projection_state {
                    (state.origin, state.readiness)
                } else {
                    (
                        map_projection_origin(source_origin),
                        map_projection_readiness(source_readiness),
                    )
                }
                #[cfg(not(feature = "test-hooks"))]
                (map_projection_origin(source_origin), map_projection_readiness(source_readiness))
            };
            (Some(status.generation_id.as_str().to_string()), origin, readiness)
        }
    } else {
        (None, GraphProjectionOriginV1::NotApplicable, GraphProjectionReadinessV1::NotApplicable)
    };
    let degradation_codes =
        graph_expansion_degradation_codes(seed_source, projection_origin, projection_readiness);

    let all_seed_ids = seeds.iter().map(|seed| seed.logical_id.clone()).collect::<HashSet<_>>();
    let mut work_units = 0_u64;
    let mut candidates: HashMap<String, GraphCandidate> = HashMap::new();
    let mut visited_by_seed =
        seeds.iter().map(|seed| HashSet::from([seed.logical_id.clone()])).collect::<Vec<_>>();
    let mut frontier = seeds
        .iter()
        .enumerate()
        .map(|(index, seed)| (index, seed.logical_id.clone()))
        .collect::<Vec<_>>();
    for depth in 0..request.max_depth {
        frontier.sort_by(|left, right| {
            left.0.cmp(&right.0).then_with(|| left.1.as_bytes().cmp(right.1.as_bytes()))
        });
        #[cfg(feature = "test-hooks")]
        let current_frontier_states = frontier.len();
        let mut next_frontier = Vec::new();
        #[cfg(feature = "test-hooks")]
        observe_retained_graph_state(
            test_controls,
            0,
            current_frontier_states,
            visited_by_seed.iter().map(HashSet::len).sum(),
            candidates.len(),
        );
        for (seed_index, current) in frontier {
            let seed = &seeds[seed_index];
            let remaining = request.max_work_units.saturating_sub(work_units);
            let mut edges =
                load_incident_edges(&tx, &current, request.direction, remaining.saturating_add(1))?;
            #[cfg(feature = "test-hooks")]
            observe_current_rss_peak(test_controls);
            #[cfg(feature = "test-hooks")]
            observe_retained_graph_state(
                test_controls,
                edges.len(),
                current_frontier_states.saturating_add(next_frontier.len()),
                visited_by_seed.iter().map(HashSet::len).sum(),
                candidates.len(),
            );
            if u64::try_from(edges.len()).unwrap_or(u64::MAX) > remaining {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::GraphExpansionBoundExceeded,
                    "/maxWorkUnits",
                ));
            }
            edges.sort_by(|left, right| {
                let (left_direction, left_next) = direction_and_next(left);
                let (right_direction, right_next) = direction_and_next(right);
                let rank = |direction| match direction {
                    TraversalDirection::Outgoing => 0_u8,
                    TraversalDirection::Incoming => 1,
                    TraversalDirection::Both => 2,
                };
                (
                    rank(left_direction),
                    left.kind.as_bytes(),
                    left_next.as_bytes(),
                    left.logical_id.as_deref().unwrap_or("").as_bytes(),
                    left.write_cursor,
                )
                    .cmp(&(
                        rank(right_direction),
                        right.kind.as_bytes(),
                        right_next.as_bytes(),
                        right.logical_id.as_deref().unwrap_or("").as_bytes(),
                        right.write_cursor,
                    ))
            });
            #[cfg(feature = "test-hooks")]
            observe_current_rss_peak(test_controls);
            #[cfg(feature = "test-hooks")]
            observe_retained_graph_state(
                test_controls,
                edges.len(),
                current_frontier_states.saturating_add(next_frontier.len()),
                visited_by_seed.iter().map(HashSet::len).sum(),
                candidates.len(),
            );
            for edge in edges {
                work_units += 1;
                if !request.edge_kinds.is_empty() && !request.edge_kinds.contains(&edge.kind) {
                    continue;
                }
                if edge.superseded_at.is_some()
                    || edge.t_invalid.is_some_and(|invalid| invalid <= view.edge_now())
                {
                    continue;
                }
                let (terminal_direction, next) = direction_and_next(&edge);
                if depth + 1 < request.max_depth
                    && !visited_by_seed[seed_index].insert(next.to_string())
                {
                    continue;
                }
                // Structural traversal order matches the output origin order. At the terminal
                // depth, later edges cannot displace a full candidate set.
                if depth + 1 == request.max_depth
                    && candidates.len() >= request.result_limit as usize
                {
                    continue;
                }
                let Some((kind, body, write_cursor)) =
                    load_node(&tx, next, view, &context.eligibility)?
                else {
                    continue;
                };
                if depth < request.max_depth {
                    next_frontier.push((seed_index, next.to_string()));
                }
                if all_seed_ids.contains(next)
                    || (!request.target_kinds.is_empty() && !request.target_kinds.contains(&kind))
                {
                    continue;
                }
                let origin = GraphOriginV1 {
                    schema_version: 1,
                    seed_logical_id: seed.logical_id.clone(),
                    seed_ordinal: seed.seed_ordinal,
                    predecessor_logical_id: current.clone(),
                    target_logical_id: next.to_string(),
                    hop_count: depth + 1,
                    terminal_edge_kind: edge.kind.clone(),
                    terminal_direction,
                };
                let target = GraphTargetV1 {
                    schema_version: 1,
                    logical_id: next.to_string(),
                    kind,
                    body,
                    write_cursor,
                    origin,
                };
                if let Some(existing) = candidates.get_mut(next) {
                    if origin_cmp(&target.origin, &existing.target.origin).is_lt() {
                        *existing =
                            GraphCandidate { target, terminal_edge_cursor: edge.write_cursor };
                    }
                } else if candidates.len() < request.result_limit as usize {
                    candidates.insert(
                        next.to_string(),
                        GraphCandidate { target, terminal_edge_cursor: edge.write_cursor },
                    );
                } else {
                    let worst = candidates
                        .iter()
                        .max_by(|(_, left), (_, right)| {
                            origin_cmp(&left.target.origin, &right.target.origin)
                        })
                        .map(|(logical_id, _)| logical_id.clone())
                        .expect("result limit is validated nonzero");
                    if origin_cmp(&target.origin, &candidates[&worst].target.origin).is_lt() {
                        candidates.remove(&worst);
                        candidates.insert(
                            next.to_string(),
                            GraphCandidate { target, terminal_edge_cursor: edge.write_cursor },
                        );
                    }
                }
                #[cfg(feature = "test-hooks")]
                observe_retained_graph_state(
                    test_controls,
                    0,
                    current_frontier_states.saturating_add(next_frontier.len()),
                    visited_by_seed.iter().map(HashSet::len).sum(),
                    candidates.len(),
                );
            }
            #[cfg(feature = "test-hooks")]
            observe_current_rss_peak(test_controls);
        }
        frontier = next_frontier;
        #[cfg(feature = "test-hooks")]
        observe_retained_graph_state(
            test_controls,
            0,
            frontier.len(),
            visited_by_seed.iter().map(HashSet::len).sum(),
            candidates.len(),
        );
    }
    let mut selected = candidates.into_values().collect::<Vec<_>>();
    selected.sort_by(|left, right| origin_cmp(&left.target.origin, &right.target.origin));
    selected.truncate(request.result_limit as usize);
    let evidence = if request.include_evidence {
        let frozen = match &request.context {
            GraphReadContextV1::Frozen { context, .. } => context,
            GraphReadContextV1::Current { .. } => {
                return Err(graph_error(
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    "/context",
                ))
            }
        };
        let authority = evidence_authority.ok_or(EngineError::Storage)?;
        let node_cursors = selected.iter().map(|item| item.target.write_cursor).collect::<Vec<_>>();
        let edge_cursors =
            selected.iter().map(|item| item.terminal_edge_cursor).collect::<Vec<_>>();
        if selected.is_empty() {
            Some(crate::GraphEvidenceSidecarV1 { schema_version: 1, entries: Vec::new() })
        } else {
            let preflight = crate::evidence::preflight_graph_evidence(
                &tx,
                frozen,
                &node_cursors,
                &edge_cursors,
            )?;
            let request_commitment = encode_graph_expand_request_v1(request)
                .map_err(|_| EngineError::Evidence(crate::EvidenceErrorV1::unavailable()))?;
            let mut entries = Vec::with_capacity(selected.len());
            for (index, candidate) in selected.iter().enumerate() {
                let target_material = preflight.nodes.get(index).ok_or(EngineError::Storage)?;
                let edge_material = preflight.edges.get(index).ok_or(EngineError::Storage)?;
                let mut disclosure = request_commitment.clone();
                disclosure.extend_from_slice(&(index as u64).to_be_bytes());
                disclosure.extend_from_slice(&candidate.target.write_cursor.to_be_bytes());
                disclosure.extend_from_slice(&candidate.terminal_edge_cursor.to_be_bytes());
                let (target_revision, target_reference) =
                    crate::evidence::mint_graph_evidence_reference(
                        &tx,
                        authority,
                        frozen,
                        target_material,
                        None,
                        &disclosure,
                    )?;
                disclosure.extend_from_slice(b"terminal-edge");
                let (edge_revision, edge_reference) =
                    crate::evidence::mint_graph_evidence_reference(
                        &tx,
                        authority,
                        frozen,
                        edge_material,
                        Some(candidate.target.origin.terminal_direction),
                        &disclosure,
                    )?;
                entries.push(crate::GraphEvidenceSidecarEntryV1 {
                    schema_version: 1,
                    target_index: u32::try_from(index).map_err(|_| EngineError::Storage)?,
                    target_artifact_revision_id: target_revision,
                    target_evidence_ref: target_reference,
                    terminal_edge_artifact_revision_id: edge_revision,
                    terminal_edge_evidence_ref: edge_reference,
                });
            }
            Some(crate::GraphEvidenceSidecarV1 { schema_version: 1, entries })
        }
    } else {
        None
    };
    let targets = selected.into_iter().map(|candidate| candidate.target).collect::<Vec<_>>();
    let per_target = if request.include_explanation {
        targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let dependency_state =
                    structural_dependency_state(&tx, target.write_cursor, view.edge_now())
                        .map_err(|_| EngineError::Storage)?;
                Ok(GraphTargetExplanationV1 {
                    schema_version: 1,
                    target_index: u32::try_from(index).unwrap_or(u32::MAX),
                    origin: target.origin.clone(),
                    lifecycle_state: StructuralLifecycleStateV1::NodeActive,
                    dependency_state,
                })
            })
            .collect::<Result<Vec<_>, EngineError>>()?
    } else {
        Vec::new()
    };
    if let Some(binding) = frozen_binding {
        frozen_read::validate_snapshot(&tx, binding)?;
    }
    let explanation = request.include_explanation.then(|| GraphExpansionExplanationV1 {
        schema_version: 1,
        correlation_id: String::new(),
        seed_source,
        read_mode,
        projection_generation_id: generation_id,
        projection_origin,
        projection_readiness,
        degradation_codes: degradation_codes.clone(),
        per_target,
    });
    tx.commit().map_err(|_| EngineError::Storage)?;
    Ok(GraphExpandResultV1 {
        schema_version: 1,
        seeds,
        targets,
        complete: true,
        work_units,
        degradation_codes,
        explanation,
        evidence,
    })
}

// Codec implementation follows the runtime implementation so the request and
// response carriers remain free of serialization derives.

fn validate_filter_attributes_on_snapshot(
    connection: &Connection,
    filter: &SearchFilter,
) -> Result<(), crate::SearchReaderError> {
    crate::validate_filter_attributes_on_snapshot(connection, filter)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct IdWire<'a> {
    space: &'a str,
    value: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct QuerySeedWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    text: &'a str,
    ranked_limit: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExplicitSeedWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    logical_ids: Vec<IdWire<'a>>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum SeedWire<'a> {
    Query(QuerySeedWire<'a>),
    Explicit(ExplicitSeedWire<'a>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ViewWire {
    include_superseded: bool,
    include_inactive: bool,
    include_out_of_window: bool,
    valid_as_of: Option<i64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FilterWire<'a> {
    source_type: Option<&'a str>,
    kind: Option<&'a str>,
    created_after: Option<i64>,
    status: Option<&'a str>,
    attributes: Vec<(&'a str, &'a str)>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadContextWire<'a> {
    schema_version: u32,
    view: ViewWire,
    eligibility: FilterWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrozenContextWire<'a> {
    schema_version: u32,
    effective_valid_at: i64,
    context: ReadContextWire<'a>,
    token: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CurrentContextWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    context: ReadContextWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FrozenGraphContextWire<'a> {
    schema_version: u32,
    r#type: &'static str,
    context: FrozenContextWire<'a>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum GraphContextWire<'a> {
    Current(CurrentContextWire<'a>),
    Frozen(FrozenGraphContextWire<'a>),
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RequestWire<'a> {
    schema_version: u32,
    seed: SeedWire<'a>,
    direction: &'static str,
    edge_kinds: &'a [String],
    target_kinds: &'a [String],
    context: GraphContextWire<'a>,
    max_depth: u32,
    result_limit: u32,
    max_work_units: String,
    include_explanation: bool,
    #[serde(skip_serializing_if = "is_false")]
    include_evidence: bool,
}

fn direction_str(value: TraversalDirection) -> &'static str {
    match value {
        TraversalDirection::Incoming => "incoming",
        TraversalDirection::Outgoing => "outgoing",
        TraversalDirection::Both => "both",
    }
}

fn read_context_wire(value: &ReadContextV1) -> ReadContextWire<'_> {
    ReadContextWire {
        schema_version: value.schema_version,
        view: ViewWire {
            include_superseded: value.view.include_superseded,
            include_inactive: value.view.include_inactive,
            include_out_of_window: value.view.include_out_of_window,
            valid_as_of: value.view.valid_as_of,
        },
        eligibility: FilterWire {
            source_type: value.eligibility.source_type.as_deref(),
            kind: value.eligibility.kind.as_deref(),
            created_after: value.eligibility.created_after,
            status: value.eligibility.status.as_deref(),
            attributes: value
                .eligibility
                .attributes
                .iter()
                .map(|(name, value)| (name.as_str(), value.as_str()))
                .collect(),
        },
    }
}

/// Encode a graph-expansion request into canonical declaration-order JSON.
pub fn encode_graph_expand_request_v1(
    value: &GraphExpandRequestV1,
) -> Result<Vec<u8>, GraphExpansionErrorV1> {
    let seed = match &value.seed {
        GraphSeedV1::Query { schema_version, text, ranked_limit } => {
            SeedWire::Query(QuerySeedWire {
                schema_version: *schema_version,
                r#type: "query",
                text,
                ranked_limit: *ranked_limit,
            })
        }
        GraphSeedV1::Explicit { schema_version, logical_ids } => {
            SeedWire::Explicit(ExplicitSeedWire {
                schema_version: *schema_version,
                r#type: "explicit",
                logical_ids: logical_ids
                    .iter()
                    .map(|id| IdWire { space: id.space.as_str(), value: &id.value })
                    .collect(),
            })
        }
    };
    let context = match &value.context {
        GraphReadContextV1::Current { schema_version, context } => {
            GraphContextWire::Current(CurrentContextWire {
                schema_version: *schema_version,
                r#type: "current",
                context: read_context_wire(context),
            })
        }
        GraphReadContextV1::Frozen { schema_version, context } => {
            GraphContextWire::Frozen(FrozenGraphContextWire {
                schema_version: *schema_version,
                r#type: "frozen",
                context: FrozenContextWire {
                    schema_version: context.schema_version,
                    effective_valid_at: context.effective_valid_at,
                    context: read_context_wire(&context.context),
                    token: &context.token,
                },
            })
        }
    };
    serde_json::to_vec(&RequestWire {
        schema_version: value.schema_version,
        seed,
        direction: direction_str(value.direction),
        edge_kinds: &value.edge_kinds,
        target_kinds: &value.target_kinds,
        context,
        max_depth: value.max_depth,
        result_limit: value.result_limit,
        max_work_units: value.max_work_units.to_string(),
        include_explanation: value.include_explanation,
        include_evidence: value.include_evidence,
    })
    .map_err(|_| GraphExpansionErrorV1::new(GraphExpansionErrorReasonV1::GraphCorrupt, ""))
}

fn request_error(
    reason: GraphExpansionErrorReasonV1,
    path: impl Into<String>,
) -> GraphExpansionErrorV1 {
    GraphExpansionErrorV1::new(reason, path)
}

fn object<'a>(
    value: &'a serde_json::Value,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, GraphExpansionErrorV1> {
    value.as_object().ok_or_else(|| request_error(reason, path))
}

fn required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<&'a serde_json::Value, GraphExpansionErrorV1> {
    object.get(field).ok_or_else(|| request_error(reason, path))
}

fn check_closed(
    object: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    base: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if let Some(field) = object.keys().filter(|field| !allowed.contains(&field.as_str())).min() {
        let escaped = field.replace('~', "~0").replace('/', "~1");
        return Err(request_error(
            GraphExpansionErrorReasonV1::UnknownField,
            format!("{base}/{escaped}"),
        ));
    }
    Ok(())
}

fn check_schema(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if object.get("schemaVersion").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(request_error(GraphExpansionErrorReasonV1::UnsupportedSchemaVersion, path));
    }
    Ok(())
}

fn parse_u32(
    value: &serde_json::Value,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<u32, GraphExpansionErrorV1> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| request_error(reason, path))
}

fn parse_canonical_u64(
    value: &serde_json::Value,
    reason: GraphExpansionErrorReasonV1,
    path: &str,
) -> Result<u64, GraphExpansionErrorV1> {
    let text = value.as_str().ok_or_else(|| request_error(reason, path))?;
    if text.is_empty()
        || (text.len() > 1 && text.starts_with('0'))
        || !text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(request_error(reason, path));
    }
    text.parse().map_err(|_| request_error(reason, path))
}

fn parse_read_context(
    value: &serde_json::Value,
    base: &str,
) -> Result<ReadContextV1, GraphExpansionErrorV1> {
    let context_object = object(value, GraphExpansionErrorReasonV1::GraphContextInvalid, base)?;
    check_schema(context_object, &format!("{base}/schemaVersion"))?;
    check_closed(context_object, &["schemaVersion", "view", "eligibility"], base)?;
    let view_base = format!("{base}/view");
    let view_object = object(
        required(
            context_object,
            "view",
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &view_base,
        )?,
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &view_base,
    )?;
    check_closed(
        view_object,
        &["includeSuperseded", "includeInactive", "includeOutOfWindow", "validAsOf"],
        &view_base,
    )?;
    let boolean = |field: &str| {
        required(
            view_object,
            field,
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &format!("{view_base}/{field}"),
        )?
        .as_bool()
        .ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{view_base}/{field}"),
            )
        })
    };
    let valid_as_of_value = required(
        view_object,
        "validAsOf",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &format!("{view_base}/validAsOf"),
    )?;
    let valid_as_of = if valid_as_of_value.is_null() {
        None
    } else {
        Some(valid_as_of_value.as_i64().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{view_base}/validAsOf"),
            )
        })?)
    };
    let eligibility_base = format!("{base}/eligibility");
    let eligibility_object = object(
        required(
            context_object,
            "eligibility",
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &eligibility_base,
        )?,
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &eligibility_base,
    )?;
    check_closed(
        eligibility_object,
        &["sourceType", "kind", "createdAfter", "status", "attributes"],
        &eligibility_base,
    )?;
    let optional_string = |field: &str| -> Result<Option<String>, GraphExpansionErrorV1> {
        let value = required(
            eligibility_object,
            field,
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            &format!("{eligibility_base}/{field}"),
        )?;
        if value.is_null() {
            Ok(None)
        } else {
            value.as_str().map(|value| Some(value.to_string())).ok_or_else(|| {
                request_error(
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    format!("{eligibility_base}/{field}"),
                )
            })
        }
    };
    let created = required(
        eligibility_object,
        "createdAfter",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &format!("{eligibility_base}/createdAfter"),
    )?;
    let created_after = if created.is_null() {
        None
    } else {
        Some(created.as_i64().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/createdAfter"),
            )
        })?)
    };
    let attributes_value = required(
        eligibility_object,
        "attributes",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        &format!("{eligibility_base}/attributes"),
    )?;
    let mut attributes = Vec::new();
    for (index, item) in attributes_value
        .as_array()
        .ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes"),
            )
        })?
        .iter()
        .enumerate()
    {
        let pair = item.as_array().filter(|pair| pair.len() == 2).ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes/{index}"),
            )
        })?;
        let name = pair[0].as_str().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes/{index}/0"),
            )
        })?;
        let value = pair[1].as_str().ok_or_else(|| {
            request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                format!("{eligibility_base}/attributes/{index}/1"),
            )
        })?;
        attributes.push((name.to_string(), value.to_string()));
    }
    Ok(ReadContextV1 {
        schema_version: 1,
        view: ReadView {
            include_superseded: boolean("includeSuperseded")?,
            include_inactive: boolean("includeInactive")?,
            include_out_of_window: boolean("includeOutOfWindow")?,
            valid_as_of,
        },
        eligibility: SearchFilter {
            source_type: optional_string("sourceType")?,
            kind: optional_string("kind")?,
            created_after,
            status: optional_string("status")?,
            attributes,
        },
    })
}

/// Decode the recursively closed canonical graph-expansion request.
pub fn decode_graph_expand_request_v1(
    bytes: &[u8],
) -> Result<GraphExpandRequestV1, GraphExpansionErrorV1> {
    if bytes.len() > 64 * 1024 {
        return Err(request_error(GraphExpansionErrorReasonV1::GraphContextInvalid, ""));
    }
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| request_error(GraphExpansionErrorReasonV1::GraphContextInvalid, ""))?;
    let root = object(&value, GraphExpansionErrorReasonV1::GraphContextInvalid, "")?;
    check_schema(root, "/schemaVersion")?;
    check_closed(
        root,
        &[
            "schemaVersion",
            "seed",
            "direction",
            "edgeKinds",
            "targetKinds",
            "context",
            "maxDepth",
            "resultLimit",
            "maxWorkUnits",
            "includeExplanation",
            "includeEvidence",
        ],
        "",
    )?;
    let seed_value =
        required(root, "seed", GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed")?;
    let seed_object = object(seed_value, GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed")?;
    check_schema(seed_object, "/seed/schemaVersion")?;
    check_closed(
        seed_object,
        &["schemaVersion", "type", "text", "rankedLimit", "logicalIds"],
        "/seed",
    )?;
    let seed_type =
        required(seed_object, "type", GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/type")?
            .as_str()
            .ok_or_else(|| {
                request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/type")
            })?;
    let seed = match seed_type {
        "query" => {
            if seed_object.contains_key("logicalIds") {
                return Err(request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed"));
            }
            GraphSeedV1::Query {
                schema_version: 1,
                text: required(
                    seed_object,
                    "text",
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/text",
                )?
                .as_str()
                .ok_or_else(|| {
                    request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/text")
                })?
                .to_string(),
                ranked_limit: parse_u32(
                    required(
                        seed_object,
                        "rankedLimit",
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        "/seed/rankedLimit",
                    )?,
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    "/seed/rankedLimit",
                )?,
            }
        }
        "explicit" => {
            if seed_object.contains_key("text") || seed_object.contains_key("rankedLimit") {
                return Err(request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed"));
            }
            let values = required(
                seed_object,
                "logicalIds",
                GraphExpansionErrorReasonV1::GraphSeedInvalid,
                "/seed/logicalIds",
            )?
            .as_array()
            .ok_or_else(|| {
                request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/logicalIds")
            })?;
            let mut logical_ids = Vec::new();
            for (index, value) in values.iter().enumerate() {
                let base = format!("/seed/logicalIds/{index}");
                let object = object(value, GraphExpansionErrorReasonV1::GraphSeedInvalid, &base)?;
                check_closed(object, &["space", "value"], &base)?;
                let space = required(
                    object,
                    "space",
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    &format!("{base}/space"),
                )?
                .as_str();
                let value = required(
                    object,
                    "value",
                    GraphExpansionErrorReasonV1::GraphSeedInvalid,
                    &format!("{base}/value"),
                )?
                .as_str()
                .ok_or_else(|| {
                    request_error(
                        GraphExpansionErrorReasonV1::GraphSeedInvalid,
                        format!("{base}/value"),
                    )
                })?;
                logical_ids.push(match space {
                    Some("logical") => IdSpace::logical(value),
                    Some("content") => IdSpace::content(value),
                    Some("passage") => IdSpace::passage(value),
                    _ => {
                        return Err(request_error(
                            GraphExpansionErrorReasonV1::GraphSeedInvalid,
                            format!("{base}/space"),
                        ))
                    }
                });
            }
            GraphSeedV1::Explicit { schema_version: 1, logical_ids }
        }
        _ => {
            return Err(request_error(GraphExpansionErrorReasonV1::GraphSeedInvalid, "/seed/type"))
        }
    };
    let direction = match required(
        root,
        "direction",
        GraphExpansionErrorReasonV1::GraphDirectionInvalid,
        "/direction",
    )?
    .as_str()
    {
        Some("incoming") => TraversalDirection::Incoming,
        Some("outgoing") => TraversalDirection::Outgoing,
        Some("both") => TraversalDirection::Both,
        _ => {
            return Err(request_error(
                GraphExpansionErrorReasonV1::GraphDirectionInvalid,
                "/direction",
            ))
        }
    };
    let strings = |field: &str, reason: GraphExpansionErrorReasonV1| {
        required(root, field, reason, &format!("/{field}"))?
            .as_array()
            .ok_or_else(|| request_error(reason, format!("/{field}")))?
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| request_error(reason, format!("/{field}/{index}")))
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let context_value =
        required(root, "context", GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")?;
    let context_object =
        object(context_value, GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")?;
    check_schema(context_object, "/context/schemaVersion")?;
    check_closed(context_object, &["schemaVersion", "type", "context"], "/context")?;
    let context_type = required(
        context_object,
        "type",
        GraphExpansionErrorReasonV1::GraphContextInvalid,
        "/context/type",
    )?
    .as_str();
    let context = match context_type {
        Some("current") => GraphReadContextV1::Current {
            schema_version: 1,
            context: parse_read_context(
                required(
                    context_object,
                    "context",
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    "/context/context",
                )?,
                "/context/context",
            )?,
        },
        Some("frozen") => {
            let base = "/context/context";
            let frozen_object = object(
                required(
                    context_object,
                    "context",
                    GraphExpansionErrorReasonV1::GraphContextInvalid,
                    base,
                )?,
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                base,
            )?;
            check_schema(frozen_object, "/context/context/schemaVersion")?;
            check_closed(
                frozen_object,
                &["schemaVersion", "effectiveValidAt", "context", "token"],
                base,
            )?;
            GraphReadContextV1::Frozen {
                schema_version: 1,
                context: FrozenReadContextV1 {
                    schema_version: 1,
                    effective_valid_at: required(
                        frozen_object,
                        "effectiveValidAt",
                        GraphExpansionErrorReasonV1::GraphContextInvalid,
                        "/context/context/effectiveValidAt",
                    )?
                    .as_i64()
                    .ok_or_else(|| {
                        request_error(
                            GraphExpansionErrorReasonV1::GraphContextInvalid,
                            "/context/context/effectiveValidAt",
                        )
                    })?,
                    context: parse_read_context(
                        required(
                            frozen_object,
                            "context",
                            GraphExpansionErrorReasonV1::GraphContextInvalid,
                            "/context/context/context",
                        )?,
                        "/context/context/context",
                    )?,
                    token: required(
                        frozen_object,
                        "token",
                        GraphExpansionErrorReasonV1::GraphContextInvalid,
                        "/context/context/token",
                    )?
                    .as_str()
                    .ok_or_else(|| {
                        request_error(
                            GraphExpansionErrorReasonV1::GraphContextInvalid,
                            "/context/context/token",
                        )
                    })?
                    .to_string(),
                },
            }
        }
        _ => {
            return Err(request_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                "/context/type",
            ))
        }
    };
    let request = GraphExpandRequestV1 {
        schema_version: 1,
        seed,
        direction,
        edge_kinds: strings("edgeKinds", GraphExpansionErrorReasonV1::GraphEdgeKindsInvalid)?,
        target_kinds: strings("targetKinds", GraphExpansionErrorReasonV1::GraphTargetKindsInvalid)?,
        context,
        max_depth: parse_u32(
            required(
                root,
                "maxDepth",
                GraphExpansionErrorReasonV1::GraphDepthInvalid,
                "/maxDepth",
            )?,
            GraphExpansionErrorReasonV1::GraphDepthInvalid,
            "/maxDepth",
        )?,
        result_limit: parse_u32(
            required(
                root,
                "resultLimit",
                GraphExpansionErrorReasonV1::GraphResultLimitInvalid,
                "/resultLimit",
            )?,
            GraphExpansionErrorReasonV1::GraphResultLimitInvalid,
            "/resultLimit",
        )?,
        max_work_units: parse_canonical_u64(
            required(
                root,
                "maxWorkUnits",
                GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
                "/maxWorkUnits",
            )?,
            GraphExpansionErrorReasonV1::GraphWorkLimitInvalid,
            "/maxWorkUnits",
        )?,
        include_explanation: required(
            root,
            "includeExplanation",
            GraphExpansionErrorReasonV1::GraphContextInvalid,
            "/includeExplanation",
        )?
        .as_bool()
        .ok_or_else(|| {
            request_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/includeExplanation")
        })?,
        include_evidence: root
            .get("includeEvidence")
            .map(|value| {
                value.as_bool().ok_or_else(|| {
                    request_error(
                        GraphExpansionErrorReasonV1::GraphContextInvalid,
                        "/includeEvidence",
                    )
                })
            })
            .transpose()?
            .unwrap_or(false),
    };
    Ok(request)
}

fn degradation_str(value: GraphExpansionDegradationCodeV1) -> &'static str {
    match value {
        GraphExpansionDegradationCodeV1::QuerySeedTextFallback => "query_seed_text_fallback",
        GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified => {
            "projection_legacy_unverified"
        }
        GraphExpansionDegradationCodeV1::ProjectionProcessing => "projection_processing",
        GraphExpansionDegradationCodeV1::ProjectionBlocked => "projection_blocked",
        GraphExpansionDegradationCodeV1::ProjectionDeferred => "projection_deferred",
        GraphExpansionDegradationCodeV1::ProjectionDegraded => "projection_degraded",
    }
}

fn projection_origin_str(value: GraphProjectionOriginV1) -> &'static str {
    match value {
        GraphProjectionOriginV1::NotApplicable => "not_applicable",
        GraphProjectionOriginV1::Fresh => "fresh",
        GraphProjectionOriginV1::LegacyUnverified => "legacy_unverified",
        GraphProjectionOriginV1::Configuration => "configuration",
        GraphProjectionOriginV1::Rebuild => "rebuild",
    }
}

fn projection_readiness_str(value: GraphProjectionReadinessV1) -> &'static str {
    match value {
        GraphProjectionReadinessV1::NotApplicable => "not_applicable",
        GraphProjectionReadinessV1::Ready => "ready",
        GraphProjectionReadinessV1::Processing => "processing",
        GraphProjectionReadinessV1::Blocked => "blocked",
        GraphProjectionReadinessV1::Deferred => "deferred",
        GraphProjectionReadinessV1::Degraded => "degraded",
    }
}

fn lifecycle_str(value: StructuralLifecycleStateV1) -> &'static str {
    match value {
        StructuralLifecycleStateV1::NodePending => "node_pending",
        StructuralLifecycleStateV1::NodeActive => "node_active",
        StructuralLifecycleStateV1::NodeDeleted => "node_deleted",
        StructuralLifecycleStateV1::EdgeValid => "edge_valid",
    }
}

fn dependency_str(value: StructuralDependencyStateV1) -> &'static str {
    match value {
        StructuralDependencyStateV1::NotApplicable => "not_applicable",
        StructuralDependencyStateV1::NotRegistered => "not_registered",
        StructuralDependencyStateV1::Registered => "registered",
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResolvedSeedWire<'a> {
    schema_version: u32,
    logical_id: &'a str,
    seed_ordinal: u32,
    query_score: Option<f64>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OriginWire<'a> {
    schema_version: u32,
    seed_logical_id: &'a str,
    seed_ordinal: u32,
    predecessor_logical_id: &'a str,
    target_logical_id: &'a str,
    hop_count: u32,
    terminal_edge_kind: &'a str,
    terminal_direction: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetWire<'a> {
    schema_version: u32,
    logical_id: &'a str,
    kind: &'a str,
    body: &'a str,
    write_cursor: String,
    origin: OriginWire<'a>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetExplanationWire<'a> {
    schema_version: u32,
    target_index: u32,
    origin: OriginWire<'a>,
    lifecycle_state: &'static str,
    dependency_state: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ExplanationWire<'a> {
    schema_version: u32,
    correlation_id: &'a str,
    seed_source: &'static str,
    read_mode: &'static str,
    projection_generation_id: Option<&'a str>,
    projection_origin: &'static str,
    projection_readiness: &'static str,
    degradation_codes: Vec<&'static str>,
    per_target: Vec<TargetExplanationWire<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceEntryWire<'a> {
    schema_version: u32,
    target_index: u32,
    target_artifact_revision_id: &'a str,
    target_evidence_ref: &'a str,
    terminal_edge_artifact_revision_id: &'a str,
    terminal_edge_evidence_ref: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceSidecarWire<'a> {
    schema_version: u32,
    entries: Vec<EvidenceEntryWire<'a>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ResultWire<'a> {
    schema_version: u32,
    seeds: Vec<ResolvedSeedWire<'a>>,
    targets: Vec<TargetWire<'a>>,
    complete: bool,
    work_units: String,
    degradation_codes: Vec<&'static str>,
    explanation: Option<ExplanationWire<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evidence: Option<EvidenceSidecarWire<'a>>,
}

fn origin_wire(value: &GraphOriginV1) -> OriginWire<'_> {
    OriginWire {
        schema_version: value.schema_version,
        seed_logical_id: &value.seed_logical_id,
        seed_ordinal: value.seed_ordinal,
        predecessor_logical_id: &value.predecessor_logical_id,
        target_logical_id: &value.target_logical_id,
        hop_count: value.hop_count,
        terminal_edge_kind: &value.terminal_edge_kind,
        terminal_direction: direction_str(value.terminal_direction),
    }
}

/// Encode a graph-expansion response into canonical declaration-order JSON.
pub fn encode_graph_expand_result_v1(
    value: &GraphExpandResultV1,
) -> Result<Vec<u8>, GraphExpansionErrorV1> {
    validate_response_coherence(value)?;
    let explanation = value.explanation.as_ref().map(|explanation| ExplanationWire {
        schema_version: explanation.schema_version,
        correlation_id: &explanation.correlation_id,
        seed_source: match explanation.seed_source {
            GraphSeedSourceV1::Query => "query",
            GraphSeedSourceV1::Explicit => "explicit",
        },
        read_mode: match explanation.read_mode {
            GraphReadModeV1::Current => "current",
            GraphReadModeV1::Frozen => "frozen",
        },
        projection_generation_id: explanation.projection_generation_id.as_deref(),
        projection_origin: projection_origin_str(explanation.projection_origin),
        projection_readiness: projection_readiness_str(explanation.projection_readiness),
        degradation_codes: explanation
            .degradation_codes
            .iter()
            .copied()
            .map(degradation_str)
            .collect(),
        per_target: explanation
            .per_target
            .iter()
            .map(|target| TargetExplanationWire {
                schema_version: target.schema_version,
                target_index: target.target_index,
                origin: origin_wire(&target.origin),
                lifecycle_state: lifecycle_str(target.lifecycle_state),
                dependency_state: dependency_str(target.dependency_state),
            })
            .collect(),
    });
    serde_json::to_vec(&ResultWire {
        schema_version: value.schema_version,
        seeds: value
            .seeds
            .iter()
            .map(|seed| ResolvedSeedWire {
                schema_version: seed.schema_version,
                logical_id: &seed.logical_id,
                seed_ordinal: seed.seed_ordinal,
                query_score: seed.query_score,
            })
            .collect(),
        targets: value
            .targets
            .iter()
            .map(|target| TargetWire {
                schema_version: target.schema_version,
                logical_id: &target.logical_id,
                kind: &target.kind,
                body: &target.body,
                write_cursor: target.write_cursor.to_string(),
                origin: origin_wire(&target.origin),
            })
            .collect(),
        complete: value.complete,
        work_units: value.work_units.to_string(),
        degradation_codes: value.degradation_codes.iter().copied().map(degradation_str).collect(),
        explanation,
        evidence: value.evidence.as_ref().map(|sidecar| EvidenceSidecarWire {
            schema_version: sidecar.schema_version,
            entries: sidecar
                .entries
                .iter()
                .map(|entry| EvidenceEntryWire {
                    schema_version: entry.schema_version,
                    target_index: entry.target_index,
                    target_artifact_revision_id: entry.target_artifact_revision_id.as_str(),
                    target_evidence_ref: entry.target_evidence_ref.as_str(),
                    terminal_edge_artifact_revision_id: entry
                        .terminal_edge_artifact_revision_id
                        .as_str(),
                    terminal_edge_evidence_ref: entry.terminal_edge_evidence_ref.as_str(),
                })
                .collect(),
        }),
    })
    .map_err(|_| GraphExpansionErrorV1::new(GraphExpansionErrorReasonV1::GraphCorrupt, ""))
}

fn response_error(
    reason: GraphExpansionErrorReasonV1,
    path: impl Into<String>,
) -> GraphExpansionErrorV1 {
    GraphExpansionErrorV1::new(reason, path)
}

fn response_object<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, GraphExpansionErrorV1> {
    value.as_object().ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn response_required<'a>(
    object: &'a serde_json::Map<String, serde_json::Value>,
    field: &str,
    path: &str,
) -> Result<&'a serde_json::Value, GraphExpansionErrorV1> {
    object.get(field).ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn response_schema(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> Result<(), GraphExpansionErrorV1> {
    if object.get("schemaVersion").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(response_error(GraphExpansionErrorReasonV1::UnsupportedSchemaVersion, path));
    }
    Ok(())
}

fn response_u32(value: &serde_json::Value, path: &str) -> Result<u32, GraphExpansionErrorV1> {
    value
        .as_u64()
        .and_then(|number| u32::try_from(number).ok())
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn response_u64(value: &serde_json::Value, path: &str) -> Result<u64, GraphExpansionErrorV1> {
    parse_canonical_u64(value, GraphExpansionErrorReasonV1::GraphCorrupt, path)
}

fn response_string(value: &serde_json::Value, path: &str) -> Result<String, GraphExpansionErrorV1> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path))
}

fn parse_direction(
    value: &serde_json::Value,
    path: &str,
) -> Result<TraversalDirection, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("incoming") => Ok(TraversalDirection::Incoming),
        Some("outgoing") => Ok(TraversalDirection::Outgoing),
        Some("both") => Ok(TraversalDirection::Both),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_origin(
    value: &serde_json::Value,
    base: &str,
) -> Result<GraphOriginV1, GraphExpansionErrorV1> {
    let object = response_object(value, base)?;
    response_schema(object, &format!("{base}/schemaVersion"))?;
    Ok(GraphOriginV1 {
        schema_version: 1,
        seed_logical_id: response_string(
            response_required(object, "seedLogicalId", &format!("{base}/seedLogicalId"))?,
            &format!("{base}/seedLogicalId"),
        )?,
        seed_ordinal: response_u32(
            response_required(object, "seedOrdinal", &format!("{base}/seedOrdinal"))?,
            &format!("{base}/seedOrdinal"),
        )?,
        predecessor_logical_id: response_string(
            response_required(
                object,
                "predecessorLogicalId",
                &format!("{base}/predecessorLogicalId"),
            )?,
            &format!("{base}/predecessorLogicalId"),
        )?,
        target_logical_id: response_string(
            response_required(object, "targetLogicalId", &format!("{base}/targetLogicalId"))?,
            &format!("{base}/targetLogicalId"),
        )?,
        hop_count: response_u32(
            response_required(object, "hopCount", &format!("{base}/hopCount"))?,
            &format!("{base}/hopCount"),
        )?,
        terminal_edge_kind: response_string(
            response_required(object, "terminalEdgeKind", &format!("{base}/terminalEdgeKind"))?,
            &format!("{base}/terminalEdgeKind"),
        )?,
        terminal_direction: parse_direction(
            response_required(object, "terminalDirection", &format!("{base}/terminalDirection"))?,
            &format!("{base}/terminalDirection"),
        )?,
    })
}

fn parse_degradation(
    value: &serde_json::Value,
    path: &str,
) -> Result<GraphExpansionDegradationCodeV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("query_seed_text_fallback") => {
            Ok(GraphExpansionDegradationCodeV1::QuerySeedTextFallback)
        }
        Some("projection_legacy_unverified") => {
            Ok(GraphExpansionDegradationCodeV1::ProjectionLegacyUnverified)
        }
        Some("projection_processing") => Ok(GraphExpansionDegradationCodeV1::ProjectionProcessing),
        Some("projection_blocked") => Ok(GraphExpansionDegradationCodeV1::ProjectionBlocked),
        Some("projection_deferred") => Ok(GraphExpansionDegradationCodeV1::ProjectionDeferred),
        Some("projection_degraded") => Ok(GraphExpansionDegradationCodeV1::ProjectionDegraded),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_degradations(
    value: &serde_json::Value,
    base: &str,
) -> Result<Vec<GraphExpansionDegradationCodeV1>, GraphExpansionErrorV1> {
    value
        .as_array()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, base))?
        .iter()
        .enumerate()
        .map(|(index, value)| parse_degradation(value, &format!("{base}/{index}")))
        .collect()
}

fn parse_projection_origin(
    value: &serde_json::Value,
    path: &str,
) -> Result<GraphProjectionOriginV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("not_applicable") => Ok(GraphProjectionOriginV1::NotApplicable),
        Some("fresh") => Ok(GraphProjectionOriginV1::Fresh),
        Some("legacy_unverified") => Ok(GraphProjectionOriginV1::LegacyUnverified),
        Some("configuration") => Ok(GraphProjectionOriginV1::Configuration),
        Some("rebuild") => Ok(GraphProjectionOriginV1::Rebuild),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_projection_readiness(
    value: &serde_json::Value,
    path: &str,
) -> Result<GraphProjectionReadinessV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("not_applicable") => Ok(GraphProjectionReadinessV1::NotApplicable),
        Some("ready") => Ok(GraphProjectionReadinessV1::Ready),
        Some("processing") => Ok(GraphProjectionReadinessV1::Processing),
        Some("blocked") => Ok(GraphProjectionReadinessV1::Blocked),
        Some("deferred") => Ok(GraphProjectionReadinessV1::Deferred),
        Some("degraded") => Ok(GraphProjectionReadinessV1::Degraded),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_lifecycle(
    value: &serde_json::Value,
    path: &str,
) -> Result<StructuralLifecycleStateV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("node_pending") => Ok(StructuralLifecycleStateV1::NodePending),
        Some("node_active") => Ok(StructuralLifecycleStateV1::NodeActive),
        Some("node_deleted") => Ok(StructuralLifecycleStateV1::NodeDeleted),
        Some("edge_valid") => Ok(StructuralLifecycleStateV1::EdgeValid),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn parse_dependency(
    value: &serde_json::Value,
    path: &str,
) -> Result<StructuralDependencyStateV1, GraphExpansionErrorV1> {
    match value.as_str() {
        Some("not_applicable") => Ok(StructuralDependencyStateV1::NotApplicable),
        Some("not_registered") => Ok(StructuralDependencyStateV1::NotRegistered),
        Some("registered") => Ok(StructuralDependencyStateV1::Registered),
        _ => Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, path)),
    }
}

fn validate_response_coherence(value: &GraphExpandResultV1) -> Result<(), GraphExpansionErrorV1> {
    if value.schema_version != 1 {
        return Err(response_error(
            GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
            "/schemaVersion",
        ));
    }
    if !value.complete {
        return Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/complete"));
    }
    for (index, seed) in value.seeds.iter().enumerate() {
        if seed.schema_version != 1 {
            return Err(response_error(
                GraphExpansionErrorReasonV1::UnsupportedSchemaVersion,
                format!("/seeds/{index}/schemaVersion"),
            ));
        }
        if seed.seed_ordinal as usize != index {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/seeds/{index}/seedOrdinal"),
            ));
        }
        if seed.query_score.is_some_and(|score| !score.is_finite()) {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/seeds/{index}/queryScore"),
            ));
        }
    }
    for (index, target) in value.targets.iter().enumerate() {
        let ordinal = target.origin.seed_ordinal as usize;
        if ordinal >= value.seeds.len() {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/targets/{index}/origin/seedOrdinal"),
            ));
        }
        if target.origin.seed_logical_id != value.seeds[ordinal].logical_id {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/targets/{index}/origin/seedLogicalId"),
            ));
        }
        if target.origin.target_logical_id != target.logical_id {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                format!("/targets/{index}/origin/targetLogicalId"),
            ));
        }
    }
    if let Some(evidence) = &value.evidence {
        if evidence.schema_version != 1 || evidence.entries.len() != value.targets.len() {
            return Err(response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/evidence"));
        }
        for (index, entry) in evidence.entries.iter().enumerate() {
            if entry.schema_version != 1 || entry.target_index as usize != index {
                return Err(response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("/evidence/entries/{index}"),
                ));
            }
        }
    }
    if let Some(explanation) = &value.explanation {
        if explanation.per_target.len() != value.targets.len() {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                "/explanation/perTarget",
            ));
        }
        for (index, item) in explanation.per_target.iter().enumerate() {
            if item.target_index as usize != index {
                return Err(response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("/explanation/perTarget/{index}/targetIndex"),
                ));
            }
            if item.origin != value.targets[index].origin {
                return Err(response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("/explanation/perTarget/{index}/origin"),
                ));
            }
        }
        if explanation.degradation_codes != value.degradation_codes {
            return Err(response_error(
                GraphExpansionErrorReasonV1::GraphCorrupt,
                "/explanation/degradationCodes",
            ));
        }
    }
    Ok(())
}

/// Decode an additive graph-expansion response and verify all cross-field coherence.
pub fn decode_graph_expand_result_v1(
    bytes: &[u8],
) -> Result<GraphExpandResultV1, GraphExpansionErrorV1> {
    let value: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|_| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, ""))?;
    let root = response_object(&value, "")?;
    response_schema(root, "/schemaVersion")?;
    let seed_values = response_required(root, "seeds", "/seeds")?
        .as_array()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/seeds"))?;
    let mut seeds = Vec::new();
    for (index, value) in seed_values.iter().enumerate() {
        let base = format!("/seeds/{index}");
        let object = response_object(value, &base)?;
        response_schema(object, &format!("{base}/schemaVersion"))?;
        let score_value = response_required(object, "queryScore", &format!("{base}/queryScore"))?;
        let query_score = if score_value.is_null() {
            None
        } else {
            Some(score_value.as_f64().filter(|score| score.is_finite()).ok_or_else(|| {
                response_error(
                    GraphExpansionErrorReasonV1::GraphCorrupt,
                    format!("{base}/queryScore"),
                )
            })?)
        };
        seeds.push(ResolvedGraphSeedV1 {
            schema_version: 1,
            logical_id: response_string(
                response_required(object, "logicalId", &format!("{base}/logicalId"))?,
                &format!("{base}/logicalId"),
            )?,
            seed_ordinal: response_u32(
                response_required(object, "seedOrdinal", &format!("{base}/seedOrdinal"))?,
                &format!("{base}/seedOrdinal"),
            )?,
            query_score,
        });
    }
    let target_values = response_required(root, "targets", "/targets")?
        .as_array()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/targets"))?;
    let mut targets = Vec::new();
    for (index, value) in target_values.iter().enumerate() {
        let base = format!("/targets/{index}");
        let object = response_object(value, &base)?;
        response_schema(object, &format!("{base}/schemaVersion"))?;
        targets.push(GraphTargetV1 {
            schema_version: 1,
            logical_id: response_string(
                response_required(object, "logicalId", &format!("{base}/logicalId"))?,
                &format!("{base}/logicalId"),
            )?,
            kind: response_string(
                response_required(object, "kind", &format!("{base}/kind"))?,
                &format!("{base}/kind"),
            )?,
            body: response_string(
                response_required(object, "body", &format!("{base}/body"))?,
                &format!("{base}/body"),
            )?,
            write_cursor: response_u64(
                response_required(object, "writeCursor", &format!("{base}/writeCursor"))?,
                &format!("{base}/writeCursor"),
            )?,
            origin: parse_origin(
                response_required(object, "origin", &format!("{base}/origin"))?,
                &format!("{base}/origin"),
            )?,
        });
    }
    let complete = response_required(root, "complete", "/complete")?
        .as_bool()
        .ok_or_else(|| response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/complete"))?;
    let work_units =
        response_u64(response_required(root, "workUnits", "/workUnits")?, "/workUnits")?;
    let degradation_codes = parse_degradations(
        response_required(root, "degradationCodes", "/degradationCodes")?,
        "/degradationCodes",
    )?;
    let explanation_value = response_required(root, "explanation", "/explanation")?;
    let explanation = if explanation_value.is_null() {
        None
    } else {
        let object = response_object(explanation_value, "/explanation")?;
        response_schema(object, "/explanation/schemaVersion")?;
        let per_target_values = response_required(object, "perTarget", "/explanation/perTarget")?
            .as_array()
            .ok_or_else(|| {
                response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/explanation/perTarget")
            })?;
        let mut per_target = Vec::new();
        for (index, value) in per_target_values.iter().enumerate() {
            let base = format!("/explanation/perTarget/{index}");
            let item = response_object(value, &base)?;
            response_schema(item, &format!("{base}/schemaVersion"))?;
            per_target.push(GraphTargetExplanationV1 {
                schema_version: 1,
                target_index: response_u32(
                    response_required(item, "targetIndex", &format!("{base}/targetIndex"))?,
                    &format!("{base}/targetIndex"),
                )?,
                origin: parse_origin(
                    response_required(item, "origin", &format!("{base}/origin"))?,
                    &format!("{base}/origin"),
                )?,
                lifecycle_state: parse_lifecycle(
                    response_required(item, "lifecycleState", &format!("{base}/lifecycleState"))?,
                    &format!("{base}/lifecycleState"),
                )?,
                dependency_state: parse_dependency(
                    response_required(item, "dependencyState", &format!("{base}/dependencyState"))?,
                    &format!("{base}/dependencyState"),
                )?,
            });
        }
        let generation = response_required(
            object,
            "projectionGenerationId",
            "/explanation/projectionGenerationId",
        )?;
        Some(GraphExpansionExplanationV1 {
            schema_version: 1,
            correlation_id: response_string(
                response_required(object, "correlationId", "/explanation/correlationId")?,
                "/explanation/correlationId",
            )?,
            seed_source: match response_required(object, "seedSource", "/explanation/seedSource")?
                .as_str()
            {
                Some("query") => GraphSeedSourceV1::Query,
                Some("explicit") => GraphSeedSourceV1::Explicit,
                _ => {
                    return Err(response_error(
                        GraphExpansionErrorReasonV1::GraphCorrupt,
                        "/explanation/seedSource",
                    ))
                }
            },
            read_mode: match response_required(object, "readMode", "/explanation/readMode")?
                .as_str()
            {
                Some("current") => GraphReadModeV1::Current,
                Some("frozen") => GraphReadModeV1::Frozen,
                _ => {
                    return Err(response_error(
                        GraphExpansionErrorReasonV1::GraphCorrupt,
                        "/explanation/readMode",
                    ))
                }
            },
            projection_generation_id: if generation.is_null() {
                None
            } else {
                Some(response_string(generation, "/explanation/projectionGenerationId")?)
            },
            projection_origin: parse_projection_origin(
                response_required(object, "projectionOrigin", "/explanation/projectionOrigin")?,
                "/explanation/projectionOrigin",
            )?,
            projection_readiness: parse_projection_readiness(
                response_required(
                    object,
                    "projectionReadiness",
                    "/explanation/projectionReadiness",
                )?,
                "/explanation/projectionReadiness",
            )?,
            degradation_codes: parse_degradations(
                response_required(object, "degradationCodes", "/explanation/degradationCodes")?,
                "/explanation/degradationCodes",
            )?,
            per_target,
        })
    };
    let evidence = match root.get("evidence") {
        None => None,
        Some(value) => {
            let object = response_object(value, "/evidence")?;
            response_schema(object, "/evidence/schemaVersion")?;
            let values = response_required(object, "entries", "/evidence/entries")?
                .as_array()
                .ok_or_else(|| {
                    response_error(GraphExpansionErrorReasonV1::GraphCorrupt, "/evidence/entries")
                })?;
            let mut entries = Vec::with_capacity(values.len());
            for (index, value) in values.iter().enumerate() {
                let base = format!("/evidence/entries/{index}");
                let object = response_object(value, &base)?;
                response_schema(object, &format!("{base}/schemaVersion"))?;
                let string = |name: &str| {
                    response_string(
                        response_required(object, name, &format!("{base}/{name}"))?,
                        &format!("{base}/{name}"),
                    )
                };
                let revision = |name: &str| {
                    let value = string(name)?;
                    crate::ArtifactRevisionId::new(value).map_err(|_| {
                        response_error(
                            GraphExpansionErrorReasonV1::GraphCorrupt,
                            format!("{base}/{name}"),
                        )
                    })
                };
                let reference = |name: &str| {
                    crate::GraphEvidenceRefV1::new(string(name)?).map_err(|_| {
                        response_error(
                            GraphExpansionErrorReasonV1::GraphCorrupt,
                            format!("{base}/{name}"),
                        )
                    })
                };
                entries.push(crate::GraphEvidenceSidecarEntryV1 {
                    schema_version: 1,
                    target_index: response_u32(
                        response_required(object, "targetIndex", &format!("{base}/targetIndex"))?,
                        &format!("{base}/targetIndex"),
                    )?,
                    target_artifact_revision_id: revision("targetArtifactRevisionId")?,
                    target_evidence_ref: reference("targetEvidenceRef")?,
                    terminal_edge_artifact_revision_id: revision("terminalEdgeArtifactRevisionId")?,
                    terminal_edge_evidence_ref: reference("terminalEdgeEvidenceRef")?,
                });
            }
            Some(crate::GraphEvidenceSidecarV1 { schema_version: 1, entries })
        }
    };
    let result = GraphExpandResultV1 {
        schema_version: 1,
        seeds,
        targets,
        complete,
        work_units,
        degradation_codes,
        explanation,
        evidence,
    };
    validate_response_coherence(&result)?;
    Ok(result)
}
