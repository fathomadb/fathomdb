use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
#[cfg(feature = "test-hooks")]
use std::sync::atomic::Ordering as AtomicOrdering;
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicBool, AtomicU64};
#[cfg(feature = "test-hooks")]
use std::sync::Mutex;

use crate::errors::EngineError;
use crate::filter::{validate_filter_attributes_on_snapshot, SearchFilter};
use crate::frozen_read;
use crate::identity::IdSpaceKind;
use crate::projection_generation::{
    self, ProjectionGenerationOriginV1, ProjectionReadinessV1, ProjectionRuntimeStateV1,
};
use crate::projection_registry::{load_projection_registry, ProjectionRole};
use crate::properties::{
    eval_property_predicate, normalize_finite_f64, validate_property_predicate_detailed,
    PropertyComparisonOp, PropertyEntityClass, PropertyPredicate, PropertyPredicateInvalidField,
    PropertyValue,
};
use crate::reader_transaction::begin_attributed_reader_tx;
use crate::search_types::StructuralLifecycleStateV1;
use crate::structural_state::{structural_dependency_state, structural_dependency_state_as_of};
use crate::temporal::FrozenView;
use crate::wal_attribution::WalAttributionCollector;
use fathomdb_query::{compile_text_query_with, StopWordSet};
use rusqlite::{Connection, OptionalExtension};

use super::types::{
    graph_expansion_degradation_codes, GraphExpandRequestV1, GraphExpandResultV1,
    GraphExpansionErrorReasonV1, GraphExpansionErrorV1, GraphExpansionExplanationV1, GraphOriginV1,
    GraphProjectionOriginV1, GraphProjectionReadinessV1, GraphReadContextV1, GraphReadModeV1,
    GraphSeedSourceV1, GraphSeedV1, GraphTargetExplanationV1, GraphTargetV1, ResolvedGraphSeedV1,
    TraversalDirection, SCHEMA_VERSION,
};

#[cfg(feature = "test-hooks")]
pub(crate) static GRAPH_EXPAND_RSS_SAMPLE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-hooks")]
pub(crate) static GRAPH_EXPAND_SQL_STATEMENTS: AtomicU64 = AtomicU64::new(0);

#[cfg(feature = "test-hooks")]
pub(crate) fn count_graph_expand_sql_statement(event: rusqlite::trace::TraceEvent<'_>) {
    if matches!(event, rusqlite::trace::TraceEvent::Stmt(_, _)) {
        GRAPH_EXPAND_SQL_STATEMENTS.fetch_add(1, AtomicOrdering::SeqCst);
    }
}

#[cfg(feature = "test-hooks")]
fn observe_current_rss_peak(test_controls: &GraphExpandReaderControlsForTest) {
    let Some(peak) = test_controls.rss_peak_bytes.as_ref() else { return };
    let observed = crate::telemetry::process_current_rss_bytes();
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
impl GraphExpandRetentionCountersForTest {
    /// Relaxed loads of `(retained_edge_batch_rows, frontier_states,
    /// visited_states, candidate_targets)`, in that order.
    pub(crate) fn load_relaxed(&self) -> (u64, u64, u64, u64) {
        (
            self.retained_edge_batch_rows.load(AtomicOrdering::Relaxed),
            self.frontier_states.load(AtomicOrdering::Relaxed),
            self.visited_states.load(AtomicOrdering::Relaxed),
            self.candidate_targets.load(AtomicOrdering::Relaxed),
        )
    }
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
impl GraphExpandProjectionGenerationForTest {
    pub(crate) fn new(
        origin: ProjectionGenerationOriginV1,
        readiness: ProjectionReadinessV1,
    ) -> Self {
        Self { origin, readiness }
    }
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

pub(crate) fn graph_error(
    reason: GraphExpansionErrorReasonV1,
    path: impl Into<String>,
) -> EngineError {
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

fn validate_predicate_list(
    predicates: &[PropertyPredicate],
    reason: GraphExpansionErrorReasonV1,
    base: &str,
) -> Result<(), EngineError> {
    if predicates.len() > 32 {
        return Err(graph_error(reason, base));
    }
    for (index, pred) in predicates.iter().enumerate() {
        if let Err((field, _)) = validate_property_predicate_detailed(pred) {
            let suffix = match field {
                PropertyPredicateInvalidField::Key => "key",
                PropertyPredicateInvalidField::Op => "op",
                PropertyPredicateInvalidField::Value => "value",
            };
            return Err(graph_error(reason, format!("{base}/{index}/{suffix}")));
        }
    }
    Ok(())
}

pub(crate) fn validate_semantics(request: &GraphExpandRequestV1) -> Result<(), EngineError> {
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
    validate_predicate_list(
        &request.edge_predicates,
        GraphExpansionErrorReasonV1::GraphEdgePredicatesInvalid,
        "/edgePredicates",
    )?;
    validate_predicate_list(
        &request.target_predicates,
        GraphExpansionErrorReasonV1::GraphTargetPredicatesInvalid,
        "/targetPredicates",
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
    if request.as_of_write_cursor.is_some() && request.as_of_epoch_ms.is_some() {
        return Err(graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/asOfEpochMs"));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ResolvedHistoryCursor {
    Current,
    AsOf(u64),
}

pub(crate) fn as_of_sql_i64(cursor: u64) -> i64 {
    i64::try_from(cursor).unwrap_or(i64::MAX)
}

pub(crate) const COMMIT_TIMESTAMP_RESOLVE_SQL: &str = "\
    SELECT write_cursor \
    FROM _fathomdb_commit_timestamps \
    WHERE committed_at_ms <= ?1 \
    ORDER BY committed_at_ms DESC, write_cursor DESC \
    LIMIT 1";

fn resolve_history_cursor(
    tx: &Connection,
    request: &GraphExpandRequestV1,
) -> Result<ResolvedHistoryCursor, EngineError> {
    match (request.as_of_write_cursor, request.as_of_epoch_ms) {
        (None, None) => Ok(ResolvedHistoryCursor::Current),
        (Some(cursor), None) => Ok(ResolvedHistoryCursor::AsOf(cursor)),
        (None, Some(epoch_ms)) => {
            let resolved = tx
                .query_row(COMMIT_TIMESTAMP_RESOLVE_SQL, [epoch_ms], |row| row.get::<_, i64>(0))
                .optional()
                .map_err(|_| EngineError::Storage)?
                .and_then(|c| u64::try_from(c).ok())
                .unwrap_or(0);
            Ok(ResolvedHistoryCursor::AsOf(resolved))
        }
        (Some(_), Some(_)) => {
            Err(graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/asOfEpochMs"))
        }
    }
}

fn has_dependency_or_closure_rows(
    tx: &Connection,
    scope: &crate::scope::ScopePredicate,
) -> Result<bool, EngineError> {
    let mut binds = Vec::new();
    let mut dep_scope_sql = String::new();
    scope.append_sql_values("scope", &mut dep_scope_sql, &mut binds);
    let mut closure_scope_sql = String::new();
    scope.append_sql_values("scope", &mut closure_scope_sql, &mut binds);
    let sql = format!(
        "SELECT EXISTS(SELECT 1 FROM _fathomdb_source_dependencies WHERE ({dep_scope_sql})) \
         OR EXISTS(SELECT 1 FROM _fathomdb_dependency_closures WHERE ({closure_scope_sql}))"
    );
    tx.query_row(&sql, rusqlite::params_from_iter(binds.iter()), |row| row.get(0))
        .map_err(|_| EngineError::Storage)
}

#[derive(Clone)]
struct EdgeRow {
    write_cursor: u64,
    kind: Box<str>,
    terminal_direction: TraversalDirection,
    superseded_at: Option<i64>,
    next_logical_id: Box<str>,
    logical_id: Option<Box<str>>,
    t_invalid: Option<i64>,
    properties: Option<Box<crate::properties::PropertyMap>>,
}

struct GraphCandidate {
    target: GraphTargetV1,
    terminal_edge_cursor: u64,
}

struct LoadedNodeRow {
    kind: String,
    body: String,
    write_cursor: u64,
    properties: Option<crate::properties::PropertyMap>,
    scope: String,
}

struct ResolvedSeedInternal {
    public: ResolvedGraphSeedV1,
    scopes: Vec<String>,
}

fn property_sql_op(op: PropertyComparisonOp) -> &'static str {
    match op {
        PropertyComparisonOp::Eq => "=",
        PropertyComparisonOp::Ne => "!=",
        PropertyComparisonOp::Lt => "<",
        PropertyComparisonOp::Lte => "<=",
        PropertyComparisonOp::Gt => ">",
        PropertyComparisonOp::Gte => ">=",
    }
}

pub(crate) fn append_cpv_exists_prefilter(
    entity_class: &str,
    cursor_expr: &str,
    cpv_alias: &str,
    pred: &PropertyPredicate,
    params: &mut Vec<rusqlite::types::Value>,
    sql: &mut String,
) {
    use rusqlite::types::Value;
    params.push(Value::Text(pred.key.clone()));
    let key_idx = params.len();
    let sql_op = property_sql_op(pred.op);
    // The table CHECK constraint on `canonical_property_values` guarantees
    // `text_val IS NOT NULL` <=> `value_type='string'`, `num_val IS NOT NULL`
    // <=> `value_type IN ('integer','float')`, and `bool_val IS NOT NULL` <=>
    // `value_type='boolean'`. Omitting the redundant `value_type` filter keeps
    // the partial secondary indexes strictly covering.
    match &pred.value {
        PropertyValue::String(s) => {
            params.push(Value::Text(s.clone()));
            let val_idx = params.len();
            sql.push_str(&format!(
                " AND EXISTS(SELECT 1 FROM canonical_property_values {cpv_alias} \
                 WHERE {cpv_alias}.entity_class='{entity_class}' \
                 AND {cpv_alias}.write_cursor={cursor_expr} \
                 AND {cpv_alias}.attr_name=?{key_idx} \
                 AND {cpv_alias}.text_val IS NOT NULL \
                 AND {cpv_alias}.text_val {sql_op} ?{val_idx})"
            ));
        }
        PropertyValue::Int(i) => {
            params.push(Value::Integer(*i));
            let val_idx = params.len();
            sql.push_str(&format!(
                " AND EXISTS(SELECT 1 FROM canonical_property_values {cpv_alias} \
                 WHERE {cpv_alias}.entity_class='{entity_class}' \
                 AND {cpv_alias}.write_cursor={cursor_expr} \
                 AND {cpv_alias}.attr_name=?{key_idx} \
                 AND {cpv_alias}.num_val IS NOT NULL \
                 AND {cpv_alias}.num_val {sql_op} ?{val_idx})"
            ));
        }
        PropertyValue::Float(f) => {
            let norm = normalize_finite_f64(*f).unwrap_or(0.0);
            params.push(Value::Real(norm));
            let val_idx = params.len();
            sql.push_str(&format!(
                " AND EXISTS(SELECT 1 FROM canonical_property_values {cpv_alias} \
                 WHERE {cpv_alias}.entity_class='{entity_class}' \
                 AND {cpv_alias}.write_cursor={cursor_expr} \
                 AND {cpv_alias}.attr_name=?{key_idx} \
                 AND {cpv_alias}.num_val IS NOT NULL \
                 AND {cpv_alias}.num_val {sql_op} ?{val_idx})"
            ));
        }
        PropertyValue::Bool(b) => {
            params.push(Value::Integer(i64::from(*b)));
            let val_idx = params.len();
            sql.push_str(&format!(
                " AND EXISTS(SELECT 1 FROM canonical_property_values {cpv_alias} \
                 WHERE {cpv_alias}.entity_class='{entity_class}' \
                 AND {cpv_alias}.write_cursor={cursor_expr} \
                 AND {cpv_alias}.attr_name=?{key_idx} \
                 AND {cpv_alias}.bool_val IS NOT NULL \
                 AND {cpv_alias}.bool_val {sql_op} ?{val_idx})"
            ));
        }
    }
}

fn append_cpv_join_prefilter(
    entity_class: &str,
    cursor_expr: &str,
    cpv_alias: &str,
    pred: &PropertyPredicate,
    params: &mut Vec<rusqlite::types::Value>,
    joins: &mut String,
) {
    use rusqlite::types::Value;
    params.push(Value::Text(pred.key.clone()));
    let key_idx = params.len();
    let sql_op = property_sql_op(pred.op);
    // PRIMARY KEY(entity_class, write_cursor, attr_name) guarantees at most
    // one CPV row per (entity, attr_name), so INNER JOIN never duplicates
    // edges while letting SQLite use the covering partial typed index.
    match &pred.value {
        PropertyValue::String(s) => {
            params.push(Value::Text(s.clone()));
            let val_idx = params.len();
            joins.push_str(&format!(
                " INNER JOIN canonical_property_values {cpv_alias} \
                  ON {cpv_alias}.entity_class='{entity_class}' \
                  AND {cpv_alias}.write_cursor={cursor_expr} \
                  AND {cpv_alias}.attr_name=?{key_idx} \
                  AND {cpv_alias}.text_val IS NOT NULL \
                  AND {cpv_alias}.text_val {sql_op} ?{val_idx}"
            ));
        }
        PropertyValue::Int(i) => {
            params.push(Value::Integer(*i));
            let val_idx = params.len();
            joins.push_str(&format!(
                " INNER JOIN canonical_property_values {cpv_alias} \
                  ON {cpv_alias}.entity_class='{entity_class}' \
                  AND {cpv_alias}.write_cursor={cursor_expr} \
                  AND {cpv_alias}.attr_name=?{key_idx} \
                  AND {cpv_alias}.num_val IS NOT NULL \
                  AND {cpv_alias}.num_val {sql_op} ?{val_idx}"
            ));
        }
        PropertyValue::Float(f) => {
            let norm = normalize_finite_f64(*f).unwrap_or(0.0);
            params.push(Value::Real(norm));
            let val_idx = params.len();
            joins.push_str(&format!(
                " INNER JOIN canonical_property_values {cpv_alias} \
                  ON {cpv_alias}.entity_class='{entity_class}' \
                  AND {cpv_alias}.write_cursor={cursor_expr} \
                  AND {cpv_alias}.attr_name=?{key_idx} \
                  AND {cpv_alias}.num_val IS NOT NULL \
                  AND {cpv_alias}.num_val {sql_op} ?{val_idx}"
            ));
        }
        PropertyValue::Bool(b) => {
            params.push(Value::Integer(i64::from(*b)));
            let val_idx = params.len();
            joins.push_str(&format!(
                " INNER JOIN canonical_property_values {cpv_alias} \
                  ON {cpv_alias}.entity_class='{entity_class}' \
                  AND {cpv_alias}.write_cursor={cursor_expr} \
                  AND {cpv_alias}.attr_name=?{key_idx} \
                  AND {cpv_alias}.bool_val IS NOT NULL \
                  AND {cpv_alias}.bool_val {sql_op} ?{val_idx}"
            ));
        }
    }
}

pub(crate) fn build_historical_load_node_query(
    tx: &Connection,
    logical_id: &str,
    view: FrozenView,
    filter: &SearchFilter,
    scope: &crate::scope::ScopePredicate,
    as_of_cursor: i64,
) -> Result<(String, Vec<rusqlite::types::Value>), EngineError> {
    let mut params = vec![
        rusqlite::types::Value::Text(logical_id.to_string()),
        rusqlite::types::Value::Integer(as_of_cursor),
    ];
    let now_idx = if let Some(now) = view.now_param() {
        params.push(rusqlite::types::Value::Integer(now));
        params.len()
    } else {
        0
    };
    let mut scope_sql = String::new();
    scope.append_sql_values("n.scope", &mut scope_sql, &mut params);
    let eligibility =
        crate::filter::append_node_eligibility_sql_historical(tx, Some(filter), "n", &mut params)
            .map_err(|_| EngineError::Storage)?;
    let validity_sql = view.validity_sql("n", now_idx);
    let dep_eligibility_sql = crate::dependency_closure::read_eligibility_sql_historical_for_cursor(
        "n.write_cursor",
        2,
        now_idx,
        view.view.include_out_of_window,
    );
    let common_where = format!(
        " AND ({scope_sql}) AND n.write_cursor<=?2 AND n.state='active' \
         AND NOT EXISTS(\
           SELECT 1 FROM canonical_nodes cur \
           WHERE cur.scope=n.scope \
             AND cur.logical_id=n.logical_id \
             AND cur.superseded_at IS NULL \
             AND cur.state!='active'\
         ){validity_sql}{dep_eligibility_sql}{eligibility}"
    );
    let sql = format!(
        "SELECT kind,body,write_cursor,properties,scope FROM (\
           SELECT n.kind,n.body,n.write_cursor,n.properties,n.scope FROM canonical_nodes n \
           WHERE n.logical_id=?1 AND n.superseded_at IS NULL{common_where} \
           UNION ALL \
           SELECT n.kind,n.body,n.write_cursor,n.properties,n.scope FROM canonical_nodes n \
           WHERE n.logical_id=?1 AND n.superseded_at IS NOT NULL AND n.superseded_at>?2{common_where}\
         ) \
         ORDER BY write_cursor + 0 DESC LIMIT 1"
    );
    Ok((sql, params))
}

fn load_node(
    tx: &Connection,
    logical_id: &str,
    view: FrozenView,
    filter: &SearchFilter,
    scope: &crate::scope::ScopePredicate,
    pushed_target_predicates: &[&PropertyPredicate],
    history_cursor: ResolvedHistoryCursor,
) -> Result<Option<LoadedNodeRow>, EngineError> {
    let (sql, params) = match history_cursor {
        ResolvedHistoryCursor::Current => {
            let mut params = vec![rusqlite::types::Value::Text(logical_id.to_string())];
            if let Some(now) = view.now_param() {
                params.push(rusqlite::types::Value::Integer(now));
            }
            let mut scope_sql = String::new();
            scope.append_sql_values("n.scope", &mut scope_sql, &mut params);
            let eligibility = crate::filter::append_node_eligibility_sql_view(
                tx,
                Some(filter),
                "n",
                &mut params,
                &view.view,
            )
            .map_err(|_| EngineError::Storage)?;
            let mut target_cpv_sql = String::new();
            for (idx, pred) in pushed_target_predicates.iter().enumerate() {
                append_cpv_exists_prefilter(
                    "node",
                    "n.write_cursor",
                    &format!("cpv_t{idx}"),
                    pred,
                    &mut params,
                    &mut target_cpv_sql,
                );
            }
            let sql = format!(
                "SELECT n.kind,n.body,n.write_cursor,n.properties,n.scope FROM canonical_nodes n WHERE n.logical_id=?1 AND ({scope_sql}){}{}{} ORDER BY n.write_cursor DESC LIMIT 1",
                view.node_sql("n", 2), eligibility, target_cpv_sql
            );
            (sql, params)
        }
        ResolvedHistoryCursor::AsOf(as_of) => build_historical_load_node_query(
            tx,
            logical_id,
            view,
            filter,
            scope,
            as_of_sql_i64(as_of),
        )?,
    };
    tx.query_row(&sql, rusqlite::params_from_iter(params.iter()), |row| {
        Ok(LoadedNodeRow {
            kind: row.get(0)?,
            body: row.get(1)?,
            write_cursor: u64::try_from(row.get::<_, i64>(2)?).unwrap_or(0),
            properties: crate::properties::decode_properties_row_col(row, 3)?,
            scope: row.get(4)?,
        })
    })
    .optional()
    .map_err(|_| EngineError::Storage)
}

/// Modular edge-visibility boundary for `graph_expand` incident queries.
///
/// `Current` preserves the zero-overhead unpredicated current-time SQL path;
/// `AsOf` emits direct compound `UNION ALL` queries over active and
/// superseded-after-`as_of` index arms.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EdgeViewBoundary {
    Current {
        edge_now: i64,
    },
    AsOf {
        edge_now: i64,
        as_of_cursor: i64,
        include_out_of_window: bool,
        check_derived_eligibility: bool,
    },
}

pub(crate) struct GraphExpandIncidentQuery {
    sql: String,
    params: Vec<rusqlite::types::Value>,
}

fn graph_expand_incident_sql(
    direction: TraversalDirection,
    scope_idx: usize,
    as_of_idx: Option<usize>,
    extra_joins: &str,
    extra_where: &str,
    limit_idx: usize,
) -> String {
    const COLS: &str =
        "canonical_edges.write_cursor,canonical_edges.kind,canonical_edges.from_id,canonical_edges.to_id,canonical_edges.logical_id,canonical_edges.superseded_at,canonical_edges.t_invalid,canonical_edges.properties";
    match (as_of_idx, direction) {
        (None, TraversalDirection::Outgoing) => format!(
            "SELECT {COLS} FROM canonical_edges{extra_joins} WHERE canonical_edges.scope=?{scope_idx} AND canonical_edges.from_id=?1 AND canonical_edges.superseded_at IS NULL{extra_where} LIMIT ?{limit_idx}"
        ),
        (None, TraversalDirection::Incoming) => format!(
            "SELECT {COLS} FROM canonical_edges{extra_joins} WHERE canonical_edges.scope=?{scope_idx} AND canonical_edges.to_id=?1 AND canonical_edges.superseded_at IS NULL{extra_where} LIMIT ?{limit_idx}"
        ),
        (None, TraversalDirection::Both) => format!(
            "SELECT {COLS} FROM canonical_edges{extra_joins} WHERE canonical_edges.scope=?{scope_idx} AND canonical_edges.from_id=?1 AND canonical_edges.superseded_at IS NULL{extra_where} \
             UNION ALL \
             SELECT {COLS} FROM canonical_edges{extra_joins} WHERE canonical_edges.scope=?{scope_idx} AND canonical_edges.to_id=?1 AND canonical_edges.from_id!=?1 AND canonical_edges.superseded_at IS NULL{extra_where} \
             LIMIT ?{limit_idx}"
        ),
        (Some(as_of_idx), TraversalDirection::Outgoing) => format!(
            "SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND from_id=?1 AND superseded_at IS NULL{extra_where} \
             UNION ALL \
             SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND from_id=?1 AND superseded_at IS NOT NULL AND superseded_at>?{as_of_idx}{extra_where} \
             LIMIT ?{limit_idx}"
        ),
        (Some(as_of_idx), TraversalDirection::Incoming) => format!(
            "SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND to_id=?1 AND superseded_at IS NULL{extra_where} \
             UNION ALL \
             SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND to_id=?1 AND superseded_at IS NOT NULL AND superseded_at>?{as_of_idx}{extra_where} \
             LIMIT ?{limit_idx}"
        ),
        (Some(as_of_idx), TraversalDirection::Both) => format!(
            "SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND from_id=?1 AND superseded_at IS NULL{extra_where} \
             UNION ALL \
             SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND from_id=?1 AND superseded_at IS NOT NULL AND superseded_at>?{as_of_idx}{extra_where} \
             UNION ALL \
             SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND to_id=?1 AND from_id!=?1 AND superseded_at IS NULL{extra_where} \
             UNION ALL \
             SELECT {COLS} FROM canonical_edges WHERE canonical_edges.scope=?{scope_idx} AND to_id=?1 AND from_id!=?1 AND superseded_at IS NOT NULL AND superseded_at>?{as_of_idx}{extra_where} \
             LIMIT ?{limit_idx}"
        ),
    }
}

fn append_edge_kind_filter(
    edge_kinds: &[String],
    params: &mut Vec<rusqlite::types::Value>,
    extra_where: &mut String,
) {
    use rusqlite::types::Value;
    if edge_kinds.len() == 1 {
        params.push(Value::Text(edge_kinds[0].clone()));
        let kind_idx = params.len();
        extra_where.push_str(&format!(" AND canonical_edges.kind=?{kind_idx}"));
    } else if !edge_kinds.is_empty() {
        extra_where.push_str(" AND canonical_edges.kind IN (");
        for (i, kind) in edge_kinds.iter().enumerate() {
            if i > 0 {
                extra_where.push(',');
            }
            params.push(Value::Text(kind.clone()));
            let kind_idx = params.len();
            extra_where.push_str(&format!("?{kind_idx}"));
        }
        extra_where.push(')');
    }
}

pub(crate) fn graph_expand_incident_query(
    logical_id: &str,
    hop_scope: &str,
    direction: TraversalDirection,
    boundary: EdgeViewBoundary,
    edge_kinds: &[String],
    pushed_edge_predicates: &[&PropertyPredicate],
    limit: u64,
) -> GraphExpandIncidentQuery {
    use rusqlite::types::Value;

    let mut params = vec![Value::Text(logical_id.to_string()), Value::Text(hop_scope.to_string())];
    let scope_idx = 2;
    let mut extra_joins = String::new();
    let mut extra_where = String::new();
    let as_of_idx = match boundary {
        EdgeViewBoundary::Current { edge_now } => {
            params.push(Value::Integer(edge_now));
            let now_idx = params.len();
            append_edge_kind_filter(edge_kinds, &mut params, &mut extra_where);
            extra_where.push_str(&format!(
                " AND (canonical_edges.t_valid IS NULL OR canonical_edges.t_valid<=?{now_idx}) AND (canonical_edges.t_invalid IS NULL OR canonical_edges.t_invalid>?{now_idx})"
            ));
            for (idx, pred) in pushed_edge_predicates.iter().enumerate() {
                append_cpv_join_prefilter(
                    "edge",
                    "canonical_edges.write_cursor",
                    &format!("cpv_e{idx}"),
                    pred,
                    &mut params,
                    &mut extra_joins,
                );
            }
            None
        }
        EdgeViewBoundary::AsOf {
            edge_now,
            as_of_cursor,
            include_out_of_window,
            check_derived_eligibility,
        } => {
            params.push(Value::Integer(edge_now));
            let now_idx = params.len();
            params.push(Value::Integer(as_of_cursor));
            let as_of_idx = params.len();
            extra_where.push_str(&format!(" AND write_cursor<=?{as_of_idx}"));
            append_edge_kind_filter(edge_kinds, &mut params, &mut extra_where);
            extra_where.push_str(&format!(
                " AND (t_valid IS NULL OR t_valid<=?{now_idx}) \
                 AND (t_invalid IS NULL OR t_invalid>?{now_idx}) \
                 AND (temporal_fallback IS NULL OR temporal_fallback=0)"
            ));
            if check_derived_eligibility {
                extra_where.push_str(
                    &crate::dependency_closure::read_eligibility_sql_historical_for_cursor(
                        "canonical_edges.write_cursor",
                        as_of_idx,
                        now_idx,
                        include_out_of_window,
                    ),
                );
            }
            Some(as_of_idx)
        }
    };

    params.push(Value::Integer(i64::try_from(limit).unwrap_or(i64::MAX)));
    let limit_idx = params.len();
    let sql = graph_expand_incident_sql(
        direction,
        scope_idx,
        as_of_idx,
        &extra_joins,
        &extra_where,
        limit_idx,
    );
    GraphExpandIncidentQuery { sql, params }
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn explain_sql_with_params(
    connection: &Connection,
    sql: &str,
    params: &[rusqlite::types::Value],
) -> Result<Vec<String>, EngineError> {
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(params.iter()), |row| row.get(3))
        .map_err(|_| EngineError::Storage)?;
    rows.collect::<rusqlite::Result<Vec<String>>>().map_err(|_| EngineError::Storage)
}

#[cfg(any(test, feature = "test-hooks"))]
pub(crate) fn explain_graph_expand_incident_query(
    connection: &Connection,
    query: &GraphExpandIncidentQuery,
) -> Result<Vec<String>, EngineError> {
    explain_sql_with_params(connection, &query.sql, &query.params)
}

#[allow(clippy::too_many_arguments)]
fn load_incident_edges(
    tx: &Connection,
    logical_id: &str,
    hop_scope: &str,
    direction: TraversalDirection,
    boundary: EdgeViewBoundary,
    edge_kinds: &[String],
    pushed_edge_predicates: &[&PropertyPredicate],
    limit: u64,
) -> Result<Vec<EdgeRow>, EngineError> {
    let query = graph_expand_incident_query(
        logical_id,
        hop_scope,
        direction,
        boundary,
        edge_kinds,
        pushed_edge_predicates,
        limit,
    );
    let mut statement = tx.prepare(&query.sql).map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(query.params.iter()), |row| {
            let kind: String = row.get(1)?;
            let from_id: String = row.get(2)?;
            let to_id: String = row.get(3)?;
            let (terminal_direction, next_logical_id) = match direction {
                TraversalDirection::Outgoing => (TraversalDirection::Outgoing, to_id),
                TraversalDirection::Incoming => (TraversalDirection::Incoming, from_id),
                TraversalDirection::Both => {
                    if from_id == logical_id {
                        (TraversalDirection::Outgoing, to_id)
                    } else {
                        (TraversalDirection::Incoming, from_id)
                    }
                }
            };
            Ok(EdgeRow {
                write_cursor: u64::try_from(row.get::<_, i64>(0)?).unwrap_or(0),
                kind: kind.into_boxed_str(),
                terminal_direction,
                superseded_at: row.get(5)?,
                next_logical_id: next_logical_id.into_boxed_str(),
                logical_id: row.get::<_, Option<String>>(4)?.map(String::into_boxed_str),
                t_invalid: row.get(6)?,
                properties: crate::properties::decode_properties_row_col(row, 7)?.map(Box::new),
            })
        })
        .map_err(|_| EngineError::Storage)?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)
}

fn direction_and_next(edge: &EdgeRow) -> (TraversalDirection, &str) {
    (edge.terminal_direction, &edge.next_logical_id)
}

fn direction_rank(value: TraversalDirection) -> u8 {
    match value {
        TraversalDirection::Outgoing => 0,
        TraversalDirection::Incoming => 1,
        TraversalDirection::Both => 2,
    }
}

fn origin_key_lt(
    hop_count: u32,
    seed_ordinal: u32,
    predecessor_logical_id: &str,
    terminal_direction: TraversalDirection,
    terminal_edge_kind: &str,
    target_logical_id: &str,
    other: &GraphOriginV1,
) -> bool {
    (
        hop_count,
        seed_ordinal,
        predecessor_logical_id.as_bytes(),
        direction_rank(terminal_direction),
        terminal_edge_kind.as_bytes(),
        target_logical_id.as_bytes(),
    ) < (
        other.hop_count,
        other.seed_ordinal,
        other.predecessor_logical_id.as_bytes(),
        direction_rank(other.terminal_direction),
        other.terminal_edge_kind.as_bytes(),
        other.target_logical_id.as_bytes(),
    )
}

fn origin_cmp(left: &GraphOriginV1, right: &GraphOriginV1) -> Ordering {
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
        ProjectionReadinessV1::Degraded => GraphProjectionReadinessV1::Degraded,
    }
}

fn resolve_seeds(
    tx: &Connection,
    request: &GraphExpandRequestV1,
    view: FrozenView,
    filter: &SearchFilter,
    scope: &crate::scope::ScopePredicate,
    stop_words: &StopWordSet,
    history_cursor: ResolvedHistoryCursor,
) -> Result<Vec<ResolvedSeedInternal>, EngineError> {
    match &request.seed {
        GraphSeedV1::Explicit { logical_ids, .. } => logical_ids
            .iter()
            .enumerate()
            .map(|(index, id)| {
                let mut matched_scopes = Vec::new();
                if scope.read_scopes().len() <= 1 {
                    if let Some(loaded) =
                        load_node(tx, &id.value, view, filter, scope, &[], history_cursor)?
                    {
                        matched_scopes.push(loaded.scope);
                    }
                } else {
                    for candidate_scope in scope.read_scopes() {
                        let single =
                            crate::scope::ScopePredicate::single_for_query(candidate_scope);
                        if load_node(tx, &id.value, view, filter, &single, &[], history_cursor)?
                            .is_some()
                        {
                            matched_scopes.push(candidate_scope.clone());
                        }
                    }
                }
                if matched_scopes.is_empty() {
                    return Err(graph_error(
                        GraphExpansionErrorReasonV1::GraphSeedUnavailable,
                        format!("/seed/logicalIds/{index}"),
                    ));
                }
                Ok(ResolvedSeedInternal {
                    public: ResolvedGraphSeedV1 {
                        schema_version: 1,
                        logical_id: id.value.clone(),
                        seed_ordinal: u32::try_from(index).unwrap_or(u32::MAX),
                        query_score: None,
                    },
                    scopes: matched_scopes,
                })
            })
            .collect(),
        GraphSeedV1::Query { text, ranked_limit, .. } => {
            let compiled = compile_text_query_with(text, stop_words);
            let (sql, params) = match history_cursor {
                ResolvedHistoryCursor::Current => {
                    let mut params = vec![rusqlite::types::Value::Text(compiled.match_expression)];
                    if let Some(now) = view.now_param() {
                        params.push(rusqlite::types::Value::Integer(now));
                    }
                    let mut scope_sql = String::new();
                    scope.append_sql_values("cn.scope", &mut scope_sql, &mut params);
                    let eligibility = crate::filter::append_node_eligibility_sql_view(
                        tx,
                        Some(filter),
                        "cn",
                        &mut params,
                        &view.view,
                    )
                    .map_err(|_| EngineError::Storage)?;
                    params.push(rusqlite::types::Value::Integer(i64::from(*ranked_limit)));
                    let limit_index = params.len();
                    let sql = format!(
                        "SELECT cn.logical_id,bm25(search_index),cn.scope FROM search_index JOIN canonical_nodes cn ON cn.write_cursor=search_index.write_cursor WHERE search_index MATCH ?1 AND cn.logical_id IS NOT NULL AND ({scope_sql}){}{}{} ORDER BY bm25(search_index),search_index.write_cursor LIMIT ?{limit_index}",
                        view.node_sql("cn", 2), eligibility, ""
                    );
                    (sql, params)
                }
                ResolvedHistoryCursor::AsOf(as_of) => {
                    let mut params = vec![
                        rusqlite::types::Value::Text(compiled.match_expression),
                        rusqlite::types::Value::Integer(as_of_sql_i64(as_of)),
                    ];
                    let now_idx = if let Some(now) = view.now_param() {
                        params.push(rusqlite::types::Value::Integer(now));
                        params.len()
                    } else {
                        0
                    };
                    let mut scope_sql = String::new();
                    scope.append_sql_values("cn.scope", &mut scope_sql, &mut params);
                    let eligibility = crate::filter::append_node_eligibility_sql_historical(
                        tx,
                        Some(filter),
                        "cn",
                        &mut params,
                    )
                    .map_err(|_| EngineError::Storage)?;
                    params.push(rusqlite::types::Value::Integer(i64::from(*ranked_limit)));
                    let limit_index = params.len();
                    let validity_sql = view.validity_sql("cn", now_idx);
                    let dep_eligibility_sql =
                        crate::dependency_closure::read_eligibility_sql_historical_for_cursor(
                            "cn.write_cursor",
                            2,
                            now_idx,
                            view.view.include_out_of_window,
                        );
                    let sql = format!(
                        "SELECT cn.logical_id,bm25(search_index),cn.scope \
                         FROM search_index \
                         JOIN canonical_nodes cn ON cn.write_cursor=search_index.write_cursor \
                         WHERE search_index MATCH ?1 AND cn.logical_id IS NOT NULL \
                           AND ({scope_sql}) \
                           AND cn.write_cursor<=?2 \
                           AND (cn.superseded_at IS NULL OR cn.superseded_at>?2) \
                           AND cn.state='active' \
                           AND NOT EXISTS(\
                             SELECT 1 FROM canonical_nodes cur \
                             WHERE cur.scope=cn.scope \
                               AND cur.logical_id=cn.logical_id \
                               AND cur.superseded_at IS NULL \
                               AND cur.state!='active'\
                           ){validity_sql}{dep_eligibility_sql}{eligibility} \
                         ORDER BY bm25(search_index),search_index.write_cursor LIMIT ?{limit_index}"
                    );
                    (sql, params)
                }
            };
            let mut statement = tx.prepare(&sql).map_err(|_| {
                graph_error(GraphExpansionErrorReasonV1::GraphProjectionUnavailable, "/projection")
            })?;
            let rows = statement
                .query_map(rusqlite::params_from_iter(params.iter()), |row| {
                    Ok((row.get::<_, String>(0)?, -row.get::<_, f64>(1)?, row.get::<_, String>(2)?))
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
                let (logical_id, score, row_scope) = row.map_err(|_| EngineError::Storage)?;
                if seen.insert((row_scope.clone(), logical_id.clone())) {
                    seeds.push(ResolvedSeedInternal {
                        public: ResolvedGraphSeedV1 {
                            schema_version: 1,
                            logical_id,
                            seed_ordinal: u32::try_from(seeds.len()).unwrap_or(u32::MAX),
                            query_score: Some(score),
                        },
                        scopes: vec![row_scope],
                    });
                }
            }
            Ok(seeds)
        }
    }
}

/// [`read_graph_expand_with_stop_words_in_tx`] with the built-in stop list,
/// for in-crate tests that drive the reader transaction directly.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_graph_expand_in_tx(
    reader: &mut Connection,
    request: &GraphExpandRequestV1,
    scope: &crate::scope::ScopePredicate,
    frozen_binding: Option<&frozen_read::FrozenReadBinding>,
    projection_runtime_state: ProjectionRuntimeStateV1,
    evidence_authority: Option<&crate::evidence::GraphEvidenceAuthority>,
    #[cfg(feature = "test-hooks")] test_controls: &GraphExpandReaderControlsForTest,
    attribution: &std::sync::Arc<WalAttributionCollector>,
    worker_idx: usize,
) -> Result<GraphExpandResultV1, EngineError> {
    let local_status_cache = projection_generation::ProjectionGenerationStatusCache::new(
        1,
        #[cfg(feature = "test-hooks")]
        std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
    );
    read_graph_expand_with_stop_words_in_tx(
        reader,
        request,
        scope,
        frozen_binding,
        projection_runtime_state,
        evidence_authority,
        #[cfg(feature = "test-hooks")]
        test_controls,
        &StopWordSet::default(),
        &local_status_cache,
        projection_generation::StatusCacheConnectionSlot::ReaderWorker(0),
        attribution,
        worker_idx,
    )
}

// Reader dispatch keeps each snapshot, evidence, attribution, and test seam explicit.
#[allow(clippy::too_many_arguments)]
pub(crate) fn read_graph_expand_with_stop_words_in_tx(
    reader: &mut Connection,
    request: &GraphExpandRequestV1,
    scope: &crate::scope::ScopePredicate,
    frozen_binding: Option<&frozen_read::FrozenReadBinding>,
    projection_runtime_state: ProjectionRuntimeStateV1,
    evidence_authority: Option<&crate::evidence::GraphEvidenceAuthority>,
    #[cfg(feature = "test-hooks")] test_controls: &GraphExpandReaderControlsForTest,
    stop_words: &StopWordSet,
    status_cache: &projection_generation::ProjectionGenerationStatusCache,
    status_slot: projection_generation::StatusCacheConnectionSlot,
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
            crate::filter::SnapshotFilterError::InvalidFilter(_) => {
                graph_error(GraphExpansionErrorReasonV1::GraphContextInvalid, "/context")
            }
            crate::filter::SnapshotFilterError::Sqlite(_) => EngineError::Storage,
        },
    )?;
    let view = context.view.freeze();
    let history_cursor = resolve_history_cursor(&tx, request)?;
    let seed_entries =
        resolve_seeds(&tx, request, view, &context.eligibility, scope, stop_words, history_cursor)?;
    let seeds: Vec<ResolvedGraphSeedV1> =
        seed_entries.iter().map(|entry| entry.public.clone()).collect();
    let seed_source = match request.seed {
        GraphSeedV1::Query { .. } => GraphSeedSourceV1::Query,
        GraphSeedV1::Explicit { .. } => GraphSeedSourceV1::Explicit,
    };
    let (generation_id, projection_origin, projection_readiness) = if seed_source
        == GraphSeedSourceV1::Query
    {
        let (status, _) = status_cache.get_or_compute(
            &tx,
            status_slot,
            scope,
            projection_runtime_state,
            view.edge_now(),
            false,
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

    let (pushed_edge_predicates, pushed_target_predicates): (
        Vec<&PropertyPredicate>,
        Vec<&PropertyPredicate>,
    ) = if matches!(history_cursor, ResolvedHistoryCursor::AsOf(_))
        || (request.edge_predicates.is_empty() && request.target_predicates.is_empty())
    {
        (Vec::new(), Vec::new())
    } else {
        let registry = load_projection_registry(&tx).map_err(|_| EngineError::Storage)?;
        let edge_pushed = request
            .edge_predicates
            .iter()
            .filter(|pred| {
                registry.get(&(PropertyEntityClass::Edge, pred.key.clone())).is_some_and(|stored| {
                    stored.roles.contains(&ProjectionRole::Filterable) && stored.source.is_none()
                })
            })
            .collect();
        let target_pushed = if view.view.include_superseded || view.view.include_inactive {
            Vec::new()
        } else {
            request
                .target_predicates
                .iter()
                .filter(|pred| {
                    registry.get(&(PropertyEntityClass::Node, pred.key.clone())).is_some_and(
                        |stored| {
                            stored.roles.contains(&ProjectionRole::Filterable)
                                && stored.source.is_none()
                        },
                    )
                })
                .collect()
        };
        (edge_pushed, target_pushed)
    };
    let edge_boundary = match history_cursor {
        ResolvedHistoryCursor::Current => EdgeViewBoundary::Current { edge_now: view.edge_now() },
        ResolvedHistoryCursor::AsOf(as_of) => EdgeViewBoundary::AsOf {
            edge_now: view.edge_now(),
            as_of_cursor: as_of_sql_i64(as_of),
            include_out_of_window: view.view.include_out_of_window,
            check_derived_eligibility: has_dependency_or_closure_rows(&tx, scope)?,
        },
    };

    let all_seed_ids = seed_entries
        .iter()
        .flat_map(|entry| {
            entry
                .scopes
                .iter()
                .map(|seed_scope| (seed_scope.clone(), entry.public.logical_id.clone()))
        })
        .collect::<HashSet<_>>();
    let mut work_units = 0_u64;
    let mut candidates: HashMap<(String, String), GraphCandidate> = HashMap::new();
    let mut worst_candidate_id: Option<(String, String)> = None;
    let mut visited_by_seed = seed_entries
        .iter()
        .map(|entry| {
            entry
                .scopes
                .iter()
                .map(|seed_scope| (seed_scope.clone(), entry.public.logical_id.clone()))
                .collect::<HashSet<_>>()
        })
        .collect::<Vec<_>>();
    let mut frontier = seed_entries
        .iter()
        .enumerate()
        .flat_map(|(index, entry)| {
            entry
                .scopes
                .iter()
                .map(move |seed_scope| (index, seed_scope.clone(), entry.public.logical_id.clone()))
        })
        .collect::<Vec<_>>();
    for depth in 0..request.max_depth {
        frontier.sort_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
                .then_with(|| left.2.as_bytes().cmp(right.2.as_bytes()))
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
        for (seed_index, hop_scope, current) in frontier {
            let seed = &seeds[seed_index];
            let remaining = request.max_work_units.saturating_sub(work_units);
            let mut edges = load_incident_edges(
                &tx,
                &current,
                &hop_scope,
                request.direction,
                edge_boundary,
                &request.edge_kinds,
                &pushed_edge_predicates,
                remaining.saturating_add(1),
            )?;
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
                (
                    direction_rank(left_direction),
                    left.kind.as_bytes(),
                    left_next.as_bytes(),
                    left.logical_id.as_deref().unwrap_or("").as_bytes(),
                    left.write_cursor,
                )
                    .cmp(&(
                        direction_rank(right_direction),
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
                if !request.edge_kinds.is_empty()
                    && !request.edge_kinds.iter().any(|k| k.as_str() == &*edge.kind)
                {
                    continue;
                }
                let edge_visible_at_history = match history_cursor {
                    ResolvedHistoryCursor::Current => edge.superseded_at.is_none(),
                    ResolvedHistoryCursor::AsOf(as_of) => {
                        edge.write_cursor <= as_of
                            && edge.superseded_at.is_none_or(|sup| sup > as_of_sql_i64(as_of))
                    }
                };
                if !edge_visible_at_history
                    || edge.t_invalid.is_some_and(|invalid| invalid <= view.edge_now())
                {
                    continue;
                }
                // Always verify inline edge_predicates before visited_by_seed and terminal skip
                // so rejected edges never poison parallel edges and body-only JSON never leaks.
                if !request.edge_predicates.is_empty()
                    && !request
                        .edge_predicates
                        .iter()
                        .all(|pred| eval_property_predicate(edge.properties.as_deref(), pred))
                {
                    continue;
                }
                let (terminal_direction, next) = direction_and_next(&edge);
                let next_key = (hop_scope.clone(), next.to_string());
                if depth + 1 < request.max_depth
                    && !visited_by_seed[seed_index].insert(next_key.clone())
                {
                    continue;
                }
                let is_terminal_depth = depth + 1 == request.max_depth;
                if is_terminal_depth {
                    if all_seed_ids.contains(&next_key) {
                        continue;
                    }
                    if candidates.len() >= request.result_limit as usize {
                        if let Some(existing) = candidates.get(&next_key) {
                            if !origin_key_lt(
                                depth + 1,
                                seed.seed_ordinal,
                                &current,
                                terminal_direction,
                                &edge.kind,
                                next,
                                &existing.target.origin,
                            ) {
                                continue;
                            }
                        } else {
                            let worst_id = worst_candidate_id.get_or_insert_with(|| {
                                candidates
                                    .iter()
                                    .max_by(|(_, left), (_, right)| {
                                        origin_cmp(&left.target.origin, &right.target.origin)
                                    })
                                    .map(|(key, _)| key.clone())
                                    .expect("result limit is validated nonzero")
                            });
                            if !origin_key_lt(
                                depth + 1,
                                seed.seed_ordinal,
                                &current,
                                terminal_direction,
                                &edge.kind,
                                next,
                                &candidates[worst_id].target.origin,
                            ) {
                                continue;
                            }
                        }
                    }
                }
                // Only push target_predicates into SQL at terminal depth; intermediate nodes
                // failing target_predicates must still load and extend multi-hop walks.
                let hop_target_cpv: &[&PropertyPredicate] =
                    if is_terminal_depth { &pushed_target_predicates } else { &[] };
                let hop_scope_predicate =
                    crate::scope::ScopePredicate::single_for_query(&hop_scope);
                let Some(LoadedNodeRow {
                    kind,
                    body,
                    write_cursor,
                    properties: node_properties,
                    scope: _,
                }) = load_node(
                    &tx,
                    next,
                    view,
                    &context.eligibility,
                    &hop_scope_predicate,
                    hop_target_cpv,
                    history_cursor,
                )?
                else {
                    continue;
                };
                if !is_terminal_depth {
                    next_frontier.push((seed_index, hop_scope.clone(), next.to_string()));
                }
                if all_seed_ids.contains(&next_key)
                    || (!request.target_kinds.is_empty() && !request.target_kinds.contains(&kind))
                    || (!request.target_predicates.is_empty()
                        && !request
                            .target_predicates
                            .iter()
                            .all(|pred| eval_property_predicate(node_properties.as_ref(), pred)))
                {
                    continue;
                }
                let next_owned = next.to_string();
                let origin = GraphOriginV1 {
                    schema_version: 1,
                    seed_logical_id: seed.logical_id.clone(),
                    seed_ordinal: seed.seed_ordinal,
                    predecessor_logical_id: current.clone(),
                    target_logical_id: next_owned.clone(),
                    hop_count: depth + 1,
                    terminal_edge_kind: edge.kind.into_string(),
                    terminal_direction,
                    terminal_edge_properties: edge.properties.map(|props| *props),
                };
                let target = GraphTargetV1 {
                    schema_version: 1,
                    logical_id: next_owned,
                    kind,
                    body,
                    write_cursor,
                    properties: node_properties,
                    origin,
                };
                if let Some(existing) = candidates.get_mut(&next_key) {
                    if origin_cmp(&target.origin, &existing.target.origin).is_lt() {
                        *existing =
                            GraphCandidate { target, terminal_edge_cursor: edge.write_cursor };
                        worst_candidate_id = None;
                    }
                } else if candidates.len() < request.result_limit as usize {
                    candidates.insert(
                        next_key,
                        GraphCandidate { target, terminal_edge_cursor: edge.write_cursor },
                    );
                    worst_candidate_id = None;
                } else {
                    let worst = worst_candidate_id.take().unwrap_or_else(|| {
                        candidates
                            .iter()
                            .max_by(|(_, left), (_, right)| {
                                origin_cmp(&left.target.origin, &right.target.origin)
                            })
                            .map(|(key, _)| key.clone())
                            .expect("result limit is validated nonzero")
                    });
                    if origin_cmp(&target.origin, &candidates[&worst].target.origin).is_lt() {
                        candidates.remove(&worst);
                        candidates.insert(
                            next_key,
                            GraphCandidate { target, terminal_edge_cursor: edge.write_cursor },
                        );
                    } else {
                        worst_candidate_id = Some(worst);
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
            Some(crate::evidence::GraphEvidenceSidecarV1 { schema_version: 1, entries: Vec::new() })
        } else {
            let history_as_of = match history_cursor {
                ResolvedHistoryCursor::Current => None,
                ResolvedHistoryCursor::AsOf(cursor) => Some(cursor),
            };
            let preflight = crate::evidence::preflight_graph_evidence(
                &tx,
                frozen,
                history_as_of,
                &node_cursors,
                &edge_cursors,
            )
            .map_err(|error| match error {
                EngineError::Evidence(error)
                    if error.reason
                        == crate::evidence::EvidenceErrorReasonV1::EvidenceUnavailable =>
                {
                    EngineError::Evidence(crate::EvidenceErrorV1::new(
                        crate::evidence::EvidenceErrorReasonV1::EvidenceUnavailable,
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
                    direction: match candidate.target.origin.terminal_direction {
                        TraversalDirection::Outgoing => {
                            crate::evidence::GraphEvidenceDirection::Outgoing
                        }
                        TraversalDirection::Incoming => {
                            crate::evidence::GraphEvidenceDirection::Incoming
                        }
                        TraversalDirection::Both => crate::evidence::GraphEvidenceDirection::Both,
                    },
                    target_logical_id: &candidate.target.logical_id,
                    predecessor_logical_id: &candidate.target.origin.predecessor_logical_id,
                    terminal_edge_kind: &candidate.target.origin.terminal_edge_kind,
                    target_revision_id: target_material.artifact_revision_id(),
                    terminal_edge_revision_id: edge_material.artifact_revision_id(),
                    request_commitment,
                    historical: history_as_of.is_some(),
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
                entries.push(crate::evidence::GraphEvidenceSidecarEntryV1 {
                    schema_version: 1,
                    target_index: u32::try_from(index).map_err(|_| EngineError::Storage)?,
                    target_artifact_revision_id: target_revision,
                    target_evidence_ref: target_reference,
                    terminal_edge_artifact_revision_id: edge_revision,
                    terminal_edge_evidence_ref: edge_reference,
                });
            }
            Some(crate::evidence::GraphEvidenceSidecarV1 { schema_version: 1, entries })
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
                let dependency_state = match history_cursor {
                    ResolvedHistoryCursor::Current => {
                        structural_dependency_state(&tx, target.write_cursor, view.edge_now())
                    }
                    ResolvedHistoryCursor::AsOf(as_of) => structural_dependency_state_as_of(
                        &tx,
                        target.write_cursor,
                        view.edge_now(),
                        as_of,
                        view.view.include_out_of_window,
                    ),
                }
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

fn canonical_predicate_bytes(pred: &PropertyPredicate) -> Vec<u8> {
    let mut out = Vec::new();
    frozen_read::encode_string(&mut out, &pred.key);
    out.push(match pred.op {
        PropertyComparisonOp::Eq => 0,
        PropertyComparisonOp::Ne => 1,
        PropertyComparisonOp::Lt => 2,
        PropertyComparisonOp::Lte => 3,
        PropertyComparisonOp::Gt => 4,
        PropertyComparisonOp::Gte => 5,
    });
    match &pred.value {
        PropertyValue::String(s) => {
            out.push(0);
            frozen_read::encode_string(&mut out, s);
        }
        PropertyValue::Int(i) => {
            out.push(1);
            out.extend_from_slice(&i.to_be_bytes());
        }
        PropertyValue::Float(f) => {
            out.push(2);
            let norm = normalize_finite_f64(*f).unwrap_or(0.0);
            out.extend_from_slice(&norm.to_bits().to_be_bytes());
        }
        PropertyValue::Bool(b) => {
            out.push(3);
            out.push(u8::from(*b));
        }
    }
    out
}

fn encode_sorted_predicates(bytes: &mut Vec<u8>, predicates: &[PropertyPredicate]) {
    let mut encoded: Vec<Vec<u8>> = predicates.iter().map(canonical_predicate_bytes).collect();
    encoded.sort_unstable();
    frozen_read::encode_u32(bytes, u32::try_from(encoded.len()).unwrap_or(u32::MAX));
    for item in encoded {
        bytes.extend_from_slice(&item);
    }
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
    if !value.edge_predicates.is_empty() || !value.target_predicates.is_empty() {
        bytes.push(1);
        encode_sorted_predicates(&mut bytes, &value.edge_predicates);
        encode_sorted_predicates(&mut bytes, &value.target_predicates);
    }
    if let Some(cursor) = value.as_of_write_cursor {
        bytes.push(2);
        frozen_read::encode_u64(&mut bytes, cursor);
    } else if let Some(epoch_ms) = value.as_of_epoch_ms {
        bytes.push(3);
        frozen_read::encode_i64(&mut bytes, epoch_ms);
    }
    bytes
}
