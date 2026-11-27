use std::fmt::{Display, Formatter};
#[cfg(feature = "test-hooks")]
use std::sync::atomic::{AtomicU64, Ordering};
#[cfg(feature = "test-hooks")]
use std::sync::Arc;
use std::sync::Mutex;

use crate::embedding::embedder_required_for;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};

use super::{
    connection_has_pending_projection_work, current_epoch_seconds, load_projection_registry,
    pending_embedding_work, unsupported_vector_kinds, valid_caller_identity, DenseReadiness,
    EmbeddingReadiness, EmbeddingReadinessState, Engine, EngineError, ProjectionRuntimeStatus,
    ProjectionRuntimeStatusEntry, ProjectionRuntimeUnavailabilityReason,
    ProjectionStatusDenseReadiness, StoredProjection,
};

const PREFIX: &str = "pgen1:";

/// Database-local identity of one in-place serving-projection epoch.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct ProjectionGenerationId(String);

impl ProjectionGenerationId {
    /// Parse a versioned projection-generation identity.
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectionGenerationError> {
        Self::parse(value.into())
    }

    /// Return the stable versioned textual identity.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    fn parse(value: String) -> Result<Self, ProjectionGenerationError> {
        let valid = value.strip_prefix(PREFIX).is_some_and(|suffix| {
            suffix.len() == 32
                && suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        });
        if valid {
            Ok(Self(value))
        } else {
            Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::InvalidGenerationId,
                "/expectedGenerationId",
            ))
        }
    }
}

impl Display for ProjectionGenerationId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Closed serving-generation origin.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionGenerationOriginV1 {
    Fresh,
    LegacyUnverified,
    Configuration,
    Rebuild,
}

impl ProjectionGenerationOriginV1 {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "fresh" => Some(Self::Fresh),
            "legacy_unverified" => Some(Self::LegacyUnverified),
            "configuration" => Some(Self::Configuration),
            "rebuild" => Some(Self::Rebuild),
            _ => None,
        }
    }

    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Fresh => "fresh",
            Self::LegacyUnverified => "legacy_unverified",
            Self::Configuration => "configuration",
            Self::Rebuild => "rebuild",
        }
    }
}

/// Truthful completeness state for the current physical serving generation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionReadinessV1 {
    Ready,
    Processing,
    Blocked,
    Degraded,
}

impl ProjectionReadinessV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Processing => "processing",
            Self::Blocked => "blocked",
            Self::Degraded => "degraded",
        }
    }
}

/// Runtime availability considered separately from physical completeness.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionRuntimeStateV1 {
    Absent,
    Usable,
}

impl ProjectionRuntimeStateV1 {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Absent => "absent",
            Self::Usable => "usable",
        }
    }
}

/// Current serving-generation status from one SQLite snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionGenerationStatusV1 {
    pub schema_version: u32,
    pub generation_id: ProjectionGenerationId,
    pub declaration_sha256: String,
    pub origin: ProjectionGenerationOriginV1,
    pub transition_boundary: u64,
    pub effective_at_epoch_s: i64,
    pub observed_boundary: u64,
    pub ready_through: u64,
    pub readiness: ProjectionReadinessV1,
    pub runtime_state: ProjectionRuntimeStateV1,
    pub pending_count: u64,
    pub failed_count: u64,
}

/// Point request for one Slice-25 pending projection cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationProjectionStatusRequestV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub write_cursor: u64,
    pub expected_generation_id: ProjectionGenerationId,
}

/// Point status for one receipt-owned pending projection cursor.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MutationProjectionStatusV1 {
    pub schema_version: u32,
    pub operation_id: String,
    pub write_cursor: u64,
    pub generation_id: ProjectionGenerationId,
    pub effective_at_epoch_s: i64,
    pub observed_boundary: u64,
    pub ready_through: u64,
    pub readiness: ProjectionReadinessV1,
    pub runtime_state: ProjectionRuntimeStateV1,
    pub pending_count: u64,
    pub failed_count: u64,
}

pub(crate) const STATUS_CACHE_MAX_ENTRIES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StatusCacheConnectionSlot {
    ReaderWorker(usize),
    Primary,
}

impl StatusCacheConnectionSlot {
    fn index(self, reader_worker_count: usize) -> usize {
        match self {
            Self::ReaderWorker(worker_idx) => {
                debug_assert!(worker_idx < reader_worker_count);
                worker_idx
            }
            Self::Primary => reader_worker_count,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SlotObservation {
    data_version: u64,
    total_changes: u64,
    visibility_generation: u64,
    state_nonce: String,
    epoch: u64,
}

#[derive(Clone)]
struct CachedStatusEntry {
    cache_epoch: u64,
    visibility_generation: u64,
    state_nonce: String,
    schema_version: u32,
    authoritative_boundary: u64,
    effective_from_epoch_s: i64,
    valid_before_epoch_s: Option<i64>,
    runtime_state: ProjectionRuntimeStateV1,
    is_unscoped: bool,
    scope_read_scopes: Vec<String>,
    status: ProjectionGenerationStatusV1,
    coverage_counts: Option<crate::StoredHeadCoverageCounts>,
}

struct StatusCacheState {
    cache_epoch: u64,
    max_visibility_generation: u64,
    slots: Vec<Option<SlotObservation>>,
    entries: Vec<CachedStatusEntry>,
}

pub(crate) struct ProjectionGenerationStatusCache {
    reader_worker_count: usize,
    state: Mutex<StatusCacheState>,
    #[cfg(feature = "test-hooks")]
    full_owner_scan_count: Arc<AtomicU64>,
}

impl ProjectionGenerationStatusCache {
    pub(crate) fn new(
        reader_worker_count: usize,
        #[cfg(feature = "test-hooks")] full_owner_scan_count: Arc<AtomicU64>,
    ) -> Self {
        Self {
            reader_worker_count,
            state: Mutex::new(StatusCacheState {
                cache_epoch: 0,
                max_visibility_generation: 0,
                slots: vec![None; reader_worker_count + 1],
                entries: Vec::new(),
            }),
            #[cfg(feature = "test-hooks")]
            full_owner_scan_count,
        }
    }

    pub(crate) fn observe_connection(
        &self,
        connection: &Connection,
        slot: StatusCacheConnectionSlot,
    ) -> Result<(), EngineError> {
        let (visibility_generation, state_nonce) = read_visibility_snapshot(connection)?;
        let data_version: u64 = connection
            .pragma_query_value(Some("main"), "data_version", |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let total_changes = connection.total_changes();
        let slot_idx = slot.index(self.reader_worker_count);
        let mut state = self.state.lock().map_err(|_| EngineError::Storage)?;
        state.max_visibility_generation =
            state.max_visibility_generation.max(visibility_generation);
        let epoch = state.cache_epoch;
        state.slots[slot_idx] = Some(SlotObservation {
            data_version,
            total_changes,
            visibility_generation,
            state_nonce,
            epoch,
        });
        Ok(())
    }

    /// Seed a pooled reader slot at startup without querying any table (avoiding
    /// eager SQLite `sqlite3Init()` schema parsing across all 8 reader connections
    /// during `Engine::open`).
    ///
    /// Reads only `PRAGMA main.data_version` (and `total_changes`) on the reader
    /// connection, and inherits `(visibility_generation, state_nonce)` from the
    /// `Primary` slot that `Engine::open` already observed under the product lock.
    /// If an external writer tampers after open (even without touching a
    /// visibility-triggered table), the reader's `PRAGMA main.data_version` bumps
    /// on its first read transaction and Subcase A1 invalidates immediately.
    pub(crate) fn observe_reader_startup(
        &self,
        connection: &Connection,
        slot: StatusCacheConnectionSlot,
    ) -> Result<(), EngineError> {
        let data_version: u64 = connection
            .pragma_query_value(Some("main"), "data_version", |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let total_changes = connection.total_changes();
        let slot_idx = slot.index(self.reader_worker_count);
        let primary_idx = StatusCacheConnectionSlot::Primary.index(self.reader_worker_count);
        let mut state = self.state.lock().map_err(|_| EngineError::Storage)?;
        let Some(primary) = state.slots[primary_idx].clone() else {
            return Ok(());
        };
        let epoch = state.cache_epoch;
        state.slots[slot_idx] = Some(SlotObservation {
            data_version,
            total_changes,
            visibility_generation: primary.visibility_generation,
            state_nonce: primary.state_nonce,
            epoch,
        });
        Ok(())
    }

    pub(crate) fn snapshot_epoch(&self) -> Result<u64, EngineError> {
        Ok(self.state.lock().map_err(|_| EngineError::Storage)?.cache_epoch)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn get_or_compute(
        &self,
        tx: &Connection,
        slot: StatusCacheConnectionSlot,
        observed_epoch: u64,
        scope: &crate::scope::ScopePredicate,
        runtime_state: ProjectionRuntimeStateV1,
        effective_at_epoch_s: i64,
        capture_coverage: bool,
    ) -> Result<
        (ProjectionGenerationStatusV1, Option<crate::stored_coverage::CachedStoredHeadCoverage>),
        EngineError,
    > {
        // Step 1: O(1) unlocked snapshot metadata read.
        let (visibility_generation, state_nonce) = read_visibility_snapshot(tx)?;
        let schema_version: u32 = tx
            .pragma_query_value(Some("main"), "schema_version", |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let data_version: u64 = tx
            .pragma_query_value(Some("main"), "data_version", |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let total_changes = tx.total_changes();
        let boundary = snapshot_observed_boundary(tx)?;
        let slot_idx = slot.index(self.reader_worker_count);

        // Step 2: Slot-verified cache probe (brief mutex hold).
        let (miss_epoch, allow_insert) = {
            let mut state = self.state.lock().map_err(|_| EngineError::Storage)?;
            if observed_epoch != state.cache_epoch {
                // `tx` may have been pinned before `cache_epoch` bumped; uncached pass-through.
                (state.cache_epoch, false)
            } else {
                let prev_matches_vis = state.slots[slot_idx].as_ref().is_some_and(|prev| {
                    prev.visibility_generation == visibility_generation
                        && prev.state_nonce == state_nonce
                });
                if prev_matches_vis {
                    let prev = state.slots[slot_idx].as_ref().expect("slot observation present");
                    if prev.data_version != data_version || prev.total_changes != total_changes {
                        // Subcase A1: out-of-band external tamper or same-connection mutation.
                        state.cache_epoch = state.cache_epoch.wrapping_add(1);
                        state.entries.clear();
                        let next_epoch = state.cache_epoch;
                        state.slots[slot_idx] = Some(SlotObservation {
                            data_version,
                            total_changes,
                            visibility_generation,
                            state_nonce: state_nonce.clone(),
                            epoch: next_epoch,
                        });
                        (next_epoch, true)
                    } else if prev.epoch != state.cache_epoch {
                        // Subcase A2: pinned to a pre-tamper snapshot; uncached pass-through.
                        (state.cache_epoch, false)
                    } else {
                        // Subcase A3: slot verified clean for (vis_gen, nonce, cache_epoch).
                        let cache_epoch = state.cache_epoch;
                        let matched = state.entries.iter().rposition(|entry| {
                            entry.cache_epoch == cache_epoch
                                && entry.visibility_generation == visibility_generation
                                && entry.state_nonce == state_nonce
                                && entry.schema_version == schema_version
                                && entry.authoritative_boundary == boundary
                                && effective_at_epoch_s >= entry.effective_from_epoch_s
                                && entry
                                    .valid_before_epoch_s
                                    .is_none_or(|upper| effective_at_epoch_s < upper)
                                && entry.runtime_state == runtime_state
                                && entry.is_unscoped == scope.is_unscoped()
                                && entry.scope_read_scopes.as_slice() == scope.read_scopes()
                                && (!capture_coverage || entry.coverage_counts.is_some())
                        });
                        if let Some(pos) = matched {
                            let entry = state.entries.remove(pos);
                            let mut status = entry.status.clone();
                            status.effective_at_epoch_s = effective_at_epoch_s;
                            let counts =
                                if capture_coverage { entry.coverage_counts.clone() } else { None };
                            state.entries.push(entry);
                            drop(state);
                            let coverage = if capture_coverage {
                                match counts {
                                    Some(counts) => {
                                        crate::stored_coverage::token(tx)?.map(|token| {
                                            crate::stored_coverage::CachedStoredHeadCoverage {
                                                token,
                                                counts,
                                            }
                                        })
                                    }
                                    None => None,
                                }
                            } else {
                                None
                            };
                            return Ok((status, coverage));
                        }
                        (cache_epoch, true)
                    }
                } else {
                    // Case B: first visit by this slot to this (vis_gen, nonce); force verification.
                    (state.cache_epoch, true)
                }
            }
        };

        // Step 3: Full snapshot computation (unlocked).
        #[cfg(feature = "test-hooks")]
        self.full_owner_scan_count.fetch_add(1, Ordering::Relaxed);
        let (status, counts) = status_with_capture_in_snapshot(
            tx,
            scope,
            runtime_state,
            effective_at_epoch_s,
            boundary,
            true,
        )?;
        let coverage = if capture_coverage {
            match counts.as_ref() {
                Some(counts) => crate::stored_coverage::token(tx)?.map(|token| {
                    crate::stored_coverage::CachedStoredHeadCoverage {
                        token,
                        counts: counts.clone(),
                    }
                }),
                None => None,
            }
        } else {
            None
        };
        let valid_before_epoch_s =
            next_status_membership_boundary_in_scope(tx, effective_at_epoch_s, scope)?;

        // Step 4: Epoch- & generation-guarded insertion (brief mutex hold).
        {
            let mut state = self.state.lock().map_err(|_| EngineError::Storage)?;
            if !allow_insert
                || state.cache_epoch != miss_epoch
                || visibility_generation < state.max_visibility_generation
            {
                return Ok((status, coverage));
            }
            if visibility_generation > state.max_visibility_generation {
                state.max_visibility_generation = visibility_generation;
                state.entries.retain(|entry| entry.visibility_generation >= visibility_generation);
            }
            state.entries.retain(|entry| {
                !(entry.cache_epoch == miss_epoch
                    && entry.visibility_generation == visibility_generation
                    && entry.state_nonce == state_nonce
                    && entry.schema_version == schema_version
                    && entry.authoritative_boundary == boundary
                    && entry.effective_from_epoch_s == effective_at_epoch_s
                    && entry.valid_before_epoch_s == valid_before_epoch_s
                    && entry.runtime_state == runtime_state
                    && entry.is_unscoped == scope.is_unscoped()
                    && entry.scope_read_scopes.as_slice() == scope.read_scopes())
            });
            while state.entries.len() >= STATUS_CACHE_MAX_ENTRIES {
                state.entries.remove(0);
            }
            state.entries.push(CachedStatusEntry {
                cache_epoch: miss_epoch,
                visibility_generation,
                state_nonce: state_nonce.clone(),
                schema_version,
                authoritative_boundary: boundary,
                effective_from_epoch_s: effective_at_epoch_s,
                valid_before_epoch_s,
                runtime_state,
                is_unscoped: scope.is_unscoped(),
                scope_read_scopes: scope.read_scopes().to_vec(),
                status: status.clone(),
                coverage_counts: counts,
            });
            state.slots[slot_idx] = Some(SlotObservation {
                data_version,
                total_changes,
                visibility_generation,
                state_nonce,
                epoch: miss_epoch,
            });
        }

        Ok((status, coverage))
    }
}

#[derive(Clone)]
pub(crate) struct CachedMutationProjectionStatus {
    visibility_generation: u64,
    data_version: u64,
    total_changes: u64,
    authoritative_boundary: u64,
    effective_at_epoch_s: i64,
    runtime_state: ProjectionRuntimeStateV1,
    is_unscoped: bool,
    scope_read_scopes: Vec<String>,
    request: MutationProjectionStatusRequestV1,
    status: MutationProjectionStatusV1,
}

/// Closed projection-generation failure reason.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionGenerationErrorReason {
    UnsupportedSchemaVersion,
    UnknownField,
    InvalidOperationId,
    InvalidWriteCursor,
    InvalidGenerationId,
    MutationNotTracked,
    WrongProjectionGeneration,
    ProjectionGenerationUnavailable,
    ProjectionGenerationCorrupt,
}

impl ProjectionGenerationErrorReason {
    /// Stable lower-snake-case wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::UnknownField => "unknown_field",
            Self::InvalidOperationId => "invalid_operation_id",
            Self::InvalidWriteCursor => "invalid_write_cursor",
            Self::InvalidGenerationId => "invalid_generation_id",
            Self::MutationNotTracked => "mutation_not_tracked",
            Self::WrongProjectionGeneration => "wrong_projection_generation",
            Self::ProjectionGenerationUnavailable => "projection_generation_unavailable",
            Self::ProjectionGenerationCorrupt => "projection_generation_corrupt",
        }
    }
}

/// Typed projection-generation error with a privacy-safe request pointer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionGenerationError {
    pub reason: ProjectionGenerationErrorReason,
    pub field_path: String,
}

impl ProjectionGenerationError {
    fn new(reason: ProjectionGenerationErrorReason, field_path: impl Into<String>) -> Self {
        Self { reason, field_path: field_path.into() }
    }
}

impl Display for ProjectionGenerationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} at {}", self.reason.as_str(), self.field_path)
    }
}

impl std::error::Error for ProjectionGenerationError {}

#[derive(Clone)]
pub(crate) struct GenerationRow {
    pub(crate) id: ProjectionGenerationId,
    pub(crate) digest: String,
    pub(crate) boundary: u64,
    pub(crate) origin: ProjectionGenerationOriginV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Completion {
    Complete,
    Pending,
    Failed,
}

#[derive(Clone, Debug)]
struct CompletionSummary {
    observed_boundary: u64,
    ready_through: u64,
    pending_count: u64,
    failed_count: u64,
}

#[derive(Default)]
struct CompletionAccumulator {
    pending_count: u64,
    failed_count: u64,
    first_incomplete: Option<u64>,
}

impl CompletionAccumulator {
    fn add(&mut self, cursor: u64, completion: Completion) -> Result<(), EngineError> {
        match completion {
            Completion::Complete => return Ok(()),
            Completion::Pending => {
                self.pending_count =
                    self.pending_count.checked_add(1).ok_or(EngineError::Storage)?;
            }
            Completion::Failed => {
                self.failed_count = self.failed_count.checked_add(1).ok_or(EngineError::Storage)?;
            }
        }
        self.first_incomplete =
            Some(self.first_incomplete.map_or(cursor, |first| first.min(cursor)));
        Ok(())
    }

    fn add_aggregate(
        &mut self,
        pending_count: u64,
        failed_count: u64,
        first_incomplete: Option<u64>,
    ) -> Result<(), EngineError> {
        self.pending_count =
            self.pending_count.checked_add(pending_count).ok_or(EngineError::Storage)?;
        self.failed_count =
            self.failed_count.checked_add(failed_count).ok_or(EngineError::Storage)?;
        if let Some(cursor) = first_incomplete {
            self.first_incomplete =
                Some(self.first_incomplete.map_or(cursor, |first| first.min(cursor)));
        }
        Ok(())
    }
}

fn corruption() -> EngineError {
    ProjectionGenerationError::new(
        ProjectionGenerationErrorReason::ProjectionGenerationCorrupt,
        "/projectionGeneration",
    )
    .into()
}

pub(crate) fn corruption_error() -> EngineError {
    corruption()
}

fn count(connection: &Connection, table: &str) -> Result<u64, EngineError> {
    let query = format!("SELECT COUNT(*) FROM {table}");
    connection.query_row(&query, [], |row| row.get::<_, u64>(0)).map_err(|_| EngineError::Storage)
}

pub(crate) fn snapshot_observed_boundary(connection: &Connection) -> Result<u64, EngineError> {
    let base = authoritative_boundary(connection)?;
    let floor = crate::physical_erasure::cursor_floor(connection)?;
    Ok(base.max(floor))
}

fn read_visibility_snapshot(connection: &Connection) -> Result<(u64, String), EngineError> {
    let (generation, state_nonce, coalescing): (u64, String, i64) = connection
        .query_row(
            "SELECT generation, state_nonce, coalescing \
             FROM _fathomdb_read_visibility_state WHERE singleton = 1",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| corruption())?;
    let valid_nonce = state_nonce.len() == 64
        && state_nonce.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b));
    if !valid_nonce || coalescing != 0 || generation > i64::MAX as u64 {
        return Err(corruption());
    }
    Ok((generation, state_nonce))
}

fn authoritative_boundary(connection: &Connection) -> Result<u64, EngineError> {
    let mut boundary = 0_u64;
    for table in
        ["canonical_nodes", "canonical_edges", "operational_mutations", "operational_state"]
    {
        let query = format!("SELECT COALESCE(MAX(write_cursor),0) FROM {table}");
        let value = connection
            .query_row(&query, [], |row| row.get::<_, u64>(0))
            .map_err(|_| EngineError::Storage)?;
        boundary = boundary.max(value);
    }
    let commit_timestamps = connection
        .query_row(
            "SELECT COALESCE(MAX(write_cursor),0) FROM _fathomdb_commit_timestamps",
            [],
            |row| row.get::<_, u64>(0),
        )
        .unwrap_or(0);
    let reserved = connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key=?1",
            [fathomdb_schema::RESERVED_WRITE_CURSOR_KEY],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?
        .map(|raw| {
            raw.parse::<u64>().ok().filter(|value| value.to_string() == raw).ok_or_else(corruption)
        })
        .transpose()?
        .unwrap_or(0);
    let closure_boundary = connection
        .query_row(
            "SELECT COALESCE(MAX(admitted_write_boundary),0) \
             FROM _fathomdb_dependency_closures",
            [],
            |row| row.get::<_, u64>(0),
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(boundary.max(commit_timestamps).max(reserved).max(closure_boundary))
}

fn bootstrap_is_fresh(connection: &Connection, schema_version: u32) -> Result<bool, EngineError> {
    for table in [
        "canonical_nodes",
        "canonical_edges",
        "operational_mutations",
        "operational_state",
        "_fathomdb_artifact_revisions",
        "_fathomdb_source_versions",
        "_fathomdb_source_links",
        "_fathomdb_source_dependencies",
        "_fathomdb_dependency_closures",
        "_fathomdb_actuation_receipts",
        "_fathomdb_actuation_receipt_source_refs",
        "_fathomdb_projection_registry",
        "canonical_attributes",
        "_fathomdb_projection_state",
        "_fathomdb_projection_terminal",
        "_fathomdb_vector_kinds",
        "_fathomdb_vector_rows",
        "_fathomdb_vector_spans",
        "_fathomdb_embed_probe",
        "search_index",
        "search_index_v2",
        "search_index_edges",
        "property_search_index",
        "vector_default",
    ] {
        if table == "_fathomdb_vector_spans" && schema_version < 35 {
            continue;
        }
        if count(connection, table)? != 0 {
            return Ok(false);
        }
    }

    if schema_version >= 36 && count(connection, "_fathomdb_erasure_requests")? != 0 {
        return Ok(false);
    }
    if schema_version >= 38
        && (count(connection, "canonical_property_values")? != 0
            || count(connection, "_fathomdb_commit_timestamps")? != 0)
    {
        return Ok(false);
    }
    if schema_version >= 39 && count(connection, "_fathomdb_scope_visibility_watermarks")? != 0 {
        return Ok(false);
    }

    let seeded_collection_is_exact: bool = connection
        .query_row(
            "SELECT COUNT(*)=1 AND EXISTS(SELECT 1 FROM operational_collections \
             WHERE name='projection_failures' AND kind='append_only_log' \
               AND schema_json='{\"type\":\"object\"}' AND retention_json='{}' \
               AND format_version=1 AND created_at=0)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !seeded_collection_is_exact {
        return Ok(false);
    }

    let profiles: (u64, u64) = connection
        .query_row(
            "SELECT COUNT(*),COALESCE(SUM(mean_vec IS NOT NULL),0) \
             FROM _fathomdb_embedder_profiles",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    if profiles != (1, 0) {
        return Ok(false);
    }

    let mut statement = connection
        .prepare("SELECT key,value FROM _fathomdb_open_state ORDER BY key")
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|_| EngineError::Storage)?;
    for row in rows {
        let (key, value) = row.map_err(|_| EngineError::Storage)?;
        let baseline = match key.as_str() {
            "_fathomdb_physical_erasure_policy" if schema_version >= 36 => {
                matches!(value.as_str(), "autonomous" | "explicit_maintenance")
            }
            "_fathomdb_erasure_store" if schema_version >= 36 => {
                value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            "_fathomdb_database_id" => {
                value.len() == 32 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            "_fathomdb_read_context_key" => {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            "_fathomdb_dependency_generation"
            | "_fathomdb_closure_sequence"
            | "projection_cursor"
            | "tc33_reserved_write_cursor" => value == "0",
            "search_index_tokenizer_reproject_complete" | "tc33_edge_vector_prune_complete" => {
                value == "1"
            }
            _ => false,
        };
        if !baseline {
            return Ok(false);
        }
    }
    Ok(authoritative_boundary(connection)? == 0)
}

fn put_scalar(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u64).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

fn put_declaration_base_row(
    bytes: &mut Vec<u8>,
    name: &str,
    roles_json: &str,
    fts: Option<String>,
    embedder: Option<String>,
    declared: i64,
    source: Option<String>,
) -> Result<(), EngineError> {
    put_scalar(bytes, name);
    let mut roles: Vec<String> = roles_json
        .split(',')
        .map(str::trim)
        .filter(|role| !role.is_empty())
        .map(str::to_string)
        .collect();
    roles.sort();
    bytes.extend_from_slice(&(roles.len() as u64).to_be_bytes());
    for role in roles {
        put_scalar(bytes, &role);
    }
    for value in [fts, embedder] {
        match value {
            Some(value) => {
                bytes.push(1);
                put_scalar(bytes, &value);
            }
            None => bytes.push(0),
        }
    }
    bytes.push(u8::from(declared != 0));
    match source {
        Some(value) => {
            bytes.push(1);
            let segments: Vec<String> = serde_json::from_str(&value).map_err(|_| corruption())?;
            bytes.extend_from_slice(&(segments.len() as u64).to_be_bytes());
            for segment in segments {
                put_scalar(bytes, &segment);
            }
        }
        None => bytes.push(0),
    }
    Ok(())
}

fn declaration_digest(connection: &Connection) -> Result<String, EngineError> {
    let schema_version: u32 = connection
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    let mut bytes = b"fathomdb.projection-serving-declaration.v1\0".to_vec();
    put_scalar(&mut bytes, "projection-serving-set/1");
    for arm in
        ["node_fts_v2", "edge_fts_v1", "attributes_v1", "property_fts_v1", "dense_default_v1"]
    {
        put_scalar(&mut bytes, arm);
    }
    if schema_version >= 38 {
        let mut statement = connection
            .prepare(
                "SELECT name,roles,fts_tokenizer,vector_embedder,vector_declared,source,entity_class,vector_kinds \
                 FROM _fathomdb_projection_registry ORDER BY CAST(name AS BLOB), entity_class",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, Option<String>>(7)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| EngineError::Storage)?;
        bytes.extend_from_slice(&(rows.len() as u64).to_be_bytes());
        for (name, roles_json, fts, embedder, declared, source, entity_class, vector_kinds) in rows
        {
            put_declaration_base_row(
                &mut bytes,
                &name,
                &roles_json,
                fts,
                embedder,
                declared,
                source,
            )?;
            put_scalar(&mut bytes, &entity_class);
            match vector_kinds {
                Some(value) => {
                    bytes.push(1);
                    put_scalar(&mut bytes, &value);
                }
                None => bytes.push(0),
            }
        }
    } else {
        let mut statement = connection
            .prepare(
                "SELECT name,roles,fts_tokenizer,vector_embedder,vector_declared,source \
                 FROM _fathomdb_projection_registry ORDER BY CAST(name AS BLOB)",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, Option<String>>(5)?,
                ))
            })
            .map_err(|_| EngineError::Storage)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| EngineError::Storage)?;
        bytes.extend_from_slice(&(rows.len() as u64).to_be_bytes());
        for (name, roles_json, fts, embedder, declared, source) in rows {
            put_declaration_base_row(
                &mut bytes,
                &name,
                &roles_json,
                fts,
                embedder,
                declared,
                source,
            )?;
        }
    }
    let profile: (String, String, String, u32) = connection
        .query_row(
            "SELECT profile,name,revision,dimension FROM _fathomdb_embedder_profiles \
             WHERE profile='default'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    for value in [profile.0, profile.1, profile.2] {
        put_scalar(&mut bytes, &value);
    }
    bytes.extend_from_slice(&u64::from(profile.3).to_be_bytes());
    Ok(crate::identity::hex_encode(&Sha256::digest(bytes)))
}

fn mint_id_from_candidates(
    connection: &Connection,
    mut next_candidate: impl FnMut() -> Result<String, EngineError>,
) -> Result<ProjectionGenerationId, EngineError> {
    for _ in 0..4 {
        let suffix = next_candidate()?;
        let id = ProjectionGenerationId(format!("{PREFIX}{suffix}"));
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_projection_generations \
                 WHERE generation_id=?1)",
                [id.as_str()],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        if !exists {
            return Ok(id);
        }
    }
    Err(EngineError::Storage)
}

fn mint_id(connection: &Connection) -> Result<ProjectionGenerationId, EngineError> {
    mint_id_from_candidates(connection, || {
        connection
            .query_row("SELECT lower(hex(randomblob(16)))", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)
    })
}

/// Install a clean serving projection generation inside an open restore
/// transaction (`fathomdb.logical-export/v1`). Retires any pre-existing empty
/// bootstrap generation, or mints the initial serving row directly when the
/// target database was bootstrapped without opening an `Engine`.
#[cfg(any(test, feature = "test-hooks", feature = "operator"))]
pub(crate) fn install_restored_generation(
    connection: &Connection,
) -> Result<ProjectionGenerationId, EngineError> {
    let existing: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_projection_generations", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if existing > 0 {
        return transition(connection, ProjectionGenerationOriginV1::Rebuild);
    }
    let boundary = authoritative_boundary(connection)?;
    let digest = declaration_digest(connection)?;
    let id = mint_id(connection)?;
    connection
        .execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,?1,?2,?3,'serving','rebuild')",
            params![id.as_str(), digest, boundary],
        )
        .map_err(|_| EngineError::Storage)?;
    connection
        .execute(
            "INSERT INTO _fathomdb_projection_generation_current(singleton,generation_id) \
             VALUES(1,?1)",
            [id.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(id)
}

/// Retire the current in-place serving epoch and install its successor inside
/// the caller's already-open write transaction.
pub(crate) fn transition(
    connection: &Connection,
    origin: ProjectionGenerationOriginV1,
) -> Result<ProjectionGenerationId, EngineError> {
    let current: String = connection
        .query_row(
            "SELECT generation_id FROM _fathomdb_projection_generation_current WHERE singleton=1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| corruption())?;
    ProjectionGenerationId::parse(current.clone()).map_err(|_| corruption())?;
    let boundary = authoritative_boundary(connection)?;
    let digest = declaration_digest(connection)?;
    let next = mint_id(connection)?;
    let changed = connection
        .execute(
            "UPDATE _fathomdb_projection_generations SET role='retired',retired_boundary=?2 \
             WHERE generation_id=?1 AND role='serving' AND retired_boundary IS NULL",
            params![current, boundary],
        )
        .map_err(|_| EngineError::Storage)?;
    if changed != 1 {
        return Err(corruption());
    }
    connection
        .execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,?1,?2,?3,'serving',?4)",
            params![next.as_str(), digest, boundary, origin.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
    connection
        .execute(
            "UPDATE _fathomdb_projection_generation_current SET generation_id=?1 \
             WHERE singleton=1",
            [next.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
    Ok(next)
}

pub(crate) fn bootstrap(
    connection: &mut Connection,
    schema_version: u32,
    allow_populated_legacy_bootstrap: bool,
) -> Result<(), EngineError> {
    if schema_version < 32 {
        return Ok(());
    }
    let generation_rows = count(connection, "_fathomdb_projection_generations")?;
    let current_rows = count(connection, "_fathomdb_projection_generation_current")?;
    if generation_rows == 0 && current_rows == 0 {
        let fresh = bootstrap_is_fresh(connection, schema_version)?;
        if !fresh && !allow_populated_legacy_bootstrap {
            return Err(corruption());
        }
        let origin = if fresh { "fresh" } else { "legacy_unverified" };
        let boundary = authoritative_boundary(connection)?;
        let digest = declaration_digest(connection)?;
        let id = mint_id(connection)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        // Only the private historical token fixtures bootstrap older layouts.
        #[cfg(any(test, feature = "migration-test-hooks"))]
        let historical_fixture = (32..37).contains(&schema_version);
        #[cfg(not(any(test, feature = "migration-test-hooks")))]
        let historical_fixture = false;
        if !historical_fixture {
            crate::frozen_read::require_clean_visibility(&tx)?;
        }
        tx.execute(
            "INSERT INTO _fathomdb_projection_generations(\
               schema_version,generation_id,declaration_sha256,transition_boundary,role,origin\
             ) VALUES(1,?1,?2,?3,'serving',?4)",
            params![id.as_str(), digest, boundary, origin],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.execute(
            "INSERT INTO _fathomdb_projection_generation_current(singleton,generation_id) \
             VALUES(1,?1)",
            [id.as_str()],
        )
        .map_err(|_| EngineError::Storage)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        return Ok(());
    }
    if generation_rows == 0 || current_rows != 1 {
        return Err(corruption());
    }
    current_generation(connection).map(|_| ())
}

pub(crate) fn current_generation(connection: &Connection) -> Result<GenerationRow, EngineError> {
    let authority_counts: (u64, u64, u64) = connection
        .query_row(
            "SELECT COUNT(*),SUM(role='serving'),\
                    (SELECT COUNT(*) FROM _fathomdb_projection_generation_current) \
             FROM _fathomdb_projection_generations",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|_| corruption())?;
    if authority_counts.0 == 0 || authority_counts.1 != 1 || authority_counts.2 != 1 {
        return Err(corruption());
    }
    let mut history = connection
        .prepare(
            "SELECT generation_id,declaration_sha256,transition_boundary,role,origin,\
                    retired_boundary,schema_version \
             FROM _fathomdb_projection_generations",
        )
        .map_err(|_| corruption())?;
    let rows = history
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, Option<i64>>(5)?,
                row.get::<_, i64>(6)?,
            ))
        })
        .map_err(|_| corruption())?;
    for row in rows {
        let (id, digest, boundary, role, origin, retired, schema) =
            row.map_err(|_| corruption())?;
        let role_is_valid = match (role.as_str(), retired) {
            ("serving", None) => true,
            ("retired", Some(value)) => value >= 0 && value >= boundary,
            _ => false,
        };
        if ProjectionGenerationId::parse(id).is_err()
            || digest.len() != 64
            || digest.bytes().any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
            || boundary < 0
            || ProjectionGenerationOriginV1::parse(&origin).is_none()
            || schema != 1
            || !role_is_valid
        {
            return Err(corruption());
        }
    }
    let row: Option<(String, String, u64, String, String, Option<u64>)> = connection
        .query_row(
            "SELECT g.generation_id,g.declaration_sha256,g.transition_boundary,g.origin,\
                    g.role,g.retired_boundary \
             FROM _fathomdb_projection_generation_current c \
             JOIN _fathomdb_projection_generations g ON g.generation_id=c.generation_id \
             WHERE c.singleton=1",
            [],
            |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
            },
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((id, digest, boundary, origin, role, retired)) = row else {
        return Err(corruption());
    };
    if role != "serving"
        || retired.is_some()
        || digest.len() != 64
        || digest.bytes().any(|byte| !(byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
    {
        return Err(corruption());
    }
    let id = ProjectionGenerationId::parse(id).map_err(|_| corruption())?;
    let origin = ProjectionGenerationOriginV1::parse(&origin).ok_or_else(corruption)?;
    if declaration_digest(connection)? != digest {
        return Err(corruption());
    }
    Ok(GenerationRow { id, digest, boundary, origin })
}

/// Recompute the serving generation's `declaration_sha256` after an offline
/// Schema 37 -> 38 upgrade has added `entity_class` and `vector_kinds` to
/// `_fathomdb_projection_registry` and stamped `PRAGMA user_version = 38`.
///
/// Must only be called inside the migration transaction after the pre-upgrade
/// Schema-37 serving generation was already validated by [`current_generation`].
#[cfg(feature = "operator")]
pub(crate) fn rehash_serving_declaration_for_schema_38(
    connection: &Connection,
) -> Result<(), EngineError> {
    let row: Option<(String, String)> = connection
        .query_row(
            "SELECT g.generation_id, g.declaration_sha256 \
             FROM _fathomdb_projection_generation_current c \
             JOIN _fathomdb_projection_generations g ON g.generation_id = c.generation_id \
             WHERE c.singleton = 1 AND g.role = 'serving'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((generation_id, stored_digest)) = row else {
        return Ok(());
    };
    let expected_digest = declaration_digest(connection)?;
    if stored_digest != expected_digest {
        let _ = generation_id;
        transition(connection, ProjectionGenerationOriginV1::Configuration)?;
    }
    Ok(())
}

pub(crate) fn member_completion(
    connection: &Connection,
    cursor: u64,
    expected_kind: &str,
    is_edge: bool,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Completion, EngineError> {
    crate::vector_spans::validate(connection, Some(cursor))?;
    let terminal: Option<String> = connection
        .query_row(
            "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor=?1",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let sidecar: Option<(u64, String)> = connection
        .query_row(
            "SELECT rowid,kind FROM _fathomdb_vector_rows WHERE write_cursor=?1",
            [cursor],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let physical: Option<(String, String)> = connection
        .query_row("SELECT source_type,kind FROM vector_default WHERE rowid=?1", [cursor], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let enrolled: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind=?1)",
            [expected_kind],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;

    let expected_source_type = if is_edge {
        std::borrow::Cow::Borrowed("edge_fact")
    } else {
        super::resolve_source_type_on(connection, expected_kind)?
    };
    classify_completion(
        terminal.as_deref(),
        sidecar.as_ref().map(|(rowid, kind)| (kind.as_str(), *rowid == cursor)),
        physical.as_ref().map(|(source_type, kind)| (source_type.as_str(), kind.as_str())),
        (expected_kind, expected_source_type.as_ref()),
        is_edge,
        enrolled,
        runtime_state,
    )
}

fn classify_completion(
    terminal: Option<&str>,
    sidecar: Option<(&str, bool)>,
    physical: Option<(&str, &str)>,
    (expected_kind, expected_source_type): (&str, &str),
    is_edge: bool,
    enrolled: bool,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Completion, EngineError> {
    if terminal == Some("up_to_date")
        && sidecar.is_some_and(|(kind, identity_matches)| kind == expected_kind && identity_matches)
        && physical.is_some_and(|(source_type, kind)| {
            source_type == expected_source_type && kind == expected_kind
        })
        && (enrolled || !is_edge)
    {
        return Ok(Completion::Complete);
    }
    if terminal == Some("failed") && sidecar.is_none() && physical.is_none() {
        return Ok(Completion::Failed);
    }
    if terminal.is_none() && sidecar.is_none() && physical.is_none() && enrolled {
        return Ok(Completion::Pending);
    }
    if !is_edge
        && terminal == Some("up_to_date")
        && sidecar.is_none()
        && physical.is_none()
        && !enrolled
        && runtime_state != ProjectionRuntimeStateV1::Usable
    {
        return Ok(Completion::Pending);
    }
    Err(corruption())
}

pub(crate) fn physical_member_completion_at(
    connection: &Connection,
    cursor: u64,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Option<Completion>, EngineError> {
    let Some(kind) = dense_member_kind_at(connection, cursor, effective_at)? else {
        return Ok(None);
    };
    let is_edge: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM canonical_edges WHERE write_cursor=?1)",
            [cursor],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    member_completion(connection, cursor, &kind, is_edge, runtime_state).map(Some)
}

pub(crate) fn receipt_point_completion_at(
    connection: &Connection,
    scope: &str,
    operation_id: &str,
    cursor: u64,
    expected_generation: &ProjectionGenerationId,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<Completion, EngineError> {
    let stored_generation: Option<Option<String>> = connection
        .query_row(
            "SELECT projection_generation_id FROM _fathomdb_actuation_receipts \
             WHERE scope=?1 AND operation_id=?2",
            rusqlite::params![scope, operation_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some(Some(stored_generation)) = stored_generation else {
        return Err(ProjectionGenerationError::new(
            ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
            "/projectionGeneration",
        )
        .into());
    };
    if stored_generation != expected_generation.as_str() {
        return Err(ProjectionGenerationError::new(
            ProjectionGenerationErrorReason::WrongProjectionGeneration,
            "/expectedGenerationId",
        )
        .into());
    }
    if current_generation(connection)?.id != *expected_generation {
        return Err(ProjectionGenerationError::new(
            ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
            "/projectionGeneration",
        )
        .into());
    }
    physical_member_completion_at(connection, cursor, effective_at, runtime_state)?.ok_or_else(
        || {
            ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
                "/projectionGeneration",
            )
            .into()
        },
    )
}

pub(crate) fn dense_member_kind_at(
    connection: &Connection,
    cursor: u64,
    effective_at: i64,
) -> Result<Option<String>, EngineError> {
    let node: Option<(String, String, String, bool, bool, bool)> = connection
        .prepare_cached(
            "SELECT n.kind,n.row_kind,n.state,\
                    EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions r \
                      JOIN _fathomdb_source_dependencies d \
                        ON d.scope=r.scope AND d.derived_revision_id=r.revision_id \
                      WHERE r.scope=n.scope AND r.artifact_class='node' AND r.write_cursor=n.write_cursor),\
                    EXISTS(SELECT 1 FROM _fathomdb_vector_kinds vk WHERE vk.kind=n.kind),\
                    EXISTS(SELECT 1 FROM canonical_edges e WHERE e.write_cursor=n.write_cursor) \
             FROM canonical_nodes n WHERE n.write_cursor=?1",
        )
        .map_err(|_| EngineError::Storage)?
        .query_row([cursor], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))
        })
        .optional()
        .map_err(|_| EngineError::Storage)?;
    if let Some((kind, row_kind, state, registered, enrolled, edge_exists)) = node {
        if edge_exists {
            return Err(corruption());
        }
        if (!enrolled
            && !super::vector_projection_declared(connection).map_err(|_| EngineError::Storage)?)
            || !matches!(row_kind.as_str(), "leaf" | "coverage")
            || !super::kind_is_vector_committable_on(connection, &kind)
        {
            return Ok(None);
        }
        // `registered` is the same source-dependency existence test used by
        // `projection_owner_is_eligible_at`. Without such a dependency that
        // helper can only return true, so do not repeat the join for ordinary
        // canonical rows.
        if registered
            && (state != "active"
                || !super::dependency_closure::projection_owner_is_eligible_at(
                    connection,
                    cursor,
                    effective_at,
                )?)
        {
            return Ok(None);
        }
        return Ok(Some(kind));
    }
    let edge: Option<(Option<String>, Option<i64>, Option<i64>)> = connection
        .prepare_cached(
            "SELECT body,superseded_at,t_invalid FROM canonical_edges WHERE write_cursor=?1",
        )
        .map_err(|_| EngineError::Storage)?
        .query_row([cursor], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .optional()
        .map_err(|_| EngineError::Storage)?;
    let Some((body, _superseded_at, t_invalid)) = edge else {
        return Ok(None);
    };
    if body.is_none()
        || t_invalid.is_some_and(|end| end <= effective_at)
        || !super::dependency_closure::projection_owner_is_eligible_at(
            connection,
            cursor,
            effective_at,
        )?
    {
        return Ok(None);
    }
    Ok(Some("edge_fact".to_string()))
}

fn completion_aggregate_row(
    connection: &Connection,
    sql: &str,
    parameters: impl rusqlite::Params,
) -> Result<(u64, u64, Option<u64>), EngineError> {
    let (pending, failed, first_incomplete, corrupt): (u64, u64, Option<u64>, u64) = connection
        .query_row(sql, parameters, |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .map_err(|_| EngineError::Storage)?;
    if corrupt != 0 {
        return Err(corruption());
    }
    Ok((pending, failed, first_incomplete))
}

fn unregistered_node_completion_sql(scope: &crate::scope::ScopePredicate) -> String {
    let builtin_kinds = super::VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES
        .iter()
        .map(|(kind, _)| format!("'{kind}'"))
        .collect::<Vec<_>>()
        .join(",");
    let remapped_cases = super::VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES
        .iter()
        .filter(|(kind, source_type)| kind != source_type)
        .map(|(kind, source_type)| format!("WHEN '{kind}' THEN '{source_type}'"))
        .collect::<Vec<_>>()
        .join(" ");
    let (scope_sql, _) = scope.sql_clause("n.scope", 3);
    format!(
        "WITH members AS (
        SELECT n.write_cursor AS cursor,
               n.kind AS expected_kind,
               CASE n.kind {remapped_cases} ELSE n.kind END AS expected_source_type,
               pt.state AS terminal,vr.rowid AS sidecar_rowid,vr.kind AS sidecar_kind,
               vd.source_type AS physical_source_type,vd.kind AS physical_kind,
               (vk.kind IS NOT NULL) AS enrolled
        FROM canonical_nodes n
        LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=n.write_cursor
        LEFT JOIN _fathomdb_vector_rows vr ON vr.write_cursor=n.write_cursor
        LEFT JOIN vector_default vd ON vd.rowid=n.write_cursor
        LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind AND vk.kind<>'edge_fact'
        WHERE ({scope_sql})
          AND n.row_kind IN ('leaf','coverage')
          AND (n.kind IN ({builtin_kinds}) OR vk.kind IS NOT NULL)
          AND (?1!=0 OR vk.kind IS NOT NULL)
          AND NOT EXISTS(
            SELECT 1 FROM _fathomdb_artifact_revisions r
            JOIN _fathomdb_source_dependencies d
              ON d.scope=r.scope AND d.derived_revision_id=r.revision_id
            WHERE r.scope=n.scope AND r.artifact_class='node' AND r.write_cursor=n.write_cursor
          )
      ), classified AS (
        SELECT cursor,
          COALESCE(terminal='up_to_date' AND sidecar_rowid=cursor
            AND sidecar_kind=expected_kind
            AND physical_source_type=expected_source_type
            AND physical_kind=expected_kind,0) AS complete,
          COALESCE(terminal='failed' AND sidecar_rowid IS NULL
            AND physical_source_type IS NULL,0) AS failed,
          COALESCE(((terminal IS NULL AND sidecar_rowid IS NULL
              AND physical_source_type IS NULL AND enrolled)
            OR (terminal='up_to_date' AND sidecar_rowid IS NULL
              AND physical_source_type IS NULL AND NOT enrolled AND ?2=0)),0) AS pending
        FROM members
      )
      SELECT COALESCE(SUM(pending),0),COALESCE(SUM(failed),0),
             MIN(CASE WHEN pending OR failed THEN cursor END),
             COALESCE(SUM(NOT (complete OR failed OR pending)),0)
      FROM classified"
    )
}

fn aggregate_unregistered_node_completion(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
    node_arm_declared: bool,
    runtime_state: ProjectionRuntimeStateV1,
) -> Result<(u64, u64, Option<u64>), EngineError> {
    let sql = unregistered_node_completion_sql(scope);
    let mut binds = vec![
        rusqlite::types::Value::Integer(i64::from(node_arm_declared)),
        rusqlite::types::Value::Integer(i64::from(
            runtime_state == ProjectionRuntimeStateV1::Usable,
        )),
    ];
    scope.push_values(&mut binds);
    completion_aggregate_row(connection, &sql, rusqlite::params_from_iter(binds.iter()))
}

fn unregistered_edge_completion_sql(scope: &crate::scope::ScopePredicate) -> String {
    let (scope_sql, _) = scope.sql_clause("e.scope", 2);
    format!(
        "WITH members AS (
        SELECT e.write_cursor AS cursor,pt.state AS terminal,
               vr.rowid AS sidecar_rowid,vr.kind AS sidecar_kind,
               vd.source_type AS physical_source_type,vd.kind AS physical_kind,
               (vk.kind IS NOT NULL) AS enrolled
        FROM canonical_edges e
        LEFT JOIN _fathomdb_projection_terminal pt ON pt.write_cursor=e.write_cursor
        LEFT JOIN _fathomdb_vector_rows vr ON vr.write_cursor=e.write_cursor
        LEFT JOIN vector_default vd ON vd.rowid=e.write_cursor
        LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind='edge_fact'
        WHERE ({scope_sql})
          AND e.body IS NOT NULL
          AND (e.t_invalid IS NULL OR e.t_invalid>?1)
          AND NOT EXISTS(
            SELECT 1 FROM _fathomdb_artifact_revisions r
            JOIN _fathomdb_source_dependencies d
              ON d.scope=r.scope AND d.derived_revision_id=r.revision_id
            WHERE r.scope=e.scope AND r.artifact_class='edge' AND r.write_cursor=e.write_cursor
          )
      ), classified AS (
        SELECT cursor,
          COALESCE(terminal='up_to_date' AND sidecar_rowid=cursor
            AND sidecar_kind='edge_fact' AND physical_source_type='edge_fact'
            AND physical_kind='edge_fact' AND enrolled,0) AS complete,
          COALESCE(terminal='failed' AND sidecar_rowid IS NULL
            AND physical_source_type IS NULL,0) AS failed,
          COALESCE(terminal IS NULL AND sidecar_rowid IS NULL
            AND physical_source_type IS NULL AND enrolled,0) AS pending
        FROM members
      )
      SELECT COALESCE(SUM(pending),0),COALESCE(SUM(failed),0),
             MIN(CASE WHEN pending OR failed THEN cursor END),
             COALESCE(SUM(NOT (complete OR failed OR pending)),0)
      FROM classified"
    )
}

fn aggregate_unregistered_edge_completion(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
    effective_at: i64,
) -> Result<(u64, u64, Option<u64>), EngineError> {
    let sql = unregistered_edge_completion_sql(scope);
    let mut binds = vec![rusqlite::types::Value::Integer(effective_at)];
    scope.push_values(&mut binds);
    completion_aggregate_row(connection, &sql, rusqlite::params_from_iter(binds.iter()))
}

fn add_registered_completion(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
    artifact_class: &str,
    accumulator: &mut CompletionAccumulator,
) -> Result<(), EngineError> {
    let owner_table = if artifact_class == "node" { "canonical_nodes" } else { "canonical_edges" };
    let sql = registered_completion_sql(owner_table, scope);
    let mut binds = vec![rusqlite::types::Value::Text(artifact_class.to_string())];
    scope.push_values(&mut binds);
    let mut statement = connection.prepare(&sql).map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(binds.iter()), |row| row.get::<_, u64>(0))
        .map_err(|_| EngineError::Storage)?;
    for row in rows {
        let cursor = row.map_err(|_| EngineError::Storage)?;
        if let Some(completion) =
            physical_member_completion_at(connection, cursor, effective_at, runtime_state)?
        {
            accumulator.add(cursor, completion)?;
        }
    }
    Ok(())
}

fn registered_completion_sql(owner_table: &str, scope: &crate::scope::ScopePredicate) -> String {
    let (scope_sql, _) = scope.sql_clause("d.scope", 2);
    format!(
        "SELECT DISTINCT owner.write_cursor FROM _fathomdb_source_dependencies d
         CROSS JOIN _fathomdb_artifact_revisions r
           ON r.scope=d.scope AND r.revision_id=d.derived_revision_id AND r.artifact_class=?1
         CROSS JOIN {owner_table} owner
           ON owner.scope=r.scope AND owner.write_cursor=r.write_cursor
         WHERE ({scope_sql})
         ORDER BY owner.write_cursor"
    )
}

fn empty_physical_fast_path(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
    effective_at: i64,
    node_arm_declared: bool,
) -> Result<Option<CompletionAccumulator>, EngineError> {
    let terminal_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_projection_terminal", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    let sidecar_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    let physical_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if terminal_rows != 0 || sidecar_rows != 0 || physical_rows != 0 {
        return Ok(None);
    }
    let dependency_rows: u64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies", [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if dependency_rows != 0 {
        return Ok(None);
    }

    let has_explicit_node_enrolment: bool = connection
        .prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind <> 'edge_fact' LIMIT 1)",
        )
        .and_then(|mut stmt| stmt.query_row([], |row| row.get(0)))
        .map_err(|_| EngineError::Storage)?;
    let (node_count, first_node, unenrolled_nodes): (u64, Option<u64>, u64) = if node_arm_declared
        || has_explicit_node_enrolment
    {
        let builtin_kinds = super::VECTOR_COMMITTABLE_NODE_KIND_SOURCE_TYPES
            .iter()
            .map(|(kind, _)| format!("'{kind}'"))
            .collect::<Vec<_>>()
            .join(",");
        let (node_scope_sql, _) = scope.sql_clause("n.scope", 2);
        let mut node_binds = vec![rusqlite::types::Value::Integer(i64::from(node_arm_declared))];
        scope.push_values(&mut node_binds);
        let node_sql = format!(
            "SELECT COUNT(*),MIN(n.write_cursor),COALESCE(SUM(vk.kind IS NULL),0)
                 FROM canonical_nodes n
                 LEFT JOIN _fathomdb_vector_kinds vk ON vk.kind=n.kind AND vk.kind<>'edge_fact'
                 WHERE ({node_scope_sql})
                   AND n.row_kind IN ('leaf','coverage')
                   AND (n.kind IN ({builtin_kinds}) OR vk.kind IS NOT NULL)
                   AND (?1!=0 OR vk.kind IS NOT NULL)"
        );
        connection
            .query_row(&node_sql, rusqlite::params_from_iter(node_binds.iter()), |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })
            .map_err(|_| EngineError::Storage)?
    } else {
        (0, None, 0)
    };
    let (edge_scope_sql, _) = scope.sql_clause("scope", 2);
    let mut edge_binds = vec![rusqlite::types::Value::Integer(effective_at)];
    scope.push_values(&mut edge_binds);
    let (edge_count, first_edge): (u64, Option<u64>) = connection
        .query_row(
            &format!(
                "SELECT COUNT(*),MIN(write_cursor) FROM canonical_edges
                 WHERE ({edge_scope_sql})
                   AND body IS NOT NULL
                   AND (t_invalid IS NULL OR t_invalid>?1)"
            ),
            rusqlite::params_from_iter(edge_binds.iter()),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| EngineError::Storage)?;
    let edge_enrolled: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind='edge_fact')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if unenrolled_nodes != 0 || (edge_count != 0 && !edge_enrolled) {
        return Err(corruption());
    }
    let pending_count = node_count.checked_add(edge_count).ok_or(EngineError::Storage)?;
    let first_incomplete = first_node.into_iter().chain(first_edge).min();
    Ok(Some(CompletionAccumulator { pending_count, failed_count: 0, first_incomplete }))
}

fn registered_provenance_integrity_sql() -> &'static str {
    "SELECT EXISTS(
               SELECT 1 FROM _fathomdb_source_dependencies d
               LEFT JOIN _fathomdb_artifact_revisions derived
                 ON derived.scope=d.scope AND derived.revision_id=d.derived_revision_id
               LEFT JOIN _fathomdb_source_links link
                 ON link.scope=d.scope AND link.artifact_revision_id=d.derived_revision_id
               LEFT JOIN _fathomdb_artifact_revisions source
                 ON source.scope=d.scope AND source.revision_id=link.source_revision_id
               LEFT JOIN _fathomdb_source_versions version
                 ON version.scope=d.scope
                AND version.source_id=link.source_id
                AND version.source_version_id=link.source_version_id
                AND version.source_revision_id=link.source_revision_id
               WHERE derived.revision_id IS NULL
                  OR derived.artifact_role!='derived_semantic'
                  OR link.artifact_revision_id IS NULL
                  OR source.revision_id IS NULL
                  OR source.artifact_role!='canonical_source'
                  OR version.source_revision_id IS NULL
               LIMIT 1
             ) OR EXISTS(
               SELECT 1 FROM _fathomdb_source_links link
               LEFT JOIN _fathomdb_artifact_revisions artifact
                 ON artifact.scope=link.scope AND artifact.revision_id=link.artifact_revision_id
               LEFT JOIN _fathomdb_artifact_revisions source
                 ON source.scope=link.scope AND source.revision_id=link.source_revision_id
               WHERE artifact.revision_id IS NULL OR source.revision_id IS NULL
               LIMIT 1
             ) OR EXISTS(
               SELECT 1 FROM _fathomdb_artifact_revisions artifact
               LEFT JOIN canonical_nodes node
                 ON artifact.artifact_class='node'
                AND node.scope=artifact.scope
                AND node.write_cursor=artifact.write_cursor
               LEFT JOIN canonical_edges edge
                 ON artifact.artifact_class='edge'
                AND edge.scope=artifact.scope
                AND edge.write_cursor=artifact.write_cursor
               WHERE (artifact.artifact_class='node' AND node.write_cursor IS NULL)
                  OR (artifact.artifact_class='edge' AND edge.write_cursor IS NULL)
               LIMIT 1
             )"
}

fn validate_registered_provenance_integrity(connection: &Connection) -> Result<(), EngineError> {
    let corrupt: bool = connection
        .query_row(registered_provenance_integrity_sql(), [], |row| row.get(0))
        .map_err(|_| EngineError::Storage)?;
    if corrupt {
        return Err(corruption());
    }
    Ok(())
}

#[cfg(feature = "test-hooks")]
fn explain_status_query(
    connection: &Connection,
    label: &str,
    sql: &str,
    parameters: Vec<rusqlite::types::Value>,
) -> Result<Vec<String>, EngineError> {
    let explain = format!("EXPLAIN QUERY PLAN {sql}");
    let mut statement = connection.prepare(&explain).map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map(rusqlite::params_from_iter(parameters), |row| row.get::<_, String>(3))
        .map_err(|_| EngineError::Storage)?;
    rows.map(|row| row.map(|detail| format!("{label}: {detail}")).map_err(|_| EngineError::Storage))
        .collect()
}

#[cfg(feature = "test-hooks")]
pub(crate) fn status_query_plans_for_test(
    connection: &Connection,
) -> Result<Vec<String>, EngineError> {
    let unscoped = crate::scope::ScopePredicate::unscoped();
    let mut plans = Vec::new();
    plans.extend(explain_status_query(
        connection,
        "unregistered_node",
        &unregistered_node_completion_sql(&unscoped),
        vec![1_i64.into(), 1_i64.into(), "".to_string().into()],
    )?);
    plans.extend(explain_status_query(
        connection,
        "unregistered_edge",
        &unregistered_edge_completion_sql(&unscoped),
        vec![current_epoch_seconds().into(), "".to_string().into()],
    )?);
    plans.extend(explain_status_query(
        connection,
        "registered_node",
        &registered_completion_sql("canonical_nodes", &unscoped),
        vec!["node".to_string().into(), "".to_string().into()],
    )?);
    plans.extend(explain_status_query(
        connection,
        "registered_edge",
        &registered_completion_sql("canonical_edges", &unscoped),
        vec!["edge".to_string().into(), "".to_string().into()],
    )?);
    plans.extend(explain_status_query(
        connection,
        "provenance_integrity",
        registered_provenance_integrity_sql(),
        vec![],
    )?);
    Ok(plans)
}

fn physical_completion(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
    effective_at: i64,
    runtime_state: ProjectionRuntimeStateV1,
    observed_boundary: u64,
    capture_coverage: bool,
) -> Result<(CompletionSummary, Option<crate::StoredHeadCoverageCounts>), EngineError> {
    let coverage = if capture_coverage {
        Some(crate::vector_spans::capture(connection)?)
    } else {
        crate::vector_spans::validate(connection, None)?;
        None
    };
    validate_registered_provenance_integrity(connection)?;
    let cross_owner_cursor: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM canonical_nodes n \
               JOIN canonical_edges e ON e.write_cursor=n.write_cursor LIMIT 1)",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if cross_owner_cursor {
        return Err(corruption());
    }

    let node_arm_declared =
        super::vector_projection_declared(connection).map_err(|_| EngineError::Storage)?;
    let has_explicit_node_enrolment: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_kinds WHERE kind!='edge_fact')",
            [],
            |row| row.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    let accumulator = if let Some(accumulator) =
        empty_physical_fast_path(connection, scope, effective_at, node_arm_declared)?
    {
        accumulator
    } else {
        let mut accumulator = CompletionAccumulator::default();
        if node_arm_declared || has_explicit_node_enrolment {
            let (pending, failed, first) = aggregate_unregistered_node_completion(
                connection,
                scope,
                node_arm_declared,
                runtime_state,
            )?;
            accumulator.add_aggregate(pending, failed, first)?;
            add_registered_completion(
                connection,
                scope,
                effective_at,
                runtime_state,
                "node",
                &mut accumulator,
            )?;
        }
        let (pending, failed, first) =
            aggregate_unregistered_edge_completion(connection, scope, effective_at)?;
        accumulator.add_aggregate(pending, failed, first)?;
        add_registered_completion(
            connection,
            scope,
            effective_at,
            runtime_state,
            "edge",
            &mut accumulator,
        )?;
        accumulator
    };

    let ready_through =
        accumulator.first_incomplete.map_or(observed_boundary, |cursor| cursor.saturating_sub(1));
    Ok((
        CompletionSummary {
            observed_boundary,
            ready_through,
            pending_count: accumulator.pending_count,
            failed_count: accumulator.failed_count,
        },
        coverage,
    ))
}

fn status_with_capture_in_snapshot(
    connection: &Connection,
    scope: &crate::scope::ScopePredicate,
    runtime_state: ProjectionRuntimeStateV1,
    effective_at_epoch_s: i64,
    observed_boundary: u64,
    capture_coverage: bool,
) -> Result<(ProjectionGenerationStatusV1, Option<crate::StoredHeadCoverageCounts>), EngineError> {
    let generation = current_generation(connection)?;
    let (completion, coverage) = physical_completion(
        connection,
        scope,
        effective_at_epoch_s,
        runtime_state,
        observed_boundary,
        capture_coverage,
    )?;
    let readiness = if generation.origin == ProjectionGenerationOriginV1::LegacyUnverified
        || completion.failed_count != 0
    {
        ProjectionReadinessV1::Degraded
    } else if completion.pending_count == 0 {
        ProjectionReadinessV1::Ready
    } else {
        match runtime_state {
            ProjectionRuntimeStateV1::Absent => ProjectionReadinessV1::Blocked,

            ProjectionRuntimeStateV1::Usable => ProjectionReadinessV1::Processing,
        }
    };
    Ok((
        ProjectionGenerationStatusV1 {
            schema_version: 1,
            generation_id: generation.id,
            declaration_sha256: generation.digest,
            origin: generation.origin,
            transition_boundary: generation.boundary,
            effective_at_epoch_s,
            observed_boundary: completion.observed_boundary,
            ready_through: completion.ready_through,
            readiness,
            runtime_state,
            pending_count: completion.pending_count,
            failed_count: completion.failed_count,
        },
        coverage,
    ))
}

pub(crate) fn next_status_membership_boundary(
    connection: &Connection,
    effective_at_epoch_s: i64,
) -> Result<Option<i64>, EngineError> {
    next_status_membership_boundary_with_scope(connection, effective_at_epoch_s, None)
}

fn next_status_membership_boundary_in_scope(
    connection: &Connection,
    effective_at_epoch_s: i64,
    scope: &crate::scope::ScopePredicate,
) -> Result<Option<i64>, EngineError> {
    next_status_membership_boundary_with_scope(connection, effective_at_epoch_s, Some(scope))
}

fn next_status_membership_boundary_with_scope(
    connection: &Connection,
    effective_at_epoch_s: i64,
    scope: Option<&crate::scope::ScopePredicate>,
) -> Result<Option<i64>, EngineError> {
    let mut binds = vec![rusqlite::types::Value::Integer(effective_at_epoch_s)];
    let (edge_scope_predicate, dep_scope_predicate_from, dep_scope_predicate_until) =
        if let Some(scope) = scope {
            let mut edge_sql = String::new();
            scope.append_sql_values("scope", &mut edge_sql, &mut binds);
            let mut dep_from_sql = String::new();
            scope.append_sql_values("dependency.scope", &mut dep_from_sql, &mut binds);
            let mut dep_until_sql = String::new();
            scope.append_sql_values("dependency.scope", &mut dep_until_sql, &mut binds);
            (
                format!(" AND ({edge_sql})"),
                format!(" AND ({dep_from_sql})"),
                format!(" AND ({dep_until_sql})"),
            )
        } else {
            (String::new(), String::new(), String::new())
        };
    let sql = format!(
        "WITH boundaries(boundary) AS (\
           SELECT t_invalid FROM canonical_edges \
           WHERE body IS NOT NULL \
             AND t_invalid>?1{edge_scope_predicate} \
           UNION ALL \
           SELECT source_node.valid_from \
           FROM _fathomdb_source_dependencies dependency \
           JOIN _fathomdb_source_links derived_link \
             ON derived_link.scope=dependency.scope \
            AND derived_link.artifact_revision_id=dependency.derived_revision_id \
           JOIN _fathomdb_artifact_revisions source_revision \
             ON source_revision.scope=dependency.scope \
            AND source_revision.revision_id=derived_link.source_revision_id \
           JOIN canonical_nodes source_node \
             ON source_node.scope=source_revision.scope \
            AND source_node.write_cursor=source_revision.write_cursor \
           WHERE source_node.valid_from>?1{dep_scope_predicate_from} \
           UNION ALL \
           SELECT source_node.valid_until \
           FROM _fathomdb_source_dependencies dependency \
           JOIN _fathomdb_source_links derived_link \
             ON derived_link.scope=dependency.scope \
            AND derived_link.artifact_revision_id=dependency.derived_revision_id \
           JOIN _fathomdb_artifact_revisions source_revision \
             ON source_revision.scope=dependency.scope \
            AND source_revision.revision_id=derived_link.source_revision_id \
           JOIN canonical_nodes source_node \
             ON source_node.scope=source_revision.scope \
            AND source_node.write_cursor=source_revision.write_cursor \
           WHERE source_node.valid_until>?1{dep_scope_predicate_until}\
         ) \
         SELECT MIN(boundary) FROM boundaries"
    );
    connection
        .query_row(&sql, rusqlite::params_from_iter(binds.iter()), |row| row.get(0))
        .map_err(|_| EngineError::Storage)
}

impl Engine {
    fn read_projection_generation_status_at(
        &self,
        effective_at_epoch_s: i64,
    ) -> Result<ProjectionGenerationStatusV1, EngineError> {
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let observed_epoch = self.projection_generation_status_cache.snapshot_epoch()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| EngineError::Storage)?;
        let runtime_state = if self.runtime_embedder.is_none() {
            ProjectionRuntimeStateV1::Absent
        } else {
            ProjectionRuntimeStateV1::Usable
        };
        let (status, coverage) = self.projection_generation_status_cache.get_or_compute(
            &tx,
            StatusCacheConnectionSlot::Primary,
            observed_epoch,
            &scope,
            runtime_state,
            effective_at_epoch_s,
            true,
        )?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        self.publish_stored_head_coverage(coverage)?;
        Ok(status)
    }

    /// Read the current serving-generation identity and physical completeness.
    pub fn read_projection_generation_status(
        &self,
    ) -> Result<ProjectionGenerationStatusV1, EngineError> {
        self.read_projection_generation_status_at(current_epoch_seconds())
    }

    /// Read generation status at a controlled effective instant for deterministic tests.
    #[cfg(feature = "test-hooks")]
    pub fn read_projection_generation_status_at_for_test(
        &self,
        effective_at_epoch_s: i64,
    ) -> Result<ProjectionGenerationStatusV1, EngineError> {
        self.read_projection_generation_status_at(effective_at_epoch_s)
    }

    /// Read readiness for one cursor already recorded by a Slice-25 receipt.
    pub fn read_mutation_projection_status(
        &self,
        request: MutationProjectionStatusRequestV1,
    ) -> Result<MutationProjectionStatusV1, EngineError> {
        if request.schema_version != 1 {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::UnsupportedSchemaVersion,
                "/schemaVersion",
            )
            .into());
        }
        if !valid_caller_identity(&request.operation_id) {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::InvalidOperationId,
                "/operationId",
            )
            .into());
        }
        if request.write_cursor == 0 {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::InvalidWriteCursor,
                "/writeCursor",
            )
            .into());
        }
        let _operation = self.ensure_open()?;
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let observed_epoch = self.projection_generation_status_cache.snapshot_epoch()?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| EngineError::Storage)?;
        let runtime_state = if self.runtime_embedder.is_none() {
            ProjectionRuntimeStateV1::Absent
        } else {
            ProjectionRuntimeStateV1::Usable
        };
        let effective_at_epoch_s = current_epoch_seconds();
        let visibility_generation = super::frozen_read::load_visibility_generation(&tx)
            .map_err(|_| EngineError::Storage)?;
        let data_version: u64 = tx
            .query_row("PRAGMA data_version", [], |row| row.get(0))
            .map_err(|_| EngineError::Storage)?;
        let total_changes = tx.total_changes();
        let boundary = snapshot_observed_boundary(&tx)?;
        {
            let cache =
                self.mutation_projection_status_cache.lock().map_err(|_| EngineError::Storage)?;
            let scope = self.scope_predicate();
            if let Some(cached) = cache.as_ref() {
                if cached.visibility_generation == visibility_generation
                    && cached.data_version == data_version
                    && cached.total_changes == total_changes
                    && cached.authoritative_boundary == boundary
                    && cached.effective_at_epoch_s == effective_at_epoch_s
                    && cached.runtime_state == runtime_state
                    && cached.is_unscoped == scope.is_unscoped()
                    && cached.scope_read_scopes.as_slice() == scope.read_scopes()
                    && cached.request == request
                {
                    let status = cached.status.clone();
                    tx.commit().map_err(|_| EngineError::Storage)?;
                    return Ok(status);
                }
            }
        }
        let cache_request = request.clone();
        let scope = self.scope_predicate();
        let mut binds = vec![rusqlite::types::Value::Text(request.operation_id.clone())];
        let mut scope_sql = String::new();
        scope.append_sql_values("scope", &mut scope_sql, &mut binds);
        let receipt_sql = format!(
            "SELECT outcome,request_sha256,scope,pending_projection_write_cursors_json \
             FROM _fathomdb_actuation_receipts \
             WHERE operation_id=?1 AND ({scope_sql}) ORDER BY scope"
        );
        let rows = {
            let mut statement = tx.prepare(&receipt_sql).map_err(|_| EngineError::Storage)?;
            let mapped = statement
                .query_map(rusqlite::params_from_iter(binds.iter()), |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, Option<String>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|_| EngineError::Storage)?;
            mapped.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)?
        };
        let mut saw_active_receipt = false;
        let mut matched: Option<(String, super::actuation::ActuationReceiptV1)> = None;
        let target_cursor_str = request.write_cursor.to_string();
        for (outcome, digest, row_scope, pending_json) in rows {
            if outcome == "erased" {
                continue;
            }
            saw_active_receipt = true;
            let cursor_strs: Vec<String> =
                serde_json::from_str(&pending_json).map_err(|_| EngineError::Storage)?;
            if !cursor_strs.iter().any(|s| s == &target_cursor_str) {
                continue;
            }
            let Some(receipt) = super::actuation::load_receipt(
                &tx,
                &row_scope,
                &request.operation_id,
                digest.as_deref().unwrap_or_default(),
                None,
            )?
            else {
                continue;
            };
            if receipt.pending_projection_write_cursors.contains(&request.write_cursor) {
                matched = Some((row_scope, receipt));
                break;
            }
        }
        let Some((row_scope, receipt)) = matched else {
            let field_path = if saw_active_receipt { "/writeCursor" } else { "/operationId" };
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::MutationNotTracked,
                field_path,
            )
            .into());
        };
        let Some(stored_generation) = receipt.projection_generation_id else {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::ProjectionGenerationUnavailable,
                "/projectionGeneration",
            )
            .into());
        };
        if request.expected_generation_id != stored_generation {
            return Err(ProjectionGenerationError::new(
                ProjectionGenerationErrorReason::WrongProjectionGeneration,
                "/expectedGenerationId",
            )
            .into());
        }
        let owner_completion = receipt_point_completion_at(
            &tx,
            &row_scope,
            &request.operation_id,
            request.write_cursor,
            &stored_generation,
            effective_at_epoch_s,
            runtime_state,
        )?;
        let (status, coverage) = self.projection_generation_status_cache.get_or_compute(
            &tx,
            StatusCacheConnectionSlot::Primary,
            observed_epoch,
            &scope,
            runtime_state,
            effective_at_epoch_s,
            true,
        )?;
        let (owner_readiness, owner_pending, owner_failed) = match owner_completion {
            Completion::Complete => (ProjectionReadinessV1::Ready, false, false),
            Completion::Failed => (ProjectionReadinessV1::Degraded, false, true),
            Completion::Pending => {
                let readiness = match runtime_state {
                    ProjectionRuntimeStateV1::Absent => ProjectionReadinessV1::Blocked,

                    ProjectionRuntimeStateV1::Usable => ProjectionReadinessV1::Processing,
                };
                (readiness, true, false)
            }
        };
        let result = MutationProjectionStatusV1 {
            schema_version: 1,
            operation_id: request.operation_id,
            write_cursor: request.write_cursor,
            generation_id: status.generation_id,
            effective_at_epoch_s: status.effective_at_epoch_s,
            observed_boundary: status.observed_boundary,
            ready_through: status.ready_through,
            readiness: owner_readiness,
            runtime_state: status.runtime_state,
            pending_count: u64::from(owner_pending),
            failed_count: u64::from(owner_failed),
        };
        tx.commit().map_err(|_| EngineError::Storage)?;
        self.publish_stored_head_coverage(coverage)?;
        *self.mutation_projection_status_cache.lock().map_err(|_| EngineError::Storage)? =
            Some(CachedMutationProjectionStatus {
                visibility_generation,
                data_version,
                total_changes,
                authoritative_boundary: boundary,
                effective_at_epoch_s,
                runtime_state,
                is_unscoped: scope.is_unscoped(),
                scope_read_scopes: scope.read_scopes().to_vec(),
                request: cache_request,
                status: result.clone(),
            });
        Ok(result)
    }
}

pub(crate) fn current_generation_id(
    connection: &Connection,
) -> Result<ProjectionGenerationId, EngineError> {
    current_generation(connection).map(|row| row.id)
}

pub(crate) fn parse_persisted_generation_id(
    value: String,
) -> Result<ProjectionGenerationId, EngineError> {
    ProjectionGenerationId::parse(value).map_err(|_| corruption())
}

/// 0.8.20 Slice 20 (R-20-DR) — the `dense_readiness` of the `searchable→vector`
/// projection, DERIVED. There is no stored flag and this feature adds no schema
/// step or `MIGRATIONS` entry; later unrelated migrations do not affect that
/// property.
///
/// **Why derived is the design, not a shortcut.** §4.1 invariant 1 requires
/// `{ vector-insert ∧ dense_readiness := ready }` to be ONE transaction, with a
/// torn `ready`-without-vector FORBIDDEN. A stored flag is precisely the thing
/// that can tear. Deriving it makes the invariant true **by construction**:
/// readiness is a pure function of state that
/// [`commit_projection_outcomes`](crate::projection_commit::commit_projection_outcomes) already writes inside a single transaction —
/// the `vector_default` / `_fathomdb_vector_rows` INSERTs, the
/// `_fathomdb_projection_terminal` row ([`record_projection_terminal`](crate::projection_commit::record_projection_terminal)) and the
/// readiness watermark ([`advance_projection_cursor`](crate::projection_commit::advance_projection_cursor), which only ever steps
/// over cursors that ALREADY hold a terminal) all commit together or not at all.
/// So `ready` cannot be observed before the vector is durable, and the only
/// reachable torn state is the tolerated one (`embedding` with the vector
/// absent — the dense arm simply reads as partial).
///
/// It reuses the EXACT predicate `drain`/`wait_for_idle` use
/// ([`connection_has_pending_projection_work`]), so "readiness is `ready`" and
/// "`drain` reports idle" cannot disagree.
///
/// **Scope note (honest boundary).** The predicate is corpus-wide, not
/// per-attribute, because Slice 15d persists the `searchable→vector` sub-object
/// but DEFERS building any per-attribute embedding (`ProjectionDelta::deferred`)
/// — every declared vector projection is served by the one engine vector
/// pipeline, so per-projection scoping has no distinct meaning yet. A stored
/// column would not have been more specific; it would only have been tearable.
/// When per-attribute embedding lands, this function is where the scoping goes.
///
/// **Failure boundary.** A row whose embed FAILED terminally records a `failed`
/// terminal (no vector row), so it stops being outstanding. With a usable dense
/// runtime, readiness returns to `ready`: the row will never embed, so reporting
/// `embedding` forever would be a lie. The usable-runtime predicate takes
/// precedence: when no usable runtime exists the state is `Unavailable`; when
/// it exists this function selects `Embedding` / `Ready` without changing the
/// failed-terminal boundary. Failures stay
/// separately observable through the `projection_failures` collection. A
/// `ready` corpus can therefore lack a vector row only for a failed terminal; it
/// is NOT a torn write because no `up_to_date` terminal exists for it.
pub(crate) fn derive_dense_readiness(
    connection: &Connection,
    dense_runtime_usable: bool,
) -> Result<DenseReadiness, EngineError> {
    if !dense_runtime_usable {
        return Ok(DenseReadiness::Unavailable);
    }
    if connection_has_pending_projection_work(connection).map_err(|_| EngineError::Storage)? {
        Ok(DenseReadiness::Embedding)
    } else {
        Ok(DenseReadiness::Ready)
    }
}

impl Engine {
    /// Read the current projection-runtime status without changing the engine.
    ///
    /// Unlike [`read_projections`][Self::read_projections], this returns a
    /// purpose-built status facade rather than a decorated caller declaration.
    /// It reads the durable registry plus current session facts only: it does
    /// not configure projections, enroll kinds, enqueue work, call `drain`, or
    /// notify the projection scheduler. It uses the ordinarily opened engine
    /// connection and may take its lock; it is not a `ReaderWorkerPool` request
    /// and does not open a separately read-only SQLite connection. A legacy
    /// stored vector sub-object that is not `searchable` is `NotDeclared`,
    /// because the effective-arm predicate is the engine's
    /// `StoredProjection::wants_vector` predicate.
    pub fn read_projection_status(&self) -> Result<ProjectionRuntimeStatus, EngineError> {
        let _operation = self.ensure_open()?;
        let runtime_embedder_available = self.usable_dense_runtime();
        let runtime_unavailability_reason = if runtime_embedder_available {
            ProjectionRuntimeUnavailabilityReason::None
        } else {
            ProjectionRuntimeUnavailabilityReason::NoRuntime
        };

        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| EngineError::Storage)?;
        let connection = &tx;
        let registry = load_projection_registry(connection).map_err(|_| EngineError::Storage)?;
        let has_effective_vector_arm = registry.values().any(StoredProjection::wants_vector);
        let effective_dense_readiness = if has_effective_vector_arm {
            match derive_dense_readiness(connection, runtime_embedder_available)? {
                DenseReadiness::Unavailable => ProjectionStatusDenseReadiness::Unavailable,
                DenseReadiness::Embedding => ProjectionStatusDenseReadiness::Embedding,
                DenseReadiness::Ready => ProjectionStatusDenseReadiness::Ready,
            }
        } else {
            ProjectionStatusDenseReadiness::NotDeclared
        };
        let mut projections: Vec<ProjectionRuntimeStatusEntry> = registry
            .into_iter()
            .map(|((entity_class, name), stored)| {
                let dense_readiness = if stored.wants_vector() {
                    effective_dense_readiness
                } else {
                    ProjectionStatusDenseReadiness::NotDeclared
                };
                ProjectionRuntimeStatusEntry { name, entity_class, dense_readiness }
            })
            .collect();
        projections
            .sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.entity_class.cmp(&b.entity_class)));
        let vector_unsupported_kinds = if has_effective_vector_arm {
            unsupported_vector_kinds(connection).map_err(|_| EngineError::Storage)?
        } else {
            Vec::new()
        };

        let status = ProjectionRuntimeStatus {
            vector_coverage: self.stored_head_coverage_in_snapshot(connection)?,
            runtime_embedder_available,
            runtime_unavailability_reason,
            projections,
            vector_unsupported_kinds,
        };
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(status)
    }

    /// Report whether this session can complete outstanding embedding work.
    ///
    /// This is a pure read over the current session and durable projection
    /// state. It neither configures an embedder nor wakes, schedules, or drains
    /// work. A `Blocked` report carries the exact payload that [`Self::drain`]
    /// returns immediately as [`EngineError::EmbedderRequired`].
    pub fn read_embedding_readiness(&self) -> Result<EmbeddingReadiness, EngineError> {
        let _operation = self.ensure_open()?;
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let pending = pending_embedding_work(connection).map_err(|_| EngineError::Storage)?;
        let pending_count = pending.iter().map(|(_, count)| *count).sum();
        let affected_kinds: Vec<String> = pending.iter().map(|(kind, _)| kind.clone()).collect();
        let usable_embedder = self.usable_dense_runtime();
        // Returned engines have either an admitted provider or explicit None.
        let blocked = if !pending.is_empty() && self.runtime_embedder.is_none() {
            Some(embedder_required_for(&affected_kinds))
        } else {
            None
        };
        let state = if blocked.is_some() {
            EmbeddingReadinessState::Blocked
        } else if pending.is_empty() {
            EmbeddingReadinessState::Ready
        } else {
            EmbeddingReadinessState::Processing
        };
        Ok(EmbeddingReadiness { state, usable_embedder, pending_count, affected_kinds, blocked })
    }
}

impl Engine {
    /// Mint a configuration-origin generation without changing declarations.
    ///
    /// This test hook isolates the worker's captured-generation publication
    /// fence. Production transitions remain owned by configuration/rebuild.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn transition_projection_generation_for_test(
        &self,
    ) -> Result<ProjectionGenerationId, EngineError> {
        let _operation = self.ensure_open()?;
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        crate::frozen_read::require_clean_visibility(&tx)?;
        let generation = transition(&tx, ProjectionGenerationOriginV1::Configuration)?;
        tx.commit().map_err(|_| EngineError::Storage)?;
        Ok(generation)
    }

    /// Return the number of uncached generation-status full-owner scans.
    ///
    /// The counter increments at the sole call site immediately before
    /// `status_in_snapshot`, whose completion summary aggregates every eligible
    /// node and edge owner. Cache hits do not increment it.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn projection_generation_status_full_owner_scan_count_for_test(&self) -> u64 {
        self.projection_generation_status_full_owner_scan_count
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Return `EXPLAIN QUERY PLAN` details for the production status queries.
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn projection_generation_status_query_plans_for_test(
        &self,
    ) -> Result<Vec<String>, EngineError> {
        let _operation = self.ensure_open()?;
        let guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_ref().ok_or(EngineError::Closing)?;
        status_query_plans_for_test(connection)
    }

    /// Execute `empty_physical_fast_path` with a SQLite authorizer denying reads
    /// on `canonical_nodes` (returns `(pending_count, failed_count, first_incomplete)`).
    #[cfg(feature = "test-hooks")]
    #[doc(hidden)]
    pub fn run_empty_physical_fast_path_with_node_read_denied_for_test(
        &self,
    ) -> Result<Option<(u64, u64, Option<u64>)>, EngineError> {
        use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
        let _operation = self.ensure_open()?;
        let scope = self.scope_predicate();
        let mut guard = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = guard.as_mut().ok_or(EngineError::Closing)?;
        let node_arm_declared =
            super::vector_projection_declared(connection).map_err(|_| EngineError::Storage)?;
        let effective_at = crate::temporal::current_epoch_seconds();
        connection
            .authorizer(Some(|context: AuthContext<'_>| match context.action {
                AuthAction::Read { table_name: "canonical_nodes", .. } => Authorization::Deny,
                _ => Authorization::Allow,
            }))
            .map_err(|_| EngineError::Storage)?;
        let outcome = empty_physical_fast_path(connection, &scope, effective_at, node_arm_declared);
        let _ = connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
        outcome
            .map(|opt| opt.map(|acc| (acc.pending_count, acc.failed_count, acc.first_incomplete)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_mint_retries_collisions_without_reusing_history() {
        crate::configure_runtime_for_open().unwrap();
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE _fathomdb_projection_generations(
                   generation_id TEXT PRIMARY KEY
                 );
                 INSERT INTO _fathomdb_projection_generations(generation_id)
                 VALUES('pgen1:00000000000000000000000000000000');",
            )
            .unwrap();
        let mut candidates = [
            "00000000000000000000000000000000",
            "00000000000000000000000000000000",
            "11111111111111111111111111111111",
        ]
        .into_iter();
        let minted =
            mint_id_from_candidates(&connection, || Ok(candidates.next().unwrap().to_string()))
                .unwrap();
        assert_eq!(minted.as_str(), "pgen1:11111111111111111111111111111111");

        let exhausted = mint_id_from_candidates(&connection, || {
            Ok("00000000000000000000000000000000".to_string())
        });
        assert!(matches!(exhausted, Err(EngineError::Storage)));
    }

    #[test]
    fn completion_classifier_is_closed_over_every_persisted_shape() {
        let terminals = [None, Some("up_to_date"), Some("failed"), Some("foreign")];
        let sidecars = [None, Some(("doc", true)), Some(("doc", false)), Some(("wrong", true))];
        let physicals =
            [None, Some(("article", "doc")), Some(("wrong", "doc")), Some(("article", "wrong"))];
        let runtimes = [ProjectionRuntimeStateV1::Absent, ProjectionRuntimeStateV1::Usable];

        for terminal in terminals {
            for sidecar in sidecars {
                for physical in physicals {
                    for is_edge in [false, true] {
                        for enrolled in [false, true] {
                            for runtime in runtimes {
                                let expected = if terminal == Some("up_to_date")
                                    && sidecar == Some(("doc", true))
                                    && physical == Some(("article", "doc"))
                                    && (enrolled || !is_edge)
                                {
                                    Some(Completion::Complete)
                                } else if terminal == Some("failed")
                                    && sidecar.is_none()
                                    && physical.is_none()
                                {
                                    Some(Completion::Failed)
                                } else if (terminal.is_none()
                                    && sidecar.is_none()
                                    && physical.is_none()
                                    && enrolled)
                                    || (!is_edge
                                        && terminal == Some("up_to_date")
                                        && sidecar.is_none()
                                        && physical.is_none()
                                        && !enrolled
                                        && runtime != ProjectionRuntimeStateV1::Usable)
                                {
                                    Some(Completion::Pending)
                                } else {
                                    None
                                };
                                let actual = classify_completion(
                                    terminal,
                                    sidecar,
                                    physical,
                                    ("doc", "article"),
                                    is_edge,
                                    enrolled,
                                    runtime,
                                );
                                match expected {
                                    Some(expected) => match actual {
                                        Ok(actual) => assert_eq!(actual, expected),
                                        Err(error) => panic!(
                                            "expected {expected:?}: terminal={terminal:?} sidecar={sidecar:?} physical={physical:?} edge={is_edge} enrolled={enrolled} runtime={runtime:?} error={error:?}"
                                        ),
                                    },
                                    None => assert!(
                                        matches!(
                                            actual,
                                            Err(EngineError::ProjectionGeneration(ref error))
                                                if error.reason
                                                    == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                                        ),
                                        "unexpected classification: terminal={terminal:?} sidecar={sidecar:?} physical={physical:?} edge={is_edge} enrolled={enrolled} runtime={runtime:?} actual={actual:?}",
                                    ),
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn cached_status_in_snapshot_partitions_cache_by_scope() {
        let dir = tempfile::TempDir::new().unwrap();
        let path =
            dir.path().join(format!("cached-scope-partition{}", fathomdb_schema::SQLITE_SUFFIX));
        let embedder = std::sync::Arc::new(fathomdb_embedder::NoopEmbedder::default());

        {
            let a = crate::Engine::open_with_test_choice_and_config_for_test(
                &path,
                crate::TestEmbedderChoice::Caller(embedder),
                crate::EngineConfig::default()
                    .with_scope_policy(crate::ScopePolicy::single("agent-a")),
            )
            .unwrap()
            .engine;
            a.configure_projections(
                &[crate::ProjectionSpec {
                    name: "memory".into(),
                    roles: [crate::ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(crate::ProjectionVector {
                        embedder: None,
                        dense_readiness: None,
                        kinds: None,
                    }),
                    source: None,
                    entity_class: crate::PropertyEntityClass::Node,
                }],
                &[],
            )
            .unwrap();
            a.write(&[crate::PreparedWrite::Node {
                kind: "doc".into(),
                body: "alpha".into(),
                source_id: crate::SourceId::new("src-a").unwrap(),
                logical_id: Some("a1".into()),
                state: crate::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .unwrap();
            a.drain(10_000).unwrap();
            a.close().unwrap();
        }
        {
            let b = crate::Engine::open_with_choice_and_config(
                &path,
                crate::EmbedderChoice::None,
                crate::EngineConfig::default()
                    .with_scope_policy(crate::ScopePolicy::single("agent-b")),
            )
            .unwrap()
            .engine;
            b.write(&[crate::PreparedWrite::Node {
                kind: "doc".into(),
                body: "beta pending".into(),
                source_id: crate::SourceId::new("src-b").unwrap(),
                logical_id: Some("b1".into()),
                state: crate::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .unwrap();
            b.close().unwrap();
        }

        let engine = crate::Engine::open_with_choice_and_config(
            &path,
            crate::EmbedderChoice::None,
            crate::EngineConfig::default(),
        )
        .unwrap()
        .engine;
        let scope_a = crate::scope::ScopePredicate::single_for_query("agent-a");
        let scope_b = crate::scope::ScopePredicate::single_for_query("agent-b");
        {
            let mut guard = engine.connection.lock().unwrap();
            let conn = guard.as_mut().unwrap();
            let observed_epoch =
                engine.projection_generation_status_cache.snapshot_epoch().unwrap();
            let tx = conn.transaction().unwrap();
            let now = 1_000_i64;

            let (status_a1, _) = engine
                .projection_generation_status_cache
                .get_or_compute(
                    &tx,
                    StatusCacheConnectionSlot::Primary,
                    observed_epoch,
                    &scope_a,
                    ProjectionRuntimeStateV1::Absent,
                    now,
                    true,
                )
                .unwrap();
            assert_eq!(status_a1.readiness, ProjectionReadinessV1::Ready);
            assert_eq!(status_a1.pending_count, 0);

            let (status_b1, _) = engine
                .projection_generation_status_cache
                .get_or_compute(
                    &tx,
                    StatusCacheConnectionSlot::Primary,
                    observed_epoch,
                    &scope_b,
                    ProjectionRuntimeStateV1::Absent,
                    now,
                    true,
                )
                .unwrap();
            assert_eq!(status_b1.readiness, ProjectionReadinessV1::Blocked);
            assert_eq!(status_b1.pending_count, 1);

            let (status_a2, _) = engine
                .projection_generation_status_cache
                .get_or_compute(
                    &tx,
                    StatusCacheConnectionSlot::Primary,
                    observed_epoch,
                    &scope_a,
                    ProjectionRuntimeStateV1::Absent,
                    now,
                    true,
                )
                .unwrap();
            assert_eq!(status_a2.readiness, ProjectionReadinessV1::Ready);
            assert_eq!(status_a2.pending_count, 0);
        }
        engine.close().unwrap();
    }

    #[test]
    fn concurrent_snapshot_race_pre_tamper_and_older_vis_gen_never_poison_cache() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("status-cache-race{}", fathomdb_schema::SQLITE_SUFFIX));
        let embedder = std::sync::Arc::new(fathomdb_embedder::NoopEmbedder::default());
        let cursor = {
            let opened = crate::Engine::open_with_embedder_for_test(&path, embedder).unwrap();
            opened
                .engine
                .configure_projections(
                    &[crate::ProjectionSpec {
                        name: "memory".into(),
                        roles: [crate::ProjectionRole::Searchable].into_iter().collect(),
                        fts: None,
                        vector: Some(crate::ProjectionVector {
                            embedder: None,
                            dense_readiness: None,
                            kinds: None,
                        }),
                        source: None,
                        entity_class: crate::PropertyEntityClass::Node,
                    }],
                    &[],
                )
                .unwrap();
            let c = opened
                .engine
                .write(&[crate::PreparedWrite::Node {
                    kind: "doc".into(),
                    body: "race seed node".into(),
                    source_id: crate::SourceId::new("src:race").unwrap(),
                    logical_id: Some("race-1".into()),
                    state: crate::InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    properties: None,
                }])
                .unwrap()
                .row_cursors[0];
            opened.engine.drain(10_000).unwrap();
            opened.engine.close().unwrap();
            c
        };

        // Scenario 1: S_pre pinned before external vector_default tamper; S_post detects tamper
        // and bumps cache_epoch. S_pre runs get_or_compute after S_post -> must hit Subcase A2
        // (allow_insert = false) and must NOT poison post-tamper epoch or overwrite slot 0.
        let cache = std::sync::Arc::new(ProjectionGenerationStatusCache::new(
            2,
            #[cfg(feature = "test-hooks")]
            std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        ));
        let mut conn0 = Connection::open(&path).unwrap();
        let mut conn1 = Connection::open(&path).unwrap();
        cache.observe_connection(&conn0, StatusCacheConnectionSlot::ReaderWorker(0)).unwrap();
        cache.observe_connection(&conn1, StatusCacheConnectionSlot::ReaderWorker(1)).unwrap();

        let scope = crate::scope::ScopePredicate::unscoped();
        let barrier_pin = std::sync::Arc::new(std::sync::Barrier::new(2));
        let barrier_tampered = std::sync::Arc::new(std::sync::Barrier::new(2));

        let cache0 = std::sync::Arc::clone(&cache);
        let scope0 = scope.clone();
        let b_pin0 = std::sync::Arc::clone(&barrier_pin);
        let b_tam0 = std::sync::Arc::clone(&barrier_tampered);
        let t0 = std::thread::spawn(move || {
            let observed_epoch = cache0.snapshot_epoch().unwrap();
            let tx_pre = conn0.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
            // Pin snapshot BEFORE tamper by executing a read query inside tx_pre.
            let _: i64 =
                tx_pre.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();
            b_pin0.wait();
            b_tam0.wait();
            // Now cache_epoch has bumped! S_pre must succeed on its own pre-tamper snapshot
            // without poisoning the post-tamper cache.
            let (st, _) = cache0
                .get_or_compute(
                    &tx_pre,
                    StatusCacheConnectionSlot::ReaderWorker(0),
                    observed_epoch,
                    &scope0,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .unwrap();
            assert_eq!(st.readiness, ProjectionReadinessV1::Ready);
            tx_pre.commit().unwrap();
            conn0
        });

        barrier_pin.wait();
        // External vector_default tamper (no visibility bump).
        let ext = Connection::open(&path).unwrap();
        ext.execute("DELETE FROM vector_default WHERE rowid=?1", [cursor]).unwrap();
        drop(ext);

        // Slot 1 observes post-tamper snapshot -> triggers Subcase A1 (bumps cache_epoch) and detects corruption.
        {
            let observed_epoch = cache.snapshot_epoch().unwrap();
            let tx_post = conn1.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
            let err = cache
                .get_or_compute(
                    &tx_post,
                    StatusCacheConnectionSlot::ReaderWorker(1),
                    observed_epoch,
                    &scope,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .map(|(s, _)| s);
            assert!(
                matches!(
                    err,
                    Err(EngineError::ProjectionGeneration(ref e))
                        if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                ),
                "post-tamper slot 1 must detect corruption, got {err:?}"
            );
            tx_post.commit().unwrap();
        }
        barrier_tampered.wait();
        let mut conn0 = t0.join().unwrap();

        // A fresh tx on conn0 must see the post-tamper state and detect corruption (not a poisoned Ready entry).
        {
            let observed_epoch = cache.snapshot_epoch().unwrap();
            let tx_fresh = conn0.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
            let err = cache
                .get_or_compute(
                    &tx_fresh,
                    StatusCacheConnectionSlot::ReaderWorker(0),
                    observed_epoch,
                    &scope,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .map(|(s, _)| s);
            assert!(
                matches!(
                    err,
                    Err(EngineError::ProjectionGeneration(ref e))
                        if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                ),
                "fresh tx on slot 0 must detect post-tamper corruption, got {err:?}"
            );
            tx_fresh.commit().unwrap();
        }

        // Scenario 2: Older concurrent snapshot S1 (vis_gen G1) completes after S2 (vis_gen G2)
        // -> step 4 rejects insert (G1 < max_visibility_generation G2) and never evicts S2!
        let path2 =
            dir.path().join(format!("status-cache-vis-race{}", fathomdb_schema::SQLITE_SUFFIX));
        let opened2 = crate::Engine::open_with_choice_and_config(
            &path2,
            crate::EmbedderChoice::None,
            crate::EngineConfig::default(),
        )
        .unwrap();
        let cache2 = std::sync::Arc::new(ProjectionGenerationStatusCache::new(
            2,
            #[cfg(feature = "test-hooks")]
            std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        ));
        let mut r0 = Connection::open(&path2).unwrap();
        let mut r1 = Connection::open(&path2).unwrap();
        cache2.observe_connection(&r0, StatusCacheConnectionSlot::ReaderWorker(0)).unwrap();
        cache2.observe_connection(&r1, StatusCacheConnectionSlot::ReaderWorker(1)).unwrap();

        let observed_epoch1 = cache2.snapshot_epoch().unwrap();
        let tx1 = r0.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
        let _: i64 =
            tx1.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();

        // Advance visibility_generation on the engine (G1 -> G2).
        opened2
            .engine
            .write(&[crate::PreparedWrite::Node {
                kind: "doc".into(),
                body: "vis advance".into(),
                source_id: crate::SourceId::new("src:v2").unwrap(),
                logical_id: Some("v2".into()),
                state: crate::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .unwrap();

        // Slot 1 computes and caches S2 (G2), advancing max_visibility_generation to G2.
        let observed_epoch2 = cache2.snapshot_epoch().unwrap();
        let tx2 = r1.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
        let (st2, _) = cache2
            .get_or_compute(
                &tx2,
                StatusCacheConnectionSlot::ReaderWorker(1),
                observed_epoch2,
                &scope,
                ProjectionRuntimeStateV1::Absent,
                1_000,
                false,
            )
            .unwrap();
        assert_eq!(st2.observed_boundary, 1);
        tx2.commit().unwrap();

        // Older snapshot S1 (G1) runs get_or_compute -> returns G1 status, must NOT insert or evict G2.
        let (st1, _) = cache2
            .get_or_compute(
                &tx1,
                StatusCacheConnectionSlot::ReaderWorker(0),
                observed_epoch1,
                &scope,
                ProjectionRuntimeStateV1::Absent,
                1_000,
                false,
            )
            .unwrap();
        assert_eq!(st1.observed_boundary, 0);
        tx1.commit().unwrap();

        // Slot 1 still hits S2 (G2) cleanly with boundary == 1.
        let observed_epoch2_again = cache2.snapshot_epoch().unwrap();
        let tx2_again = r1.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
        let (st2_again, _) = cache2
            .get_or_compute(
                &tx2_again,
                StatusCacheConnectionSlot::ReaderWorker(1),
                observed_epoch2_again,
                &scope,
                ProjectionRuntimeStateV1::Absent,
                1_000,
                false,
            )
            .unwrap();
        assert_eq!(st2_again.observed_boundary, 1);
        tx2_again.commit().unwrap();
        opened2.engine.close().unwrap();

        // Scenario 3 (Case B cross-slot pre-tamper poisoning):
        // conn0 is seeded at G0; a write advances vis_gen to G1 and conn1 warms G1.
        // conn0 samples observed_epoch and pins tx_pre at G1 BEFORE external vector_default tamper
        // (still holding G0 in slots[0]). After tamper (same G1), conn1 runs tx_post ->
        // Err(ProjectionGenerationCorrupt) and bumps cache_epoch 0 -> 1. Then conn0 completes
        // get_or_compute on tx_pre: it MUST return Ok for its own snapshot WITHOUT poisoning
        // cache_epoch = 1 via Case B!
        let path3 =
            dir.path().join(format!("status-cache-case-b-race{}", fathomdb_schema::SQLITE_SUFFIX));
        let embedder3 = std::sync::Arc::new(fathomdb_embedder::NoopEmbedder::default());
        let opened3 = crate::Engine::open_with_embedder_for_test(&path3, embedder3).unwrap();
        opened3
            .engine
            .configure_projections(
                &[crate::ProjectionSpec {
                    name: "memory".into(),
                    roles: [crate::ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(crate::ProjectionVector {
                        embedder: None,
                        dense_readiness: None,
                        kinds: None,
                    }),
                    source: None,
                    entity_class: crate::PropertyEntityClass::Node,
                }],
                &[],
            )
            .unwrap();

        let cache3 = std::sync::Arc::new(ProjectionGenerationStatusCache::new(
            2,
            #[cfg(feature = "test-hooks")]
            std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0)),
        ));
        let mut c0 = Connection::open(&path3).unwrap();
        let mut c1 = Connection::open(&path3).unwrap();
        // Seed c0 at G0 (cache_epoch = 0).
        cache3.observe_connection(&c0, StatusCacheConnectionSlot::ReaderWorker(0)).unwrap();

        // Advance vis_gen G0 -> G1 and embed so G1 is Ready.
        let c3_cursor = opened3
            .engine
            .write(&[crate::PreparedWrite::Node {
                kind: "doc".into(),
                body: "case-b race node".into(),
                source_id: crate::SourceId::new("src:case-b").unwrap(),
                logical_id: Some("case-b-1".into()),
                state: crate::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            }])
            .unwrap()
            .row_cursors[0];
        opened3.engine.drain(10_000).unwrap();

        // Warm c1 at G1 (slots[1] = (G1, dv1, epoch=0); slots[0] is still at G0!).
        {
            let observed_epoch_warm = cache3.snapshot_epoch().unwrap();
            let tx_warm = c1.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
            let (st_warm, _) = cache3
                .get_or_compute(
                    &tx_warm,
                    StatusCacheConnectionSlot::ReaderWorker(1),
                    observed_epoch_warm,
                    &scope,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .unwrap();
            assert_eq!(st_warm.readiness, ProjectionReadinessV1::Ready);
            tx_warm.commit().unwrap();
        }

        // c0 samples observed_epoch and pins tx_pre at G1 BEFORE external tamper.
        let observed_epoch_pre = cache3.snapshot_epoch().unwrap();
        let tx_pre_b = c0.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
        let _: i64 =
            tx_pre_b.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();

        // External tamper on vector_default (same G1, no visibility bump).
        let ext3 = Connection::open(&path3).unwrap();
        ext3.execute("DELETE FROM vector_default WHERE rowid=?1", [c3_cursor]).unwrap();
        drop(ext3);

        // c1 runs tx_post -> detects tamper (Subcase A1 bumps cache_epoch 0 -> 1) -> Corrupt.
        {
            let observed_epoch_post = cache3.snapshot_epoch().unwrap();
            let tx_post_b = c1.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
            let err = cache3
                .get_or_compute(
                    &tx_post_b,
                    StatusCacheConnectionSlot::ReaderWorker(1),
                    observed_epoch_post,
                    &scope,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .map(|(s, _)| s);
            assert!(
                matches!(
                    err,
                    Err(EngineError::ProjectionGeneration(ref e))
                        if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                ),
                "post-tamper tx_post on c1 must detect corruption, got {err:?}"
            );
            tx_post_b.commit().unwrap();
        }

        // c0 completes get_or_compute on tx_pre -> returns Ok(Ready) for its own snapshot
        // WITHOUT poisoning cache_epoch = 1!
        {
            let (st_pre_b, _) = cache3
                .get_or_compute(
                    &tx_pre_b,
                    StatusCacheConnectionSlot::ReaderWorker(0),
                    observed_epoch_pre,
                    &scope,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .unwrap();
            assert_eq!(st_pre_b.readiness, ProjectionReadinessV1::Ready);
            tx_pre_b.commit().unwrap();
        }

        // Fresh query on c1 MUST STILL return Err(ProjectionGenerationCorrupt) (not poisoned Ok(Ready)!).
        {
            let observed_epoch_check = cache3.snapshot_epoch().unwrap();
            let tx_check = c1.transaction_with_behavior(TransactionBehavior::Deferred).unwrap();
            let err = cache3
                .get_or_compute(
                    &tx_check,
                    StatusCacheConnectionSlot::ReaderWorker(1),
                    observed_epoch_check,
                    &scope,
                    ProjectionRuntimeStateV1::Absent,
                    1_000,
                    false,
                )
                .map(|(s, _)| s);
            assert!(
                matches!(
                    err,
                    Err(EngineError::ProjectionGeneration(ref e))
                        if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                ),
                "fresh query on c1 must NOT receive poisoned Ok(Ready) after Case B race, got {err:?}"
            );
            tx_check.commit().unwrap();
        }
        opened3.engine.close().unwrap();
    }

    #[test]
    fn primary_connection_self_write_total_changes_invalidates_generation_and_mutation_caches() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join(format!("primary-self-write{}", fathomdb_schema::SQLITE_SUFFIX));
        let embedder = std::sync::Arc::new(fathomdb_embedder::NoopEmbedder::default());
        let opened = crate::Engine::open_with_embedder_for_test(&path, embedder).unwrap();
        opened
            .engine
            .configure_projections(
                &[crate::ProjectionSpec {
                    name: "memory".into(),
                    roles: [crate::ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(crate::ProjectionVector {
                        embedder: None,
                        dense_readiness: None,
                        kinds: None,
                    }),
                    source: None,
                    entity_class: crate::PropertyEntityClass::Node,
                }],
                &[],
            )
            .unwrap();
        let receipt = opened
            .engine
            .actuate(
                crate::ActuationBatchV1::new(
                    "self-write-op",
                    vec![crate::ActuationOperationV1::PutCanonicalNode(crate::ProvenancedNodeV1 {
                        kind: "doc".into(),
                        body: "self write body".into(),
                        source_id: crate::SourceId::new("src:sw").unwrap(),
                        logical_id: Some("sw-1".into()),
                        state: crate::InitialState::Active,
                        reason: None,
                        valid_from: None,
                        valid_until: None,
                        provenance: crate::WriteProvenanceV1::canonical(
                            crate::ArtifactRevisionId::new("rev-sw").unwrap(),
                            crate::SourceVersionId::new("ver-sw").unwrap(),
                        ),
                        properties: None,
                    })],
                )
                .unwrap(),
            )
            .unwrap();
        opened.engine.drain(10_000).unwrap();
        let cursor = receipt.pending_projection_write_cursors[0];
        let gen_id = receipt.projection_generation_id.unwrap();
        let req = MutationProjectionStatusRequestV1 {
            schema_version: 1,
            operation_id: "self-write-op".into(),
            write_cursor: cursor,
            expected_generation_id: gen_id,
        };

        // Warm both Primary generation status cache and mutation status cache.
        assert_eq!(
            opened.engine.read_projection_generation_status().unwrap().readiness,
            ProjectionReadinessV1::Ready
        );
        assert_eq!(
            opened.engine.read_mutation_projection_status(req.clone()).unwrap().readiness,
            ProjectionReadinessV1::Ready
        );

        // Mutate vector_default directly ON Engine.connection (PRAGMA data_version stays CONSTANT,
        // _fathomdb_read_visibility_state stays CONSTANT, ONLY total_changes() advances!).
        {
            let mut guard = opened.engine.connection.lock().unwrap();
            let conn = guard.as_mut().unwrap();
            let dv_before: u64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0)).unwrap();
            let tc_before = conn.total_changes();
            conn.execute("DELETE FROM vector_default WHERE rowid=?1", [cursor]).unwrap();
            let dv_after: u64 = conn.query_row("PRAGMA data_version", [], |r| r.get(0)).unwrap();
            let tc_after = conn.total_changes();
            assert_eq!(dv_before, dv_after, "same-connection write keeps data_version constant");
            assert!(tc_after > tc_before, "same-connection write must advance total_changes");
        }

        // Both read_projection_generation_status and read_mutation_projection_status MUST detect
        // the self-write immediately via total_changes instead of returning stale Ready!
        assert!(
            matches!(
                opened.engine.read_projection_generation_status(),
                Err(EngineError::ProjectionGeneration(ref e))
                    if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
            ),
            "Primary generation cache must detect same-connection total_changes mutation"
        );
        assert!(
            matches!(
                opened.engine.read_mutation_projection_status(req),
                Err(EngineError::ProjectionGeneration(ref e))
                    if e.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
            ),
            "Primary mutation cache must detect same-connection total_changes mutation"
        );
        opened.engine.close().unwrap();
    }

    #[test]
    fn observe_reader_startup_defers_sqlite_schema_init_and_detects_external_write() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("reader-startup-lazy.sqlite");
        let opened = Engine::open(&path).unwrap();
        opened.engine.close().unwrap();

        let db_status = |c: &Connection| -> (i32, i32) {
            let (mut cm, mut cm_hi, mut su, mut su_hi) = (0, 0, 0, 0);
            unsafe {
                rusqlite::ffi::sqlite3_db_status(
                    c.handle(),
                    rusqlite::ffi::SQLITE_DBSTATUS_CACHE_MISS,
                    &mut cm,
                    &mut cm_hi,
                    0,
                );
                rusqlite::ffi::sqlite3_db_status(
                    c.handle(),
                    rusqlite::ffi::SQLITE_DBSTATUS_SCHEMA_USED,
                    &mut su,
                    &mut su_hi,
                    0,
                );
            }
            (cm, su)
        };

        let c = Connection::open(&path).unwrap();
        c.pragma_update(None, "secure_delete", "ON").unwrap();
        c.pragma_update(None, "query_only", "ON").unwrap();
        let dv1: u64 = c.pragma_query_value(Some("main"), "data_version", |r| r.get(0)).unwrap();
        let (cm_after_startup, schema_used_after_startup) = db_status(&c);
        assert!(cm_after_startup <= 2, "expected <=2 cache misses, got {cm_after_startup}");
        assert_eq!(
            schema_used_after_startup, 0,
            "reader startup must not eagerly run sqlite3Init()"
        );

        let ext = Connection::open(&path).unwrap();
        ext.execute("INSERT INTO _fathomdb_open_state(key, value) VALUES('probe_k', 'v1')", [])
            .unwrap();
        drop(ext);

        let dv2: u64 = c.pragma_query_value(Some("main"), "data_version", |r| r.get(0)).unwrap();
        assert!(dv2 > dv1, "external write must bump PRAGMA main.data_version ({dv1} -> {dv2})");
    }
}
