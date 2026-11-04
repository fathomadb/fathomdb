use super::*;
use crate::vector_storage::DEFAULT_VECTOR_PARTITION;
use serde::{Deserialize, Serialize};

mod admission;
pub(crate) use admission::{
    certify_plain_write, closure_safe_boundary, envelope, guard_plain_write,
};

const ROOT_KEY: &str = "_fathomdb_physical_erasure";
const FTS_KEY: &str = "_fathomdb_fts_sanitation";

pub(crate) fn faultpoint(phase: &str) -> Result<(), EngineError> {
    #[cfg(any(test, feature = "test-hooks"))]
    crate::test_hooks::physical_erasure_hooks::fire(phase)?;
    let _ = phase;
    Ok(())
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootDebt {
    #[serde(default)]
    pub(crate) ticket: Option<String>,
    version: u32,
    retry: String,
    cursors: Vec<i64>,
    vectors: Vec<i64>,
    boundary: u64,
    generation: u64,
    closures: Vec<String>,
    #[serde(default)]
    safe_boundary: u64,
    #[serde(default)]
    denied_sources: Vec<String>,
    #[serde(default)]
    denied_identities: Vec<String>,
    #[serde(default)]
    redacted: bool,
    #[serde(default)]
    blocker: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Sanitation {
    version: u32,
    phase: String,
    family: usize,
    started: bool,
}

fn families() -> Vec<&'static str> {
    ROW_OWNED_PROJECTIONS.iter().filter(|p| p.is_fts()).map(|p| p.table).collect()
}

fn load<T: serde::de::DeserializeOwned>(
    c: &Connection,
    key: &str,
) -> Result<Option<T>, EngineError> {
    c.query_row("SELECT value FROM _fathomdb_open_state WHERE key=?1", [key], |r| {
        r.get::<_, String>(0)
    })
    .optional()
    .map_err(|_| EngineError::Storage)?
    .map(|raw| serde_json::from_str(&raw).map_err(|_| EngineError::Storage))
    .transpose()
}

fn store(c: &Connection, key: &str, value: &impl Serialize) -> Result<(), EngineError> {
    let _timer = physical_metrics::stage("journal_update");
    c.execute("INSERT INTO _fathomdb_open_state(key,value) VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        params![key, serde_json::to_string(value).map_err(|_| EngineError::Storage)?])
        .map_err(|_| EngineError::Storage)?;
    Ok(())
}

fn state(c: &Connection) -> Result<Sanitation, EngineError> {
    let value = load::<Sanitation>(c, FTS_KEY)?.unwrap_or(Sanitation {
        version: 1,
        phase: "dirty".into(),
        family: 0,
        started: false,
    });
    let valid = value.version == 1
        && match value.phase.as_str() {
            "dirty" => value.family == 0 && !value.started,
            "cleaning" => value.family < families().len(),
            "finalizing" | "clean" => value.family == families().len() && !value.started,
            _ => false,
        };
    if !valid {
        return Err(EngineError::Storage);
    }
    Ok(value)
}

pub(crate) fn initialize_fresh(c: &Connection) -> Result<(), EngineError> {
    store(
        c,
        FTS_KEY,
        &Sanitation { version: 1, phase: "clean".into(), family: families().len(), started: false },
    )
}

pub(crate) fn mark_dirty(c: &Connection) -> rusqlite::Result<()> {
    // Absence already means legacy dirty; only an existing clean certificate changes.
    c.execute("UPDATE _fathomdb_open_state SET value='{\"version\":1,\"phase\":\"dirty\",\"family\":0,\"started\":false}' WHERE key=?1 AND json_extract(value,'$.phase')='clean'", [FTS_KEY])?;
    Ok(())
}

pub(crate) fn root(c: &Connection) -> Result<Option<RootDebt>, EngineError> {
    let _timer = physical_metrics::stage("root_decode_validate");
    let raw = load::<serde_json::Value>(c, ROOT_KEY)?;
    if let Some(raw) = &raw {
        if raw.get("version").and_then(|v| v.as_u64()) == Some(2)
            && ["safe_boundary", "denied_sources", "denied_identities", "redacted"]
                .iter()
                .any(|key| raw.get(*key).is_none())
        {
            return Err(EngineError::Storage);
        }
    }
    let value: Option<RootDebt> =
        raw.map(serde_json::from_value).transpose().map_err(|_| EngineError::Storage)?;
    if let Some(debt) = &value {
        let ordered =
            |ids: &[i64]| ids.iter().all(|id| *id > 0) && ids.windows(2).all(|p| p[0] < p[1]);
        let valid_hash = |hash: &str| {
            hash.len() == 64
                && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if debt.denied_sources.iter().chain(&debt.denied_identities).any(|hash| !valid_hash(hash))
            || (debt.version == 2
                && !debt.redacted
                && debt.denied_sources.is_empty()
                && debt.denied_identities.is_empty())
            || ![1, 2].contains(&debt.version)
            || (!debt.redacted && !valid_hash(&debt.retry))
            || (debt.redacted
                && (debt.version != 2
                    || !debt.retry.is_empty()
                    || !debt.cursors.is_empty()
                    || !debt.vectors.is_empty()
                    || !debt.denied_sources.is_empty()
                    || !debt.denied_identities.is_empty()
                    || state(c)?.phase != "finalizing"))
            || (debt.version == 2
                && (debt.safe_boundary < debt.boundary || debt.safe_boundary > i64::MAX as u64))
            || debt.boundary > i64::MAX as u64
            || debt.generation > i64::MAX as u64
            || !ordered(&debt.cursors)
            || !ordered(&debt.vectors)
            || debt.cursors.iter().any(|id| *id as u64 > debt.boundary)
            || debt.closures.iter().any(|id| ClosureLookupV1::new(id).is_err())
        {
            return Err(EngineError::Storage);
        }
        if !matches!(state(c)?.phase.as_str(), "cleaning" | "finalizing") {
            return Err(EngineError::Storage);
        }
    }
    Ok(value)
}

pub(crate) fn validate_state(c: &Connection) -> Result<(), EngineError> {
    state(c)?;
    root(c)?;
    Ok(())
}

pub(crate) fn active(c: &Connection) -> Result<bool, EngineError> {
    if root(c)?.is_some() || matches!(state(c)?.phase.as_str(), "cleaning" | "finalizing") {
        return Ok(true);
    }
    let has_closures: bool = c
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name='_fathomdb_dependency_closures')",
            [],
            |r| r.get(0),
        )
        .map_err(|_| EngineError::Storage)?;
    if !has_closures {
        return Ok(false);
    }
    c.query_row("SELECT EXISTS(SELECT 1 FROM _fathomdb_dependency_closures WHERE phase!='complete' AND cause IN ('purged','source_erased'))", [], |r| r.get(0)).map_err(|_| EngineError::Storage)
}

pub(crate) fn cursor_floor(c: &Connection) -> Result<u64, EngineError> {
    Ok(root(c)?.map_or(0, |debt| debt.boundary))
}

fn fingerprint(verb: &str, argument: &str) -> String {
    use sha2::Digest;
    let mut hash = sha2::Sha256::new();
    hash.update(b"fathomdb.physical-root.v1\0");
    hash.update((verb.len() as u64).to_le_bytes());
    hash.update(verb.as_bytes());
    hash.update(argument.as_bytes());
    crate::identity::hex_encode(&hash.finalize())
}

pub(crate) fn retry(c: &Connection, verb: &str, argument: &str) -> Result<bool, EngineError> {
    Ok(root(c)?.is_some_and(|debt| debt.retry == fingerprint(verb, argument)))
}

pub(crate) fn capture(
    c: &Connection,
    cursors: &[i64],
    boundary: u64,
) -> Result<RootDebt, EngineError> {
    let mut cursors = cursors.to_vec();
    cursors.sort_unstable();
    cursors.dedup();
    let mut vectors = Vec::new();
    for cursor in &cursors {
        vectors.extend(
            vector_storage::vector_ids_for_cursor(c, *cursor).map_err(|_| EngineError::Storage)?,
        );
    }
    vectors.sort_unstable();
    vectors.dedup();
    let mut debt = RootDebt {
        ticket: None,
        version: 2,
        retry: String::new(),
        cursors,
        vectors,
        boundary,
        generation: 0,
        closures: Vec::new(),
        safe_boundary: boundary,
        denied_sources: Vec::new(),
        denied_identities: Vec::new(),
        redacted: false,
        blocker: None,
    };
    if debt.cursors.len() > 16_384 {
        return Err(admission::refused());
    }
    admission::capture_denial(c, &mut debt)?;
    Ok(debt)
}

pub(crate) fn admit(
    c: &Connection,
    mut debt: RootDebt,
    verb: &str,
    argument: &str,
    ids: &[ClosureOperationId],
) -> Result<(), EngineError> {
    debt.ticket = crate::erasure_queue::activate_in(c, verb, argument)?;
    debt.retry = fingerprint(verb, argument);
    if verb == "purge" {
        debt.denied_identities.push(admission::deny_hash("identity", &format!("l:{argument}")));
    } else {
        debt.denied_sources.push(admission::deny_hash("source", argument));
    }
    admission::set_prospective_policy(c, true)?;
    debt.generation = load_dependency_generation(c)?;
    debt.closures = ids.iter().map(|id| id.as_str().to_owned()).collect();
    validate_zero(c, &debt)?;
    store(c, ROOT_KEY, &debt)?;
    store(
        c,
        FTS_KEY,
        &Sanitation { version: 1, phase: "cleaning".into(), family: 0, started: false },
    )
}

fn validate_zero(c: &Connection, debt: &RootDebt) -> Result<(), EngineError> {
    let _timer = physical_metrics::stage("zero_proof");
    let boundary_timer = physical_metrics::stage("zero_boundary");
    let boundary: u64 = c.query_row("SELECT COALESCE(MAX(write_cursor),0) FROM (SELECT write_cursor FROM canonical_nodes UNION ALL SELECT write_cursor FROM canonical_edges UNION ALL SELECT write_cursor FROM operational_mutations UNION ALL SELECT write_cursor FROM operational_state)", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
    drop(boundary_timer);
    if boundary > if debt.version == 2 { debt.safe_boundary } else { debt.boundary }
        || load_dependency_generation(c)? != debt.generation
    {
        return Err(EngineError::Storage);
    }
    let cursor_timer = physical_metrics::stage("zero_registered_cursors");
    let cursors = serde_json::to_string(&debt.cursors).map_err(|_| EngineError::Storage)?;
    for (table, column) in [
        ("canonical_nodes", "write_cursor"),
        ("canonical_edges", "write_cursor"),
        ("_fathomdb_artifact_revisions", "write_cursor"),
    ]
    .into_iter()
    .chain(
        ROW_OWNED_PROJECTIONS
            .iter()
            .filter(|p| p.table != DEFAULT_VECTOR_PARTITION)
            .map(|p| (p.table, p.cursor_column)),
    ) {
        let count: i64 = c.query_row(&format!("SELECT COUNT(*) FROM {table} WHERE {column} IN (SELECT value FROM json_each(?1))"), [&cursors], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
        if count != 0 {
            return Err(EngineError::Storage);
        }
    }
    drop(cursor_timer);
    let _vectors = physical_metrics::stage("zero_mapped_vectors");
    if !debt.vectors.is_empty() {
        let mut probe = c
            .prepare_cached("SELECT EXISTS(SELECT 1 FROM vector_default WHERE rowid=?1)")
            .map_err(|_| EngineError::Storage)?;
        for id in &debt.vectors {
            let exists: bool =
                probe.query_row([id], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
            if exists {
                return Err(EngineError::Storage);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_pending(
    c: &Connection,
    ids: &[ClosureOperationId],
) -> Result<(), EngineError> {
    if let Some(debt) = root(c)? {
        if debt.closures != ids.iter().map(|id| id.as_str().to_owned()).collect::<Vec<_>>() {
            return Err(EngineError::Storage);
        }
        if !debt.redacted {
            validate_zero(c, &debt)?;
        }
    }
    Ok(())
}

pub(crate) fn transaction<T>(
    c: &Connection,
    operation: impl FnOnce() -> Result<T, EngineError>,
) -> Result<T, EngineError> {
    c.execute_batch("BEGIN IMMEDIATE").map_err(|_| EngineError::Storage)?;
    match operation() {
        Ok(value) => {
            if c.execute_batch("COMMIT").is_err() {
                let _ = c.execute_batch("ROLLBACK");
                return Err(EngineError::Storage);
            }
            Ok(value)
        }
        Err(error) => {
            let _ = c.execute_batch("ROLLBACK");
            Err(error)
        }
    }
}

pub(crate) fn sanitize_unit(c: &Connection) -> Result<bool, EngineError> {
    let done = transaction(c, || {
        let mut progress = state(c)?;
        if matches!(progress.phase.as_str(), "clean" | "finalizing") {
            return Ok(true);
        }
        if progress.phase == "dirty" {
            progress.phase = "cleaning".into();
        }
        let tables = families();
        let table = tables[progress.family];
        let before = c.total_changes();
        let pages = if progress.started { 64 } else { -64 };
        c.execute(&format!("INSERT INTO {table}({table},rank) VALUES('merge',?1)"), [pages])
            .map_err(|_| EngineError::Storage)?;
        let finished = c.total_changes() - before < 2;
        progress.started = !finished;
        if finished {
            progress.family += 1;
        }
        if progress.family == tables.len() {
            progress.phase = "finalizing".into();
        }
        store(c, FTS_KEY, &progress)?;
        faultpoint("before_sanitation_commit")?;
        Ok(progress.phase == "finalizing")
    })?;
    Ok(done)
}

#[cfg(any(test, feature = "operator"))]
pub(crate) fn sanitize(c: &Connection) -> Result<(), EngineError> {
    loop {
        let done = sanitize_unit(c)?;
        faultpoint("after_sanitation_unit")?;
        if done {
            break;
        }
    }
    Ok(())
}

pub(crate) fn pending_ids(c: &Connection) -> Result<Vec<ClosureOperationId>, EngineError> {
    let mut statement = c.prepare("SELECT closure_operation_id FROM _fathomdb_dependency_closures WHERE phase!='complete' AND cause IN ('purged','source_erased') ORDER BY closure_sequence").map_err(|_| EngineError::Storage)?;
    let result = statement
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(|_| EngineError::Storage)?
        .map(|row| {
            ClosureLookupV1::new(row.map_err(|_| EngineError::Storage)?)
                .map(|value| value.closure_operation_id)
                .map_err(|_| EngineError::Storage)
        })
        .collect();
    result
}

pub(crate) fn record_blocker(c: &Connection, error: &EngineError) -> Result<(), EngineError> {
    let stage = match error {
        EngineError::ErasureIncomplete { stage, .. } => stage.as_str(),
        _ => "storage",
    };
    if let Some(mut debt) = root(c)? {
        debt.blocker = Some(stage.into());
        transaction(c, || store(c, ROOT_KEY, &debt))?;
    }
    if ["telemetry_redaction", "wal_checkpoint"].contains(&stage) {
        dependency_closure::mark_physical_incomplete(c, &pending_ids(c)?, stage)?;
    }
    Ok(())
}

pub(crate) fn redacted_ids(c: &Connection) -> Result<Option<Vec<ClosureOperationId>>, EngineError> {
    root(c)?
        .filter(|debt| debt.redacted)
        .map(|debt| {
            debt.closures
                .iter()
                .map(|id| {
                    ClosureLookupV1::new(id)
                        .map(|lookup| lookup.closure_operation_id)
                        .map_err(|_| EngineError::Storage)
                })
                .collect()
        })
        .transpose()
}

pub(crate) fn prepare_redacted(
    c: &Connection,
    ids: &[ClosureOperationId],
) -> Result<(), EngineError> {
    crate::erasure_queue::validate_active(c)?;
    let Some(mut debt) = root(c)? else {
        return Ok(());
    };
    if debt.version != 2 || debt.redacted {
        return Ok(());
    }
    if state(c)?.phase != "finalizing" {
        return Err(EngineError::Storage);
    }
    transaction(c, || {
        dependency_closure::validate_physical_closures(c, ids)?;
        validate_pending(c, ids)?;
        if let Some(ticket) = &debt.ticket {
            crate::erasure_queue::scrub_in(c, ticket)?;
        }
        debt.redacted = true;
        debt.retry.clear();
        debt.cursors.clear();
        debt.vectors.clear();
        debt.denied_sources.clear();
        debt.denied_identities.clear();
        store(c, ROOT_KEY, &debt)?;
        faultpoint("before_redacted_commit")?;
        Ok(())
    })?;
    faultpoint("after_redacted_commit")
}

pub(crate) fn finalize(c: &Connection, ids: &[ClosureOperationId]) -> Result<(), EngineError> {
    crate::erasure_queue::validate_active(c)?;
    let ticket = root(c)?.and_then(|debt| debt.ticket);
    validate_pending(c, ids)?;
    if state(c)?.phase != "finalizing" && state(c)?.phase != "clean" {
        return Err(EngineError::Storage);
    }
    transaction(c, || {
        dependency_closure::complete_physical_closures_in(c, ids)?;
        admission::set_prospective_policy(c, false)?;
        store(
            c,
            FTS_KEY,
            &Sanitation {
                version: 1,
                phase: "clean".into(),
                family: families().len(),
                started: false,
            },
        )?;
        c.execute("DELETE FROM _fathomdb_open_state WHERE key=?1", [ROOT_KEY])
            .map_err(|_| EngineError::Storage)?;
        if let Some(ticket) = &ticket {
            crate::erasure_queue::complete_in(c, ticket)?;
        }
        faultpoint("before_completion_commit")?;
        Ok(())
    })?;
    faultpoint("after_completion_commit")
}

#[cfg(feature = "operator")]
pub(crate) fn diagnostics(c: &Connection) -> Result<Vec<PendingErasure>, EngineError> {
    let mut rows = Vec::new();
    if let Some(debt) = root(c)? {
        rows.push(PendingErasure {
            closure_id: "root-erasure".into(),
            cause: "physical_erasure".into(),
            phase: "at_rest_pending".into(),
            blocker: debt.blocker,
            sequence: 0,
        });
    }
    let progress = state(c)?;
    if progress.phase != "clean" {
        rows.push(PendingErasure {
            closure_id: "retained-terms".into(),
            cause: "retained_terms".into(),
            phase: progress.phase,
            blocker: None,
            sequence: 0,
        });
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "operator")]
    #[test]
    fn only_known_fresh_stores_start_clean_missing_existing_metadata_stays_legacy_dirty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fresh.sqlite");
        let engine = Engine::open(&path).unwrap().engine;
        assert!(
            engine.pending_erasures().unwrap().is_empty(),
            "a provably fresh empty store has no retained-term history"
        );
        engine.close().unwrap();
        let engine = Engine::open(&path).unwrap().engine;
        assert!(engine.pending_erasures().unwrap().is_empty());
        engine
            .execute_for_test(
                "DELETE FROM _fathomdb_open_state WHERE key='_fathomdb_fts_sanitation'",
            )
            .unwrap();
        engine.close().unwrap();
        let engine = Engine::open(&path).unwrap().engine;
        assert!(
            engine.pending_erasures().unwrap().iter().any(|r| r.cause == "retained_terms"),
            "empty canonical tables cannot certify unknown existing history"
        );
        engine.close().unwrap();
    }

    #[test]
    fn failed_commit_rolls_back_and_leaves_writer_retryable() {
        crate::configure_runtime(crate::RuntimeSqliteMode::Performance).unwrap();
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE parent(id INTEGER PRIMARY KEY); CREATE TABLE child(id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);").unwrap();
        assert!(transaction(&c, || {
            c.execute("INSERT INTO child VALUES(1)", []).map_err(|_| EngineError::Storage)?;
            Ok(())
        })
        .is_err());
        assert!(c.is_autocommit(), "failed COMMIT must roll back its deferred transaction");
        transaction(&c, || Ok(())).unwrap();
    }

    #[test]
    fn mapped_vector_proof_preparation_work_does_not_scale_with_scope_size() {
        use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
        let mut c = vector_storage::slice70_mapping_fixture(false);
        let mut debt = capture(&c, &[10], 10).unwrap();
        erase_row_projections(&c, &[10_i64][..]).unwrap();
        debt.vectors.extend(1000..2000);
        admit(&c, debt, "purge", "subject", &[]).unwrap();
        let debt = root(&c).unwrap().unwrap();
        let prepared_reads = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&prepared_reads);
        c.authorizer(Some(move |context: AuthContext<'_>| {
            if matches!(context.action, AuthAction::Read { table_name: "vector_default", .. }) {
                observed.fetch_add(1, Ordering::Relaxed);
            }
            Authorization::Allow
        }))
        .unwrap();
        validate_zero(&c, &debt).unwrap();
        assert!(prepared_reads.load(Ordering::Relaxed) <= 4,
            "mapped absence proof must reuse bounded SQL preparation work for every captured ID; observed {} prepared reads",
            prepared_reads.load(Ordering::Relaxed));
    }

    #[test]
    fn root_replay_proves_every_mapped_vector_id_not_only_the_owner_cursor() {
        let c = vector_storage::slice70_mapping_fixture(false);
        let debt = capture(&c, &[10], 10).unwrap();
        assert_eq!(debt.vectors, vec![101, 102]);
        erase_row_projections(&c, &[10_i64][..]).unwrap();
        admit(&c, debt, "purge", "subject", &[]).unwrap();
        validate_pending(&c, &[]).unwrap();
        let blob = vector_storage::encode_vector_blob(&[1.; 8]);
        c.execute("INSERT INTO vector_default(rowid,embedding,embedding_bin,source_type,kind,created_at,status,lifecycle,valid_from,valid_until) VALUES(102,?1,vec_quantize_binary(?1),'article','doc',0,'',0,-9223372036854775808,9223372036854775807)", [blob]).unwrap();
        assert!(
            validate_pending(&c, &[]).is_err(),
            "late non-head vector residue cannot satisfy a replay proof"
        );
        c.execute("DELETE FROM vector_default WHERE rowid=102", []).unwrap();
        sanitize(&c).unwrap();
        finalize(&c, &[]).unwrap();
        assert!(root(&c).unwrap().is_none());
    }

    #[test]
    fn deferred_graft_commit_failure_rolls_back_for_live_engine_retry() {
        let dir = tempfile::tempdir().unwrap();
        let engine = Engine::open(dir.path().join("graft-commit.sqlite")).unwrap().engine;
        engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "summary".into(),
                    roles: BTreeSet::from([ProjectionRole::Searchable]),
                    fts: None,
                    vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                    source: None,
                }],
                &[],
            )
            .unwrap();
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "{\"summary\":\"survivor\"}".into(),
                source_id: SourceId::new("other").unwrap(),
                logical_id: Some("survivor".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        let guard = engine.connection.lock().unwrap();
        let c = guard.as_ref().unwrap();
        c.execute_batch("PRAGMA foreign_keys=ON; CREATE TEMP TABLE parent(id INTEGER PRIMARY KEY); CREATE TEMP TABLE child(id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED); CREATE TEMP TRIGGER graft_fault AFTER INSERT ON main._fathomdb_vector_kinds BEGIN INSERT INTO child VALUES(1); END;").unwrap();
        assert!(boot_graft_declared_vector_backfill(c).is_err());
        assert!(c.is_autocommit(), "failed graft COMMIT must leave a live Engine retryable");
        c.execute_batch("DROP TRIGGER graft_fault").unwrap();
        boot_graft_declared_vector_backfill(c).unwrap();
        drop(guard);
        engine.close().unwrap();
    }

    #[test]
    fn retry_sequences_preserve_atomic_progress_and_survivors() {
        let mut runner =
            proptest::test_runner::TestRunner::new(proptest::test_runner::Config::with_cases(24));
        runner
            .run(&proptest::collection::vec(proptest::bool::ANY, 1..8), |failures| {
                let dir = tempfile::tempdir().unwrap();
                let engine = Engine::open(dir.path().join("sequence.sqlite")).unwrap().engine;
                let make_node = |id: &str, source: &str| PreparedWrite::Node {
                    kind: "doc".into(),
                    body: format!("unique {id}"),
                    source_id: SourceId::new(source).unwrap(),
                    logical_id: Some(id.into()),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                };
                engine
                    .write(&[make_node("subject", "target"), make_node("survivor", "other")])
                    .unwrap();
                for fail in failures {
                    if fail {
                        crate::test_hooks::physical_erasure_hooks::arm(
                            "before_sanitation_commit",
                            Box::new(|| Err(EngineError::Storage)),
                        );
                    }
                    let outcome = engine.erase_source("target");
                    proptest::prop_assert_eq!(outcome.is_err(), fail);
                    let guard = engine.connection.lock().unwrap();
                    let c = guard.as_ref().unwrap();
                    proptest::prop_assert_eq!(root(c)?.is_some(), fail);
                    if fail {
                        proptest::prop_assert_eq!(state(c)?.family, 0);
                    }
                    let rows: i64 = c
                        .query_row(
                            "SELECT count(*) FROM canonical_nodes WHERE logical_id='survivor'",
                            [],
                            |r| r.get(0),
                        )
                        .unwrap();
                    proptest::prop_assert_eq!(rows, 1);
                }
                engine.erase_source("target").unwrap();
                engine.close().unwrap();
                Ok(())
            })
            .unwrap();
    }
}
