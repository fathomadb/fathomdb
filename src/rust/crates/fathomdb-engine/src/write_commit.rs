use super::*;
use crate::erasure::{ERASURE_AUDIT_COLLECTIONS, ERASURE_PENDING_REDACTION_COLLECTION};
use crate::identity::hex_encode;

pub(crate) fn load_next_cursor(connection: &Connection) -> u64 {
    let nodes = max_cursor(connection, "canonical_nodes").unwrap_or(0);
    let edges = max_cursor(connection, "canonical_edges").unwrap_or(0);
    let mutations = max_cursor(connection, "operational_mutations").unwrap_or(0);
    let state = max_cursor(connection, "operational_state").unwrap_or(0);
    let commit_timestamps = max_cursor(connection, "_fathomdb_commit_timestamps").unwrap_or(0);
    let projection_terminal = max_cursor(connection, "_fathomdb_projection_terminal").unwrap_or(0);
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
    nodes
        .max(edges)
        .max(mutations)
        .max(state)
        .max(commit_timestamps)
        .max(projection_terminal)
        .max(reserved)
        .max(closure_boundary)
        .max(physical_erasure::cursor_floor(connection).unwrap_or(0))
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CommitEndpointValidationMode {
    /// Enforce active, non-superseded endpoints for every edge in `batch`.
    Strict,
    /// Opt-in bulk mode: admit edges whose same-scope endpoints are missing,
    /// report the exact unresolved endpoint count, and abort atomically with
    /// [`EngineError::ScopeViolation`] if any missing endpoint resolves to an
    /// active node in another scope.
    BulkNonStrict,
    /// Single-operation actuation loop: `validate_prospective_edge_endpoints`
    /// runs post-loop in `simulate_request` so same-batch `[PutDerivedEdge, PutNode]`
    /// succeeds and dangling actuation edges return `ActuationOutcomeV1::Refused`.
    DeferredToActuationSimulation,
}

impl From<crate::EndpointValidationMode> for CommitEndpointValidationMode {
    fn from(mode: crate::EndpointValidationMode) -> Self {
        match mode {
            crate::EndpointValidationMode::Strict => Self::Strict,
            crate::EndpointValidationMode::BulkNonStrict => Self::BulkNonStrict,
        }
    }
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

fn runtime_revision_id(write: &PreparedWrite, scope: &str, cursor: u64) -> String {
    let write = storage_write_shape(write);
    let mut hasher = Sha256::new();
    revision_hash_field(&mut hasher, b"fathomdb:artifact-revision:runtime:v1");
    if !scope.is_empty() {
        revision_hash_field(&mut hasher, b"scope");
        revision_hash_field(&mut hasher, scope.as_bytes());
    }
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

#[cfg(any(test, feature = "test-hooks"))]
thread_local! {
    pub(crate) static CANONICAL_BODY_HASH_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(any(test, feature = "test-hooks"))]
static CANONICAL_BODY_HASH_CALLS_GLOBAL: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

#[cfg(any(test, feature = "test-hooks"))]
#[doc(hidden)]
pub fn canonical_body_hash_calls_for_test() -> usize {
    CANONICAL_BODY_HASH_CALLS_GLOBAL.load(std::sync::atomic::Ordering::SeqCst)
}

pub(crate) fn canonical_body_hash(body: &str) -> String {
    #[cfg(any(test, feature = "test-hooks"))]
    {
        CANONICAL_BODY_HASH_CALLS.with(|calls| calls.set(calls.get() + 1));
        CANONICAL_BODY_HASH_CALLS_GLOBAL.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
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

#[derive(Clone, Copy, Debug)]
pub(crate) struct PrecheckedWriteGuards {
    pub(crate) scoped_physical: bool,
    pub(crate) queued_erasure: bool,
}

fn prefetch_active_node_cursors_by_logical_id(
    tx: &Connection,
    scope: &str,
    batch: &[PreparedWrite],
) -> rusqlite::Result<std::collections::HashMap<String, Vec<(i64, String)>>> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut distinct: Vec<&str> = Vec::new();
    for write in batch {
        let lid = match write {
            PreparedWrite::Node { logical_id: Some(id), .. } => Some(id.as_str()),
            PreparedWrite::ProvenancedNode(node) => node.logical_id.as_deref(),
            _ => None,
        };
        if let Some(id) = lid {
            if seen.insert(id) {
                distinct.push(id);
            }
        }
    }
    let mut map: std::collections::HashMap<String, Vec<(i64, String)>> =
        std::collections::HashMap::new();
    if distinct.is_empty() {
        return Ok(map);
    }
    const CHUNK_SIZE: usize = 64;
    for chunk in distinct.chunks(CHUNK_SIZE) {
        let mut stmt = if chunk.len() == CHUNK_SIZE {
            tx.prepare_cached(
                "SELECT logical_id, write_cursor, kind FROM canonical_nodes \
                 WHERE scope = ?1 AND logical_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 ) AND superseded_at IS NULL",
            )?
        } else {
            let mut sql = String::from(
                "SELECT logical_id, write_cursor, kind FROM canonical_nodes WHERE scope = ?1 AND logical_id IN (",
            );
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push_str(") AND superseded_at IS NULL");
            tx.prepare_cached(&sql)?
        };
        let rows = stmt.query_map(
            rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)),
        )?;
        for row in rows {
            let (lid, cursor, kind) = row?;
            map.entry(lid).or_default().push((cursor, kind));
        }
    }
    Ok(map)
}

fn prefetch_active_edge_cursors_by_logical_id(
    tx: &Connection,
    scope: &str,
    batch: &[PreparedWrite],
) -> rusqlite::Result<std::collections::HashMap<String, Vec<(i64, String)>>> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut distinct: Vec<&str> = Vec::new();
    for write in batch {
        let lid = match write {
            PreparedWrite::Edge { logical_id: Some(id), .. } => Some(id.as_str()),
            PreparedWrite::ProvenancedEdge(edge) => edge.logical_id.as_deref(),
            _ => None,
        };
        if let Some(id) = lid {
            if seen.insert(id) {
                distinct.push(id);
            }
        }
    }
    let mut map: std::collections::HashMap<String, Vec<(i64, String)>> =
        std::collections::HashMap::new();
    if distinct.is_empty() {
        return Ok(map);
    }
    const CHUNK_SIZE: usize = 64;
    for chunk in distinct.chunks(CHUNK_SIZE) {
        let mut stmt = if chunk.len() == CHUNK_SIZE {
            tx.prepare_cached(
                "SELECT logical_id, write_cursor, kind FROM canonical_edges \
                 WHERE scope = ?1 AND logical_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 ) AND superseded_at IS NULL",
            )?
        } else {
            let mut sql = String::from(
                "SELECT logical_id, write_cursor, kind FROM canonical_edges WHERE scope = ?1 AND logical_id IN (",
            );
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push_str(") AND superseded_at IS NULL");
            tx.prepare_cached(&sql)?
        };
        let rows = stmt.query_map(
            rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?)),
        )?;
        for row in rows {
            let (lid, cursor, kind) = row?;
            map.entry(lid).or_default().push((cursor, kind));
        }
    }
    Ok(map)
}

fn prefetch_registered_revisions(
    tx: &Connection,
    scope: &str,
    revision_ids: &[&str],
) -> rusqlite::Result<std::collections::HashSet<String>> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut distinct: Vec<&str> = Vec::new();
    for &rev in revision_ids {
        if seen.insert(rev) {
            distinct.push(rev);
        }
    }
    let mut registered: std::collections::HashSet<String> = std::collections::HashSet::new();
    if distinct.is_empty() {
        return Ok(registered);
    }
    const CHUNK_SIZE: usize = 64;
    for chunk in distinct.chunks(CHUNK_SIZE) {
        let any_exists: bool = if chunk.len() == CHUNK_SIZE {
            tx.prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions \
                 WHERE scope = ?1 AND revision_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 ))",
            )?
            .query_row(
                rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
                |row| row.get(0),
            )?
        } else {
            let mut sql = String::from(
                "SELECT EXISTS(SELECT 1 FROM _fathomdb_artifact_revisions WHERE scope = ?1 AND revision_id IN (",
            );
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push_str("))");
            tx.prepare_cached(&sql)?.query_row(
                rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
                |row| row.get(0),
            )?
        };
        if !any_exists {
            continue;
        }
        let mut stmt = if chunk.len() == CHUNK_SIZE {
            tx.prepare_cached(
                "SELECT revision_id FROM _fathomdb_artifact_revisions \
                 WHERE scope = ?1 AND revision_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 )",
            )?
        } else {
            let mut sql = String::from(
                "SELECT revision_id FROM _fathomdb_artifact_revisions WHERE scope = ?1 AND revision_id IN (",
            );
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push(')');
            tx.prepare_cached(&sql)?
        };
        let rows = stmt.query_map(
            rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
            |row| row.get::<_, String>(0),
        )?;
        for row in rows {
            registered.insert(row?);
        }
    }
    Ok(registered)
}

fn register_artifact_identity(
    tx: &Connection,
    scope: &str,
    original: &PreparedWrite,
    cursor: u64,
    precomputed_runtime_rev: Option<&str>,
    registered_revisions: &mut std::collections::HashSet<String>,
) -> Result<(), CommitBatchError> {
    let (artifact_class, provenance) = match original {
        PreparedWrite::ProvenancedNode(node) => ("node", Some(&node.provenance)),
        PreparedWrite::ProvenancedEdge(edge) => ("edge", Some(&edge.provenance)),
        PreparedWrite::Node { .. } => ("node", None),
        PreparedWrite::Edge { .. } => ("edge", None),
        _ => return Ok(()),
    };

    let Some(provenance) = provenance else {
        let computed;
        let revision_id = match precomputed_runtime_rev {
            Some(rev) => rev,
            None => {
                computed = runtime_revision_id(original, scope, cursor);
                computed.as_str()
            }
        };
        if registered_revisions.contains(revision_id) {
            return Err(ProvenanceError::new(ProvenanceErrorReason::RevisionIdConflict, "").into());
        }
        tx.prepare_cached(
            "INSERT INTO _fathomdb_artifact_revisions(\
               schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness, scope\
             ) VALUES(1, ?1, ?2, ?3, 'legacy', ?4, ?5)",
        )?
        .execute(params![
            revision_id,
            artifact_class,
            cursor,
            ProvenanceCompleteness::MigratedIncomplete.as_str(),
            scope,
        ])?;
        registered_revisions.insert(revision_id.to_string());
        return Ok(());
    };

    let revision_id = provenance.artifact_revision_id.as_str();
    if registered_revisions.contains(revision_id) {
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
                .prepare_cached(
                    "SELECT source_revision_id FROM _fathomdb_source_versions \
                     WHERE scope = ?1 AND source_id = ?2 AND source_version_id = ?3",
                )?
                .query_row(
                    params![scope, node.source_id.as_str(), provenance.source_version_id.as_str()],
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
            tx.prepare_cached(
                "INSERT INTO _fathomdb_artifact_revisions(\
                   schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness, scope\
                 ) VALUES(1, ?1, 'node', ?2, 'canonical_source', ?3, ?4)",
            )?
            .execute(params![
                revision_id,
                cursor,
                ProvenanceCompleteness::Complete.as_str(),
                scope
            ])?;
            tx.prepare_cached(
                "INSERT INTO _fathomdb_source_versions(\
                   schema_version, source_id, source_version_id, source_revision_id, scope\
                 ) VALUES(1, ?1, ?2, ?3, ?4)",
            )?
            .execute(params![
                node.source_id.as_str(),
                provenance.source_version_id.as_str(),
                revision_id,
                scope,
            ])?;
            tx.prepare_cached(
                "INSERT INTO _fathomdb_source_links(\
                   schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
                   locator_kind, start_byte, end_byte, hash_algorithm, hash_digest, scope\
                 ) VALUES(1, ?1, ?2, ?3, ?1, 'whole_body', NULL, NULL, 'sha256', ?4, ?5)",
            )?
            .execute(params![
                revision_id,
                node.source_id.as_str(),
                provenance.source_version_id.as_str(),
                hash,
                scope,
            ])?;
        }
        ProvenanceRole::Derived => {
            let source_revision_id = provenance
                .source_revision_id
                .as_ref()
                .expect("derived constructor always sets source revision");
            let source: Option<(String, String, String)> = tx
                .prepare_cached(
                    "SELECT sv.source_id, sv.source_version_id, n.body \
                     FROM _fathomdb_artifact_revisions ar \
                     JOIN _fathomdb_source_versions sv \
                       ON sv.scope = ar.scope AND sv.source_revision_id = ar.revision_id \
                     JOIN canonical_nodes n \
                       ON n.scope = ar.scope AND n.write_cursor = ar.write_cursor \
                     WHERE ar.scope = ?1 AND ar.revision_id = ?2 AND ar.artifact_class = 'node' \
                       AND ar.artifact_role = 'canonical_source'",
                )?
                .query_row(params![scope, source_revision_id.as_str()], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
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
            if dependency_closure::active_barrier_for_source(tx, scope, source_revision_id.as_str())
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
                scope,
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
            tx.prepare_cached(
                "INSERT INTO _fathomdb_artifact_revisions(\
                   schema_version, revision_id, artifact_class, write_cursor, artifact_role, completeness, scope\
                 ) VALUES(1, ?1, ?2, ?3, 'derived_semantic', ?4, ?5)",
            )?
            .execute(params![
                revision_id,
                artifact_class,
                cursor,
                ProvenanceCompleteness::Complete.as_str(),
                scope,
            ])?;
            tx.prepare_cached(
                "INSERT INTO _fathomdb_source_links(\
                   schema_version, artifact_revision_id, source_id, source_version_id, source_revision_id,\
                   locator_kind, start_byte, end_byte, hash_algorithm, hash_digest, scope\
                 ) VALUES(1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, 'sha256', ?8, ?9)",
            )?
            .execute(params![
                revision_id,
                stored_source_id,
                stored_version_id,
                source_revision_id.as_str(),
                locator_kind,
                start_byte,
                end_byte,
                supplied_hash.digest_hex(),
                scope,
            ])?;
        }
    }
    registered_revisions.insert(revision_id.to_string());
    Ok(())
}

/// Own visibility coalescing inside an already-owned writer transaction.
/// Transaction rollback is the only destructor cleanup: SQL here must never
/// reopen a write after SQLite has automatically rolled the transaction back.
pub(crate) struct VisibilityCoalescingGuard<'transaction, 'connection> {
    transaction: &'transaction rusqlite::Transaction<'connection>,
    restored: bool,
}

impl<'transaction, 'connection> VisibilityCoalescingGuard<'transaction, 'connection> {
    pub(crate) fn arm(
        transaction: &'transaction rusqlite::Transaction<'connection>,
    ) -> Result<Self, CommitBatchError> {
        if !transaction
            .db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER)
            .map_err(|_| CommitBatchError::WriterPoisoned)?
        {
            return Err(CommitBatchError::WriterPoisoned);
        }
        crate::frozen_read::require_clean_visibility(transaction)?;
        if transaction.is_autocommit() {
            return Err(rusqlite::Error::InvalidQuery.into());
        }
        let changed = transaction
            .prepare_cached(
                "UPDATE _fathomdb_read_visibility_state SET coalescing=1
                 WHERE singleton=1 AND typeof(coalescing)='integer' AND coalescing=0",
            )?
            .execute([])?;
        if changed != 1 {
            return Err(rusqlite::Error::InvalidQuery.into());
        }
        Ok(Self { transaction, restored: false })
    }

    pub(crate) fn restore(&mut self) -> Result<(), CommitBatchError> {
        if self.restored || self.transaction.is_autocommit() {
            self.restored = true;
            return Ok(());
        }
        let flag = self
            .transaction
            .prepare_cached(
                "SELECT coalescing FROM _fathomdb_read_visibility_state WHERE singleton=1",
            )
            .and_then(|mut stmt| stmt.query_row([], |row| row.get::<_, i64>(0)))
            .map_err(|_| CommitBatchError::WriterPoisoned)?;
        if flag == 1 {
            let changed = self
                .transaction
                .prepare_cached(
                    "UPDATE _fathomdb_read_visibility_state SET coalescing=0
                     WHERE singleton=1 AND typeof(coalescing)='integer' AND coalescing=1",
                )
                .and_then(|mut stmt| stmt.execute([]))
                .map_err(|_| CommitBatchError::WriterPoisoned)?;
            if changed != 1 {
                return Err(CommitBatchError::WriterPoisoned);
            }
        } else if flag != 0 {
            return Err(CommitBatchError::WriterPoisoned);
        }
        self.restored = true;
        Ok(())
    }
}

impl Drop for VisibilityCoalescingGuard<'_, '_> {
    fn drop(&mut self) {
        // The owned transaction rolls back on error; cleanup SQL could otherwise
        // autocommit after SQLite has already performed RAISE(ROLLBACK).
    }
}

fn quote_sql_list<'a>(names: impl Iterator<Item = &'a str>) -> String {
    names.map(|name| format!("'{}'", name.replace('\'', "''"))).collect::<Vec<_>>().join(",")
}

static CANONICAL_CUSTOM_TRIGGER_CHECK_SQL: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| {
        let tables = quote_sql_list(
            fathomdb_schema::TABLE_MANIFEST
                .iter()
                .filter(|entry| entry.guard_canonical_custom_triggers)
                .map(|entry| entry.name),
        );
        let builtins =
            quote_sql_list(fathomdb_schema::BUILTIN_NON_VISIBILITY_TRIGGERS.iter().copied());
        format!(
            "SELECT EXISTS(\
             SELECT 1 FROM sqlite_master \
             WHERE type='trigger' \
               AND tbl_name IN ({tables}) \
               AND name NOT LIKE '_fathomdb_read_visibility_%' \
               AND name NOT IN ({builtins}) \
             UNION ALL \
             SELECT 1 FROM sqlite_temp_master \
             WHERE type='trigger' \
               AND tbl_name IN ({tables}) \
               AND name NOT LIKE '_fathomdb_read_visibility_%'\
         )"
        )
    });

pub(crate) fn canonical_batch_has_no_custom_triggers(
    connection: &Connection,
) -> rusqlite::Result<bool> {
    if !crate::vector_spans::cleanup_trigger_is_valid(connection)? {
        return Ok(false);
    }
    let unexpected: bool = connection
        .prepare_cached(CANONICAL_CUSTOM_TRIGGER_CHECK_SQL.as_str())?
        .query_row([], |row| row.get(0))?;
    Ok(!unexpected)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn commit_batch(
    connection: &mut Connection,
    scope: &str,
    batch: &[PreparedWrite],
    plans: &[WritePlan],
    base_cursor: u64,
    provenance_row_cap: u64,
    vector_kinds_to_enrol: &[String],
    endpoint_validation: CommitEndpointValidationMode,
    preloaded_registry: Option<&BTreeMap<(crate::PropertyEntityClass, String), StoredProjection>>,
    prechecked_guards: Option<PrecheckedWriteGuards>,
    #[cfg(feature = "test-hooks")] abort_next_commit_for_test: bool,
) -> Result<(u64, bool, Vec<ClosureOperationId>), CommitBatchError> {
    #[cfg(feature = "test-hooks")]
    if abort_next_commit_for_test {
        connection.commit_hook(Some(|| true))?;
    }
    let result = commit_batch_transaction(
        connection,
        scope,
        batch,
        plans,
        base_cursor,
        provenance_row_cap,
        vector_kinds_to_enrol,
        endpoint_validation,
        preloaded_registry,
        prechecked_guards,
    );
    #[cfg(feature = "test-hooks")]
    if abort_next_commit_for_test {
        connection.commit_hook(None::<fn() -> bool>)?;
    }
    result
}

#[allow(clippy::too_many_arguments)]
fn commit_batch_transaction(
    connection: &mut Connection,
    scope: &str,
    batch: &[PreparedWrite],
    plans: &[WritePlan],
    base_cursor: u64,
    provenance_row_cap: u64,
    vector_kinds_to_enrol: &[String],
    endpoint_validation: CommitEndpointValidationMode,
    preloaded_registry: Option<&BTreeMap<(crate::PropertyEntityClass, String), StoredProjection>>,
    prechecked_guards: Option<PrecheckedWriteGuards>,
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
    let coalesceable_batch =
        batch.iter().all(|write| !matches!(write, PreparedWrite::AdminSchema { .. }));
    let tx = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    crate::frozen_read::require_clean_visibility(&tx)?;
    // Slice 71B / Slice 41 (ADR #103): coalesce visibility triggers across any
    // non-AdminSchema batch (canonical, OpStore AppendOnlyLog/LatestState, and
    // mixed batches), upserting only the affected `(scope, partition_kind)`
    // watermarks once after all mutations succeed, then clearing coalescing
    // before commit. An unexpected main or TEMP trigger falls back to the
    // uncoalesced row-trigger path so application extensions observe every row.
    if coalesceable_batch && canonical_batch_has_no_custom_triggers(&tx)? {
        let mut trigger_guard = VisibilityCoalescingGuard::arm(&tx)?;
        let mut visibility_delta = VisibilityDelta::default();
        let attempted = apply_batch_in_transaction(
            &tx,
            scope,
            batch,
            plans,
            base_cursor,
            provenance_row_cap,
            vector_kinds_to_enrol,
            dependency_closure::SoftClosureMode::Complete,
            endpoint_validation,
            Some(&mut visibility_delta),
            preloaded_registry,
            prechecked_guards,
        )
        .and_then(|result| {
            if !batch.is_empty() {
                crate::temporal::record_batch_commit_timestamp(
                    &tx,
                    base_cursor.saturating_add(batch.len() as u64),
                    crate::temporal::current_epoch_ms(),
                )?;
            }
            visibility_delta.apply(&tx)?;
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
        scope,
        batch,
        plans,
        base_cursor,
        provenance_row_cap,
        vector_kinds_to_enrol,
        dependency_closure::SoftClosureMode::Complete,
        endpoint_validation,
        None,
        preloaded_registry,
        prechecked_guards,
    )?;
    if !batch.is_empty() {
        crate::temporal::record_batch_commit_timestamp(
            &tx,
            base_cursor.saturating_add(batch.len() as u64),
            crate::temporal::current_epoch_ms(),
        )?;
    }
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

#[derive(Default)]
pub(crate) struct VisibilityDelta {
    pub(crate) additive: std::collections::BTreeMap<(String, String), u64>,
    pub(crate) destructive_scopes: std::collections::BTreeSet<String>,
    pub(crate) global_struct: bool,
    pub(crate) any_visibility_change: bool,
}

impl VisibilityDelta {
    pub(crate) fn record_add(
        &mut self,
        scope: &str,
        partition_kind: impl Into<String>,
        cursor: u64,
    ) {
        self.any_visibility_change = true;
        let key = (scope.to_string(), partition_kind.into());
        self.additive
            .entry(key)
            .and_modify(|existing| *existing = (*existing).max(cursor))
            .or_insert(cursor);
    }

    pub(crate) fn record_destructive(&mut self, scope: &str) {
        self.any_visibility_change = true;
        self.destructive_scopes.insert(scope.to_string());
    }

    pub(crate) fn record_global_struct(&mut self) {
        self.any_visibility_change = true;
        self.global_struct = true;
    }

    pub(crate) fn apply(&self, tx: &Connection) -> rusqlite::Result<()> {
        if !self.any_visibility_change
            && self.additive.is_empty()
            && self.destructive_scopes.is_empty()
            && !self.global_struct
        {
            return Ok(());
        }
        for ((scope, partition_kind), max_cursor) in &self.additive {
            let max_cursor_i64 = i64::try_from(*max_cursor).unwrap_or(i64::MAX);
            let changed = tx
                .prepare_cached(
                    "INSERT INTO _fathomdb_scope_visibility_watermarks(
                        scope, partition_kind, add_generation, max_cursor, destructive_generation, state_nonce
                     ) VALUES(?1, ?2, 1, ?3, 0, lower(hex(randomblob(32))))
                     ON CONFLICT(scope, partition_kind) DO UPDATE SET
                        add_generation = add_generation + 1,
                        max_cursor = MAX(max_cursor, excluded.max_cursor),
                        state_nonce = lower(hex(randomblob(32)))
                     WHERE add_generation < 9223372036854775807",
                )?
                .execute(params![scope, partition_kind, max_cursor_i64])?;
            if changed != 1 {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER),
                    Some("read visibility generation exhausted".to_string()),
                ));
            }
        }
        for scope in &self.destructive_scopes {
            let changed = tx
                .prepare_cached(
                    "INSERT INTO _fathomdb_scope_visibility_watermarks(
                        scope, partition_kind, add_generation, max_cursor, destructive_generation, state_nonce
                     ) VALUES(?1, 'scope_destructive', 0, 0, 1, lower(hex(randomblob(32))))
                     ON CONFLICT(scope, partition_kind) DO UPDATE SET
                        destructive_generation = destructive_generation + 1,
                        state_nonce = lower(hex(randomblob(32)))
                     WHERE destructive_generation < 9223372036854775807",
                )?
                .execute([scope])?;
            if changed != 1 {
                return Err(rusqlite::Error::SqliteFailure(
                    rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_TRIGGER),
                    Some("read visibility generation exhausted".to_string()),
                ));
            }
        }
        if self.global_struct {
            bump_global_struct(tx)?;
        } else {
            advance_read_visibility(tx)?;
        }
        Ok(())
    }
}

pub(crate) fn advance_read_visibility(connection: &Connection) -> rusqlite::Result<()> {
    let changed = connection
        .prepare_cached(
            "UPDATE _fathomdb_read_visibility_state
         SET generation=generation+1,
             state_nonce=lower(hex(randomblob(32))),coalescing=0
         WHERE singleton=1 AND generation<9223372036854775807
           AND typeof(coalescing)='integer' AND coalescing IN(0,1)",
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

pub(crate) fn bump_global_struct(connection: &Connection) -> rusqlite::Result<()> {
    let changed = connection
        .prepare_cached(
            "UPDATE _fathomdb_read_visibility_state
         SET generation=generation+1,
             state_nonce=lower(hex(randomblob(32))),
             global_struct_gen=global_struct_gen+1,
             global_struct_nonce=lower(hex(randomblob(32))),
             coalescing=0
         WHERE singleton=1
           AND generation<9223372036854775807
           AND global_struct_gen<9223372036854775807
           AND typeof(coalescing)='integer' AND coalescing IN(0,1)",
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
#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_batch_in_transaction(
    tx: &Connection,
    scope: &str,
    batch: &[PreparedWrite],
    plans: &[WritePlan],
    base_cursor: u64,
    provenance_row_cap: u64,
    vector_kinds_to_enrol: &[String],
    closure_mode: dependency_closure::SoftClosureMode,
    endpoint_validation: CommitEndpointValidationMode,
    visibility_delta: Option<&mut VisibilityDelta>,
    preloaded_registry: Option<&BTreeMap<(crate::PropertyEntityClass, String), StoredProjection>>,
    prechecked_guards: Option<PrecheckedWriteGuards>,
) -> Result<(u64, bool, Vec<ClosureOperationId>), CommitBatchError> {
    let (scoped_physical, queued_erasure) = match prechecked_guards {
        Some(guards) => (guards.scoped_physical, guards.queued_erasure),
        None => {
            let scoped_physical = physical_erasure::guard_plain_write(tx, scope, batch)?;
            let queued_erasure = erasure_queue::guard_plain(tx, scope, batch)?;
            if !scoped_physical {
                dependency_closure::guard_no_pending_physical(tx)?;
            }
            (scoped_physical, queued_erasure)
        }
    };
    let mut closure_ids = Vec::new();
    for kind in vector_kinds_to_enrol {
        register_vector_kind(tx, kind)?;
    }
    let unstranded =
        if vector_kinds_to_enrol.is_empty() { false } else { reenqueue_stranded_vector_rows(tx)? };
    let has_canonical_writes = batch.iter().any(|write| {
        matches!(
            write,
            PreparedWrite::Node { .. }
                | PreparedWrite::Edge { .. }
                | PreparedWrite::ProvenancedNode(_)
                | PreparedWrite::ProvenancedEdge(_)
        )
    });
    let loaded_registry = if has_canonical_writes && preloaded_registry.is_none() {
        Some(load_projection_registry(tx)?)
    } else {
        None
    };
    let projection_registry = preloaded_registry.or(loaded_registry.as_ref());

    // Pre-fetch active prior cursors and registered revisions in chunks of 64
    // to collapse ~400 per-row point lookups per 200-row batch down to ~8.
    let mut node_active_by_logical_id = if batch.len() > 1 {
        Some(prefetch_active_node_cursors_by_logical_id(tx, scope, batch)?)
    } else {
        None
    };
    let mut edge_active_by_logical_id = if batch.len() > 1 {
        Some(prefetch_active_edge_cursors_by_logical_id(tx, scope, batch)?)
    } else {
        None
    };
    let runtime_revisions: Vec<Option<String>> = batch
        .iter()
        .enumerate()
        .map(|(i, write)| match write {
            PreparedWrite::Node { .. } | PreparedWrite::Edge { .. } => {
                let cursor = base_cursor.saturating_add((i as u64).saturating_add(1));
                Some(runtime_revision_id(write, scope, cursor))
            }
            _ => None,
        })
        .collect();
    let revision_refs: Vec<&str> = batch
        .iter()
        .enumerate()
        .filter_map(|(i, write)| match write {
            PreparedWrite::ProvenancedNode(node) => {
                Some(node.provenance.artifact_revision_id.as_str())
            }
            PreparedWrite::ProvenancedEdge(edge) => {
                Some(edge.provenance.artifact_revision_id.as_str())
            }
            PreparedWrite::Node { .. } | PreparedWrite::Edge { .. } => {
                runtime_revisions[i].as_deref()
            }
            _ => None,
        })
        .collect();
    let mut registered_revisions = prefetch_registered_revisions(tx, scope, &revision_refs)?;

    let mut batch_fts_dirty = false;
    let mut vector_kind_cache: std::collections::HashMap<String, bool> =
        std::collections::HashMap::new();

    // Deduplicate VisibilityDelta keys locally across the batch so 200-row
    // batches do not allocate `format!("node:{kind}")` / `scope.to_string()`
    // on every row.
    let track_delta = visibility_delta.is_some();
    let mut delta_node_all: Option<u64> = None;
    let mut delta_edge_any: Option<u64> = None;
    let mut delta_op_state: Option<u64> = None;
    let mut delta_node_kinds: std::collections::HashMap<&str, u64> =
        std::collections::HashMap::new();
    let mut delta_edge_kinds: std::collections::HashMap<&str, u64> =
        std::collections::HashMap::new();
    let mut delta_op_logs: std::collections::HashMap<&str, u64> = std::collections::HashMap::new();
    let mut delta_prior_node_kinds: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut delta_prior_edge_kinds: std::collections::HashMap<String, u64> =
        std::collections::HashMap::new();
    let mut delta_destructive = false;
    let mut delta_soft_closure = false;
    let mut delta_global_struct = !vector_kinds_to_enrol.is_empty() || unstranded;

    for (i, (original_write, plan)) in batch.iter().zip(plans).enumerate() {
        // Per-row cursor: row i gets `base_cursor + i + 1`. See the
        // comment in `Engine::write_inner`.
        let cursor = base_cursor.saturating_add((i as u64).saturating_add(1));
        if scoped_physical {
            physical_erasure::guard_plain_write(tx, scope, std::slice::from_ref(original_write))?;
        }
        if queued_erasure {
            erasure_queue::guard_plain(tx, scope, std::slice::from_ref(original_write))?;
        }
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
                    properties,
                },
                WritePlan::Node,
            ) => {
                if let Some(logical_id) = logical_id {
                    let prior_entries: Vec<(i64, Option<String>)> =
                        if let Some(ref mut node_map) = node_active_by_logical_id {
                            node_map
                                .remove(logical_id.as_str())
                                .unwrap_or_default()
                                .into_iter()
                                .map(|(sc, pk)| (sc, Some(pk)))
                                .collect()
                        } else {
                            prior_node_cursors_by_logical_id(tx, scope, logical_id)?
                                .into_iter()
                                .map(|sc| (sc, None))
                                .collect()
                        };
                    let mut had_soft_closure = false;
                    for (prior_cursor, _) in &prior_entries {
                        if let Some(source_revision) =
                            dependency_closure::source_revision_for_cursor(
                                tx,
                                scope,
                                *prior_cursor,
                            )?
                        {
                            if let Some(id) = dependency_closure::admit_soft_closure(
                                tx,
                                scope,
                                &source_revision,
                                ClosureCauseV1::Superseded,
                                cursor,
                                closure_mode,
                            )? {
                                had_soft_closure = true;
                                closure_ids.push(id);
                            }
                        }
                    }
                    if had_soft_closure {
                        // `apply_soft_effects` may supersede derived edges; fall back to
                        // live edge lookups for any subsequent edges in this batch.
                        edge_active_by_logical_id = None;
                    }
                    if !prior_entries.is_empty() {
                        if track_delta {
                            delta_destructive = true;
                            for (sc, cached_kind) in &prior_entries {
                                let pk = match cached_kind {
                                    Some(pk) => Some(pk.clone()),
                                    None => tx
                                        .prepare_cached(
                                            "SELECT kind FROM canonical_nodes WHERE scope = ?1 AND write_cursor = ?2",
                                        )?
                                        .query_row(params![scope, sc], |row| {
                                            row.get::<_, String>(0)
                                        })
                                        .optional()?,
                                };
                                if let Some(pk) = pk {
                                    if pk != *kind {
                                        delta_prior_node_kinds
                                            .entry(pk)
                                            .and_modify(|c| *c = (*c).max(cursor))
                                            .or_insert(cursor);
                                    }
                                }
                            }
                            if had_soft_closure {
                                delta_soft_closure = true;
                            }
                        }
                        tx.prepare_cached(
                            "UPDATE canonical_nodes SET superseded_at = ?1
                             WHERE scope = ?2 AND logical_id = ?3 AND superseded_at IS NULL",
                        )?
                        .execute(params![cursor, scope, logical_id])?;
                        for (sc, _) in &prior_entries {
                            vector_storage::refresh_vector_eligibility_for_cursor(tx, *sc)?;
                            purge_row_projections_for_cursor_in(
                                tx,
                                *sc,
                                &[
                                    ProjectionClass::Attribute,
                                    ProjectionClass::PropertyFts,
                                    ProjectionClass::PropertyValue,
                                ],
                            )?;
                        }
                    }
                }
                let encoded_properties =
                    crate::properties::encode_properties_json(properties.as_ref())
                        .map_err(CommitBatchError::Engine)?;
                tx.prepare_cached(
                    "INSERT INTO canonical_nodes(write_cursor, kind, body, source_id, logical_id, row_kind, state, reason, valid_from, valid_until, properties, scope)
                     VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                )?
                .execute(params![cursor, kind, body, source_id.as_str(), logical_id, RowKind::Leaf.as_str(), state.as_str(), reason, valid_from, valid_until, encoded_properties, scope])?;
                if let (Some(logical_id), Some(ref mut node_map)) =
                    (logical_id, &mut node_active_by_logical_id)
                {
                    node_map
                        .entry(logical_id.clone())
                        .or_default()
                        .push((cursor as i64, kind.clone()));
                }
                project_canonical_node_row(
                    tx,
                    cursor,
                    kind,
                    body,
                    RowKind::Leaf,
                    ProjectionPass::Write,
                    matches!(state, InitialState::Active),
                    properties.as_ref(),
                    projection_registry,
                    Some(&mut batch_fts_dirty),
                    Some(&mut vector_kind_cache),
                )?;
                if track_delta {
                    let kind_ref = match original_write {
                        PreparedWrite::Node { kind, .. } => kind.as_str(),
                        PreparedWrite::ProvenancedNode(node) => node.kind.as_str(),
                        _ => unreachable!(),
                    };
                    delta_node_kinds
                        .entry(kind_ref)
                        .and_modify(|c| *c = (*c).max(cursor))
                        .or_insert(cursor);
                    delta_node_all = Some(delta_node_all.map_or(cursor, |c| c.max(cursor)));
                }
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
                    properties,
                },
                WritePlan::Edge,
            ) => {
                let mut edge_superseded = false;
                let mut prior_edge_kinds = Vec::new();
                if let Some(logical_id) = logical_id {
                    let prior_entries: Vec<(i64, Option<String>)> =
                        if let Some(ref mut edge_map) = edge_active_by_logical_id {
                            edge_map
                                .remove(logical_id.as_str())
                                .unwrap_or_default()
                                .into_iter()
                                .map(|(sc, pk)| (sc, Some(pk)))
                                .collect()
                        } else {
                            prior_edge_cursors_by_logical_id(tx, scope, logical_id)?
                                .into_iter()
                                .map(|sc| (sc, None))
                                .collect()
                        };
                    if !prior_entries.is_empty() {
                        edge_superseded = true;
                        if track_delta {
                            for (sc, cached_kind) in &prior_entries {
                                if let Some(pk) = cached_kind {
                                    prior_edge_kinds.push(pk.clone());
                                } else if let Some(pk) = tx
                                    .prepare_cached(
                                        "SELECT kind FROM canonical_edges WHERE scope = ?1 AND write_cursor = ?2",
                                    )?
                                    .query_row(params![scope, sc], |row| row.get::<_, String>(0))
                                    .optional()?
                                {
                                    prior_edge_kinds.push(pk);
                                }
                            }
                        }
                        tx.prepare_cached(
                            "UPDATE canonical_edges SET superseded_at = ?1
                             WHERE scope = ?2 AND logical_id = ?3 AND superseded_at IS NULL",
                        )?
                        .execute(params![cursor, scope, logical_id])?;
                        for (sc, _) in &prior_entries {
                            vector_storage::refresh_vector_eligibility_for_cursor(tx, *sc)?;
                            purge_row_projections_for_cursor_in(
                                tx,
                                *sc,
                                &[ProjectionClass::PropertyValue],
                            )?;
                        }
                    }
                }
                if body.is_some() {
                    let prior_g11 = prior_edge_cursors_by_triple(tx, scope, from, to, kind)?;
                    if !prior_g11.is_empty() {
                        edge_superseded = true;
                        // Triple supersession may supersede an edge cached under another
                        // logical_id; invalidate the pre-fetched map for remaining edges.
                        edge_active_by_logical_id = None;
                        tx.prepare_cached(
                            "UPDATE canonical_edges SET superseded_at = ?1
                             WHERE scope = ?2 AND from_id = ?3 AND to_id = ?4 AND kind = ?5 AND superseded_at IS NULL",
                        )?
                        .execute(params![cursor, scope, from, to, kind])?;
                        for sc in &prior_g11 {
                            vector_storage::refresh_vector_eligibility_for_cursor(tx, *sc)?;
                            purge_row_projections_for_cursor_in(
                                tx,
                                *sc,
                                &[ProjectionClass::PropertyValue],
                            )?;
                        }
                    }
                }
                let temporal_fallback_i: Option<i64> =
                    temporal_fallback.and_then(|f| if f { Some(1) } else { None });
                let encoded_properties =
                    crate::properties::encode_properties_json(properties.as_ref())
                        .map_err(CommitBatchError::Engine)?;
                tx.prepare_cached(
                    "INSERT INTO canonical_edges(
                         write_cursor, kind, from_id, to_id, source_id, logical_id,
                         body, t_valid, t_invalid, confidence, extractor_model_id,
                         temporal_fallback, properties, scope
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
                )?
                .execute(params![
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
                    temporal_fallback_i,
                    encoded_properties,
                    scope,
                ])?;
                if let (Some(logical_id), Some(ref mut edge_map)) =
                    (logical_id, &mut edge_active_by_logical_id)
                {
                    edge_map
                        .entry(logical_id.clone())
                        .or_default()
                        .push((cursor as i64, kind.clone()));
                }
                let (_enqueued_vec, enrolled_vk) = project_canonical_edge_row(
                    tx,
                    cursor,
                    kind,
                    body.as_deref(),
                    ProjectionPass::Write,
                    properties.as_ref(),
                    true,
                    projection_registry,
                    Some(&mut batch_fts_dirty),
                )?;
                if track_delta {
                    let kind_ref = match original_write {
                        PreparedWrite::Edge { kind, .. } => kind.as_str(),
                        PreparedWrite::ProvenancedEdge(edge) => edge.kind.as_str(),
                        _ => unreachable!(),
                    };
                    delta_edge_kinds
                        .entry(kind_ref)
                        .and_modify(|c| *c = (*c).max(cursor))
                        .or_insert(cursor);
                    delta_edge_any = Some(delta_edge_any.map_or(cursor, |c| c.max(cursor)));
                    if edge_superseded {
                        delta_destructive = true;
                        for pk in prior_edge_kinds {
                            if pk != *kind {
                                delta_prior_edge_kinds
                                    .entry(pk)
                                    .and_modify(|c| *c = (*c).max(cursor))
                                    .or_insert(cursor);
                            }
                        }
                    }
                    if enrolled_vk {
                        delta_global_struct = true;
                    }
                }
            }
            (
                PreparedWrite::AdminSchema { name, kind, schema_json, retention_json },
                WritePlan::AdminSchema,
            ) => {
                let existing_kind: Option<String> = tx
                    .prepare_cached("SELECT kind FROM operational_collections WHERE name = ?1")?
                    .query_row([name], |row| row.get(0))
                    .optional()?;
                if existing_kind.as_deref().is_some_and(|existing| existing != kind.as_str()) {
                    return Err(EngineError::WriteValidation.into());
                }
                tx.prepare_cached(
                    "INSERT INTO operational_collections(
                        name, kind, schema_json, retention_json, format_version, created_at
                     ) VALUES(?1, ?2, ?3, ?4, 1, 0)
                     ON CONFLICT(name) DO UPDATE SET
                        schema_json = excluded.schema_json,
                        retention_json = excluded.retention_json",
                )?
                .execute(params![name, kind, schema_json, retention_json])?;
                record_projection_terminal(
                    tx,
                    cursor,
                    "up_to_date",
                    ProjectionTerminalOwnerKind::NonCanonical,
                )?;
                if track_delta {
                    delta_global_struct = true;
                }
            }
            (
                PreparedWrite::OpStore { collection, record_key, schema_id, body },
                WritePlan::AppendOnlyLog,
            ) => {
                tx.prepare_cached(
                    "INSERT INTO operational_mutations(
                        collection_name, record_key, op_kind, payload_json, schema_id, write_cursor, scope
                     ) VALUES(?1, ?2, 'append', ?3, ?4, ?5, ?6)",
                )?
                .execute(params![collection, record_key, body, schema_id, cursor, scope])?;
                record_projection_terminal(
                    tx,
                    cursor,
                    "up_to_date",
                    ProjectionTerminalOwnerKind::NonCanonical,
                )?;
                if track_delta {
                    if let PreparedWrite::OpStore { collection, .. } = original_write {
                        delta_op_logs
                            .entry(collection.as_str())
                            .and_modify(|c| *c = (*c).max(cursor))
                            .or_insert(cursor);
                    }
                }
            }
            (
                PreparedWrite::OpStore { collection, record_key, schema_id, body },
                WritePlan::LatestState,
            ) => {
                tx.prepare_cached(
                    "INSERT INTO operational_state(
                        collection_name, record_key, payload_json, schema_id, write_cursor, scope
                     ) VALUES(?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(scope, collection_name, record_key) DO UPDATE SET
                        payload_json = excluded.payload_json,
                        schema_id = excluded.schema_id,
                        write_cursor = excluded.write_cursor",
                )?
                .execute(params![collection, record_key, body, schema_id, cursor, scope])?;
                record_projection_terminal(
                    tx,
                    cursor,
                    "up_to_date",
                    ProjectionTerminalOwnerKind::NonCanonical,
                )?;
                if track_delta {
                    delta_op_state = Some(delta_op_state.map_or(cursor, |c| c.max(cursor)));
                }
            }
            _ => return Err(rusqlite::Error::InvalidQuery.into()),
        }
        register_artifact_identity(
            tx,
            scope,
            original_write,
            cursor,
            runtime_revisions[i].as_deref(),
            &mut registered_revisions,
        )?;
    }

    if let Some(delta) = visibility_delta {
        for (kind, max_cursor) in delta_node_kinds {
            delta.record_add(scope, format!("node:{kind}"), max_cursor);
        }
        for (kind, max_cursor) in delta_prior_node_kinds {
            delta.record_add(scope, format!("node:{kind}"), max_cursor);
        }
        if let Some(max_cursor) = delta_node_all {
            delta.record_add(scope, "node_all", max_cursor);
        }
        for (kind, max_cursor) in delta_edge_kinds {
            delta.record_add(scope, format!("edge:{kind}"), max_cursor);
        }
        for (kind, max_cursor) in delta_prior_edge_kinds {
            delta.record_add(scope, format!("edge:{kind}"), max_cursor);
        }
        if let Some(max_cursor) = delta_edge_any {
            delta.record_add(scope, "edge_any", max_cursor);
        }
        for (collection, max_cursor) in delta_op_logs {
            delta.record_add(scope, format!("op_log:{collection}"), max_cursor);
        }
        if let Some(max_cursor) = delta_op_state {
            delta.record_add(scope, "op_state", max_cursor);
        }
        if delta_destructive {
            delta.record_destructive(scope);
        }
        if delta_soft_closure {
            delta.record_add(scope, "scope_dependency", 0);
            delta.record_add(scope, "edge_any", 0);
            delta.record_add(scope, "vector_ready_all", 0);
        }
        if delta_global_struct {
            delta.record_global_struct();
        }
    }

    let dangling_edge_endpoints: u64 = match endpoint_validation {
        CommitEndpointValidationMode::Strict => {
            validate_strict_batch_edge_endpoints(tx, scope, batch)?;
            0
        }
        CommitEndpointValidationMode::BulkNonStrict => {
            account_bulk_non_strict_batch_edge_endpoints(tx, scope, batch, base_cursor)?
        }
        CommitEndpointValidationMode::DeferredToActuationSimulation => 0,
    };

    enforce_provenance_retention(tx, provenance_row_cap)?;
    advance_projection_cursor(tx)?;
    if scoped_physical {
        physical_erasure::certify_plain_write(tx, base_cursor.saturating_add(batch.len() as u64))?;
    }

    Ok((dangling_edge_endpoints, unstranded, closure_ids))
}

fn validate_strict_batch_edge_endpoints(
    tx: &Connection,
    scope: &str,
    batch: &[PreparedWrite],
) -> Result<(), CommitBatchError> {
    let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut distinct: Vec<&str> = Vec::new();
    for write in batch {
        let endpoints = match write {
            PreparedWrite::Edge { from, to, .. } => Some((from.as_str(), to.as_str())),
            PreparedWrite::ProvenancedEdge(edge) => Some((edge.from.as_str(), edge.to.as_str())),
            _ => None,
        };
        if let Some((from, to)) = endpoints {
            for endpoint in [from, to] {
                if seen.insert(endpoint) {
                    distinct.push(endpoint);
                }
            }
        }
    }
    if distinct.is_empty() {
        return Ok(());
    }

    const CHUNK_SIZE: usize = 64;
    for chunk in distinct.chunks(CHUNK_SIZE) {
        let count: i64 = if chunk.len() == CHUNK_SIZE {
            let mut stmt = tx.prepare_cached(
                "SELECT COUNT(*) FROM canonical_nodes \
                 WHERE scope = ?1 AND logical_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 ) AND superseded_at IS NULL AND state = 'active'",
            )?;
            stmt.query_row(
                rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
                |row| row.get(0),
            )?
        } else {
            let mut sql = String::from(
                "SELECT COUNT(*) FROM canonical_nodes WHERE scope = ?1 AND logical_id IN (",
            );
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push_str(") AND superseded_at IS NULL AND state = 'active'");
            let mut stmt = tx.prepare_cached(&sql)?;
            stmt.query_row(
                rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
                |row| row.get(0),
            )?
        };
        if count as usize != chunk.len() {
            return Err(CommitBatchError::Engine(EngineError::WriteValidation));
        }
    }
    Ok(())
}

pub(crate) fn other_active_node_scope_exists(
    connection: &Connection,
    scope: &str,
) -> rusqlite::Result<bool> {
    connection
        .prepare_cached(
            "SELECT EXISTS(\
                 SELECT 1 FROM canonical_nodes \
                 WHERE scope < ?1 AND state = 'active' AND superseded_at IS NULL LIMIT 1\
             ) OR EXISTS(\
                 SELECT 1 FROM canonical_nodes \
                 WHERE scope > ?1 AND state = 'active' AND superseded_at IS NULL LIMIT 1\
             )",
        )?
        .query_row([scope], |row| row.get(0))
}

pub(crate) fn next_active_node_scope(
    connection: &Connection,
    after_scope: Option<&str>,
) -> rusqlite::Result<Option<String>> {
    match after_scope {
        None => connection
            .prepare_cached(
                "SELECT scope FROM canonical_nodes \
                 WHERE state = 'active' AND superseded_at IS NULL \
                 ORDER BY scope LIMIT 1",
            )?
            .query_row([], |row| row.get(0))
            .optional(),
        Some(prev) => connection
            .prepare_cached(
                "SELECT scope FROM canonical_nodes \
                 WHERE scope > ?1 AND state = 'active' AND superseded_at IS NULL \
                 ORDER BY scope LIMIT 1",
            )?
            .query_row([prev], |row| row.get(0))
            .optional(),
    }
}

fn scope_contains_any_active_node(
    connection: &Connection,
    scope: &str,
    logical_ids: &[&str],
) -> rusqlite::Result<bool> {
    const CHUNK_SIZE: usize = 64;
    for chunk in logical_ids.chunks(CHUNK_SIZE) {
        let mut stmt = if chunk.len() == CHUNK_SIZE {
            connection.prepare_cached(
                "SELECT 1 FROM canonical_nodes \
                 WHERE scope = ?1 AND logical_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 ) AND superseded_at IS NULL AND state = 'active' LIMIT 1",
            )?
        } else {
            let mut sql =
                String::from("SELECT 1 FROM canonical_nodes WHERE scope = ?1 AND logical_id IN (");
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push_str(") AND superseded_at IS NULL AND state = 'active' LIMIT 1");
            connection.prepare_cached(&sql)?
        };
        let found: Option<i64> = stmt
            .query_row(
                rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
                |row| row.get(0),
            )
            .optional()?;
        if found.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn account_bulk_non_strict_batch_edge_endpoints(
    tx: &Connection,
    scope: &str,
    batch: &[PreparedWrite],
    base_cursor: u64,
) -> Result<u64, CommitBatchError> {
    let has_edges = batch.iter().any(|write| {
        matches!(write, PreparedWrite::Edge { .. } | PreparedWrite::ProvenancedEdge(_))
    });
    if !has_edges {
        return Ok(0);
    }

    let first_cursor = base_cursor.saturating_add(1);
    let last_cursor = base_cursor.saturating_add(u64::try_from(batch.len()).unwrap_or(u64::MAX));
    let mut edge_stmt = tx.prepare_cached(
        "SELECT from_id, to_id FROM canonical_edges \
         WHERE scope = ?1 AND write_cursor BETWEEN ?2 AND ?3 AND superseded_at IS NULL",
    )?;
    let surviving_edges: Vec<(String, String)> = edge_stmt
        .query_map(params![scope, first_cursor, last_cursor], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if surviving_edges.is_empty() {
        return Ok(0);
    }

    let mut distinct_seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
    let mut distinct_endpoints: Vec<&str> = Vec::new();
    for (from_id, to_id) in &surviving_edges {
        for endpoint in [from_id.as_str(), to_id.as_str()] {
            if distinct_seen.insert(endpoint) {
                distinct_endpoints.push(endpoint);
            }
        }
    }

    const CHUNK_SIZE: usize = 64;
    let mut present_in_scope: std::collections::HashSet<String> = std::collections::HashSet::new();
    for chunk in distinct_endpoints.chunks(CHUNK_SIZE) {
        let mut stmt = if chunk.len() == CHUNK_SIZE {
            tx.prepare_cached(
                "SELECT logical_id FROM canonical_nodes \
                 WHERE scope = ?1 AND logical_id IN (\
                     ?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,\
                     ?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30,?31,?32,?33,\
                     ?34,?35,?36,?37,?38,?39,?40,?41,?42,?43,?44,?45,?46,?47,?48,?49,\
                     ?50,?51,?52,?53,?54,?55,?56,?57,?58,?59,?60,?61,?62,?63,?64,?65\
                 ) AND superseded_at IS NULL AND state = 'active'",
            )?
        } else {
            let mut sql = String::from(
                "SELECT logical_id FROM canonical_nodes WHERE scope = ?1 AND logical_id IN (",
            );
            for idx in 0..chunk.len() {
                if idx > 0 {
                    sql.push(',');
                }
                sql.push_str(&format!("?{}", idx + 2));
            }
            sql.push_str(") AND superseded_at IS NULL AND state = 'active'");
            tx.prepare_cached(&sql)?
        };
        let rows = stmt.query_map(
            rusqlite::params_from_iter(std::iter::once(scope).chain(chunk.iter().copied())),
            |row| row.get::<_, String>(0),
        )?;
        for row in rows {
            present_in_scope.insert(row?);
        }
    }

    let missing: Vec<&str> = distinct_endpoints
        .iter()
        .copied()
        .filter(|endpoint| !present_in_scope.contains(*endpoint))
        .collect();

    if !missing.is_empty() && other_active_node_scope_exists(tx, scope)? {
        let mut prev_scope: Option<String> = None;
        while let Some(other_scope) = next_active_node_scope(tx, prev_scope.as_deref())? {
            prev_scope = Some(other_scope.clone());
            if other_scope == scope {
                continue;
            }
            if scope_contains_any_active_node(tx, &other_scope, &missing)? {
                return Err(CommitBatchError::Engine(EngineError::ScopeViolation {
                    detail: "bulk non-strict write refused: edge endpoint resolves to an active node in another scope".to_string(),
                }));
            }
        }
    }

    let mut dangling: u64 = 0;
    for (from_id, to_id) in &surviving_edges {
        if !present_in_scope.contains(from_id.as_str()) {
            dangling = dangling.saturating_add(1);
        }
        if !present_in_scope.contains(to_id.as_str()) {
            dangling = dangling.saturating_add(1);
        }
    }
    Ok(dangling)
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

impl Engine {
    #[cfg(any(test, feature = "test-hooks"))]
    #[doc(hidden)]
    pub fn force_next_commit_failure_for_test(&self) {
        self.force_next_commit_failure.store(true, Ordering::SeqCst);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        account_bulk_non_strict_batch_edge_endpoints, validate_strict_batch_edge_endpoints,
        CommitBatchError,
    };
    use crate::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
    use rusqlite::hooks::{AuthAction, AuthContext, Authorization};

    #[test]
    fn bulk_non_strict_node_only_batch_takes_zero_sql_fast_path() {
        let dir = tempfile::TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("bulk-fast-path.sqlite")).unwrap();
        let node = PreparedWrite::Node {
            kind: "note".to_string(),
            body: "fastpath".to_string(),
            source_id: SourceId::new("test:bulk-fast-path").unwrap(),
            logical_id: Some("n1".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        };
        let mut guard = opened.engine.connection.lock().unwrap();
        let connection = guard.as_mut().unwrap();
        connection
            .authorizer(Some(|context: AuthContext<'_>| {
                if context.accessor.is_none()
                    && matches!(
                        context.action,
                        AuthAction::Read { table_name: "canonical_nodes" | "canonical_edges", .. }
                    )
                {
                    Authorization::Deny
                } else {
                    Authorization::Allow
                }
            }))
            .unwrap();
        let result = account_bulk_non_strict_batch_edge_endpoints(
            connection,
            "",
            std::slice::from_ref(&node),
            0,
        );
        connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>).unwrap();
        assert_eq!(result.unwrap(), 0);
    }

    #[test]
    fn node_only_dangling_check_does_not_require_endpoint_reads() {
        let dir = tempfile::TempDir::new().unwrap();
        let opened = Engine::open(dir.path().join("endpoint-independent.sqlite")).unwrap();
        let source_id = SourceId::new("test:endpoint-independent").unwrap();
        let node = PreparedWrite::Node {
            kind: "note".to_string(),
            body: "endpointindependent".to_string(),
            source_id: source_id.clone(),
            logical_id: None,
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        };
        let edge = PreparedWrite::Edge {
            kind: "related".to_string(),
            from: "missing-from".to_string(),
            to: "missing-to".to_string(),
            source_id,
            logical_id: None,
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
        };
        {
            let mut guard = opened.engine.connection.lock().unwrap();
            let connection = guard.as_mut().unwrap();
            // Restrict only the isolated endpoint pass, not other Node write SQL.
            connection
                .authorizer(Some(|context: AuthContext<'_>| {
                    if context.accessor.is_none()
                        && matches!(
                            context.action,
                            AuthAction::Read { table_name: "canonical_nodes", .. }
                        )
                    {
                        Authorization::Deny
                    } else {
                        Authorization::Allow
                    }
                }))
                .unwrap();
            let node_result =
                validate_strict_batch_edge_endpoints(connection, "", std::slice::from_ref(&node));
            let edge_error =
                validate_strict_batch_edge_endpoints(connection, "", std::slice::from_ref(&edge))
                    .expect_err("Edge endpoint checks must consult SQLite");
            connection.authorizer(None::<fn(AuthContext<'_>) -> Authorization>).unwrap();
            assert!(matches!(
                edge_error,
                CommitBatchError::Sql(rusqlite::Error::SqliteFailure(error, _))
                    if error.code == rusqlite::ErrorCode::AuthorizationForStatementDenied
            ));
            let strict_err = validate_strict_batch_edge_endpoints(connection, "", &[edge])
                .expect_err("Dangling edge must be rejected with WriteValidation");
            assert!(
                matches!(strict_err, CommitBatchError::Engine(EngineError::WriteValidation)),
                "expected CommitBatchError::Engine(WriteValidation), got {strict_err:?}"
            );
            assert!(
                matches!(node_result, Ok(())),
                "Node-only endpoint check must not read canonical_nodes: {node_result:?}"
            );
        }
        let receipt = opened.engine.write(&[node]).unwrap();
        assert_eq!(receipt.cursor, 1);
        assert_eq!(receipt.dangling_edge_endpoints, 0);
        let search = opened.engine.search_text_only("endpointindependent").unwrap();
        assert_eq!(search.results.len(), 1);
        assert_eq!(search.results[0].body, "endpointindependent");
    }
}

#[cfg(test)]
#[test]
fn coalescing_refuses_pre_disabled_native_triggers_without_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let opened = Engine::open(dir.path().join("native-disabled.sqlite")).unwrap();
    let mut primary = opened.engine.connection.lock().unwrap();
    let connection = primary.as_mut().unwrap();
    connection.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER, false).unwrap();
    let tx =
        connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).unwrap();
    assert!(matches!(VisibilityCoalescingGuard::arm(&tx), Err(CommitBatchError::WriterPoisoned)));
    assert_eq!(
        tx.query_row("SELECT coalescing FROM _fathomdb_read_visibility_state", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(!tx.db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_TRIGGER).unwrap());
}

#[cfg(test)]
#[test]
fn writer_connection_sets_cache_size_4mib_and_batch_write_eliminates_redundant_sql() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    let dir = tempfile::tempdir().unwrap();
    let opened = Engine::open(dir.path().join("batch-perf-red.sqlite")).unwrap();

    // Fix 2A: writer connection must configure PRAGMA cache_size = -4000 (4 MiB).
    let writer_cache_size: i64 = {
        let mut guard = opened.engine.connection.lock().unwrap();
        let conn = guard.as_mut().unwrap();
        conn.query_row("PRAGMA cache_size", [], |row| row.get(0)).unwrap()
    };
    assert_eq!(writer_cache_size, -4000, "writer connection cache_size must be -4000 (4 MiB)");

    // Fix 2B: instrument writer SQL statements across a 200-node batch.
    let mark_dirty_calls = Arc::new(AtomicUsize::new(0));
    let terminal_point_lookups = Arc::new(AtomicUsize::new(0));
    let prior_node_point_lookups = Arc::new(AtomicUsize::new(0));
    let revision_point_lookups = Arc::new(AtomicUsize::new(0));
    let vector_kind_lookups = Arc::new(AtomicUsize::new(0));

    {
        let md = Arc::clone(&mark_dirty_calls);
        let tp = Arc::clone(&terminal_point_lookups);
        let pn = Arc::clone(&prior_node_point_lookups);
        let rv = Arc::clone(&revision_point_lookups);
        let vk = Arc::clone(&vector_kind_lookups);
        let mut guard = opened.engine.connection.lock().unwrap();
        let conn = guard.as_mut().unwrap();
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::SQLITE_TRACE_STMT,
            Some(move |event: rusqlite::trace::TraceEvent<'_>| {
                if let rusqlite::trace::TraceEvent::Stmt(_, sql) = event {
                    if sql.contains("UPDATE _fathomdb_open_state") && sql.contains("dirty") {
                        md.fetch_add(1, Ordering::Relaxed);
                    }
                    if sql.contains(
                        "SELECT state FROM _fathomdb_projection_terminal WHERE write_cursor = ?1",
                    ) {
                        tp.fetch_add(1, Ordering::Relaxed);
                    }
                    if sql.contains("SELECT write_cursor FROM canonical_nodes")
                        && sql.contains("logical_id = ?2")
                    {
                        pn.fetch_add(1, Ordering::Relaxed);
                    }
                    if sql.contains(
                        "FROM _fathomdb_artifact_revisions WHERE scope = ?1 AND revision_id = ?2",
                    ) {
                        rv.fetch_add(1, Ordering::Relaxed);
                    }
                    if sql.contains("FROM _fathomdb_vector_kinds WHERE kind = ?1") {
                        vk.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }),
        )
        .unwrap();
    }

    let batch: Vec<PreparedWrite> = (0..200)
        .map(|i| PreparedWrite::Node {
            kind: "note".to_string(),
            body: format!("batch performance test body {i}"),
            source_id: SourceId::new(format!("src:batch-{i}")).unwrap(),
            logical_id: Some(format!("node:{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        })
        .collect();
    let receipt = opened.engine.write(&batch).unwrap();
    assert_eq!(receipt.cursor, 200);

    {
        let mut guard = opened.engine.connection.lock().unwrap();
        let conn = guard.as_mut().unwrap();
        conn.trace_v2(
            rusqlite::trace::TraceEventCodes::empty(),
            None::<fn(rusqlite::trace::TraceEvent<'_>)>,
        )
        .unwrap();
    }

    assert!(
        mark_dirty_calls.load(Ordering::Relaxed) <= 1,
        "mark_dirty must be hoisted out of per-row loop, got {}",
        mark_dirty_calls.load(Ordering::Relaxed)
    );
    assert_eq!(
        terminal_point_lookups.load(Ordering::Relaxed),
        0,
        "advance_projection_cursor must use range fast-path instead of per-row point lookups"
    );
    assert_eq!(
        prior_node_point_lookups.load(Ordering::Relaxed),
        0,
        "prior_node_cursors must use batched IN lookups instead of per-row point lookups"
    );
    assert_eq!(
        revision_point_lookups.load(Ordering::Relaxed),
        0,
        "revision_is_registered must use batched IN lookups instead of per-row point lookups"
    );
    assert!(
        vector_kind_lookups.load(Ordering::Relaxed) <= 4,
        "kind_is_vector_indexed must deduplicate per distinct kind, got {}",
        vector_kind_lookups.load(Ordering::Relaxed)
    );

    // Intra-batch supersession parity: duplicate node logical_id in one batch supersedes earlier item.
    let dup_batch = vec![
        PreparedWrite::Node {
            kind: "note".to_string(),
            body: "first version".to_string(),
            source_id: SourceId::new("src:dup-1").unwrap(),
            logical_id: Some("dup:node".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        },
        PreparedWrite::Node {
            kind: "note".to_string(),
            body: "second version".to_string(),
            source_id: SourceId::new("src:dup-2").unwrap(),
            logical_id: Some("dup:node".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        },
    ];
    let dup_receipt = opened.engine.write(&dup_batch).unwrap();
    assert_eq!(dup_receipt.cursor, 202);
    let active_dup_bodies: Vec<String> = {
        let mut guard = opened.engine.connection.lock().unwrap();
        let conn = guard.as_mut().unwrap();
        let mut stmt = conn
            .prepare("SELECT body FROM canonical_nodes WHERE logical_id = 'dup:node' AND superseded_at IS NULL")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    };
    assert_eq!(active_dup_bodies, vec!["second version".to_string()]);

    // Intra-batch duplicate explicit artifact_revision_id must still fail with RevisionIdConflict.
    let conflict_batch = vec![
        PreparedWrite::ProvenancedNode(crate::ProvenancedNodeV1 {
            kind: "note".to_string(),
            body: "rev one".to_string(),
            source_id: SourceId::new("src:rev-1").unwrap(),
            logical_id: Some("rev:1".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: crate::WriteProvenanceV1::canonical(
                crate::ArtifactRevisionId::new("rev-dup-same-batch").unwrap(),
                crate::SourceVersionId::new("ver-dup-1").unwrap(),
            ),
            properties: None,
        }),
        PreparedWrite::ProvenancedNode(crate::ProvenancedNodeV1 {
            kind: "note".to_string(),
            body: "rev two".to_string(),
            source_id: SourceId::new("src:rev-2").unwrap(),
            logical_id: Some("rev:2".to_string()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: crate::WriteProvenanceV1::canonical(
                crate::ArtifactRevisionId::new("rev-dup-same-batch").unwrap(),
                crate::SourceVersionId::new("ver-dup-2").unwrap(),
            ),
            properties: None,
        }),
    ];
    let conflict_err = opened.engine.write(&conflict_batch).unwrap_err();
    assert!(
        matches!(
            conflict_err,
            EngineError::Provenance(ref e)
                if e.reason == crate::ProvenanceErrorReason::RevisionIdConflict
        ),
        "intra-batch duplicate artifact_revision_id must fail with RevisionIdConflict, got {conflict_err:?}"
    );
    // Intra-batch duplicate edge logical_id must supersede earlier item cleanly.
    let edge_dup_batch = vec![
        PreparedWrite::Edge {
            kind: "links".to_string(),
            from: "node:0".to_string(),
            to: "node:1".to_string(),
            source_id: SourceId::new("src:edup-1").unwrap(),
            logical_id: Some("dup:edge".to_string()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.5),
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
        },
        PreparedWrite::Edge {
            kind: "links".to_string(),
            from: "node:0".to_string(),
            to: "node:2".to_string(),
            source_id: SourceId::new("src:edup-2").unwrap(),
            logical_id: Some("dup:edge".to_string()),
            body: None,
            t_valid: None,
            t_invalid: None,
            confidence: Some(0.9),
            extractor_model_id: None,
            temporal_fallback: None,
            properties: None,
        },
    ];
    opened.engine.write(&edge_dup_batch).unwrap();
    let active_dup_edges: i64 = {
        let mut guard = opened.engine.connection.lock().unwrap();
        let conn = guard.as_mut().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM canonical_edges WHERE logical_id = 'dup:edge' AND superseded_at IS NULL",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    assert_eq!(active_dup_edges, 1);

    opened.engine.close().unwrap();
}
