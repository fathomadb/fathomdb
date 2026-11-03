use super::*;
use crate::vector_storage::DEFAULT_VECTOR_PARTITION;
use serde::{Deserialize, Serialize};

const ROOT_KEY: &str = "_fathomdb_physical_erasure";
const FTS_KEY: &str = "_fathomdb_fts_sanitation";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RootDebt {
    version: u32,
    retry: String,
    cursors: Vec<i64>,
    vectors: Vec<i64>,
    boundary: u64,
    generation: u64,
    closures: Vec<String>,
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

pub(crate) fn mark_dirty(c: &Connection) -> rusqlite::Result<()> {
    // Absence already means legacy dirty; only an existing clean certificate changes.
    c.execute("UPDATE _fathomdb_open_state SET value='{\"version\":1,\"phase\":\"dirty\",\"family\":0,\"started\":false}' WHERE key=?1 AND json_extract(value,'$.phase')='clean'", [FTS_KEY])?;
    Ok(())
}

pub(crate) fn root(c: &Connection) -> Result<Option<RootDebt>, EngineError> {
    let value = load::<RootDebt>(c, ROOT_KEY)?;
    if let Some(debt) = &value {
        let ordered =
            |ids: &[i64]| ids.iter().all(|id| *id > 0) && ids.windows(2).all(|p| p[0] < p[1]);
        if debt.version != 1
            || debt.retry.len() != 64
            || !debt.retry.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
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
    Ok(root(c)?.is_some() || matches!(state(c)?.phase.as_str(), "cleaning" | "finalizing"))
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
    Ok(RootDebt {
        version: 1,
        retry: String::new(),
        cursors,
        vectors,
        boundary,
        generation: 0,
        closures: Vec::new(),
    })
}

pub(crate) fn admit(
    c: &Connection,
    mut debt: RootDebt,
    verb: &str,
    argument: &str,
    ids: &[ClosureOperationId],
) -> Result<(), EngineError> {
    debt.retry = fingerprint(verb, argument);
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
    let boundary: u64 = c.query_row("SELECT COALESCE(MAX(write_cursor),0) FROM (SELECT write_cursor FROM canonical_nodes UNION ALL SELECT write_cursor FROM canonical_edges UNION ALL SELECT write_cursor FROM operational_mutations UNION ALL SELECT write_cursor FROM operational_state)", [], |r| r.get(0)).map_err(|_| EngineError::Storage)?;
    if boundary > debt.boundary || load_dependency_generation(c)? != debt.generation {
        return Err(EngineError::Storage);
    }
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
    for id in &debt.vectors {
        let exists: bool = c
            .query_row("SELECT EXISTS(SELECT 1 FROM vector_default WHERE rowid=?1)", [id], |r| {
                r.get(0)
            })
            .map_err(|_| EngineError::Storage)?;
        if exists {
            return Err(EngineError::Storage);
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
        validate_zero(c, &debt)?;
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
    transaction(c, || {
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
        Ok(progress.phase == "finalizing")
    })
}

pub(crate) fn sanitize(c: &Connection) -> Result<(), EngineError> {
    while !sanitize_unit(c)? {}
    Ok(())
}

pub(crate) fn finalize(c: &Connection, ids: &[ClosureOperationId]) -> Result<(), EngineError> {
    validate_pending(c, ids)?;
    if state(c)?.phase != "finalizing" && state(c)?.phase != "clean" {
        return Err(EngineError::Storage);
    }
    transaction(c, || {
        dependency_closure::complete_physical_closures_in(c, ids)?;
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
        Ok(())
    })
}

#[cfg(feature = "operator")]
pub(crate) fn diagnostics(c: &Connection) -> Result<Vec<PendingErasure>, EngineError> {
    let mut rows = Vec::new();
    if root(c)?.is_some() {
        rows.push(PendingErasure {
            closure_id: "root-erasure".into(),
            cause: "physical_erasure".into(),
            phase: "at_rest_pending".into(),
            blocker: None,
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

    #[test]
    fn failed_commit_rolls_back_and_leaves_writer_retryable() {
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
}
