use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ProjectionFts, ProjectionRole, ProjectionSpec, SourceId,
};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Experiment errors retain diagnostic context; no public API consumes this type.
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;
/// Positive nonstemming physical witness; never persisted in job metadata.
pub const MARKER: &str = "qzxvplmnbvcxzasdfghjklqwertx";
/// Production-registered FTS families, including the coexisting multi-column index.
pub const TABLES: [&str; 4] =
    ["search_index", "search_index_edges", "property_search_index", "search_index_v2"];

struct Fixed;
impl Embedder for Fixed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("erasure-poc", "v1", 8)
    }
    fn embed(&self, _: &str) -> std::result::Result<Vector, EmbedderError> {
        Ok(vec![1.; 8])
    }
}
/// Open a real Engine with an offline deterministic provider.
pub fn open(path: &Path) -> Result<Engine> {
    Ok(Engine::open_with_embedder_for_test(path, Arc::new(Fixed))?.engine)
}
/// A governed canonical node write, also used for unrelated foreground progress.
pub fn node(id: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
/// A real current-format fixture; construction never inserts source data directly.
pub struct Fixture {
    pub engine: Engine,
    pub path: PathBuf,
    pub family: String,
}
impl Fixture {
    /// Populate target and survivor revisions through Engine write and projections.
    pub fn build(
        path: &Path,
        family: &str,
        secure: bool,
        owners: usize,
        history: usize,
    ) -> Result<Self> {
        fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)?;
        if path.exists() {
            return Err("fixture output already exists".into());
        }
        let engine = open(path)?;
        if secure {
            prepare_eager(&engine)?;
        }
        if family == "property" {
            let specs = ["p1", "p2"]
                .into_iter()
                .map(|name| ProjectionSpec {
                    name: name.into(),
                    roles: BTreeSet::from([ProjectionRole::Searchable]),
                    fts: Some(ProjectionFts { tokenizer: None }),
                    vector: None,
                    source: None,
                })
                .collect::<Vec<_>>();
            engine.configure_projections(&specs, &[])?;
        }
        if family == "edge" {
            engine.write(&[
                node("left", "endpoints", "left endpoint"),
                node("right", "endpoints", "right endpoint"),
            ])?;
        }
        for revision in 0..=history {
            let mut writes = Vec::new();
            for i in 0..owners {
                let target = i % 2 == 0;
                let source = if target { "target" } else { "survivor" };
                let word = if target { MARKER } else { "survivorneedle" };
                let body = format!(
                    "{word} common owner{i} revision{revision} {}",
                    "alpha beta gamma delta ".repeat(45)
                );
                let id = format!("owner-{i}");
                writes.push(match family {
                    "node" => node(&id, source, &body),
                    "property" => {
                        node(&id, source, &serde_json::json!({"p1":body,"p2":body}).to_string())
                    }
                    "edge" => PreparedWrite::Edge {
                        kind: "rel".into(),
                        from: "left".into(),
                        to: "right".into(),
                        source_id: SourceId::new(source)?,
                        logical_id: Some(id),
                        body: Some(body),
                        t_valid: None,
                        t_invalid: None,
                        confidence: None,
                        extractor_model_id: None,
                        temporal_fallback: None,
                    },
                    _ => return Err("unknown family".into()),
                });
            }
            engine.write(&writes)?;
            engine.drain(60_000)?;
        }
        Ok(Self { engine, path: path.to_path_buf(), family: family.into() })
    }
    /// Observe real table SQL, logical matches and shadow marker prepresence.
    pub fn inventory(&self) -> Result<Vec<FamilyEvidence>> {
        inventory(&self.path, MARKER)
    }
    /// Retain native erasure semantics and registered cleanup obligations.
    pub fn erase(&self) -> Result<()> {
        self.engine.erase_source("target")?;
        Ok(())
    }
}
/// Observed production family, not an inferred mode or an empty-family pass.
#[derive(Debug, Serialize)]
pub struct FamilyEvidence {
    pub table: String,
    pub ddl: String,
    pub rows: i64,
    pub matches: i64,
    pub shadow_markers: i64,
}
/// Read a committed database snapshot without acquiring an independent writer.
pub fn reader(path: &Path) -> Result<Connection> {
    Ok(Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?)
}
/// Inventory all registered FTS modes using positive physical markers.
pub fn inventory(path: &Path, marker: &str) -> Result<Vec<FamilyEvidence>> {
    let c = reader(path)?;
    TABLES
        .into_iter()
        .map(|table| {
            Ok(FamilyEvidence {
                table: table.into(),
                ddl: c.query_row("SELECT sql FROM sqlite_master WHERE name=?1", [table], |r| {
                    r.get(0)
                })?,
                rows: c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?,
                matches: c.query_row(
                    &format!("SELECT count(*) FROM {table} WHERE {table} MATCH ?1"),
                    [marker],
                    |r| r.get(0),
                )?,
                shadow_markers: c.query_row(
                    &format!(
                        "SELECT count(*) FROM {table}_data WHERE instr(block,CAST(?1 AS BLOB))>0"
                    ),
                    [marker],
                    |r| r.get(0),
                )?,
            })
        })
        .collect()
}
/// Configure A prospectively before any governed source write.
pub fn prepare_eager(engine: &Engine) -> Result<()> {
    engine.maintenance_transaction_for_test(|tx| {
        for table in TABLES {
            tx.execute(
                &format!("INSERT INTO {table}({table},rank) VALUES('secure-delete',1)"),
                [],
            )?;
        }
        Ok(())
    })?;
    Ok(())
}
/// Rebuild A's legacy FTS indexes from their surviving content rows.
pub fn rebuild_eager(engine: &Engine) -> Result<()> {
    engine.maintenance_transaction_for_test(|tx| {
        for table in TABLES {
            tx.execute(&format!("INSERT INTO {table}({table}) VALUES('rebuild')"), [])?;
        }
        Ok(())
    })?;
    Ok(())
}
/// Count literal witnesses over all allocated/freelist bytes and the current WAL.
pub fn physical_markers(path: &Path, marker: &str) -> Result<usize> {
    let mut count = 0;
    for p in [path.to_path_buf(), PathBuf::from(format!("{}-wal", path.display()))] {
        if p.exists() {
            count +=
                std::fs::read(p)?.windows(marker.len()).filter(|w| *w == marker.as_bytes()).count();
        }
    }
    Ok(count)
}
/// Require a completed native TRUNCATE report; Ok(all-busy) is incomplete.
pub fn checkpoint(engine: &Engine) -> Result<bool> {
    Ok(engine.checkpoint_at_rest_for_test()?.last().is_some_and(|(busy, _, _)| !busy))
}

/// Durable sanitation progress. Scope is whole-family maintenance, never native erasure admission.
#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Job {
    pub accepted: i64,
    pub deadline: i64,
    pub family: usize,
    pub started: bool,
    pub steps: usize,
    pub phase: String,
    pub overdue: bool,
}
/// Admit one bounded experimental sanitation job after native erasure returns success.
pub fn admit(engine: &Engine, accepted: i64, deadline: i64) -> Result<()> {
    if deadline <= accepted {
        return Err("deadline must follow acceptance".into());
    }
    engine.maintenance_transaction_for_test(|tx| {
        let pending:i64=tx.query_row("SELECT count(*) FROM _fathomdb_dependency_closures WHERE phase!='complete' AND cause IN ('purged','source_erased')",[],|r|r.get(0))?;
        if pending!=0 { return Err(rusqlite::Error::InvalidQuery); }
        tx.execute_batch("CREATE TABLE IF NOT EXISTS poc_sanitation(singleton INTEGER PRIMARY KEY CHECK(singleton=1),accepted INTEGER NOT NULL,deadline INTEGER NOT NULL,family INTEGER NOT NULL,started INTEGER NOT NULL,steps INTEGER NOT NULL,phase TEXT NOT NULL,overdue INTEGER NOT NULL,last_wall INTEGER NOT NULL,scope TEXT NOT NULL)")?;
        tx.execute("INSERT INTO poc_sanitation VALUES(1,?1,?2,0,0,0,'cleaning',0,?1,'all-four-registered-fts-families')",[accepted,deadline])?;
        Ok(())
    })?;
    Ok(())
}
fn load(c: &Connection) -> rusqlite::Result<Job> {
    c.query_row("SELECT accepted,deadline,family,started,steps,phase,overdue FROM poc_sanitation WHERE singleton=1",[],|r|Ok(Job {accepted:r.get(0)?,deadline:r.get(1)?,family:r.get(2)?,started:r.get(3)?,steps:r.get(4)?,phase:r.get(5)?,overdue:r.get(6)?}))
}
/// Load the durable job on the real writer.
pub fn job(engine: &Engine) -> Result<Job> {
    Ok(engine.maintenance_transaction_for_test(|tx| load(tx))?.0)
}
/// Commit one merge command and its progress; optional failure models rollback before commit.
/// Returned nanoseconds are mutex wait and successful lock occupancy, excluding failed calls.
pub fn step(engine: &Engine, pages: i64, now: i64, fail_after_work: bool) -> Result<(u64, u64)> {
    if !(1..=1024).contains(&pages) {
        return Err("page budget outside experiment bounds".into());
    }
    let (_,wait,held)=engine.maintenance_transaction_for_test(|tx| {
        let job=load(tx)?;
        if job.phase!="cleaning" || job.family>=TABLES.len() { return Err(rusqlite::Error::InvalidQuery); }
        let table=TABLES[job.family];
        let pages=if job.started {pages}else{-pages};
        let before=tx.total_changes();
        tx.execute(&format!("INSERT INTO {table}({table},rank) VALUES('merge',?1)"),[pages])?;
        let changes=tx.total_changes()-before;
        if fail_after_work { return Err(rusqlite::Error::InvalidQuery); }
        let finished=changes<2;
        let next=job.family+usize::from(finished);
        let phase=if next==TABLES.len(){"proof_pending"}else{"cleaning"};
        tx.execute("UPDATE poc_sanitation SET family=?1,started=?2,steps=steps+1,phase=?3,overdue=overdue OR max(last_wall,?4)>deadline,last_wall=max(last_wall,?4) WHERE singleton=1",rusqlite::params![next,!finished,phase,now])?;
        Ok(())
    })?;
    Ok((wait.as_nanos() as u64, held.as_nanos() as u64))
}
/// Finalize only after physical inspection and a non-busy WAL checkpoint.
/// Requires exclusive fixture use during finalization, including no source reingestion.
/// A residual marker is a rejected mechanism, while busy WAL remains proof-pending.
pub fn finalize(engine: &Engine, path: &Path, marker: &str) -> Result<bool> {
    let state = job(engine)?;
    if state.phase != "proof_pending" && state.phase != "residual" {
        return Err("job not ready for physical proof".into());
    }
    if !checkpoint(engine)? {
        return Ok(false);
    }
    let clean = inventory(path, marker)?.iter().all(|r| r.matches == 0 && r.shadow_markers == 0)
        && physical_markers(path, marker)? == 0;
    engine.maintenance_transaction_for_test(|tx| {
        tx.execute(
            "UPDATE poc_sanitation SET phase=?1 WHERE singleton=1",
            [if clean { "complete" } else { "residual" }],
        )?;
        Ok(())
    })?;
    Ok(clean)
}

/// Finalize against an explicit experiment wall clock; retries keep the original deadline.
pub fn finalize_at(engine: &Engine, path: &Path, marker: &str, _now: i64) -> Result<bool> {
    finalize(engine, path, marker)
}
