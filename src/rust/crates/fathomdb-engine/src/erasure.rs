use super::*;

/// 0.8.20 Slice 5b (R-20-E5) — how many times an erasure verb re-tries
/// `PRAGMA wal_checkpoint(TRUNCATE)` before refusing with
/// [`EngineError::ErasureIncomplete`]. Deliberately small: a concurrent reader
/// pinning a WAL snapshot can hold it for an unbounded time, and an erasure verb
/// must fail loudly rather than block a caller indefinitely.
pub(crate) const ERASURE_WAL_TRUNCATE_ATTEMPTS: u32 = 5;
/// 0.8.20 Slice 5b (R-20-E5) — pause between WAL-truncation attempts
/// (~100 ms total budget across [`ERASURE_WAL_TRUNCATE_ATTEMPTS`]).
pub(crate) const ERASURE_WAL_TRUNCATE_BACKOFF_MS: u64 = 25;

/// Phase 9 Pack B excise report (AC-028a/b/c). Counts are post-excise
/// totals; `projections_invalidated` reports the shadow-row invalidation
/// total (FTS5 + vec0 + projection terminal) for the excised source.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExciseReport {
    pub source_ref: String,
    pub nodes_excised: u64,
    pub edges_excised: u64,
    pub projections_invalidated: u64,
}

/// 0.8.20 Slice 5b (R-20-E7) — outcome of
/// [`Engine::excise_collection_record`]. `records_excised` counts the erased
/// `operational_mutations` versions (an append-only-log collection keeps every
/// version of a key); `state_rows_excised` counts the erased
/// `operational_state` row (0 or 1).
///
/// `record_digest` is `SHA-256(collection + 0x1F + record_key)` — the audit
/// handle. The raw `record_key` is deliberately NOT carried: it is arbitrary
/// caller-supplied text and may itself be the identifier being erased, so
/// echoing it into a durable audit row would defeat the erasure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExciseRecordReport {
    pub collection: String,
    pub record_digest: String,
    pub records_excised: u64,
    pub state_rows_excised: u64,
}

/// 0.8.20 Slice 5b (R-20-E6) — rewrite a telemetry JSONL sink with every
/// `result_stable_ids` element in `erased` replaced by [`REDACTED_STABLE_ID`].
///
/// **Selective, never truncating.** Every line is carried across: records that
/// reference no erased id are byte-identical, records that do keep their shape
/// and only lose the matching id VALUES, and a line that is not an
/// engine-authored JSON event (an operator note, a hand-appended record) is
/// copied through verbatim. The sink is a caller-supplied path that may hold
/// unrelated eval history; destroying it is not part of any erasure obligation.
///
/// **Crash safety.** Write-temp-then-`rename`: the redacted content goes to a
/// sibling `<sink>.redact.tmp`, is `sync_all`ed, and is then atomically renamed
/// over the sink. A crash at any point leaves either the intact old file or the
/// complete new one — never a half-rewritten sink. `rename` is atomic because
/// the temp file is a sibling (same directory ⇒ same filesystem).
///
/// **Concurrent appends.** The caller holds the telemetry mutex, so no
/// in-process `capture_telemetry` can append into the window. An out-of-process
/// appender is still possible (the sink is just a file), so before renaming we
/// re-check the source length: if it grew, the tail delta is read, redacted and
/// appended, and the check repeats — bounded, so a pathologically hot external
/// writer surfaces as an error rather than an unbounded loop.
fn redact_jsonl_stable_ids(
    path: &Path,
    erased: &std::collections::HashSet<&str>,
) -> std::io::Result<()> {
    /// Bound on the re-check loop for an out-of-process appender.
    const MAX_TAIL_FOLDS: usize = 8;

    let mut source = std::fs::read(path)?;
    let mut redacted = redact_jsonl_bytes(&source, erased);

    for _ in 0..MAX_TAIL_FOLDS {
        let current = std::fs::read(path)?;
        if current.len() == source.len() {
            let mut tmp_name = path.file_name().unwrap_or_default().to_os_string();
            tmp_name.push(".redact.tmp");
            let tmp = path.with_file_name(tmp_name);
            {
                let mut file = std::fs::File::create(&tmp)?;
                file.write_all(&redacted)?;
                file.sync_all()?;
            }
            std::fs::rename(&tmp, path)?;
            return Ok(());
        }
        // Someone appended while we were building the replacement: fold the
        // delta in (redacted) rather than dropping it, then re-check.
        if current.len() > source.len() && current.starts_with(&source) {
            redacted.extend_from_slice(&redact_jsonl_bytes(&current[source.len()..], erased));
        } else {
            // The file was rewritten under us, not appended to. Start over.
            redacted = redact_jsonl_bytes(&current, erased);
        }
        source = current;
    }
    Err(std::io::Error::other(format!(
        "telemetry sink {} is being appended to faster than it can be redacted",
        path.display()
    )))
}

/// Line-wise redaction of a JSONL byte buffer. Non-JSON and non-event lines are
/// passed through unchanged, as is a trailing partial line (no terminating
/// newline) — the sink is append-only, so a partial tail is a torn write, not
/// ours to normalize.
fn redact_jsonl_bytes(bytes: &[u8], erased: &std::collections::HashSet<&str>) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    let mut rest = bytes;
    while !rest.is_empty() {
        let (line, tail) = match rest.iter().position(|b| *b == b'\n') {
            Some(idx) => (&rest[..idx], &rest[idx + 1..]),
            // No trailing newline: a torn/partial final line. Pass it through.
            None => {
                out.extend_from_slice(rest);
                break;
            }
        };
        match redact_jsonl_line(line, erased) {
            Some(replacement) => out.extend_from_slice(replacement.as_bytes()),
            None => out.extend_from_slice(line),
        }
        out.push(b'\n');
        rest = tail;
    }
    out
}

/// `Some(replacement)` when the line is an engine-authored telemetry event whose
/// `result_stable_ids` referenced an erased id; `None` to pass it through.
fn redact_jsonl_line(line: &[u8], erased: &std::collections::HashSet<&str>) -> Option<String> {
    let text = std::str::from_utf8(line).ok()?;
    let mut value: serde_json::Value = serde_json::from_str(text).ok()?;
    let ids = value.get_mut("result_stable_ids")?.as_array_mut()?;
    let mut touched = false;
    for id in ids.iter_mut() {
        if id.as_str().is_some_and(|s| erased.contains(s)) {
            *id = serde_json::Value::from(REDACTED_STABLE_ID);
            touched = true;
        }
    }
    touched.then(|| value.to_string())
}

/// 0.8.20 Slice 5b (R-20-E6) — the prefixed stable ids
/// ([`IdSpace::to_prefixed`]) of the canonical rows an erasure verb is about to
/// delete, so they can be redacted from the telemetry sink.
///
/// Must be called INSIDE the erasing transaction and BEFORE the DELETEs — after
/// them the rows, and with them the `logical_id`/`body` the ids derive from, are
/// gone. Both queries take one bound parameter (`?1`), applied to nodes and
/// edges respectively; `derive_stable_id` reproduces exactly what
/// `capture_telemetry` wrote into `result_stable_ids`.
fn collect_erased_stable_ids(
    tx: &Connection,
    node_sql: &str,
    edge_sql: &str,
    bind: &str,
) -> Result<Vec<String>, EngineError> {
    let mut ids = Vec::new();
    for sql in [node_sql, edge_sql] {
        let mut stmt = tx.prepare(sql).map_err(|_| EngineError::Storage)?;
        let rows = stmt
            .query_map(params![bind], |row| {
                Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?))
            })
            .map_err(|_| EngineError::Storage)?;
        for row in rows {
            let (logical_id, body) = row.map_err(|_| EngineError::Storage)?;
            ids.push(
                derive_stable_id(logical_id.as_deref(), body.as_deref().unwrap_or(""))
                    .to_prefixed(),
            );
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

fn collect_erased_stable_ids_for_cursors(
    connection: &Connection,
    node_cursors: &[i64],
    edge_cursors: &[i64],
) -> Result<Vec<String>, EngineError> {
    let mut ids = Vec::new();
    for (table, cursors) in [("canonical_nodes", node_cursors), ("canonical_edges", edge_cursors)] {
        let sql = format!("SELECT logical_id,body FROM {table} WHERE write_cursor=?1");
        for cursor in cursors {
            let row: Option<(Option<String>, Option<String>)> = connection
                .query_row(&sql, [cursor], |row| Ok((row.get(0)?, row.get(1)?)))
                .optional()
                .map_err(|_| EngineError::Storage)?;
            if let Some((logical_id, body)) = row {
                ids.push(
                    derive_stable_id(logical_id.as_deref(), body.as_deref().unwrap_or(""))
                        .to_prefixed(),
                );
            }
        }
    }
    ids.sort_unstable();
    ids.dedup();
    Ok(ids)
}

/// 0.8.20 Slice 5 fix-1 (codex §9 P2) — record, INSIDE the erasing transaction,
/// that a telemetry redaction is owed for `erased_stable_ids`.
///
/// Must be called in the same transaction as the DELETEs. That is the whole
/// point: "the rows are gone" and "a redaction is owed for them" then commit
/// atomically, so no crash or failure can leave the first true and the second
/// unrecorded. [`Engine::discharge_pending_redactions`] drains the queue and
/// deletes the entry only once the sink has actually been rewritten.
///
/// `record_key` is the VERB, never a stable id — the ids live in the payload,
/// which is deleted on discharge.
fn enqueue_pending_redaction(
    tx: &Connection,
    verb: &str,
    erased_stable_ids: &[String],
    write_cursor: u64,
) -> Result<(), EngineError> {
    if erased_stable_ids.is_empty() {
        return Ok(());
    }
    let payload =
        serde_json::json!({ "verb": verb, "erased_stable_ids": erased_stable_ids }).to_string();
    tx.execute(
        "INSERT INTO operational_mutations(
            collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
         ) VALUES(?1, ?2, 'append', ?3, NULL, ?4)",
        params![ERASURE_PENDING_REDACTION_COLLECTION, verb, payload, write_cursor],
    )
    .map_err(|_| EngineError::Storage)?;
    Ok(())
}

fn delete_dependencies_for_erased_revisions(
    connection: &Connection,
    affected: &std::collections::HashSet<i64>,
) -> Result<(), EngineError> {
    let mut statement = connection
        .prepare(
            "SELECT d.dependency_id, d.derived_revision_id, l.source_revision_id, \
                    ar.write_cursor \
             FROM _fathomdb_source_dependencies d \
             LEFT JOIN _fathomdb_source_links l \
               ON l.artifact_revision_id=d.derived_revision_id \
             LEFT JOIN _fathomdb_artifact_revisions ar \
               ON ar.revision_id=d.derived_revision_id",
        )
        .map_err(|_| EngineError::Storage)?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<i64>>(3)?,
            ))
        })
        .map_err(|_| EngineError::Storage)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(|_| EngineError::Storage)?;
    let mut delete_ids = Vec::new();
    for (dependency_id, derived_revision, source_revision, owner_cursor) in rows {
        let source_revision = source_revision.ok_or(EngineError::Storage)?;
        validate_dependency_chain(
            connection,
            &source_revision,
            &derived_revision,
            DependencyValidationMode::Persisted,
        )?;
        let derived_erased = owner_cursor.is_some_and(|cursor| affected.contains(&cursor));
        if derived_erased {
            delete_ids.push(dependency_id);
        }
    }
    if delete_ids.is_empty() {
        return Ok(());
    }
    let next = reserve_dependency_generation(connection)?;
    for dependency_id in delete_ids {
        connection
            .execute(
                "DELETE FROM _fathomdb_source_dependencies WHERE dependency_id=?1",
                [dependency_id],
            )
            .map_err(|_| EngineError::Storage)?;
    }
    store_dependency_generation(connection, next)
}

fn actuation_erasure_refs_for_cursors(
    connection: &Connection,
    cursors: &[i64],
    source_ids: impl IntoIterator<Item = String>,
) -> Result<BTreeSet<(String, String)>, EngineError> {
    let mut refs = source_ids
        .into_iter()
        .map(|value| ("source_id".to_string(), value))
        .collect::<BTreeSet<_>>();
    for cursor in cursors {
        let revision: Option<String> = connection
            .query_row(
                "SELECT revision_id FROM _fathomdb_artifact_revisions WHERE write_cursor=?1",
                [cursor],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        if let Some(revision) = revision {
            refs.insert(("artifact_revision_id".to_string(), revision.clone()));
            refs.insert(("source_revision_id".to_string(), revision));
        }
    }
    Ok(refs)
}

fn erase_artifact_identity_for_cursors(
    tx: &Connection,
    cursors: &[i64],
    refuse_surviving_dependents: bool,
    source_bucket: Option<&str>,
) -> Result<(), EngineError> {
    let affected: std::collections::HashSet<i64> = cursors.iter().copied().collect();
    let mut revisions: Vec<(String, String)> = Vec::new();
    for cursor in cursors {
        let row = tx
            .query_row(
                "SELECT revision_id, artifact_role FROM _fathomdb_artifact_revisions \
                 WHERE write_cursor = ?1",
                [cursor],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        if let Some(row) = row {
            revisions.push(row);
        }
    }

    delete_dependencies_for_erased_revisions(tx, &affected)?;

    if refuse_surviving_dependents {
        for (revision_id, role) in &revisions {
            if role != "canonical_source" {
                continue;
            }
            let mut statement = tx
                .prepare(
                    "SELECT ar.write_cursor FROM _fathomdb_source_links link \
                     LEFT JOIN _fathomdb_artifact_revisions ar \
                       ON ar.revision_id = link.artifact_revision_id \
                     WHERE link.source_revision_id = ?1",
                )
                .map_err(|_| EngineError::Storage)?;
            let dependent_cursors = statement
                .query_map([revision_id], |row| row.get::<_, Option<i64>>(0))
                .map_err(|_| EngineError::Storage)?;
            for dependent in dependent_cursors {
                let dependent = dependent.map_err(|_| EngineError::Storage)?;
                if dependent.is_none_or(|cursor| !affected.contains(&cursor)) {
                    return Err(EngineError::Provenance(ProvenanceError::new(
                        ProvenanceErrorReason::ProvenanceInUse,
                        "",
                    )));
                }
            }
        }
    }

    // Source erasure owns the complete source bucket. Before deleting raw
    // ownerless rows from that bucket, fail closed if a corrupt link points at
    // an owner outside the canonical cursor set: deleting only its link would
    // turn a complete artifact into a dangling owner.
    if let Some(source_id) = source_bucket {
        let mut statement = tx
            .prepare(
                "SELECT ar.write_cursor FROM _fathomdb_source_links link \
                 LEFT JOIN _fathomdb_artifact_revisions ar \
                   ON ar.revision_id = link.artifact_revision_id \
                 WHERE link.source_id = ?1",
            )
            .map_err(|_| EngineError::Storage)?;
        let owner_cursors = statement
            .query_map([source_id], |row| row.get::<_, Option<i64>>(0))
            .map_err(|_| EngineError::Storage)?;
        for owner_cursor in owner_cursors {
            if owner_cursor
                .map_err(|_| EngineError::Storage)?
                .is_some_and(|cursor| !affected.contains(&cursor))
            {
                return Err(EngineError::Storage);
            }
        }
    }

    for (revision_id, _) in &revisions {
        tx.execute(
            "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id = ?1",
            [revision_id],
        )
        .map_err(|_| EngineError::Storage)?;
    }
    for (revision_id, role) in &revisions {
        if role == "canonical_source" {
            tx.execute(
                "DELETE FROM _fathomdb_source_links WHERE source_revision_id = ?1",
                [revision_id],
            )
            .map_err(|_| EngineError::Storage)?;
            tx.execute(
                "DELETE FROM _fathomdb_source_versions WHERE source_revision_id = ?1",
                [revision_id],
            )
            .map_err(|_| EngineError::Storage)?;
        }
    }
    if let Some(source_id) = source_bucket {
        tx.execute("DELETE FROM _fathomdb_source_links WHERE source_id = ?1", [source_id])
            .map_err(|_| EngineError::Storage)?;
        tx.execute("DELETE FROM _fathomdb_source_versions WHERE source_id = ?1", [source_id])
            .map_err(|_| EngineError::Storage)?;
    }
    for cursor in cursors {
        tx.execute("DELETE FROM _fathomdb_artifact_revisions WHERE write_cursor = ?1", [cursor])
            .map_err(|_| EngineError::Storage)?;
    }

    for (revision_id, _) in &revisions {
        let orphans: i64 = tx
            .query_row(
                "SELECT \
                   EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions WHERE revision_id = ?1) + \
                   EXISTS(SELECT 1 FROM _fathomdb_source_links WHERE artifact_revision_id = ?1) + \
                   EXISTS(SELECT 1 FROM _fathomdb_source_links WHERE source_revision_id = ?1) + \
                   EXISTS(SELECT 1 FROM _fathomdb_source_versions WHERE source_revision_id = ?1)",
                [revision_id],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        if orphans != 0 {
            return Err(EngineError::Storage);
        }
    }
    if let Some(source_id) = source_bucket {
        let orphans: i64 = tx
            .query_row(
                "SELECT \
                   EXISTS(SELECT 1 FROM _fathomdb_source_links WHERE source_id = ?1) + \
                   EXISTS(SELECT 1 FROM _fathomdb_source_versions WHERE source_id = ?1)",
                [source_id],
                |row| row.get(0),
            )
            .map_err(|_| EngineError::Storage)?;
        if orphans != 0 {
            return Err(EngineError::Storage);
        }
    }
    Ok(())
}

impl Engine {
    /// Settle projection execution and freeze new scans for an erasure
    /// transaction. A direct [`Self::drain`] keeps Slice 30's immediate typed
    /// configuration feedback, but erasure must be able to remove pending work
    /// from a no-embedder session rather than returning that feedback instead
    /// of discharging the destructive request.
    fn freeze_projection_for_erasure(&self) -> Result<(), EngineError> {
        let no_configured_embedder = match self.drain(LIFECYCLE_DRAIN_TIMEOUT_MS) {
            Ok(()) => false,
            Err(EngineError::EmbedderRequired(_)) => true,
            Err(error) => return Err(error),
        };

        self.projection_runtime.set_frozen(true);
        let settled = if no_configured_embedder {
            // The dispatcher is frozen before the transaction. Existing jobs
            // may still be completing their final batch, so wait for those
            // workers only; durable rows that need an absent embedder are the
            // rows the following erasure transaction removes.
            if self.projection_runtime.wait_for_workers_idle(LIFECYCLE_DRAIN_TIMEOUT_MS) {
                Ok(())
            } else {
                Err(EngineError::Scheduler)
            }
        } else {
            // With a configured runtime, preserve the established two-drain
            // ordering: all durable work settles unfrozen, then no worker is
            // active once new scans are frozen.
            self.drain(LIFECYCLE_DRAIN_TIMEOUT_MS)
        };
        if settled.is_err() {
            self.projection_runtime.set_frozen(false);
        }
        settled
    }

    fn freeze_projection_for_closure_retry(&self) -> Result<(), EngineError> {
        self.projection_runtime.set_frozen(true);
        if self.projection_runtime.wait_for_workers_idle(LIFECYCLE_DRAIN_TIMEOUT_MS) {
            Ok(())
        } else {
            self.projection_runtime.set_frozen(false);
            Err(EngineError::Scheduler)
        }
    }

    fn finish_physical_dependency_closures(
        &self,
        verb: &'static str,
        closure_ids: &[ClosureOperationId],
    ) -> Result<(), EngineError> {
        {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            dependency_closure::validate_physical_closures(connection, closure_ids)?;
        }
        if let Err(error) = self.complete_erasure_at_rest(verb) {
            let blocker = match &error {
                EngineError::ErasureIncomplete { stage, .. }
                    if stage == "telemetry_redaction" || stage == "wal_checkpoint" =>
                {
                    Some(stage.as_str())
                }
                _ => None,
            };
            if let Some(blocker) = blocker {
                let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
                let connection = connection.as_ref().ok_or(EngineError::Closing)?;
                dependency_closure::mark_physical_incomplete(connection, closure_ids, blocker)?;
            }
            return Err(error);
        }
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        dependency_closure::complete_physical_closures(connection, closure_ids)
    }

    /// OPP-12 Phase-1 (0.8.19 Slice 10, R-PG-1/2) — irreversibly hard-erase a
    /// governed node. A SEPARATE verb from [`Engine::transition`] (NOT on the
    /// `recovery_denylist`). Precondition: DELETED-FIRST — legal only from
    /// `deleted` (else a typed [`EngineError::IllegalTransition`] to `purged`);
    /// IDEMPOTENT — purging an already-absent/already-purged id is a no-op
    /// success. Keys on the bare `logical_id` (`l:` only); `h:`/`p:` →
    /// [`EngineError::NotLifecycleAddressable`].
    ///
    /// In ONE transaction, physically erases every ROW-OWNED target for the node
    /// (design §3 / gap-3): all `canonical_nodes` versions; its `search_index`,
    /// `search_index_edges`, `search_index_v2` FTS rows; its `vector_default`
    /// (vec0) + `_fathomdb_vector_rows` vectors; its `_fathomdb_projection_terminal`
    /// bookkeeping; and — CASCADE-REMOVE, no content-free stubs — every
    /// `canonical_edges` row touching it (`from_id`/`to_id`) plus those edges'
    /// projection shadows. The global/kind-level registries
    /// `_fathomdb_projection_state` and `_fathomdb_vector_kinds` are NOT keyed to
    /// a node id and are DELIBERATELY untouched.
    ///
    /// Erasure completeness relies on the standing `PRAGMA secure_delete=ON`
    /// (design §3 gap-4) which zeroes every freed page — so no per-purge `VACUUM`.
    /// (Freelist content written on a pre-20 DB before `secure_delete` was on is a
    /// documented residual; there is no forced migration-time `VACUUM`.)
    pub fn purge(&self, logical_id: &str) -> Result<(), EngineError> {
        self.ensure_open()?;
        let lid = Self::resolve_lifecycle_target(logical_id)?;

        let pending = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            dependency_closure::pending_physical_retry(connection, "purge", &lid)?
        };
        if !pending.is_empty() {
            self.freeze_projection_for_closure_retry()?;
            let result = self.finish_physical_dependency_closures("purge", &pending);
            self.projection_runtime.set_frozen(false);
            return result;
        }

        // Drain in-flight projection work before the erase, exactly as
        // `excise_source` does: SQLite-WAL would otherwise let a worker that
        // already dequeued a job for a purged cursor commit its vec0 /
        // `_fathomdb_vector_rows` INSERT after our DELETE releases the writer
        // lock, leaving residue that defeats the erasure sweep.
        // With a configured runtime, settle every pending projection unfrozen
        // before freezing the scanner; with no configured runtime, freeze after
        // the immediate Slice-30 feedback and remove the pending row instead.
        self.freeze_projection_for_erasure()?;
        let outcome = (|| {
            let closure_ids = self.purge_inner(&lid)?;
            self.finish_physical_dependency_closures("purge", &closure_ids)
        })();
        self.projection_runtime.set_frozen(false);
        // 0.8.20 Slice 5b (R-20-E5/E6) — the rows are gone from the tables; now
        // finish the erasure AT REST (telemetry sink + `-wal` bytes) before
        // reporting success. Runs after the connection guard inside
        // `purge_inner` has been dropped: `complete_erasure_at_rest` re-acquires
        // it for the checkpoint.
        outcome
    }

    /// The erased rows' prefixed stable ids ([`IdSpace::to_prefixed`]) are NOT
    /// returned: they are enqueued for redaction inside this transaction (see
    /// [`enqueue_pending_redaction`]), because a caller-held vector is lost on the
    /// retry path that codex Â§9 P2 found.
    fn purge_inner(&self, lid: &str) -> Result<Vec<ClosureOperationId>, EngineError> {
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;

        // Precondition on the active row's state. Absent (never-created or
        // already-purged) → idempotent no-op success.
        let current: Option<String> = tx
            .query_row(
                "SELECT state FROM canonical_nodes \
                 WHERE logical_id = ?1 AND superseded_at IS NULL",
                params![lid],
                |r| r.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| EngineError::Storage)?;
        let from_state = match current {
            None => {
                // Idempotent: nothing to erase.
                tx.commit().map_err(|_| EngineError::Storage)?;
                return Ok(Vec::new());
            }
            Some(s) => LifecycleState::from_str_opt(&s).ok_or(EngineError::Storage)?,
        };
        if from_state != LifecycleState::Deleted {
            // Deleted-first precondition. Dropping `tx` rolls back (no-op read).
            return Err(EngineError::IllegalTransition {
                from_state,
                to_state: LifecycleState::Purged,
                legal: from_state.legal_next_states(),
            });
        }

        // Collect every version cursor for the node, plus every cursor of an edge
        // that touches it (either endpoint), across ALL versions — the projection
        // shadow tables are keyed by these per-row `write_cursor`s.
        let mut node_cursors: Vec<i64> = {
            let mut stmt = tx
                .prepare("SELECT write_cursor FROM canonical_nodes WHERE logical_id = ?1")
                .map_err(|_| EngineError::Storage)?;
            let rows = stmt
                .query_map(params![lid], |row| row.get::<_, i64>(0))
                .map_err(|_| EngineError::Storage)?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)?
        };
        let mut edge_cursors: Vec<i64> = {
            let mut stmt = tx
                .prepare(
                    "SELECT write_cursor FROM canonical_edges \
                     WHERE from_id = ?1 OR to_id = ?1",
                )
                .map_err(|_| EngineError::Storage)?;
            let rows = stmt
                .query_map(params![lid], |row| row.get::<_, i64>(0))
                .map_err(|_| EngineError::Storage)?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)?
        };

        let source_revisions = node_cursors
            .iter()
            .map(|cursor| dependency_closure::source_revision_for_cursor(&tx, *cursor))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let physical_plans =
            dependency_closure::physical_dependents_for_sources(&tx, &source_revisions)?;
        for (_, dependents) in &physical_plans {
            for (class, cursor) in dependents {
                match class.as_str() {
                    "node" => node_cursors.push(*cursor),
                    "edge" => edge_cursors.push(*cursor),
                    _ => return Err(EngineError::Storage),
                }
            }
        }
        node_cursors.sort_unstable();
        node_cursors.dedup();
        edge_cursors.sort_unstable();
        edge_cursors.dedup();

        // The stable ids the telemetry sink may have persisted, collected for
        // the complete expanded cursor set before any DELETE.
        let erased_stable_ids =
            collect_erased_stable_ids_for_cursors(&tx, &node_cursors, &edge_cursors)?;
        let pending_cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        let enqueued =
            self.telemetry_enabled.load(Ordering::Acquire) && !erased_stable_ids.is_empty();

        let mut closure_ids = Vec::new();
        let proof_boundary =
            if enqueued { pending_cursor } else { self.next_cursor.load(Ordering::SeqCst) };

        let affected_cursors: Vec<i64> =
            node_cursors.iter().chain(edge_cursors.iter()).copied().collect();
        let mut erased_source_ids = BTreeSet::new();
        for sql in [
            "SELECT source_id FROM canonical_nodes WHERE logical_id=?1 AND source_id IS NOT NULL",
            "SELECT source_id FROM canonical_edges WHERE (from_id=?1 OR to_id=?1) AND source_id IS NOT NULL",
        ] {
            let mut statement = tx.prepare(sql).map_err(|_| EngineError::Storage)?;
            let rows = statement
                .query_map([lid], |row| row.get::<_, String>(0))
                .map_err(|_| EngineError::Storage)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(|_| EngineError::Storage)?;
            erased_source_ids.extend(rows);
        }
        let receipt_refs =
            actuation_erasure_refs_for_cursors(&tx, &affected_cursors, erased_source_ids)?;
        let proof_scope =
            dependency_closure::physical_proof_scope(&tx, &affected_cursors, &receipt_refs, None)?;
        for (source_revision, dependents) in &physical_plans {
            if let Some(id) = dependency_closure::record_physical_closure(
                &tx,
                dependency_closure::PhysicalClosureAdmission {
                    root_kind: "source_revision",
                    root_value: source_revision,
                    retry_verb: "purge",
                    retry_argument: lid,
                    cause: ClosureCauseV1::Purged,
                    boundary: proof_boundary,
                    affected_count: dependents.len(),
                },
            )? {
                closure_ids.push(id);
            }
        }
        actuation::redact_actuation_receipts_for_refs(&tx, &receipt_refs)?;
        // Same hazard as `excise_source_inner`: receipt validation reads the
        // correction closures, so they are deleted only afterwards, inside
        // this transaction. Every erased revision is covered, not only those
        // with a current dependent plan, because a nonterminal soft closure can
        // outlive its dependents. This purge's own proof shares the
        // `source_revision` root, so only nonphysical causes are removed.
        for source_revision in &source_revisions {
            tx.execute(
                "DELETE FROM _fathomdb_dependency_closures \
                 WHERE root_kind='source_revision' AND root_value=?1 \
                   AND cause IN ('superseded','soft_deleted')",
                [source_revision],
            )
            .map_err(|_| EngineError::Storage)?;
        }
        erase_artifact_identity_for_cursors(&tx, &affected_cursors, true, None)?;

        // Erase the row-owned projection shadows for every collected cursor.
        // 0.8.20 Slice 5a (R-20-E1): registry-driven — the hand-rolled delete
        // list is gone, so a newly registered projection table is erased here
        // without touching this site. vec0 rowid == the canonical row's
        // write_cursor (see `_fathomdb_vector_rows`).
        for cursor in node_cursors.iter().chain(edge_cursors.iter()) {
            erase_row_projections(&tx, *cursor).map_err(|_| EngineError::Storage)?;
        }

        // Erase every exact row in the admitted physical closure, including
        // registered dependents whose logical identity is unrelated to the
        // purged root. The root-wide deletes that follow retain the pre-Slice-30
        // all-version and endpoint cascade contract.
        for cursor in &node_cursors {
            tx.execute("DELETE FROM canonical_nodes WHERE write_cursor = ?1", [cursor])
                .map_err(|_| EngineError::Storage)?;
        }
        for cursor in &edge_cursors {
            tx.execute("DELETE FROM canonical_edges WHERE write_cursor = ?1", [cursor])
                .map_err(|_| EngineError::Storage)?;
        }

        // Erase the canonical root rows: all node versions + all touching edges
        // (gap-3 CASCADE-REMOVE — no content-free stubs in Phase-1).
        tx.execute("DELETE FROM canonical_nodes WHERE logical_id = ?1", params![lid])
            .map_err(|_| EngineError::Storage)?;
        tx.execute("DELETE FROM canonical_edges WHERE from_id = ?1 OR to_id = ?1", params![lid])
            .map_err(|_| EngineError::Storage)?;

        // 0.8.20 Slice 5 fix-1 (codex §9 P2) — durably record the redaction this
        // erasure now owes, atomically with the deletes above. Only when a sink
        // is attached: with telemetry never enabled there is no file the ids
        // could have leaked into, so there is nothing to owe.
        if enqueued {
            enqueue_pending_redaction(&tx, "purge", &erased_stable_ids, pending_cursor)?;
        }

        dependency_closure::measure_physical_closures(&tx, &closure_ids, &proof_scope)?;

        tx.commit().map_err(|_| EngineError::Storage)?;
        if enqueued {
            self.next_cursor.store(pending_cursor, Ordering::SeqCst);
        }
        self.counters.record_admin();
        Ok(closure_ids)
    }

    /// 0.8.20 Slice 5d (R-20-E4, design §4 item 9b) — the **governed SDK
    /// erasure verb**. Deletes every canonical row attributable to `source_id`,
    /// plus its row-owned projections, and finishes the erasure at rest.
    ///
    /// This is NOT `operator`-gated: erasing content a consumer wrote is an
    /// application obligation, not a recovery workflow. Before this slice the
    /// only erasure path was [`Engine::excise_source`], which lives behind the
    /// operator feature (i.e. the CLI), so an SDK-only consumer holding a
    /// deletion obligation over ANONYMOUS content — content with no
    /// `logical_id`, therefore not reachable by [`Engine::purge`] — had no way
    /// to discharge it at all. That gap is what R-20-E4 closes.
    ///
    /// **One engine path.** `erase_source` and `excise_source` are the SAME
    /// operation: both delegate to [`Engine::erase_source_shared`]. They are
    /// not competing implementations, and no behaviour is duplicated.
    ///
    /// **Validation differs, deliberately.** `erase_source` admits only ids
    /// [`SourceId::new`] would admit, so a caller cannot aim the governed verb
    /// at the engine's reserved `_`-prefixed namespace (`_engine:*` substrate,
    /// or the `_legacy:pre-0.8.20` cohort migration step 21 back-filled — a
    /// single call against which would erase every pre-0.8.20 anonymous row).
    /// `excise_source` stays permissive precisely BECAUSE it is the recovery
    /// seam: R-20-E8 requires an operator to be able to excise `_legacy:`.
    ///
    /// **Not a recovery verb.** `erase_source` carries no REQ-054
    /// recovery-denylist name (`{recover, restore, repair, fix, rebuild}`); it
    /// is a lifecycle verb alongside `transition`/`purge`. AC-041 is unaffected.
    ///
    /// # Errors
    ///
    /// [`EngineError::WriteValidation`] for an empty, whitespace-only or
    /// reserved `source_id`; [`EngineError::ErasureIncomplete`] if the erasure
    /// could not be completed at rest (see [`Engine::complete_erasure_at_rest`]).
    pub fn erase_source(&self, source_id: &str) -> Result<ExciseReport, EngineError> {
        // Construct-to-validate: reuse the newtype's rule rather than restating
        // it, so the erasure boundary and the write boundary cannot drift.
        let _validated = SourceId::new(source_id)?;
        self.erase_source_shared("erase_source", source_id)
    }

    /// Phase 9 Pack B / AC-028a/b/c source excise — the **operator/recovery**
    /// spelling of [`Engine::erase_source`], sharing one engine path with it.
    ///
    /// Kept `operator`-gated and kept permissive about reserved ids: this is
    /// the seam an operator uses to excise `_legacy:pre-0.8.20` (R-20-E8) or
    /// `_engine:*` substrate, which the governed SDK verb refuses.
    #[cfg(feature = "operator")]
    pub fn excise_source(&self, source_id: &str) -> Result<ExciseReport, EngineError> {
        if source_id.is_empty() {
            self.ensure_open()?;
            return Err(EngineError::WriteValidation);
        }
        self.erase_source_shared("excise_source", source_id)
    }

    /// The single erasure implementation behind [`Engine::erase_source`] and
    /// [`Engine::excise_source`]. `verb` names the caller for the telemetry
    /// redaction record only; the deletion semantics are identical.
    ///
    /// Non-perturbation: rows from other sources (and rows with NULL
    /// `source_id`) are untouched; the projection cursor is NOT reset
    /// and no blanket projection rebuild is issued.
    fn erase_source_shared(
        &self,
        verb: &'static str,
        source_id: &str,
    ) -> Result<ExciseReport, EngineError> {
        self.ensure_open()?;

        #[cfg(feature = "test-hooks")]
        {
            if let Some(hook) = self
                .erasure_before_primary_lock_hook
                .lock()
                .map_err(|_| EngineError::Storage)?
                .take()
            {
                hook();
            }
            slice15_erasure_lock_hook::fire();
        }

        let pending = {
            let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
            let connection = connection.as_ref().ok_or(EngineError::Closing)?;
            dependency_closure::pending_physical_retry(connection, verb, source_id)?
        };
        if !pending.is_empty() {
            self.freeze_projection_for_closure_retry()?;
            let result = (|| {
                self.finish_physical_dependency_closures(verb, &pending)?;
                Ok(ExciseReport {
                    source_ref: source_id.to_string(),
                    nodes_excised: 0,
                    edges_excised: 0,
                    projections_invalidated: 0,
                })
            })();
            self.projection_runtime.set_frozen(false);
            return result;
        }

        // Drain MUST succeed before the excise transaction. SQLite-WAL
        // would otherwise allow a worker that already dequeued a job
        // for an excised cursor to commit its INSERT into vec0 /
        // _fathomdb_vector_rows after our DELETE releases the writer
        // lock, leaving residue and breaking AC-028b. Surface the
        // timeout instead of swallowing it (Pack A pattern).
        //
        // The helper preserves the configured-runtime drain-before-freeze
        // ordering, while allowing this destructive operation to delete
        // otherwise-unserviceable no-embedder rows.
        self.freeze_projection_for_erasure()?;
        let outcome = (|| {
            let (report, closure_ids) = self.excise_source_inner(verb, source_id)?;
            self.finish_physical_dependency_closures(verb, &closure_ids)?;
            Ok(report)
        })();
        self.projection_runtime.set_frozen(false);
        // 0.8.20 Slice 5b (R-20-E5/E6) — finish the erasure AT REST before
        // reporting success: redact the erased stable ids out of the telemetry
        // sink, then truncate the `-wal` so the erased bytes are not still
        // readable on disk. On persistent checkpoint BUSY this returns
        // `ErasureIncomplete` rather than an `ExciseReport`.
        outcome
    }

    /// 0.8.20 Slice 5b (R-20-E5) — complete an erasure **at rest** after the
    /// erasing transaction has committed. Two obligations, in order:
    ///
    /// 1. **Telemetry redaction** — drop the erased stable ids out of the opt-in
    ///    telemetry sink. Driven from the DURABLE pending queue
    ///    ([`Engine::discharge_pending_redactions`]), NOT from the ids the caller
    ///    happens to be holding, so a retry after a failed redaction still knows
    ///    what it owes.
    /// 2. **WAL truncation** — `wal_checkpoint(TRUNCATE)` with a BOUNDED retry.
    ///    `PRAGMA secure_delete=ON` zeroes pages freed inside the database file,
    ///    but the erased content also lives in the write-ahead log as committed
    ///    frames from the ORIGINAL insert; the erasure DELETE appends new frames
    ///    rather than rewriting old ones, so without a truncating checkpoint the
    ///    erased body stays `grep`-able in `<db>-wal`.
    ///
    /// A concurrent reader pinning a WAL snapshot makes the checkpoint report
    /// `busy`. After [`ERASURE_WAL_TRUNCATE_ATTEMPTS`] tries the verb raises
    /// [`EngineError::ErasureIncomplete`] — **an erasure verb must never report
    /// success on an incomplete erasure.** The retry budget is deliberately small
    /// (~100 ms total): the caller retries the verb, the verb does not block.
    pub(crate) fn complete_erasure_at_rest(&self, verb: &'static str) -> Result<(), EngineError> {
        // The ids are NOT passed in: they were persisted inside the erasing
        // transaction, and this drains that queue. The WAL truncation below then
        // runs AFTER the pending rows have been deleted, so the freed pages
        // holding them (zeroed by `secure_delete=ON`) are checkpointed out too.
        self.discharge_pending_redactions(verb)?;

        let mut last: Option<TruncateWalReport> = None;
        for attempt in 0..ERASURE_WAL_TRUNCATE_ATTEMPTS {
            self.wal_attribution.set(WalAttributionRole::Writer, 0, true, "checkpoint_start");
            let overlap = self.wal_attribution.checkpoint_begin();
            #[cfg(any(test, feature = "test-hooks"))]
            self.actual_checkpoint_observation_for_test(
                "before",
                (attempt + 1) as usize,
                overlap,
                None,
                None,
            );
            let started = Instant::now();
            let checkpoint_result = self.wal_checkpoint_truncate_once(false);
            let elapsed = started.elapsed();
            #[cfg(any(test, feature = "test-hooks"))]
            self.actual_checkpoint_observation_for_test(
                "after",
                (attempt + 1) as usize,
                overlap,
                Some(elapsed),
                checkpoint_result.as_ref().ok().cloned(),
            );
            let snapshot = self.wal_attribution.snapshot();
            self.wal_attribution.checkpoint_end();
            self.wal_attribution.set(WalAttributionRole::Writer, 0, false, "idle");
            let report = checkpoint_result?;
            let mut classification = self.wal_attribution.classification(&snapshot, overlap);
            if report.status == TruncateWalStatus::Busy && classification == "no_owned_snapshot" {
                classification = "unclassified_external";
            }
            self.wal_attribution.checkpoint_event(
                (attempt + 1) as usize,
                elapsed,
                &report,
                classification,
                snapshot.active_roles,
            );
            if report.status == TruncateWalStatus::Done {
                return Ok(());
            }
            last = Some(report);
            if attempt + 1 < ERASURE_WAL_TRUNCATE_ATTEMPTS {
                std::thread::sleep(Duration::from_millis(ERASURE_WAL_TRUNCATE_BACKOFF_MS));
            }
        }
        let frames = last.map_or(0, |r| r.log_frames);
        Err(EngineError::ErasureIncomplete {
            stage: "wal_checkpoint".to_string(),
            detail: format!(
                "`{verb}` deleted its rows, but `wal_checkpoint(TRUNCATE)` reported BUSY on all \
                 {ERASURE_WAL_TRUNCATE_ATTEMPTS} attempts ({frames} frames still in the log) — a \
                 concurrent reader is pinning a WAL snapshot, so the erased bytes remain readable \
                 in the `-wal` file. Retry once the reader has finished."
            ),
        })
    }

    /// 0.8.20 Slice 5 fix-1 (codex §9 P2) — perform every telemetry redaction the
    /// engine still OWES, from the durable pending queue.
    ///
    /// **The defect this closes.** Redaction necessarily runs after the erasing
    /// transaction commits (the sink is a file, not a table, so it cannot join
    /// the transaction). When it failed, the verb correctly raised
    /// `ErasureIncomplete { stage: "telemetry_redaction" }` and told the operator
    /// to retry — but the retry recomputed the id set by querying the canonical
    /// tables, whose rows the FIRST call had already deleted. It therefore got an
    /// EMPTY set, hit the empty-id fast path in
    /// [`Engine::redact_telemetry_stable_ids`], and returned success while the
    /// leaked `l:`/`h:` ids were still sitting in the sink. An erasure verb
    /// reporting success on an incomplete erasure is precisely what R-20-E5
    /// forbids, and it is the worst failure mode available to this slice: silent,
    /// and indistinguishable from a real erasure.
    ///
    /// **The mechanism — an intent log.** The ids are captured BEFORE the deletes
    /// (they are derived from `logical_id`/`body`, which the deletes destroy) and
    /// written into [`ERASURE_PENDING_REDACTION_COLLECTION`] INSIDE the same
    /// transaction, so "the rows are gone" and "a redaction is owed for them"
    /// commit atomically. There is no window in which the rows are deleted and
    /// the obligation is unrecorded. A pending row is deleted only once its
    /// redaction has actually been performed, so the obligation survives process
    /// death, and the empty-id fast path is unreachable while one is outstanding:
    /// this drains the QUEUE, never the caller's id vector.
    ///
    /// The queue is drained by EVERY erasure verb, not just a retry of the one
    /// that failed — an outstanding obligation is the engine's, not one call's.
    ///
    /// **Honest refusal.** If a redaction is owed but no telemetry sink is
    /// attached to this `Engine` (only reachable if the process restarted between
    /// the failure and the retry without re-enabling telemetry), the ids really
    /// are still in the sink file and this returns `ErasureIncomplete` rather
    /// than guessing. Re-enable telemetry on the same sink and retry.
    ///
    /// **Exposure tradeoff, stated plainly.** A pending row holds the stable ids
    /// in the database for the window between the delete and the redaction. That
    /// is a strict improvement: those ids are, during exactly that window,
    /// already readable in the telemetry sink — which is the leak being closed —
    /// and the pending row is deleted the moment the sink is clean, on pages
    /// `secure_delete=ON` zeroes and the subsequent `TRUNCATE` checkpoint clears
    /// from the log.
    fn discharge_pending_redactions(&self, verb: &'static str) -> Result<(), EngineError> {
        let pending = self.load_pending_redactions()?;
        if pending.is_empty() {
            return Ok(());
        }

        let mut ids: Vec<String> =
            pending.iter().flat_map(|(_, ids)| ids.iter().cloned()).collect();
        ids.sort_unstable();
        ids.dedup();

        // A queue entry exists ⇒ a sink was attached when the rows were deleted ⇒
        // the ids are in that file. Never clear the queue without redacting.
        if !self.telemetry_enabled.load(Ordering::Acquire) {
            return Err(EngineError::ErasureIncomplete {
                stage: "telemetry_redaction".to_string(),
                detail: format!(
                    "`{verb}` has {} outstanding telemetry redaction(s) covering {} erased \
                     stable id(s), but no telemetry sink is attached to this engine — the ids \
                     cannot be removed from the sink file. Re-enable telemetry on the same sink \
                     path and retry.",
                    pending.len(),
                    ids.len()
                ),
            });
        }

        // On failure the queue rows stay put and the error propagates: the verb
        // does not report success, and the next call retries the same obligation.
        self.redact_telemetry_stable_ids(verb, &ids)?;

        let row_ids: Vec<i64> = pending.iter().map(|(row_id, _)| *row_id).collect();
        self.clear_pending_redactions(&row_ids)
    }

    /// Read the outstanding redaction queue: `(operational_mutations.id, ids)`.
    fn load_pending_redactions(&self) -> Result<Vec<(i64, Vec<String>)>, EngineError> {
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut stmt = connection
            .prepare(
                "SELECT id, payload_json FROM operational_mutations \
                 WHERE collection_name = ?1 ORDER BY id",
            )
            .map_err(|_| EngineError::Storage)?;
        let rows = stmt
            .query_map([ERASURE_PENDING_REDACTION_COLLECTION], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|_| EngineError::Storage)?;
        let mut pending = Vec::new();
        for row in rows {
            let (row_id, payload) = row.map_err(|_| EngineError::Storage)?;
            // A payload we cannot parse is an obligation we cannot discharge;
            // keeping it (empty) is safe — it never unblocks a false success,
            // and `discharge_pending_redactions` still refuses.
            let ids = serde_json::from_str::<serde_json::Value>(&payload)
                .ok()
                .and_then(|v| v.get("erased_stable_ids").cloned())
                .and_then(|v| serde_json::from_value::<Vec<String>>(v).ok())
                .unwrap_or_default();
            pending.push((row_id, ids));
        }
        Ok(pending)
    }

    /// Retire queue entries whose redaction has been PERFORMED. Committed before
    /// the caller's WAL truncation so the freed pages are checkpointed out.
    fn clear_pending_redactions(&self, row_ids: &[i64]) -> Result<(), EngineError> {
        if row_ids.is_empty() {
            return Ok(());
        }
        let connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_ref().ok_or(EngineError::Closing)?;
        let mut stmt = connection
            .prepare("DELETE FROM operational_mutations WHERE id = ?1")
            .map_err(|_| EngineError::Storage)?;
        for row_id in row_ids {
            stmt.execute([row_id]).map_err(|_| EngineError::Storage)?;
        }
        Ok(())
    }

    /// 0.8.20 Slice 5b (R-20-E6) — SELECTIVE redaction of erased stable ids from
    /// the opt-in telemetry sink.
    ///
    /// `capture_telemetry` persists `result_stable_ids` — `l:`/`h:` prefixed ids
    /// — into a JSONL file that outlives the erased rows, and nothing in the
    /// engine could previously remove them. A retained `l:` id is not inert:
    /// [`derive_logical_id`] is `SHA256(lowercase(kind) + ":" + lowercase(name))`,
    /// and the case-folding of BOTH inputs shrinks the preimage space, so a
    /// surviving id is dictionary-attackable back to the natural key it was
    /// derived from. An `h:` id is a plain `SHA256(body)`, confirmable against a
    /// guessed body.
    ///
    /// **This MUST NOT truncate the sink.** `sink_path` is CALLER-SUPPLIED and
    /// may hold unrelated operator eval history that the erasure obligation never
    /// covered; the v3 truncation approach was rejected as unsafe. Only the
    /// matching `result_stable_ids` ELEMENTS are replaced with
    /// [`REDACTED_STABLE_ID`], preserving record count, record order and
    /// positional alignment with the parallel `result_ids` array. Lines that are
    /// not engine-authored JSON events are copied through verbatim.
    ///
    /// **Crash safety.** The rewrite is write-temp-then-`rename`: a sibling
    /// `.redact.tmp` is written and fsynced, then atomically renamed over the
    /// sink, so a crash leaves either the old file or the new one — never a
    /// half-rewritten sink. The telemetry mutex is held across the whole rewrite,
    /// so no in-process `capture_telemetry` can append into the window; an
    /// out-of-process appender is handled by re-reading and folding in the tail
    /// delta before the rename (bounded retry).
    ///
    /// The privacy contract is unchanged: query TEXT and `source_id` are never
    /// captured (ADR-0.8.8 §C), so there is nothing else in the sink to redact.
    /// The fast-OFF atomic guard is preserved — when telemetry was never enabled
    /// this is a single relaxed-ordering load and no mutex acquisition.
    fn redact_telemetry_stable_ids(
        &self,
        verb: &'static str,
        erased_stable_ids: &[String],
    ) -> Result<(), EngineError> {
        // Fast OFF path — mirrors `capture_telemetry`. No mutex, no I/O.
        if erased_stable_ids.is_empty() || !self.telemetry_enabled.load(Ordering::Acquire) {
            return Ok(());
        }
        let guard = self.telemetry.lock().map_err(|_| EngineError::Storage)?;
        let Some(sink) = guard.as_ref() else { return Ok(()) };
        let erased: std::collections::HashSet<&str> =
            erased_stable_ids.iter().map(String::as_str).collect();

        match redact_jsonl_stable_ids(sink.path(), &erased) {
            Ok(()) => Ok(()),
            // 0.8.20 Slice 5 fix-3 (codex §9 round-3 P2) — `NotFound` is NOT a
            // discharge. It previously returned `Ok(())` ("the sink is gone,
            // nothing to redact"), which cleared the durable pending queue and
            // let the verb report success. That inference does not hold: a path
            // cannot distinguish `rm` from `mv`, and log rotation of a
            // caller-supplied sink is an ordinary operational event that leaves
            // the erased `l:`/`h:` ids fully readable under the rotated name.
            //
            // The burden of proof is on DISCHARGING the obligation, and the
            // engine cannot meet it here: `TelemetrySink` holds a PATH, not an
            // open handle, so there is no `nlink == 0` witness that the inode was
            // actually unlinked — and even that would not cover a copy taken
            // before the deletion. So there is no narrow provable case to carve
            // out, and `NotFound` fails closed.
            //
            // This cannot fire spuriously for a sink that never existed:
            // `enable_telemetry` CREATES the file before arming capture, so for
            // any engine with telemetry enabled the sink demonstrably existed and
            // `NotFound` means it existed and then vanished.
            Err(err) => Err(EngineError::ErasureIncomplete {
                stage: "telemetry_redaction".to_string(),
                detail: if err.kind() == std::io::ErrorKind::NotFound {
                    format!(
                        "`{verb}` deleted its rows, but the telemetry sink {} no longer exists, \
                         so the erased stable ids could not be redacted from it. A missing path \
                         does NOT prove the sink was deleted — if it was rotated or moved aside, \
                         the erased ids are still readable under its new name. The pending \
                         redaction is durable: restore the sink at this path and retry (if the \
                         sink really was destroyed, an empty file at this path discharges the \
                         obligation).",
                        sink.path().display()
                    )
                } else {
                    format!(
                        "`{verb}` deleted its rows, but the erased stable ids could not be \
                         redacted from the telemetry sink {}: {err}",
                        sink.path().display()
                    )
                },
            }),
        }
    }

    /// The erased rows' prefixed stable ids ([`IdSpace::to_prefixed`]) are NOT
    /// returned to the caller for redaction (R-20-E6). They are enqueued INSIDE
    /// this transaction via [`enqueue_pending_redaction`]: a caller-held vector
    /// is lost on the retry path, which is exactly the false-success codex §9 P2
    /// found. Only the report comes back.
    ///
    /// 0.8.20 Slice 5d (R-20-E4): no longer `operator`-gated — it is the shared
    /// body behind BOTH `erase_source` (governed SDK) and `excise_source`
    /// (operator seam). Still private; the gate that matters is on the two
    /// public spellings.
    fn excise_source_inner(
        &self,
        verb: &'static str,
        source_id: &str,
    ) -> Result<(ExciseReport, Vec<ClosureOperationId>), EngineError> {
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;

        // Collect the cursor sets up-front so we can targeted-delete
        // shadow rows AND emit an accurate audit row in one txn.
        let node_cursors: Vec<i64> = {
            let mut stmt = tx
                .prepare("SELECT write_cursor FROM canonical_nodes WHERE source_id = ?1")
                .map_err(|_| EngineError::Storage)?;
            let rows = stmt
                .query_map([source_id], |row| row.get::<_, i64>(0))
                .map_err(|_| EngineError::Storage)?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)?
        };
        let edge_cursors: Vec<i64> = {
            let mut stmt = tx
                .prepare("SELECT write_cursor FROM canonical_edges WHERE source_id = ?1")
                .map_err(|_| EngineError::Storage)?;
            let rows = stmt
                .query_map([source_id], |row| row.get::<_, i64>(0))
                .map_err(|_| EngineError::Storage)?;
            rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|_| EngineError::Storage)?
        };

        let source_revisions = node_cursors
            .iter()
            .map(|cursor| dependency_closure::source_revision_for_cursor(&tx, *cursor))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let physical_plans =
            dependency_closure::physical_dependents_for_sources(&tx, &source_revisions)?;
        let affected_dependencies = physical_plans.iter().map(|(_, rows)| rows.len()).sum();
        let proof_boundary = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);

        // 0.8.20 Slice 5b (R-20-E6) — stable ids the telemetry sink may hold for
        // these rows, collected BEFORE the DELETEs.
        let erased_stable_ids = collect_erased_stable_ids(
            &tx,
            "SELECT logical_id, body FROM canonical_nodes WHERE source_id = ?1",
            "SELECT logical_id, body FROM canonical_edges WHERE source_id = ?1",
            source_id,
        )?;

        let affected_cursors: Vec<i64> =
            node_cursors.iter().chain(edge_cursors.iter()).copied().collect();
        let receipt_refs =
            actuation_erasure_refs_for_cursors(&tx, &affected_cursors, [source_id.to_string()])?;
        let proof_scope = dependency_closure::physical_proof_scope(
            &tx,
            &affected_cursors,
            &receipt_refs,
            Some(source_id),
        )?;
        let closure_ids = dependency_closure::record_physical_closure(
            &tx,
            dependency_closure::PhysicalClosureAdmission {
                root_kind: "source_bucket",
                root_value: source_id,
                retry_verb: verb,
                retry_argument: source_id,
                cause: ClosureCauseV1::SourceErased,
                boundary: proof_boundary,
                affected_count: affected_dependencies,
            },
        )?
        .into_iter()
        .collect::<Vec<_>>();
        actuation::redact_actuation_receipts_for_refs(&tx, &receipt_refs)?;
        // A correction receipt can reference soft closures for the source
        // revision being erased. Receipt validation therefore has to run while
        // those closures still exist; deleting them first makes the valid
        // receipt appear corrupt and rolls the erasure back. Both steps remain
        // in this transaction, so any validation failure is atomic.
        for source_revision in &source_revisions {
            tx.execute(
                "DELETE FROM _fathomdb_dependency_closures \
                 WHERE root_kind='source_revision' AND root_value=?1 \
                   AND cause IN ('superseded','soft_deleted')",
                [source_revision],
            )
            .map_err(|_| EngineError::Storage)?;
        }
        erase_artifact_identity_for_cursors(&tx, &affected_cursors, false, Some(source_id))?;

        // 0.8.20 Slice 5a (R-20-E1) — registry-driven erasure. The previous
        // hand-rolled list here OMITTED `search_index_v2`, a CONTENT-STORING
        // FTS5 table (no `content=''`) that keeps the document body verbatim:
        // after `excise_source` the erased body was still on disk, invisible to
        // every functional test because both v2 read paths discard candidates
        // lacking a live `canonical_nodes` row. `erase_row_projections` covers
        // every registered projection, so the omission cannot recur.
        let mut shadow_invalidated: u64 = 0;
        for cursor in node_cursors.iter().chain(edge_cursors.iter()) {
            shadow_invalidated = shadow_invalidated.saturating_add(
                erase_row_projections(&tx, *cursor).map_err(|_| EngineError::Storage)?,
            );
        }

        let nodes_excised = tx
            .execute("DELETE FROM canonical_nodes WHERE source_id = ?1", [source_id])
            .map_err(|_| EngineError::Storage)? as u64;
        let edges_excised = tx
            .execute("DELETE FROM canonical_edges WHERE source_id = ?1", [source_id])
            .map_err(|_| EngineError::Storage)? as u64;

        // AC-028a audit row: a single append on the
        // `excise_source_audit` collection naming the excised source.
        //
        // DURABILITY (0.8.20 Slice 5b, design v5 §2 defect D-A; HITL-ruled
        // 2026-07-19: *"there must be an auditable record of deletion event."*).
        // This row lands in `operational_mutations`, the same table the retention
        // sweep drains — and it is written BEFORE the workload that follows it,
        // so an oldest-`id`-first sweep evicted it FIRST. It is now protected:
        // `excise_source_audit` is in `ERASURE_AUDIT_COLLECTIONS`, which
        // `enforce_provenance_retention` excludes. The proof of erasure is no
        // longer destructible by ordinary retention pressure.
        //
        // NON-PII `source_id` (rationale corrected in this slice). v4 §3.6
        // justified the "`source_id` must not be PII" rule by claiming the audit
        // row retains it *permanently, by design*. That premise was FALSE — the
        // row was sweepable. The rule stands on a different and simpler footing:
        // this row persists the caller's raw `source_id` verbatim, and an
        // `excise_source` that erased the payload while keeping an identifying
        // source label would not be an erasure. The exemption above makes the
        // retention now genuinely indefinite, which makes the rule MORE
        // load-bearing, not less.
        //
        // `next_cursor` after a prior write holds the LAST committed cursor;
        // mirror the vec writer pattern (load + 1, then store post-commit)
        // so the audit row's `write_cursor` is strictly greater than every
        // canonical row that preceded it.
        let excised_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let payload = serde_json::json!({
            "source_id": source_id,
            "excised_at": excised_at,
            "nodes_excised": nodes_excised,
            "edges_excised": edges_excised,
            "projections_invalidated": shadow_invalidated,
        })
        .to_string();
        let audit_cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        tx.execute(
            "INSERT INTO operational_mutations(
                collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
             ) VALUES('excise_source_audit', ?1, 'append', ?2, NULL, ?3)",
            params![source_id, payload, audit_cursor],
        )
        .map_err(|_| EngineError::Storage)?;

        // 0.8.20 Slice 5 fix-1 (codex §9 P2) — durably record the redaction this
        // erasure owes, atomically with the deletes. Shares `audit_cursor`: one
        // erasure event, and this row is retired as soon as the sink is clean.
        if self.telemetry_enabled.load(Ordering::Acquire) {
            enqueue_pending_redaction(&tx, verb, &erased_stable_ids, audit_cursor)?;
        }

        dependency_closure::measure_physical_closures(&tx, &closure_ids, &proof_scope)?;

        tx.commit().map_err(|_| EngineError::Storage)?;
        self.next_cursor.store(audit_cursor, Ordering::SeqCst);
        Ok((
            ExciseReport {
                source_ref: source_id.to_string(),
                nodes_excised,
                edges_excised,
                projections_invalidated: shadow_invalidated,
            },
            closure_ids,
        ))
    }

    /// 0.8.20 Slice 5b (R-20-E7) — erase ONE op-store record, by collection and
    /// record key, from both op-store shapes: every `operational_mutations`
    /// version of the key (append-only-log collections) and its
    /// `operational_state` row (latest-state collections).
    ///
    /// **Refuses the engine's own erasure bookkeeping** (0.8.20 Slice 5 fix-3,
    /// codex §9 round-3 P1): a `collection` for which
    /// [`is_erasure_bookkeeping_collection`] holds raises
    /// [`EngineError::InvalidArgument`] before anything is deleted. Aimed at the
    /// pending-redaction queue this verb otherwise destroys an outstanding
    /// erasure obligation, after which the next erasure verb reports success with
    /// the erased ids still in the telemetry sink; aimed at the audit trail it
    /// destroys the auditable record of the deletion event.
    ///
    /// Before this slice the op-store had NO record-level delete at all:
    /// [`enforce_provenance_retention`] is a cap sweep, not an erasure verb, so a
    /// caller holding an erasure obligation over an op-store record had no way to
    /// discharge it. Idempotent — erasing an absent key is a zero-count success.
    ///
    /// Like the other erasure verbs this finishes at rest (telemetry is not
    /// involved — op-store record keys never reach the telemetry sink — but the
    /// `-wal` is), so it can return [`EngineError::ErasureIncomplete`].
    ///
    /// AUDIT (D-A). Appends a row to the retention-exempt `excise_record_audit`
    /// collection. Unlike `source_id`, a `record_key` carries NO non-PII rule:
    /// it is arbitrary caller-supplied text and may itself be the identifier
    /// being erased. The audit therefore records a SHA-256 digest of
    /// `collection` + `record_key`, never the key — enough to prove *that* a
    /// specific record was erased to anyone who already knows the key, and
    /// useless to anyone who does not.
    #[cfg(feature = "operator")]
    pub fn excise_collection_record(
        &self,
        collection: &str,
        record_key: &str,
    ) -> Result<ExciseRecordReport, EngineError> {
        self.ensure_open()?;
        if collection.is_empty() || record_key.is_empty() {
            return Err(EngineError::WriteValidation);
        }
        // 0.8.20 Slice 5 fix-3 (codex §9 round-3 P1) — the engine's own erasure
        // bookkeeping is not caller data and is not excisable. See
        // `is_erasure_bookkeeping_collection` for why each member is protected.
        // Checked BEFORE any deletion so the refusal is total, not partial.
        if is_erasure_bookkeeping_collection(collection) {
            return Err(EngineError::InvalidArgument {
                msg: format!(
                    "`{collection}` is engine-internal erasure bookkeeping and cannot be excised \
                     by `excise_collection_record`. The pending-redaction queue records an \
                     erasure the engine still owes (deleting it would let a later verb report \
                     success on an incomplete erasure, R-20-E5), and the erasure-audit \
                     collections are the auditable record of the deletion event. Pending \
                     redactions retire themselves once performed; retry the erasure verb instead."
                ),
            });
        }
        let report = self.excise_collection_record_inner(collection, record_key)?;
        self.complete_erasure_at_rest("excise_collection_record")?;
        Ok(report)
    }

    #[cfg(feature = "operator")]
    fn excise_collection_record_inner(
        &self,
        collection: &str,
        record_key: &str,
    ) -> Result<ExciseRecordReport, EngineError> {
        let mut connection = self.connection.lock().map_err(|_| EngineError::Storage)?;
        let connection = connection.as_mut().ok_or(EngineError::Closing)?;
        dependency_closure::maintain_before_writer(connection)?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(|_| EngineError::Storage)?;
        dependency_closure::guard_no_pending_physical(&tx)?;

        let records_excised = tx
            .execute(
                "DELETE FROM operational_mutations
                 WHERE collection_name = ?1 AND record_key = ?2",
                params![collection, record_key],
            )
            .map_err(|_| EngineError::Storage)? as u64;
        let state_rows_excised = tx
            .execute(
                "DELETE FROM operational_state
                 WHERE collection_name = ?1 AND record_key = ?2",
                params![collection, record_key],
            )
            .map_err(|_| EngineError::Storage)? as u64;

        let record_digest = digest_record_identity(collection, record_key);
        let excised_at = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        let payload = serde_json::json!({
            "collection": collection,
            "record_digest": record_digest,
            "excised_at": excised_at,
            "records_excised": records_excised,
            "state_rows_excised": state_rows_excised,
        })
        .to_string();
        let audit_cursor = self.next_cursor.load(Ordering::SeqCst).saturating_add(1);
        tx.execute(
            "INSERT INTO operational_mutations(
                collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
             ) VALUES('excise_record_audit', ?1, 'append', ?2, NULL, ?3)",
            params![record_digest, payload, audit_cursor],
        )
        .map_err(|_| EngineError::Storage)?;

        tx.commit().map_err(|_| EngineError::Storage)?;
        self.next_cursor.store(audit_cursor, Ordering::SeqCst);
        self.counters.record_admin();
        Ok(ExciseRecordReport {
            collection: collection.to_string(),
            record_digest,
            records_excised,
            state_rows_excised,
        })
    }
}
