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
    projection_generation, structural_dependency_state, Engine, EngineError, FrozenView,
    IdSpaceKind, ProjectionGenerationOriginV1, ProjectionReadinessV1, ProjectionRuntimeStateV1,
    SearchFilter, StructuralLifecycleStateV1, WalAttributionCollector,
};
#[cfg(feature = "test-hooks")]
use crate::{dependency_closure, ClosureCauseV1};
#[cfg(feature = "test-hooks")]
use rusqlite::params;
use rusqlite::{Connection, OptionalExtension};

use super::*;

#[cfg(feature = "test-hooks")]
static GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-hooks")]
static GRAPH_EXPAND_SQL_STATEMENTS: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-hooks")]
pub(crate) fn count_graph_expand_sql_statement(event: rusqlite::trace::TraceEvent<'_>) {
    if matches!(event, rusqlite::trace::TraceEvent::Stmt(_, _)) {
        GRAPH_EXPAND_SQL_STATEMENTS.fetch_add(1, AtomicOrdering::SeqCst);
    }
}

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

impl GraphExpansionErrorV1 {
    pub(super) fn new(reason: GraphExpansionErrorReasonV1, field_path: impl Into<String>) -> Self {
        Self { schema_version: SCHEMA_VERSION, reason, field_path: field_path.into() }
    }
}

impl Display for GraphExpansionErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for GraphExpansionErrorV1 {}

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
    pub(crate) count_sql_statements: bool,
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

    /// Execute one graph expansion while counting every SQLite statement on
    /// its reader connection.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn graph_expand_with_statement_count_for_test(
        &self,
        request: &GraphExpandRequestV1,
    ) -> Result<(GraphExpandResultV1, u64), EngineError> {
        GRAPH_EXPAND_SQL_STATEMENTS.store(0, AtomicOrdering::SeqCst);
        let result = self.graph_expand_inner(
            request,
            GraphExpandReaderControlsForTest {
                count_sql_statements: true,
                ..GraphExpandReaderControlsForTest::default()
            },
        )?;
        Ok((result, GRAPH_EXPAND_SQL_STATEMENTS.load(AtomicOrdering::SeqCst)))
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
        if request.include_evidence && context.view.include_out_of_window {
            return Err(graph_error(
                GraphExpansionErrorReasonV1::GraphContextInvalid,
                "/context/context/view/includeOutOfWindow",
            ));
        }
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
        let received = receive.recv().map_err(|_| EngineError::Storage)?;
        let mut result = received?;
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

// Reader dispatch keeps each snapshot, evidence, attribution, and test seam explicit.
#[allow(clippy::too_many_arguments)]
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
            crate::SnapshotFilterError::InvalidFilter(_) => {
                graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")
            }
            crate::SnapshotFilterError::Sqlite(_) => EngineError::Storage,
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
            )
            .map_err(|error| match error {
                EngineError::Evidence(error)
                    if error.reason == crate::EvidenceErrorReasonV1::EvidenceUnavailable =>
                {
                    EngineError::Evidence(crate::EvidenceErrorV1::new(
                        crate::EvidenceErrorReasonV1::EvidenceUnavailable,
                        "/evidence",
                    ))
                }
                other => other,
            })?;
            let canonical_request = encode_graph_evidence_request(request);
            let request_commitment =
                crate::evidence::graph_request_commitment(authority, &canonical_request);
            let mut entries = Vec::with_capacity(selected.len());
            for (index, candidate) in selected.iter().enumerate() {
                let target_material = preflight.nodes.get(index).ok_or(EngineError::Storage)?;
                let edge_material = preflight.edges.get(index).ok_or(EngineError::Storage)?;
                let disclosure = crate::evidence::GraphEvidenceDisclosure {
                    target_index: u32::try_from(index).map_err(|_| EngineError::Storage)?,
                    target_cursor: candidate.target.write_cursor,
                    terminal_edge_cursor: candidate.terminal_edge_cursor,
                    direction: candidate.target.origin.terminal_direction,
                    target_logical_id: &candidate.target.logical_id,
                    predecessor_logical_id: &candidate.target.origin.predecessor_logical_id,
                    terminal_edge_kind: &candidate.target.origin.terminal_edge_kind,
                    target_revision_id: target_material.artifact_revision_id(),
                    terminal_edge_revision_id: edge_material.artifact_revision_id(),
                    request_commitment,
                };
                let (target_revision, target_reference) =
                    crate::evidence::mint_graph_evidence_reference(
                        authority,
                        frozen,
                        target_material,
                        0,
                        &disclosure,
                    )?;
                let (edge_revision, edge_reference) =
                    crate::evidence::mint_graph_evidence_reference(
                        authority,
                        frozen,
                        edge_material,
                        1,
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

pub(super) fn encode_graph_evidence_request(value: &GraphExpandRequestV1) -> Vec<u8> {
    let mut bytes = Vec::new();
    frozen_read::encode_u32(&mut bytes, value.schema_version);
    match &value.seed {
        GraphSeedV1::Query { schema_version, text, ranked_limit } => {
            bytes.push(0);
            frozen_read::encode_u32(&mut bytes, *schema_version);
            frozen_read::encode_string(&mut bytes, text);
            frozen_read::encode_u32(&mut bytes, *ranked_limit);
        }
        GraphSeedV1::Explicit { schema_version, logical_ids } => {
            bytes.push(1);
            frozen_read::encode_u32(&mut bytes, *schema_version);
            frozen_read::encode_u32(
                &mut bytes,
                u32::try_from(logical_ids.len()).unwrap_or(u32::MAX),
            );
            for logical_id in logical_ids {
                frozen_read::encode_string(&mut bytes, logical_id.space.as_str());
                frozen_read::encode_string(&mut bytes, &logical_id.value);
            }
        }
    }
    bytes.push(match value.direction {
        TraversalDirection::Outgoing => 0,
        TraversalDirection::Incoming => 1,
        TraversalDirection::Both => 2,
    });
    let mut edge_kinds = value.edge_kinds.iter().map(String::as_str).collect::<Vec<_>>();
    edge_kinds.sort_unstable_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    frozen_read::encode_u32(&mut bytes, u32::try_from(edge_kinds.len()).unwrap_or(u32::MAX));
    for kind in edge_kinds {
        frozen_read::encode_string(&mut bytes, kind);
    }
    let mut target_kinds = value.target_kinds.iter().map(String::as_str).collect::<Vec<_>>();
    target_kinds.sort_unstable_by(|left, right| left.as_bytes().cmp(right.as_bytes()));
    frozen_read::encode_u32(&mut bytes, u32::try_from(target_kinds.len()).unwrap_or(u32::MAX));
    for kind in target_kinds {
        frozen_read::encode_string(&mut bytes, kind);
    }
    frozen_read::encode_u32(&mut bytes, value.max_depth);
    frozen_read::encode_u32(&mut bytes, value.result_limit);
    frozen_read::encode_u64(&mut bytes, value.max_work_units);
    bytes.push(u8::from(value.include_explanation));
    bytes.push(u8::from(value.include_evidence));
    bytes
}

fn validate_filter_attributes_on_snapshot(
    connection: &Connection,
    filter: &SearchFilter,
) -> Result<(), crate::SnapshotFilterError> {
    crate::validate_filter_attributes_on_snapshot(connection, filter)
}
