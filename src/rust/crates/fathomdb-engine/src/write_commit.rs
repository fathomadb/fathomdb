use super::*;
use crate::erasure::{ERASURE_AUDIT_COLLECTIONS, ERASURE_PENDING_REDACTION_COLLECTION};

pub(crate) fn load_next_cursor(connection: &Connection) -> u64 {
    let nodes = max_cursor(connection, "canonical_nodes").unwrap_or(0);
    let edges = max_cursor(connection, "canonical_edges").unwrap_or(0);
    let mutations = max_cursor(connection, "operational_mutations").unwrap_or(0);
    let state = max_cursor(connection, "operational_state").unwrap_or(0);
    // TC-33: schema step 23 RECREATES `canonical_edges` (no data migration), so
    // the edge rows that used to hold the high-water mark are gone. Without this
    // term the allocator can hand out a cursor a PREVIOUS edge already used —
    // and stale `_fathomdb_projection_terminal` / `_fathomdb_vector_rows` / vec0
    // rows still key on it, so a brand-new row would be treated as
    // already-projected and never get indexed. Step 23 stashes the pre-drop
    // maximum here; folding it in keeps cursors monotonic across the migration.
    let reserved = reserved_write_cursor(connection);
    let closure_boundary = connection
        .query_row(
            "SELECT COALESCE(MAX(admitted_write_boundary),0) \
             FROM _fathomdb_dependency_closures",
            [],
            |row| row.get::<_, u64>(0),
        )
        .unwrap_or(0);
    nodes.max(edges).max(mutations).max(state).max(reserved).max(closure_boundary)
}

/// The write-cursor high-water mark reserved by schema step 23, or 0 when the
/// key is absent (fresh DB, or a DB that never had edges). Never fails the
/// caller: a missing/unparseable value degrades to 0, which is the pre-TC-33
/// behaviour.
pub(crate) fn reserved_write_cursor(connection: &Connection) -> u64 {
    connection
        .query_row(
            "SELECT value FROM _fathomdb_open_state WHERE key = ?1",
            params![fathomdb_schema::RESERVED_WRITE_CURSOR_KEY],
            |row| row.get::<_, String>(0),
        )
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(0)
}

pub(crate) fn max_cursor(connection: &Connection, table: &str) -> rusqlite::Result<u64> {
    let sql = format!("SELECT COALESCE(MAX(write_cursor), 0) FROM {table}");
    connection.query_row(&sql, [], |row| row.get::<_, u64>(0))
}

#[derive(Debug)]
pub(crate) enum CommitBatchError {
    Sql(rusqlite::Error),
    Provenance(ProvenanceError),
    Engine(EngineError),
    WriterPoisoned,
}

impl From<rusqlite::Error> for CommitBatchError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error)
    }
}

impl From<ProvenanceError> for CommitBatchError {
    fn from(error: ProvenanceError) -> Self {
        Self::Provenance(error)
    }
}

impl From<EngineError> for CommitBatchError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

pub(crate) fn revision_hash_field(hasher: &mut Sha256, value: &[u8]) {
    hasher.update((value.len() as u64).to_be_bytes());
    hasher.update(value);
}

fn runtime_revision_id(write: &PreparedWrite, cursor: u64) -> String {
    let write = storage_write_shape(write);
    let mut hasher = Sha256::new();
    revision_hash_field(&mut hasher, b"fathomdb:artifact-revision:runtime:v1");
    match write.as_ref() {
        PreparedWrite::Node { body, source_id, .. } => {
            revision_hash_field(&mut hasher, b"node");
            revision_hash_field(&mut hasher, cursor.to_string().as_bytes());
            revision_hash_field(&mut hasher, source_id.as_str().as_bytes());
            revision_hash_field(&mut hasher, b"source-version:none");
            revision_hash_field(&mut hasher, body.as_bytes());
        }
        PreparedWrite::Edge { body, source_id, .. } => {
            revision_hash_field(&mut hasher, b"edge");
            revision_hash_field(&mut hasher, cursor.to_string().as_bytes());
            revision_hash_field(&mut hasher, source_id.as_str().as_bytes());
            revision_hash_field(&mut hasher, b"source-version:none");
            match body {
                None => revision_hash_field(&mut hasher, b"body:none"),
                Some(body) => {
                    revision_hash_field(&mut hasher, b"body:some");
                    revision_hash_field(&mut hasher, body.as_bytes());
                }
            }
        }
        _ => unreachable!("only canonical entities have artifact revisions"),
    }
    format!("_fdb:r:{}", hex_encode(&hasher.finalize()))
}

#[cfg(test)]
thread_local! {
    pub(crate) static CANONICAL_BODY_HASH_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

pub(crate) fn canonical_body_hash(body: &str) -> String {
    #[cfg(test)]
    CANONICAL_BODY_HASH_CALLS.with(|calls| calls.set(calls.get() + 1));
    hex_encode(&Sha256::digest(body.as_bytes()))
}

pub(crate) fn checked_locator_columns(
    locator: &SourceLocator,
    source_body: &str,
) -> Result<(&'static str, Option<i64>, Option<i64>), ProvenanceError> {
    match locator {
        SourceLocator::WholeBody => Ok(("whole_body", None, None)),
        SourceLocator::Utf8Bytes { start_inclusive, end_exclusive } => {
            let start = usize::try_from(*start_inclusive).ok();
            let end = usize::try_from(*end_exclusive).ok();
            let valid = start.zip(end).is_some_and(|(start, end)| {
                start <= end
                    && end <= source_body.len()
                    && source_body.is_char_boundary(start)
                    && source_body.is_char_boundary(end)
            });
            if !valid || *start_inclusive > i64::MAX as u64 || *end_exclusive > i64::MAX as u64 {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::LocatorInvalid,
                    "/provenance/sourceLocator",
                ));
            }
            Ok(("utf8_bytes", Some(*start_inclusive as i64), Some(*end_exclusive as i64)))
        }
    }
}

fn revision_is_registered(tx: &Connection, revision_id: &str) -> rusqlite::Result<bool> {
    tx.prepare_cached(
        "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions WHERE revision_id = ?1)",
    )?
    .query_row([revision_id], |row| row.get(0))
}

fn register_artifact_identity(
    tx: &Connection,
    original: &PreparedWrite,
    cursor: u64,
) -> Result<(), CommitBatchError> {
    let (artifact_class, provenance) = match original {
        PreparedWrite::ProvenancedNode(node) => ("node", Some(&node.provenance)),
        PreparedWrite::ProvenancedEdge(edge) => ("edge", Some(&edge.provenance)),
        PreparedWrite::Node { .. } => ("node", None),
        PreparedWrite::Edge { .. } => ("edge", None),
        _ => return Ok(()),
    };

    let Some(provenance) = provenance else {
        let revision_id = runtime_revision_id(original, cursor);
        if revision_is_registered(tx, &revision_id)? {
            return Err(ProvenanceError::new(ProvenanceErrorReason::RevisionIdConflict, "").into());
        }
        tx.prepare_cached(
            "INSERT INTO _fathomdb_artifact_revisions(\
               schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness\
             ) VALUES(1, ?1, ?2, ?3, 'legacy', ?4)",
        )?
        .execute(params![
            revision_id,
            artifact_class,
            cursor,
            ProvenanceCompleteness::MigratedIncomplete.as_str()
        ])?;
        return Ok(());
    };

    let revision_id = provenance.artifact_revision_id.as_str();
    if revision_is_registered(tx, revision_id)? {
        return Err(ProvenanceError::new(ProvenanceErrorReason::RevisionIdConflict, "").into());
    }

    match provenance.role {
        ProvenanceRole::Canonical => {
            let PreparedWrite::ProvenancedNode(node) = original else {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::RoleInvalid,
                    "/provenance/role",
                )
                .into());
            };
            let existing: Option<String> = tx
                .query_row(
                    "SELECT source_revision_id FROM _fathomdb_source_versions \
                     WHERE source_id = ?1 AND source_version_id = ?2",
                    params![node.source_id.as_str(), provenance.source_version_id.as_str()],
                    |row| row.get(0),
                )
                .optional()?;
            if existing.as_deref().is_some_and(|stored| stored != revision_id) {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::SourceVersionConflict,
                    "/provenance/sourceVersionId",
                )
                .into());
            }
            let hash = canonical_body_hash(&node.body);
            tx.execute(
                "INSERT INTO _fathomdb_artifact_revisions(\
                   schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness\
                 ) VALUES(1, ?1, 'node', ?2, 'canonical_source', ?3)",
                params![revision_id, cursor, ProvenanceCompleteness::Complete.as_str()],
            )?;
            tx.execute(
                "INSERT INTO _fathomdb_source_versions(\
                   schema_version, source_id, source_version_id, source_revision_id\
                 ) VALUES(1, ?1, ?2, ?3)",
                params![
                    node.source_id.as_str(),
                    provenance.source_version_id.as_str(),
                    revision_id
                ],
            )?;
            tx.execute(
                "INSERT INTO _fathomdb_source_links(\
                   schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
                   locator_kind, start_byte, end_byte, hash_algorithm, hash_digest\
                 ) VALUES(1, ?1, ?2, ?3, ?1, 'whole_body', NULL, NULL, 'sha256', ?4)",
                params![
                    revision_id,
                    node.source_id.as_str(),
                    provenance.source_version_id.as_str(),
                    hash
                ],
            )?;
        }
        ProvenanceRole::Derived => {
            let source_revision_id = provenance
                .source_revision_id
                .as_ref()
                .expect("derived constructor always sets source revision");
            let source: Option<(String, String, String)> = tx
                .query_row(
                    "SELECT sv.source_id, sv.source_version_id, n.body \
                     FROM _fathomdb_artifact_revisions ar \
                     JOIN _fathomdb_source_versions sv ON sv.source_revision_id = ar.revision_id \
                     JOIN canonical_nodes n ON n.write_cursor = ar.write_cursor \
                     WHERE ar.revision_id = ?1 AND ar.artifact_class = 'node' \
                       AND ar.artifact_role = 'canonical_source'",
                    [source_revision_id.as_str()],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
                .optional()?;
            let Some((stored_source_id, stored_version_id, source_body)) = source else {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::SourceRevisionMissing,
                    "/provenance/sourceRevisionId",
                )
                .into());
            };
            let written_source_id = match original {
                PreparedWrite::ProvenancedNode(node) => node.source_id.as_str(),
                PreparedWrite::ProvenancedEdge(edge) => edge.source_id.as_str(),
                _ => unreachable!(),
            };
            if stored_source_id != written_source_id
                || stored_version_id != provenance.source_version_id.as_str()
            {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::SourceMismatch,
                    "/provenance/sourceRevisionId",
                )
                .into());
            }
            let locator =
                provenance.locator.as_ref().expect("derived constructor always sets locator");
            let (locator_kind, start_byte, end_byte) =
                checked_locator_columns(locator, &source_body)?;
            let supplied_hash = provenance
                .canonical_source_hash
                .as_ref()
                .expect("derived constructor always sets hash");
            if supplied_hash.digest_hex() != canonical_body_hash(&source_body) {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::HashMismatch,
                    "/provenance/canonicalSourceHash",
                )
                .into());
            }
            if dependency_closure::active_barrier_for_source(tx, source_revision_id.as_str())
                .map_err(|_| rusqlite::Error::InvalidQuery)?
            {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::SourceClosureActive,
                    "/provenance/sourceRevisionId",
                )
                .into());
            }
            if !dependency_closure::source_revision_is_strictly_eligible(
                tx,
                source_revision_id.as_str(),
                current_epoch_seconds(),
            )
            .map_err(|_| rusqlite::Error::InvalidQuery)?
            {
                return Err(ProvenanceError::new(
                    ProvenanceErrorReason::SourceRevisionIneligible,
                    "/provenance/sourceRevisionId",
                )
                .into());
            }
            tx.execute(
                "INSERT INTO _fathomdb_artifact_revisions(\
                   schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness\
                 ) VALUES(1, ?1, ?2, ?3, 'derived_semantic', ?4)",
                params![revision_id, artifact_class, cursor, ProvenanceCompleteness::Complete.as_str()],
            )?;
            tx.execute(
                "INSERT INTO _fathomdb_source_links(\
                   schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
                   locator_kind, start_byte, end_byte, hash_algorithm, hash_digest\
                 ) VALUES(1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'sha256', ?8)",
                params![
                    revision_id,
                    stored_source_id,
                    stored_version_id,
                    source_revision_id.as_str(),
                    locator_kind,
                    start_byte,
                    end_byte,
                    supplied_hash.digest_hex()
                ],
            )?;
        }
    }
    Ok(())
}

/// Temporarily suppress schema triggers on the writer connection. External
/// connections retain the schema-33 triggers, and `Drop` restores this
/// connection during error unwinding before the transaction rolls back.
pub(crate) struct TriggerStateGuard<'connection> {
    connection: &'connection Connection,
    restored: bool,
}

impl<'connection> TriggerStateGuard<'connection> {
    pub(crate) fn disable(connection: &'connection Connection) -> Result<Self, CommitBatchError> {
        if !connection
            .db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER)
            .map_err(|_| CommitBatchError::WriterPoisoned)?
        {
            return Err(CommitBatchError::WriterPoisoned);
        }
        let enabled = connection
            .set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false)
            .map_err(|_| CommitBatchError::WriterPoisoned)?;
        if enabled {
            return Err(CommitBatchError::WriterPoisoned);
        }
        Ok(Self { connection, restored: false })
    }

    pub(crate) fn restore(&mut self) -> Result<(), CommitBatchError> {
        match self.connection.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, true) {
            Ok(true) => {
                self.restored = true;
                Ok(())
            }
            _ => Err(CommitBatchError::WriterPoisoned),
        }
    }
}

impl Drop for TriggerStateGuard<'_> {
    fn drop(&mut self) {
        if !self.restored {
            self.restored = self
                .connection
                .set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, true)
                .unwrap_or(false);
        }
    }
}

fn canonical_batch_has_no_custom_triggers(connection: &Connection) -> rusqlite::Result<bool> {
    let unexpected: bool = connection
        .prepare_cached(
            "SELECT EXISTS(
             SELECT 1 FROM sqlite_master
             WHERE type='trigger'
               AND tbl_name IN (
                   'canonical_nodes','canonical_edges','_fathomdb_artifact_revisions',
                   '_fathomdb_source_versions','_fathomdb_source_links',
                   '_fathomdb_source_dependencies','_fathomdb_dependency_closures',
                   'canonical_attributes','_fathomdb_projection_state',
                   '_fathomdb_projection_terminal','_fathomdb_vector_kinds',
                   '_fathomdb_vector_rows','operational_mutations',
                   '_fathomdb_open_state','_fathomdb_read_visibility_state'
               )
               AND name NOT LIKE '_fathomdb_read_visibility_%'
             UNION ALL
             SELECT 1 FROM sqlite_temp_master
             WHERE type='trigger'
               AND tbl_name IN (
                   'canonical_nodes','canonical_edges','_fathomdb_artifact_revisions',
                   '_fathomdb_source_versions','_fathomdb_source_links',
                   '_fathomdb_source_dependencies','_fathomdb_dependency_closures',
                   'canonical_attributes','_fathomdb_projection_state',
                   '_fathomdb_projection_terminal','_fathomdb_vector_kinds',
                   '_fathomdb_vector_rows','operational_mutations',
                   '_fathomdb_open_state','_fathomdb_read_visibility_state'
               )
               AND name NOT LIKE '_fathomdb_read_visibility_%'
         )",
        )?
        .query_row([], |row| row.get(0))?;
    Ok(!unexpected)
}

pub(crate) fn commit_batch(
    connection: &mut Connection,
    batch: &[PreparedWrite],
    plans: &[WritePlan],
    base_cursor: u64,
    provenance_row_cap: u64,
    vector_kinds_to_enrol: &[String],
    #[cfg(feature = "test-hooks")] abort_next_commit_for_test: bool,
) -> Result<(u64, bool, Vec<ClosureOperationId>), CommitBatchError> {
    #[cfg(feature = "test-hooks")]
    if abort_next_commit_for_test {
        connection.commit_hook(Some(|| true))?;
    }
    let result = commit_batch_transaction(
        connection,
        batch,
        plans,
        base_cursor,
        provenance_row_cap,
        vector_kinds_to_enrol,
    );
    #[cfg(feature = "test-hooks")]
    if abort_next_commit_for_test {
        connection.commit_hook(None::<fn() -> bool>)?;
    }
    result
}

fn commit_batch_transaction(
    connection: &mut Connection,
    batch: &[PreparedWrite],
    plans: &[WritePlan],
    base_cursor: u64,
    provenance_row_cap: u64,
    vector_kinds_to_enrol: &[String],
) -> Result<(u64, bool, Vec<ClosureOperationId>), CommitBatchError> {
    // 0.8.20 Slice 21a-2 (TC-57) — `BEGIN IMMEDIATE`, not rusqlite's `BEGIN
    // DEFERRED` default. Take the WAL write lock AT `BEGIN`, before the
    // supersession SELECT below, so this transaction never has to PROMOTE a read
    // lock to a write lock.
    //
    // The defect this closes (characterized in
    // `dev/design/0.8.20-tc57-write-race-characterization.md`, repro 10/10 at
    // baseline `41a81c17`): for a GOVERNED write (`logical_id: Some`) the first
    // statement in this transaction is a read —
    // `prior_node_cursors_by_logical_id` — and the second is the supersession
    // UPDATE. When the async projection worker holds the write lock on its own
    // connection at that instant, SQLite refuses the promotion with plain
    // `SQLITE_BUSY` (5) and SKIPS the busy handler entirely, for deadlock
    // avoidance (`sqlite3_busy_handler`: "if SQLite determines that invoking the
    // busy handler could result in a deadlock, it will go ahead and return
    // SQLITE_BUSY"). MEASURED: handler invoked ZERO times, error returned in 0 ms
    // against rusqlite's 5 000 ms default timeout. So NO `busy_timeout` value
    // could ever have absorbed it, and the caller saw an opaque, un-retryable
    // `EngineError::Storage` mid-ingest. The same shape also has a second,
    // narrower exit — `SQLITE_BUSY_SNAPSHOT` (517) when the WAL advances past the
    // read snapshot — which this closes too, by construction.
    //
    // UNCONDITIONAL rather than gated on `logical_id`, deliberately: an anonymous
    // batch's first statement is already the INSERT below, so it takes the write
    // lock essentially immediately anyway and the delta is microseconds, whereas a
    // content-dependent transaction behaviour would be a NEW correctness surface
    // (mixed batches, edge arms, future write kinds) with a place to be wrong in
    // each. MEASURED cost on the anonymous arm: none detectable
    // (`tc57_worker_commit_pressure.rs`).
    //
    // `BEGIN IMMEDIATE` can itself return `SQLITE_BUSY` — but WITH the busy
    // handler consulted, i.e. absorbed by the existing 5 s default instead of
    // surfaced (pinned by `tc57_mechanism_control_write_first_is_retryable`).
    let canonical_batch = batch.iter().all(|write| {
        matches!(
            write,
            PreparedWrite::Node { .. }
                | PreparedWrite::Edge { .. }
                | PreparedWrite::ProvenancedNode(_)
                | PreparedWrite::ProvenancedEdge(_)
        )
    });
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    // Slice 71B: the standard canonical mutation closure contains only the
    // visibility triggers. Suppress them on this connection, perform the same
    // checked nonce-bearing invalidation once after all mutations succeed, then
    // restore trigger execution before commit. An unexpected main or TEMP
    // trigger selects the unchanged row-trigger path so application extensions
    // still observe every row.
    if canonical_batch && canonical_batch_has_no_custom_triggers(&tx)? {
        let mut trigger_guard = TriggerStateGuard::disable(&tx)?;
        let attempted = apply_batch_in_transaction(
            &tx,
            batch,
            plans,
            base_cursor,
            provenance_row_cap,
            vector_kinds_to_enrol,
            dependency_closure::SoftClosureMode::Complete,
        )
        .and_then(|result| {
            advance_read_visibility(&tx)?;
            Ok(result)
        });
        let restored = trigger_guard.restore();
        drop(trigger_guard);
        return match (attempted, restored) {
            (Ok(result), Ok(())) => {
                tx.commit()?;
                Ok(result)
            }
            (Err(error), Ok(())) => Err(error),
            (_, Err(_)) => Err(CommitBatchError::WriterPoisoned),
        };
    }

    let result = apply_batch_in_transaction(
        &tx,
        batch,
        plans,
        base_cursor,
        provenance_row_cap,
        vector_kinds_to_enrol,
        dependency_closure::SoftClosureMode::Complete,
    )?;
    tx.commit()?;
    Ok(result)
}

#[cfg(feature = "test-hooks")]
pub(super) fn take_write_commit_abort_marker_for_test(
    connection: &Connection,
) -> rusqlite::Result<bool> {
    let armed = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM temp.sqlite_temp_master \
         WHERE type='table' AND name='_fathomdb_test_abort_next_write_commit')",
        [],
        |row| row.get(0),
    )?;
    if armed {
        connection.execute_batch("DROP TABLE temp._fathomdb_test_abort_next_write_commit")?;
    }
    Ok(armed)
}

pub(crate) fn advance_read_visibility(connection: &Connection) -> rusqlite::Result<()> {
    let changed = connection
        .prepare_cached(
            "UPDATE _fathomdb_read_visibility_state
         SET generation=generation+1,
             state_nonce=lower(hex(randomblob(32)))
         WHERE singleton=1 AND generation<9223372036854775807",
        )?
        .execute([])?;
    if changed != 1 {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER),
            Some("read visibility generation exhausted".to_string()),
        ));
    }
    Ok(())
}

/// Apply a validated ordinary write batch inside an already-owned transaction.
///
/// Slice 25 reuses this seam so caller-decided writes, dependency registrations,
/// lifecycle changes, and their terminal receipt share one SQLite commit. The
/// enclosing owner remains responsible for committing, publishing the cursor,
/// and notifying the projection worker.
pub(crate) fn apply_batch_in_transaction(
    tx: &Connection,
    batch: &[PreparedWrite],
    plans: &[WritePlan],
    base_cursor: u64,
    provenance_row_cap: u64,
    vector_kinds_to_enrol: &[String],
    closure_mode: dependency_closure::SoftClosureMode,
) -> Result<(u64, bool, Vec<ClosureOperationId>), CommitBatchError> {
    dependency_closure::guard_no_pending_physical(tx)?;
    let mut closure_ids = Vec::new();
    for kind in vector_kinds_to_enrol {
        register_vector_kind(tx, kind)?;
    }
    let unstranded =
        if vector_kinds_to_enrol.is_empty() { false } else { reenqueue_stranded_vector_rows(tx)? };

    for (i, (original_write, plan)) in batch.iter().zip(plans).enumerate() {
        // Per-row cursor: row i gets `base_cursor + i + 1`. See the
        // comment in `Engine::write_inner`.
        let cursor = base_cursor.saturating_add((i as u64).saturating_add(1));
        let write = storage_write_shape(original_write);
        match (write.as_ref(), plan) {
            (
                PreparedWrite::Node {
                    kind,
                    body,
                    source_id,
                    logical_id,
                    state,
                    reason,
                    valid_from,
                    valid_until,
                },
                WritePlan::Node,
            ) => {
                // G0 — supersession is tombstone-then-insert in this same txn:
                // mark the prior active version superseded BEFORE inserting the
                // new active row, so the partial-unique-active index never sees
                // two active rows for one logical_id. Scoped to logical_id ALONE
                // (Decision 5, HITL-SIGNED 2026-06-05): a kind-change re-ingest of
                // the same logical_id SUPERSEDES, never forks. No-op when logical_id
                // is None (legacy/own-identity insert, behavior-identical to 0.7.x).
                if let Some(logical_id) = logical_id {
                    // fix-1 finding 2 [P2]: collect the prior active cursor(s)
                    // BEFORE tombstoning so we can purge the superseded row's
                    // row-owned attribute projections and keep the at-rest EAV /
                    // property-FTS store ACTIVE-ONLY. Without this, a same-session
                    // property filter / property-FTS saw BOTH the stale and the
                    // current value until a boot re-derive/reconfigure cleared the
                    // table — a stale read that violates the active-only invariant.
                    let prior_g0 = prior_node_cursors_by_logical_id(tx, logical_id)?;
                    for prior_cursor in &prior_g0 {
                        if let Some(source_revision) =
                            dependency_closure::source_revision_for_cursor(tx, *prior_cursor)?
                        {
                            if let Some(id) = dependency_closure::admit_soft_closure(
                                tx,
                                &source_revision,
                                ClosureCauseV1::Superseded,
                                cursor,
                                closure_mode,
                            )? {
                                closure_ids.push(id);
                            }
                        }
                    }
                    tx.prepare_cached(
                        "UPDATE canonical_nodes SET superseded_at = ?1
                         WHERE logical_id = ?2 AND superseded_at IS NULL",
                    )?
                    .execute(params![cursor, logical_id])?;
                    // Purge only the Attribute + PropertyFts classes: those tables
                    // have NO `superseded_at IS NULL` read-side filter (the FTS5
                    // `property_search_index` cannot carry one), so their stale rows
                    // MUST be deleted at rest. The NodeFts (`search_index` /
                    // `search_index_v2`) + Vector shadows are left intact — the node
                    // read path already excludes their superseded rows via the
                    // `canonical_nodes WHERE superseded_at IS NULL` join, so purging
                    // them here would be a behaviour change outside this fix's scope.
                    for sc in &prior_g0 {
                        purge_row_projections_for_cursor_in(
                            tx,
                            *sc,
                            &[ProjectionClass::Attribute, ProjectionClass::PropertyFts],
                        )?;
                    }
                }
                // EXP-S (0.8.14 Slice 5, D1) — a `PreparedWrite::Node` is the
                // `leaf` structural row_kind (a normal record). coverage/graph
                // rows are written via internal paths (row_kind is a SEPARATE
                // axis from the doc-type `kind`, and there is no public SDK
                // surface for it this release). Writing `leaf` explicitly is
                // value-identical to the column DEFAULT.
                // OPP-12 Phase-1 (0.8.19 Slice 5) — persist the create-time
                // existence state + advisory reason. `InitialState::Active`
                // (the default) writes `state = 'active'`, value-identical to the
                // migration step-20 column DEFAULT; `Pending` quarantines the node
                // out of default retrieval (the `state = 'active'` read exclusion).
                // 0.8.20 Slice 15b (TC-34) — persist the world-time validity
                // window. A `None` binds SQL NULL, which is what the migration
                // step-22 columns already hold for every pre-existing row and what
                // `ReadView::validity_sql` reads as UNBOUNDED on that side. So a
                // write that omits the window is byte-identical on disk to a
                // pre-slice write, and default-view visibility cannot drift.
                tx.prepare_cached(
                    "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id, row_kind, state, reason, valid_from, valid_until)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                )?
                .execute(params![cursor, kind, body, source_id.as_str(), logical_id, RowKind::Leaf.as_str(), state.as_str(), reason, valid_from, valid_until])?;
                // EXP-S (D2/D5) — per-row_kind index-target dispatch. For `leaf`
                // this is behavior-identical to the pre-EXP-S inline path: FTS
                // (sync, in-tx) + vector (async, gated by kind_is_vector_indexed);
                // else the cursor is terminated up-front.
                // fix-2 [P2]: gate the attribute projection on the create-time
                // state. A fresh insert is always non-superseded, so the backfill
                // predicate (`state = 'active' AND superseded_at IS NULL`) reduces
                // to `state == Active` here. A `Pending` node is quarantined out of
                // the canonical read model — its declared attributes must NOT reach
                // the property store until a `transition(pending → active)` promotes
                // it (which projects them then). Node-FTS / vector shadows are left
                // to their read-side lifecycle filter, exactly as for supersession.
                project_canonical_node_row(
                    tx,
                    cursor,
                    kind,
                    body,
                    RowKind::Leaf,
                    ProjectionPass::Write,
                    matches!(state, InitialState::Active),
                )?;
            }
            (
                PreparedWrite::Edge {
                    kind,
                    from,
                    to,
                    source_id,
                    logical_id,
                    body,
                    t_valid,
                    t_invalid,
                    confidence,
                    extractor_model_id,
                    temporal_fallback,
                },
                WritePlan::Edge,
            ) => {
                // G0 — identical tombstone-then-insert supersession on edges,
                // keyed by logical_id ALONE (Decision 5, HITL-SIGNED 2026-06-05;
                // edge `kind` is relationship-type, not identity — a kind-change
                // re-ingest of the same edge logical_id SUPERSEDES, never forks).
                // No-op when logical_id is None.
                if let Some(logical_id) = logical_id {
                    // fix-30 [P2]: collect prior active cursors BEFORE tombstoning
                    // so stale vector_default rows can be pruned.
                    let prior_g0 = prior_edge_cursors_by_logical_id(tx, logical_id)?;
                    tx.execute(
                        "UPDATE canonical_edges SET superseded_at = ?1
                         WHERE logical_id = ?2 AND superseded_at IS NULL",
                        params![cursor, logical_id],
                    )?;
                    for sc in &prior_g0 {
                        delete_vector_partition_row(tx, *sc)?;
                        tx.execute(
                            "DELETE FROM _fathomdb_vector_rows WHERE write_cursor = ?1",
                            [sc],
                        )?;
                        // fix-32 [P2]: record terminal so advance_projection_cursor
                        // can walk past this now-superseded cursor.
                        // TC-45: the token MUST be 'up_to_date', NOT 'superseded'.
                        // The terminal table (schema step 7) carries
                        // CHECK(state IN ('failed','up_to_date')) and the writer is
                        // INSERT OR IGNORE, which SILENTLY SKIPS a CHECK-violating
                        // row — so 'superseded' was dropped without error and this
                        // cursor stalled forever (nothing backfills it: the job
                        // query and the pending-work probe both exclude superseded
                        // edges). 'up_to_date' is the CHECK-valid, non-'failed'
                        // terminal and is semantically exact here: the row is
                        // tombstoned and its vector shadow just deleted, so there is
                        // no further projection work for this cursor. Same reasoning
                        // and same token as the step-23 backfill (fix-4, TC-33).
                        record_projection_terminal(tx, *sc as u64, "up_to_date")?;
                    }
                }
                // G11 — invalidate-not-accumulate: for fact-edges (body IS NOT NULL),
                // tombstone any prior active edge on the same (from_id, to_id, kind)
                // BEFORE inserting the new row. This is DIFFERENT from the G0
                // logical_id tombstone: it is keyed on the triple, not the identity.
                // Regular edges (body=None) skip this path — they retain G0 semantics.
                if body.is_some() {
                    // fix-30 [P2]: collect and prune vector shadow for the superseded edge.
                    let prior_g11 = prior_edge_cursors_by_triple(tx, from, to, kind)?;
                    tx.execute(
                        "UPDATE canonical_edges SET superseded_at = ?1
                         WHERE from_id = ?2 AND to_id = ?3 AND kind = ?4 AND superseded_at IS NULL",
                        params![cursor, from, to, kind],
                    )?;
                    for sc in &prior_g11 {
                        delete_vector_partition_row(tx, *sc)?;
                        tx.execute(
                            "DELETE FROM _fathomdb_vector_rows WHERE write_cursor = ?1",
                            [sc],
                        )?;
                        // fix-32 [P2]: mark terminal so projection cursor can advance.
                        // TC-45: 'up_to_date', NOT 'superseded' — see the identical
                        // note on the G0 prune loop above. The step-7 CHECK admits
                        // only ('failed','up_to_date') and INSERT OR IGNORE swallows
                        // a violating row, so 'superseded' never landed and wedged
                        // the shared readiness watermark.
                        record_projection_terminal(tx, *sc as u64, "up_to_date")?;
                    }
                }
                let temporal_fallback_i: Option<i64> =
                    temporal_fallback.and_then(|f| if f { Some(1) } else { None });
                tx.execute(
                    "INSERT INTO canonical_edges(
                         write_cursor, kind, from_id, to_id, source_id, logical_id,
                         body, t_valid, t_invalid, confidence, extractor_model_id,
                         temporal_fallback
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        cursor,
                        kind,
                        from,
                        to,
                        source_id.as_str(),
                        logical_id,
                        body,
                        t_valid,
                        t_invalid,
                        confidence,
                        extractor_model_id,
                        temporal_fallback_i
                    ],
                )?;
                // 0.8.20 Slice 5a (R-20-E1, work item 1) — edge projection is no
                // longer inlined here: the write path and the rebuild replay
                // share ONE projector, so they cannot drift.
                project_canonical_edge_row(
                    tx,
                    cursor,
                    kind,
                    body.as_deref(),
                    ProjectionPass::Write,
                )?;
            }
            (
                PreparedWrite::AdminSchema { name, kind, schema_json, retention_json },
                WritePlan::AdminSchema,
            ) => {
                tx.execute(
                    "INSERT INTO operational_collections(
                        name, kind, schema_json, retention_json, format_version, created_at
                     ) VALUES(?1, ?2, ?3, ?4, 1, 0)
                     ON CONFLICT(name) DO UPDATE SET
                        schema_json = excluded.schema_json,
                        retention_json = excluded.retention_json",
                    params![name, kind, schema_json, retention_json],
                )?;
                record_projection_terminal(tx, cursor, "up_to_date")?;
            }
            (
                PreparedWrite::OpStore { collection, record_key, schema_id, body },
                WritePlan::AppendOnlyLog,
            ) => {
                tx.execute(
                    "INSERT INTO operational_mutations(
                        collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
                     ) VALUES(?1, ?2, 'append', ?3, ?4, ?5)",
                    params![collection, record_key, body, schema_id, cursor],
                )?;
                record_projection_terminal(tx, cursor, "up_to_date")?;
            }
            (
                PreparedWrite::OpStore { collection, record_key, schema_id, body },
                WritePlan::LatestState,
            ) => {
                tx.execute(
                    "INSERT INTO operational_state(
                        collection_name, record_key, payload_json, schema_id, write_cursor
                     ) VALUES(?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(collection_name, record_key) DO UPDATE SET
                        payload_json = excluded.payload_json,
                        schema_id = excluded.schema_id,
                        write_cursor = excluded.write_cursor",
                    params![collection, record_key, body, schema_id, cursor],
                )?;
                record_projection_terminal(tx, cursor, "up_to_date")?;
            }
            _ => return Err(rusqlite::Error::InvalidQuery.into()),
        }
        register_artifact_identity(tx, original_write, cursor)?;
    }

    // G8 (Slice 20 / F10) — cross-row dangling-edge flag-and-count. This runs
    // AFTER the batch loop (so every same-batch node is already on disk in `tx`
    // and a same-batch later-inserted endpoint is visible) and BEFORE retention /
    // projection-cursor / commit. It is the cross-row reason this lives here and
    // not in single-row pre-insert `validate_write`. Default is FLAG-AND-COUNT:
    // we only COUNT, never roll back (strict-mode rollback is deferred to
    // reserved-gap band 22 — adding a write-options surface is out of scope).
    //
    // Probe is `logical_id`-alone against the step-12 partial index
    // `canonical_nodes_logical_active_idx ON canonical_nodes(logical_id)
    // WHERE superseded_at IS NULL` (its leading column + partial predicate), so
    // it SEARCHes the index with no SCAN (see `tests/pr_g8_dangling_edges.rs`
    // case (f)). There is no node-kind to match: `canonical_edges` stores only
    // the edge's own kind, not the endpoint node's kind.
    let dangling_edge_endpoints = {
        // O(N) pre-pass: record, per `logical_id`, the LAST (highest) index at
        // which an `Edge { logical_id: Some(_), .. }` with that id appears. Keyed
        // by `logical_id` ALONE (Decision 5, HITL-SIGNED 2026-06-05) to match the
        // supersession UPDATE, which keys by logical_id alone: a kind-change
        // re-ingest of the same edge logical_id SUPERSEDES the earlier one.
        // Iterating front-to-back and overwriting means the stored value ends up
        // as the final index for each id. An edge at index `i` with that id is
        // then in-batch-superseded iff `last_index[lid] > i`. This is
        // behavior-identical to the prior per-edge `batch[i+1..]` `.any(..)` scan
        // (which was O(N²) under the single-writer txn) — same skip-set, same count.
        let mut last_index: HashMap<String, usize> = HashMap::new();
        for (i, write) in batch.iter().enumerate() {
            let write = storage_write_shape(write);
            if let PreparedWrite::Edge { logical_id: Some(lid), .. } = write.as_ref() {
                last_index.insert(lid.clone(), i);
            }
        }

        let mut probe = tx.prepare(
            "SELECT 1 FROM canonical_nodes WHERE logical_id = ?1 AND superseded_at IS NULL LIMIT 1",
        )?;
        let mut count: u64 = 0;
        for (i, write) in batch.iter().enumerate() {
            let write = storage_write_shape(write);
            if let PreparedWrite::Edge { from, to, logical_id, .. } = write.as_ref() {
                // Honor `edge.superseded_at IS NULL`: an edge inserted in this
                // batch is active unless a LATER same-batch edge with the same
                // `Some(logical_id)` tombstoned it (the loop's supersession
                // UPDATE). Skip such an in-batch-superseded edge. Edges with
                // `logical_id: None` are never superseded-in-batch.
                if let Some(lid) = logical_id {
                    let superseded_in_batch =
                        last_index.get(lid.as_str()).is_some_and(|&last| last > i);
                    if superseded_in_batch {
                        continue;
                    }
                }
                // Probe `from_id` and `to_id` independently (0, 1, or 2 per edge).
                for endpoint in [from, to] {
                    if !probe.exists(params![endpoint])? {
                        count = count.saturating_add(1);
                    }
                }
            }
        }
        count
    };

    enforce_provenance_retention(tx, provenance_row_cap)?;
    advance_projection_cursor(tx)?;

    Ok((dangling_edge_endpoints, unstranded, closure_ids))
}

/// Cap sweep over the op-store mutation log: keeps the newest `cap` SWEEPABLE
/// rows, dropping the oldest by `id`.
///
/// **Pending-redaction exemption (0.8.20 Slice 5 fix-1).**
/// [`ERASURE_PENDING_REDACTION_COLLECTION`] is exempt on the same principle for a
/// stronger reason: that row is not a record of a discharged obligation but an
/// UNDISCHARGED one. Sweeping it would silently drop an erasure the engine still
/// owes, and the next retry would then report success with the leaked stable ids
/// still in the telemetry sink — exactly the R-20-E5 violation this mechanism
/// exists to prevent.
///
/// **Erasure-audit exemption (0.8.20 Slice 5b, design v5 §2 defect D-A;
/// HITL-ruled 2026-07-19: *"there must be an auditable record of deletion
/// event."*).** Rows in [`ERASURE_AUDIT_COLLECTIONS`] are excluded from BOTH the
/// count and the DELETE, and are therefore **never removed by retention
/// pressure**. Previously this swept `operational_mutations` cap-first,
/// oldest-`id`-first, with no collection filter — so the `excise_source_audit`
/// row proving an erasure occurred shared one retention pool with the very
/// payloads it must prove erased, and (being written before whatever workload
/// followed) was among the first evicted. Accountability is a distinct
/// obligation from erasure; a sweep must not silently discharge it.
///
/// Consequence of excluding audit rows from the count: `cap` is a cap on
/// SWEEPABLE rows, not on the physical table size. That is deliberate — the
/// alternative (counting exempt rows toward the cap) would let a growing audit
/// trail evict ordinary provenance ever more aggressively, and in the limit
/// leave nothing sweepable while the sweep churned every write.
fn enforce_provenance_retention(connection: &Connection, cap: u64) -> rusqlite::Result<()> {
    if cap == 0 {
        return Ok(());
    }
    // Static, engine-internal identifiers — no caller input reaches this SQL.
    let exempt = ERASURE_AUDIT_COLLECTIONS
        .iter()
        .copied()
        .chain(std::iter::once(ERASURE_PENDING_REDACTION_COLLECTION))
        .map(|name| format!("'{name}'"))
        .collect::<Vec<_>>()
        .join(", ");
    let slack = cap.max(20) / 20;
    let upper = cap.saturating_add(slack.max(1));
    let count: u64 = connection.query_row(
        &format!(
            "SELECT COUNT(*) FROM operational_mutations
             WHERE collection_name NOT IN ({exempt})"
        ),
        [],
        |row| row.get(0),
    )?;
    if count <= upper {
        return Ok(());
    }
    let to_delete = count.saturating_sub(cap);
    connection.execute(
        &format!(
            "DELETE FROM operational_mutations
             WHERE id IN (
                 SELECT id FROM operational_mutations
                 WHERE collection_name NOT IN ({exempt})
                 ORDER BY id
                 LIMIT ?1
             )"
        ),
        [to_delete],
    )?;
    Ok(())
}
