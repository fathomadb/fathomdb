//! Ordinary D scheduling seam; observations and scientific admission are separate.
use fathomdb_embedder_api::Embedder;
use fathomdb_engine::{
    Engine, EngineConfig, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, SourceId, TestEmbedderChoice,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

const CLOSE_RESERVE: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
pub(crate) enum Mode {
    Held,
    Overlap,
}

pub(crate) struct OrdinaryCell<'a> {
    pub(crate) path: &'a Path,
    pub(crate) mode: Mode,
    pub(crate) queries: &'a [String],
    pub(crate) origin: Instant,
    pub(crate) deadline: Instant,
}

fn elapsed(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_nanos()).expect("D cell monotonic time fits u64")
}

fn work_remaining(cell: &OrdinaryCell<'_>) -> Result<Duration, String> {
    cell.deadline
        .checked_sub(CLOSE_RESERVE)
        .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| "D whole-cell allowance has no work time before its close reserve".into())
}

struct Frozen<'a>(&'a Engine);
impl<'a> Frozen<'a> {
    fn new(engine: &'a Engine) -> Self {
        engine.set_projection_scheduler_frozen_for_test(true);
        Self(engine)
    }
}
impl Drop for Frozen<'_> {
    fn drop(&mut self) {
        self.0.set_projection_scheduler_frozen_for_test(false);
    }
}

fn runtime_boundary(engine: &Engine) -> Result<Value, String> {
    let state =
        engine.slice30_projection_snapshot_for_test().map_err(|error| format!("{error:?}"))?;
    if state.scheduler_runtime_threads != 2
        || state.embedder_pool_size != 5
        || state.admission_capacity != 128
        || state.active_jobs != 0
        || state.active_scans != 0
        || state.queued_jobs != 0
        || state.in_flight_count != 0
        || state.stopping
    {
        return Err(format!("D fresh runtime is not quiescent at accepted defaults: {state:?}"));
    }
    Ok(json!({"scheduler_workers":state.scheduler_runtime_threads,
        "embed_workers":state.embedder_pool_size,"admission_capacity":state.admission_capacity,
        "active_jobs":state.active_jobs,"active_scans":state.active_scans,
        "queued_jobs":state.queued_jobs,"in_flight_count":state.in_flight_count,
        "frozen":state.frozen,"pending_scan":state.pending_scan}))
}

fn boundary(engine: &Engine, cell: &OrdinaryCell<'_>) -> Result<Value, String> {
    work_remaining(cell)?;
    let ready_start = elapsed(cell.origin);
    let ready = engine.read_embedding_readiness().map_err(|error| format!("{error:?}"))?;
    let ready_end = elapsed(cell.origin);
    let generation_start = elapsed(cell.origin);
    let generation =
        engine.read_projection_generation_status().map_err(|error| format!("{error:?}"))?;
    let generation_end = elapsed(cell.origin);
    let count_start = elapsed(cell.origin);
    let connection = Connection::open_with_flags(cell.path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("{error:?}"))?;
    let counts: (u64, u64, u64, u64, u64, u64) = connection.query_row(
        "SELECT (SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND state='active' AND superseded_at IS NULL), (SELECT COUNT(*) FROM vector_default), (SELECT COUNT(*) FROM _fathomdb_vector_rows), (SELECT COUNT(*) FROM _fathomdb_vector_spans), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='up_to_date'), (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE state='failed')",
        [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
    ).map_err(|error| format!("{error:?}"))?;
    Ok(json!({"pending_count":ready.pending_count,"readiness":ready.state.as_str(),
        "usable_embedder":ready.usable_embedder,"generation_id":generation.generation_id.as_str(),
        "generation_pending_count":generation.pending_count,"generation_failed_count":generation.failed_count,
        "generation_readiness":generation.readiness.as_str(),"natural_rows":counts.0,
        "physical_rows":counts.1,"vector_rows":counts.2,"span_rows":counts.3,
        "terminal_rows":counts.4,"failed_rows":counts.5,
        "readiness_bounds_ns":[ready_start,ready_end],"generation_bounds_ns":[generation_start,generation_end],
        "count_bounds_ns":[count_start,elapsed(cell.origin)],
        "scope":"adjacent public readiness/generation and separate physical count transactions"}))
}

fn freshness(cell: &OrdinaryCell<'_>, cursor: u64, body: &str) -> Result<Value, String> {
    work_remaining(cell)?;
    let connection = Connection::open_with_flags(cell.path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|error| format!("{error:?}"))?;
    let started = elapsed(cell.origin);
    let actual: Option<String> = connection
        .query_row(
            "SELECT body FROM canonical_nodes WHERE write_cursor=?1 AND superseded_at IS NULL",
            [cursor],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("{error:?}"))?;
    let ended = elapsed(cell.origin);
    let fts_started = elapsed(cell.origin);
    let expression = fathomdb_query::compile_text_query(body).match_expression;
    let member: bool = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM search_index WHERE search_index MATCH ?1 AND rowid=?2)",
            rusqlite::params![expression, cursor],
            |row| row.get(0),
        )
        .map_err(|error| format!("{error:?}"))?;
    let fts_ended = elapsed(cell.origin);
    if actual.as_deref() != Some(body) || !member {
        return Err(format!("D committed cursor {cursor} failed natural point/FTS freshness"));
    }
    Ok(json!({"cursor":cursor,"body":body,
        "point":{"body":actual,"started_ns":started,"ended_ns":ended},
        "fts":{"member":member,"started_ns":fts_started,"ended_ns":fts_ended},
        "scope":"separate cursor-restricted point and current-compiler FTS reads"}))
}

fn seed_held(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    mut bodies: impl Iterator<Item = String>,
) -> Result<(Vec<Value>, Vec<Value>), String> {
    let source = SourceId::new("test:fixture").map_err(|error| format!("{error:?}"))?;
    let (mut batches, mut probes) = (Vec::new(), Vec::new());
    loop {
        work_remaining(cell)?;
        let batch = bodies
            .by_ref()
            .take(1000)
            .map(|body| PreparedWrite::Node {
                kind: "doc".into(),
                body,
                source_id: source.clone(),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect::<Vec<_>>();
        if batch.is_empty() {
            return Ok((batches, probes));
        }
        let started = elapsed(cell.origin);
        let receipt = engine.write(&batch).map_err(|error| format!("{error:?}"))?;
        let ended = elapsed(cell.origin);
        let PreparedWrite::Node { body, .. } = batch.last().expect("nonempty governed batch")
        else {
            unreachable!("D seeding constructs only natural nodes")
        };
        batches.push(json!({"rows":batch.len(),"cursor":receipt.cursor,"started_ns":started,"ended_ns":ended}));
        probes.push(freshness(cell, receipt.cursor, body)?);
    }
}

fn natural_calls(engine: &Engine, cell: &OrdinaryCell<'_>) -> Result<Vec<Value>, String> {
    let mut calls = Vec::with_capacity(1000);
    for (index, query) in super::workload::natural_queries(cell.queries).enumerate() {
        work_remaining(cell)?;
        let started = elapsed(cell.origin);
        let result = engine.search_with_limit(query, 10).map_err(|error| format!("{error:?}"))?;
        let ended = elapsed(cell.origin);
        calls.push(json!({"sample_index":index,"query_index":index % 500,
            "route":"engine_hybrid","limit":10,"ok":true,"started_ns":started,"ended_ns":ended,
            "returned_hits":result.results.len(),"projection_cursor":result.projection_cursor,
            "ids":result.results.iter().map(|hit|hit.id.to_prefixed()).collect::<Vec<_>>(),
            "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations)}));
    }
    Ok(calls)
}

fn held(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String>,
) -> Result<Value, String> {
    let frozen = Frozen::new(engine);
    engine
        .drain(u64::try_from(work_remaining(cell)?.as_millis()).unwrap())
        .map_err(|error| format!("{error:?}"))?;
    let runtime = runtime_boundary(engine)?;
    let (batches, probes) = seed_held(engine, cell, bodies)?;
    let held = boundary(engine, cell)?;
    let n = batches.iter().map(|batch| batch["rows"].as_u64().unwrap()).sum::<u64>();
    if n == 0 || held["pending_count"] != n || held["physical_rows"] != 0 {
        return Err(
            "D held boundary does not contain all natural pending rows and zero vectors".into()
        );
    }
    drop(frozen);
    let released = elapsed(cell.origin);
    let drain_start = elapsed(cell.origin);
    engine
        .drain(u64::try_from(work_remaining(cell)?.as_millis()).unwrap())
        .map_err(|error| format!("{error:?}"))?;
    let drain_end = elapsed(cell.origin);
    let calls = natural_calls(engine, cell)?;
    let terminal = boundary(engine, cell)?;
    if terminal["pending_count"] != 0
        || terminal["failed_rows"] != 0
        || terminal["generation_failed_count"] != 0
        || ["natural_rows", "physical_rows", "vector_rows", "span_rows", "terminal_rows"]
            .iter()
            .any(|field| terminal[*field] != n)
    {
        return Err("D normal drain did not establish exact physical and terminal closure".into());
    }
    Ok(json!({"complete":true,"calls":calls,"batches":batches,"probes":probes,
        "held":held,"final":terminal,"release_ns":released,
        "drain":{"started_ns":drain_start,"ended_ns":drain_end,"calls_after_seed_started":1},
        "writer":null,"reader":null,"runtime":runtime,"collection_enabled":false,
        "periodic_numeric_polls":0,"periodic_readiness_polls":0,
        "actual_overlap":"ACTUAL_OVERLAP_UNOBSERVED"}))
}

pub(crate) fn run_ordinary(
    cell: OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String> + Send,
    provider: Arc<dyn Embedder>,
) -> Result<Value, String> {
    work_remaining(&cell)?;
    if cell.path.exists() || cell.queries.len() != 500 {
        return Err("D ordinary cell requires a fresh store and exactly500 query bodies".into());
    }
    let engine = Engine::open_with_test_choice_and_config_for_test(
        cell.path,
        TestEmbedderChoice::Caller(provider),
        EngineConfig::default(),
    )
    .map_err(|error| format!("{error:?}"))?
    .engine;
    let result = (|| {
        engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "doc".into(),
                    roles: [ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(ProjectionVector::default()),
                    source: None,
                }],
                &[],
            )
            .map_err(|error| format!("{error:?}"))?;
        match cell.mode {
            Mode::Held => held(&engine, &cell, bodies),
            Mode::Overlap => Ok(json!({"complete":false,"state":"not_run",
                "reason":"ordinary overlap schedule is not implemented","calls":[]})),
        }
    })();
    let closed = engine.close().map_err(|error| format!("{error:?}"));
    let mut report = match (result, closed) {
        (Ok(report), Ok(())) => report,
        (Err(error), Ok(())) => return Err(error),
        (Ok(_), Err(error)) => return Err(format!("D normal close failed: {error}")),
        (Err(error), Err(close)) => return Err(format!("{error}; D normal close failed: {close}")),
    };
    report["close_succeeded"] = json!(true);
    report["closed_ns"] = json!(elapsed(cell.origin));
    if Instant::now() >= cell.deadline {
        return Err("D whole-cell deadline reached including normal close".into());
    }
    Ok(report)
}
