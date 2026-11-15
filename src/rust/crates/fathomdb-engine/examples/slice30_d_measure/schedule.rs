//! Ordinary D scheduling seam; observations and scientific admission are separate.
use fathomdb_embedder_api::Embedder;
use fathomdb_engine::{
    Engine, EngineConfig, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec,
    ProjectionVector, SourceId, TestEmbedderChoice,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
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

pub(super) fn elapsed(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_nanos()).expect("D cell monotonic time fits u64")
}

pub(super) fn work_remaining(cell: &OrdinaryCell<'_>) -> Result<Duration, String> {
    cell.deadline
        .checked_sub(CLOSE_RESERVE)
        .and_then(|deadline| deadline.checked_duration_since(Instant::now()))
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| "D whole-cell allowance has no work time before its close reserve".into())
}

pub(super) struct Frozen<'a>(&'a Engine);
impl<'a> Frozen<'a> {
    pub(super) fn new(engine: &'a Engine) -> Self {
        engine.set_projection_scheduler_frozen_for_test(true);
        Self(engine)
    }
}
impl Drop for Frozen<'_> {
    fn drop(&mut self) {
        self.0.set_projection_scheduler_frozen_for_test(false);
    }
}

pub(super) fn runtime_boundary(engine: &Engine) -> Result<Value, String> {
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

pub(super) fn boundary(engine: &Engine, cell: &OrdinaryCell<'_>) -> Result<Value, String> {
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

pub(super) fn seed_held(
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

pub(super) fn natural_call(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    index: usize,
    query: &str,
) -> Result<Value, String> {
    work_remaining(cell)?;
    let started = elapsed(cell.origin);
    let result = engine.search_with_limit(query, 10).map_err(|error| format!("{error:?}"))?;
    let ended = elapsed(cell.origin);
    Ok(json!({"sample_index":index,"query_index":index % 500,
        "route":"engine_hybrid","limit":10,"ok":true,"started_ns":started,"ended_ns":ended,
        "returned_hits":result.results.len(),"projection_cursor":result.projection_cursor,
        "ids":result.results.iter().map(|hit|hit.id.to_prefixed()).collect::<Vec<_>>(),
        "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations)}))
}

fn natural_calls(engine: &Engine, cell: &OrdinaryCell<'_>) -> Result<Vec<Value>, String> {
    super::workload::natural_queries(cell.queries)
        .enumerate()
        .map(|(index, query)| natural_call(engine, cell, index, query))
        .collect()
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

struct Committed {
    cursor: u64,
    body: String,
}

struct CancelOnFailure<'a> {
    cancel: &'a AtomicBool,
    succeeded: bool,
}
impl Drop for CancelOnFailure<'_> {
    fn drop(&mut self) {
        if !self.succeeded {
            self.cancel.store(true, Ordering::Release);
        }
    }
}

struct WriterResult {
    batches: Vec<Value>,
    span: Value,
}

struct ReaderResult {
    calls: Vec<Value>,
    probes: Vec<Value>,
    span: Value,
}

fn write_overlap(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    mut bodies: impl Iterator<Item = String>,
    sender: mpsc::SyncSender<Committed>,
    cancel: &AtomicBool,
) -> Result<WriterResult, String> {
    let started = elapsed(cell.origin);
    let source = SourceId::new("test:fixture").map_err(|error| format!("{error:?}"))?;
    let mut batches = Vec::new();
    loop {
        work_remaining(cell)?;
        if cancel.load(Ordering::Acquire) {
            return Err("D reader cancelled writer before the next governed batch".into());
        }
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
            break;
        }
        let write_started = elapsed(cell.origin);
        let receipt = engine.write(&batch).map_err(|error| format!("{error:?}"))?;
        let write_ended = elapsed(cell.origin);
        let PreparedWrite::Node { body, .. } = batch.last().expect("nonempty governed batch")
        else {
            unreachable!("D seeding constructs only natural nodes")
        };
        batches.push(json!({"rows":batch.len(),"cursor":receipt.cursor,
            "started_ns":write_started,"ended_ns":write_ended}));
        let mut notice = Committed { cursor: receipt.cursor, body: body.clone() };
        loop {
            work_remaining(cell)?;
            if cancel.load(Ordering::Acquire) {
                return Err("D reader cancelled bounded commit notification".into());
            }
            match sender.try_send(notice) {
                Ok(()) => break,
                Err(mpsc::TrySendError::Full(returned)) => {
                    notice = returned;
                    // Capacity backpressure only; no experimental delay or provider speed control.
                    std::thread::yield_now();
                }
                Err(mpsc::TrySendError::Disconnected(_)) => {
                    return Err("D reader disconnected before all commit notifications".into());
                }
            }
        }
    }
    Ok(WriterResult {
        batches,
        span: json!({"started_ns":started,"ended_ns":elapsed(cell.origin)}),
    })
}

fn read_overlap(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    receiver: mpsc::Receiver<Committed>,
    cancel: &AtomicBool,
) -> Result<ReaderResult, String> {
    let started = elapsed(cell.origin);
    let first = receiver
        .recv_timeout(work_remaining(cell)?)
        .map_err(|error| format!("D first real commit was unavailable: {error:?}"))?;
    let mut probes = vec![freshness(cell, first.cursor, &first.body)?];
    let mut calls = Vec::with_capacity(1000);
    for (index, query) in super::workload::natural_queries(cell.queries).enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Err("D writer failed before reader completion".into());
        }
        while let Ok(notice) = receiver.try_recv() {
            probes.push(freshness(cell, notice.cursor, &notice.body)?);
        }
        calls.push(natural_call(engine, cell, index, query)?);
    }
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("D writer failed before final commit notification".into());
        }
        match receiver.recv_timeout(work_remaining(cell)?) {
            Ok(notice) => probes.push(freshness(cell, notice.cursor, &notice.body)?),
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err("D commit notification deadline reached".into())
            }
        }
    }
    Ok(ReaderResult {
        calls,
        probes,
        span: json!({"started_ns":started,"ended_ns":elapsed(cell.origin)}),
    })
}

fn overlap(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String> + Send,
) -> Result<Value, String> {
    engine
        .drain(u64::try_from(work_remaining(cell)?.as_millis()).unwrap())
        .map_err(|error| format!("{error:?}"))?;
    let runtime = runtime_boundary(engine)?;
    if runtime["frozen"] != false || runtime["pending_scan"] != false {
        return Err("D overlap entry cannot establish an unfrozen empty runtime".into());
    }
    let cancel = AtomicBool::new(false);
    let (sender, receiver) = mpsc::sync_channel(1);
    let (writer, reader) = std::thread::scope(|scope| {
        let writer = scope.spawn(|| {
            let mut custody = CancelOnFailure { cancel: &cancel, succeeded: false };
            let result = write_overlap(engine, cell, bodies, sender, &cancel);
            custody.succeeded = result.is_ok();
            result
        });
        let reader = scope.spawn(|| {
            let mut custody = CancelOnFailure { cancel: &cancel, succeeded: false };
            let result = read_overlap(engine, cell, receiver, &cancel);
            custody.succeeded = result.is_ok();
            result
        });
        // Join both owners before propagating either error or starting close.
        (writer.join(), reader.join())
    });
    let writer = writer.map_err(|_| "D writer panicked after bounded custody".to_string())??;
    let reader = reader.map_err(|_| "D reader panicked after bounded custody".to_string())??;
    let n = writer.batches.iter().map(|batch| batch["rows"].as_u64().unwrap()).sum::<u64>();
    if n == 0 || writer.batches.len() != reader.probes.len() {
        return Err(
            "D overlap did not retain one natural freshness probe per committed cohort".into()
        );
    }
    let drain_started = elapsed(cell.origin);
    engine
        .drain(u64::try_from(work_remaining(cell)?.as_millis()).unwrap())
        .map_err(|error| format!("{error:?}"))?;
    let drain_ended = elapsed(cell.origin);
    let terminal = boundary(engine, cell)?;
    if terminal["pending_count"] != 0
        || terminal["failed_rows"] != 0
        || terminal["generation_failed_count"] != 0
        || ["natural_rows", "physical_rows", "vector_rows", "span_rows", "terminal_rows"]
            .iter()
            .any(|field| terminal[*field] != n)
    {
        return Err("D overlap drain did not establish exact physical and terminal closure".into());
    }
    Ok(json!({"complete":true,"calls":reader.calls,"batches":writer.batches,"probes":reader.probes,
        "held":null,"release_ns":null,"final":terminal,"writer":writer.span,"reader":reader.span,
        "drain":{"started_ns":drain_started,"ended_ns":drain_ended,"calls_after_seed_started":1},
        "runtime":runtime,"collection_enabled":false,"periodic_numeric_polls":0,
        "periodic_readiness_polls":0,"actual_overlap":"ACTUAL_OVERLAP_UNOBSERVED"}))
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
            Mode::Overlap => overlap(&engine, &cell, bodies),
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
