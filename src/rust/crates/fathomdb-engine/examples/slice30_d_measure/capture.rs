//! C capture streams actual records and adjacent samples under one whole-cell clock.
use super::schedule::{self, Mode, OrdinaryCell};
use fathomdb_embedder_api::Embedder;
use fathomdb_engine::{
    Engine, EngineConfig, ProjectionRole, ProjectionScanForTest, ProjectionSpec, ProjectionVector,
    Slice30ProjectionObservationGuard, TestEmbedderChoice,
};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

#[derive(Clone)]
struct Paths {
    records: PathBuf,
    runtime: PathBuf,
    pending: PathBuf,
    wal: PathBuf,
}
impl Paths {
    fn create(output: &Path) -> Result<Self, String> {
        fs::create_dir(output).map_err(|error| format!("create fresh C output: {error:?}"))?;
        Ok(Self {
            records: output.join("observations.jsonl"),
            runtime: output.join("runtime.jsonl"),
            pending: output.join("pending.jsonl"),
            wal: output.join("wal.jsonl"),
        })
    }
}

#[derive(Default)]
struct Counts {
    takes: u64,
    dropped: u64,
    issues: BTreeSet<String>,
    runtime_polls: u64,
    readiness_polls: u64,
    wal_samples: u64,
}

fn write_line(writer: &mut impl Write, value: &impl serde::Serialize) -> Result<(), String> {
    serde_json::to_writer(&mut *writer, value)
        .map_err(|error| format!("C stream JSON: {error:?}"))?;
    writer.write_all(b"\n").map_err(|error| format!("C stream newline: {error:?}"))
}

fn flush(writer: &mut impl Write) -> Result<(), String> {
    writer.flush().map_err(|error| format!("C stream flush: {error:?}"))
}

fn take_records(
    observer: &Slice30ProjectionObservationGuard,
    identity: &Value,
    writer: &mut impl Write,
    counts: &mut Counts,
) -> Result<(), String> {
    let batch = observer.take();
    counts.takes += 1;
    counts.dropped = counts.dropped.max(batch.dropped_records);
    counts.issues.extend(batch.issues.iter().map(|issue| (*issue).to_owned()));
    for record in batch.records {
        write_line(
            writer,
            &json!({"cell":identity,"take_sequence":batch.session_sequence,"record":record}),
        )?;
    }
    flush(writer)
}

fn runtime_sample(
    engine: &Engine,
    observer: &Slice30ProjectionObservationGuard,
    cell: &OrdinaryCell<'_>,
    identity: &Value,
    writer: &mut impl Write,
) -> Result<(), String> {
    let (engine_id, session_id) = observer.owner_identity();
    let started = schedule::elapsed(cell.origin);
    let state = engine.slice30_projection_snapshot_for_test().map_err(|error| format!("{error:?}"));
    let ended = schedule::elapsed(cell.origin);
    match state {
        Ok(state) => write_line(
            writer,
            &json!({"cell":identity,"engine_id":engine_id,
            "session_id":session_id,"started_ns":started,"ended_ns":ended,"state":state}),
        ),
        Err(error) => {
            write_line(
                writer,
                &json!({"cell":identity,"engine_id":engine_id,"session_id":session_id,
                "started_ns":started,"ended_ns":ended,"state":null,"error":error}),
            )?;
            Err(format!("C numeric state unavailable: {error}"))
        }
    }
}

fn pending_sample(
    engine: &Engine,
    observer: &Slice30ProjectionObservationGuard,
    cell: &OrdinaryCell<'_>,
    identity: &Value,
    writer: &mut impl Write,
) -> Result<(), String> {
    let (engine_id, session_id) = observer.owner_identity();
    let started = schedule::elapsed(cell.origin);
    let ready = engine.read_embedding_readiness().map_err(|error| format!("{error:?}"));
    let ended = schedule::elapsed(cell.origin);
    match ready {
        Ok(ready) => write_line(
            writer,
            &json!({"cell":identity,"engine_id":engine_id,"session_id":session_id,
            "started_ns":started,"ended_ns":ended,"pending_count":ready.pending_count,
            "readiness":ready.state.as_str(),"usable_embedder":ready.usable_embedder,
            "scope":"own adjacent durable readiness transaction"}),
        ),
        Err(error) => {
            write_line(
                writer,
                &json!({"cell":identity,"engine_id":engine_id,"session_id":session_id,
                "started_ns":started,"ended_ns":ended,"pending_count":null,"error":error}),
            )?;
            Err(format!("C durable readiness unavailable: {error}"))
        }
    }
}

fn wal_sample(
    observer: &Slice30ProjectionObservationGuard,
    cell: &OrdinaryCell<'_>,
    identity: &Value,
    writer: &mut impl Write,
) -> Result<(), String> {
    let (engine_id, session_id) = observer.owner_identity();
    let started = schedule::elapsed(cell.origin);
    let mut wal_name = cell.path.as_os_str().to_owned();
    wal_name.push("-wal");
    let db = fs::metadata(cell.path).map_err(|error| format!("C own DB stat: {error:?}"))?;
    let (wal_exists, wal_bytes) = match fs::metadata(PathBuf::from(wal_name)) {
        Ok(metadata) => (true, metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => (false, 0),
        Err(error) => return Err(format!("C own WAL stat unavailable: {error:?}")),
    };
    write_line(
        writer,
        &json!({"cell":identity,"engine_id":engine_id,"session_id":session_id,
        "started_ns":started,"ended_ns":schedule::elapsed(cell.origin),"db_bytes":db.len(),
        "wal_exists":wal_exists,"wal_bytes":wal_bytes,"frame_count":null,"checkpoint_status":null,
        "scope":"own DB/WAL file stat only; no checkpoint or frame-count inference"}),
    )
}

fn sample_streams(
    engine: &Engine,
    observer: &Slice30ProjectionObservationGuard,
    cell: &OrdinaryCell<'_>,
    identity: &Value,
    paths: &Paths,
    ready: mpsc::SyncSender<Result<(), String>>,
    stop: mpsc::Receiver<()>,
) -> Result<Counts, String> {
    let run = || -> Result<Counts, String> {
        let make = |path| {
            File::create(path)
                .map(BufWriter::new)
                .map_err(|error| format!("open fresh C stream: {error:?}"))
        };
        let mut records = make(&paths.records)?;
        let mut runtime = make(&paths.runtime)?;
        let mut pending = make(&paths.pending)?;
        let mut wal = make(&paths.wal)?;
        let mut counts = Counts::default();
        take_records(observer, identity, &mut records, &mut counts)?;
        runtime_sample(engine, observer, cell, identity, &mut runtime)?;
        pending_sample(engine, observer, cell, identity, &mut pending)?;
        wal_sample(observer, cell, identity, &mut wal)?;
        counts.wal_samples += 1;
        flush(&mut runtime)?;
        flush(&mut pending)?;
        flush(&mut wal)?;
        ready.send(Ok(())).map_err(|error| format!("C startup acknowledgement: {error:?}"))?;
        let mut fast = Instant::now() + Duration::from_millis(100);
        let mut slow = Instant::now() + Duration::from_secs(1);
        loop {
            match stop.recv_timeout(fast.saturating_duration_since(Instant::now())) {
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
            schedule::work_remaining(cell)?;
            take_records(observer, identity, &mut records, &mut counts)?;
            runtime_sample(engine, observer, cell, identity, &mut runtime)?;
            counts.runtime_polls += 1;
            fast += Duration::from_millis(100);
            if Instant::now() >= slow {
                pending_sample(engine, observer, cell, identity, &mut pending)?;
                wal_sample(observer, cell, identity, &mut wal)?;
                counts.readiness_polls += 1;
                counts.wal_samples += 1;
                slow += Duration::from_secs(1);
            }
            flush(&mut runtime)?;
            flush(&mut pending)?;
            flush(&mut wal)?;
        }
        take_records(observer, identity, &mut records, &mut counts)?;
        flush(&mut runtime)?;
        flush(&mut pending)?;
        flush(&mut wal)?;
        Ok(counts)
    };
    let result = run();
    if let Err(error) = &result {
        let _ = ready.try_send(Err(error.clone()));
    }
    result
}

struct StopOnDrop(mpsc::SyncSender<()>);
impl Drop for StopOnDrop {
    fn drop(&mut self) {
        let _ = self.0.try_send(());
    }
}

fn held_scan_probes(
    engine: &Engine,
    cell: &OrdinaryCell<'_>,
    through: u64,
) -> Result<Value, String> {
    let effective_at = i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("C epoch clock: {error:?}"))?
            .as_secs(),
    )
    .map_err(|error| format!("C epoch range: {error:?}"))?;
    let mut probes = Vec::new();
    for (name, scan) in [
        ("scheduler", ProjectionScanForTest::Scheduler),
        ("candidate_page", ProjectionScanForTest::CandidatePage),
    ] {
        schedule::work_remaining(cell)?;
        let started = schedule::elapsed(cell.origin);
        let page = engine
            .projection_scan_page_for_test(scan, 0, Some(through), effective_at)
            .map_err(|error| format!("{error:?}"))?;
        probes.push(json!({"scan":name,"effective_at_epoch_s":effective_at,"started_ns":started,
            "ended_ns":schedule::elapsed(cell.origin),"rows":page.rows,"vm_steps":page.vm_steps,
            "plan":page.plan,"sql":page.sql,
            "scope":"held diagnostic statement; not a background dispatcher interval"}));
    }
    Ok(json!(probes))
}

fn held_work(
    engine: &Engine,
    observer: &Slice30ProjectionObservationGuard,
    cell: &OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String>,
    frozen: schedule::Frozen<'_>,
    runtime: Value,
    identity: &Value,
) -> Result<Value, String> {
    let (batches, probes) = schedule::seed_held(engine, cell, bodies)?;
    let held = schedule::boundary(engine, cell)?;
    let n = batches.iter().map(|batch| batch["rows"].as_u64().unwrap()).sum::<u64>();
    if n == 0
        || identity["n"].as_u64() != Some(n)
        || held["pending_count"] != n
        || held["physical_rows"] != 0
    {
        return Err("C held boundary lacks exact natural pending rows and zero vectors".into());
    }
    let scan_probes =
        held_scan_probes(engine, cell, batches.last().unwrap()["cursor"].as_u64().unwrap())?;
    drop(frozen);
    let released = schedule::elapsed(cell.origin);
    let drain_started = schedule::elapsed(cell.origin);
    engine
        .drain(u64::try_from(schedule::work_remaining(cell)?.as_millis()).unwrap())
        .map_err(|error| format!("{error:?}"))?;
    let drain_ended = schedule::elapsed(cell.origin);
    let mut calls = Vec::with_capacity(1000);
    for (index, query) in super::workload::natural_queries(cell.queries).enumerate() {
        let operation = u64::try_from(index + 1).unwrap();
        let mut call = observer
            .with_operation(operation, || schedule::natural_call(engine, cell, index, query))?;
        call["operation_id"] = json!(operation);
        calls.push(call);
    }
    let terminal = schedule::boundary(engine, cell)?;
    if terminal["pending_count"] != 0
        || terminal["failed_rows"] != 0
        || terminal["generation_failed_count"] != 0
        || ["natural_rows", "physical_rows", "vector_rows", "span_rows", "terminal_rows"]
            .iter()
            .any(|field| terminal[*field] != n)
    {
        return Err("C drain did not establish exact physical/terminal closure".into());
    }
    Ok(json!({"complete":true,"cell":identity,"calls":calls,"batches":batches,"probes":probes,
        "held":held,"held_scan_probes":scan_probes,"final":terminal,"release_ns":released,
        "drain":{"started_ns":drain_started,"ended_ns":drain_ended,"calls_after_seed_started":1},
        "writer":null,"reader":null,"runtime":runtime,"collection_enabled":true,"actual_overlap":null}))
}

pub(crate) fn run_characterized(
    cell: OrdinaryCell<'_>,
    bodies: impl Iterator<Item = String> + Send,
    provider: Arc<dyn Embedder>,
    identity: &Value,
    output: &Path,
) -> Result<Value, String> {
    schedule::work_remaining(&cell)?;
    if cell.path.exists() || cell.queries.len() != 500 || identity["layer"] != "C" {
        return Err(
            "C cell requires a fresh store, fixed500 queries and characterization identity".into(),
        );
    }
    let paths = Paths::create(output)?;
    let engine = Engine::open_with_test_choice_and_config_for_test(
        cell.path,
        TestEmbedderChoice::Caller(provider),
        EngineConfig::default(),
    )
    .map_err(|error| format!("{error:?}"))?
    .engine;
    let mut armed = None;
    let mut counts = None;
    let mut joined_ns = None;
    let work = (|| {
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
        let frozen = matches!(cell.mode, Mode::Held).then(|| schedule::Frozen::new(&engine));
        engine
            .drain(u64::try_from(schedule::work_remaining(&cell)?.as_millis()).unwrap())
            .map_err(|error| format!("{error:?}"))?;
        let runtime = schedule::runtime_boundary(&engine)?;
        if matches!(cell.mode, Mode::Overlap)
            && (runtime["frozen"] != false || runtime["pending_scan"] != false)
        {
            return Err("C-O entry cannot establish an unfrozen empty runtime".into());
        }
        let observer = Arc::new(
            engine
                .begin_slice30_projection_observation_for_test(32768, cell.origin)
                .map_err(|error| format!("{error:?}"))?,
        );
        armed = Some(Arc::clone(&observer));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (result, sampled) = std::thread::scope(|scope| {
            let helper = scope.spawn(|| {
                sample_streams(&engine, &observer, &cell, identity, &paths, ready_tx, stop_rx)
            });
            let stop = StopOnDrop(stop_tx);
            let result = (|| {
                ready_rx
                    .recv_timeout(schedule::work_remaining(&cell)?)
                    .map_err(|error| format!("C startup wait: {error:?}"))??;
                let mut report = match cell.mode {
                    Mode::Held => held_work(
                        &engine,
                        &observer,
                        &cell,
                        bodies,
                        frozen.expect("held mode established scheduler custody"),
                        runtime,
                        identity,
                    )?,
                    Mode::Overlap => {
                        schedule::overlap_work(&engine, &cell, bodies, runtime, Some(&observer))?
                    }
                };
                report["cell"] = identity.clone();
                report["collection_enabled"] = json!(true);
                report["actual_overlap"] = Value::Null;
                Ok(report)
            })();
            drop(stop);
            (result, helper.join())
        });
        joined_ns = Some(schedule::elapsed(cell.origin));
        let sampled =
            sampled.map_err(|_| "C stream helper panicked after bounded custody".to_string())?;
        match sampled {
            Ok(sampled) => counts = Some(sampled),
            Err(error) => return Err(format!("{error}; C work result: {result:?}")),
        }
        result
    })();
    let close = engine.close().map_err(|error| format!("{error:?}"));
    let closed_ns = schedule::elapsed(cell.origin);
    let mut final_take = Ok(());
    if let Some(observer) = &armed {
        final_take = (|| {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&paths.records)
                .map_err(|error| format!("C final stream open: {error:?}"))?;
            let mut writer = BufWriter::new(file);
            let mut unavailable = Counts::default();
            take_records(
                observer,
                identity,
                &mut writer,
                counts.as_mut().unwrap_or(&mut unavailable),
            )
        })();
    }
    let mut report = match (work, close, final_take) {
        (Ok(report), Ok(()), Ok(())) => report,
        (work, close, final_take) => {
            return Err(format!(
                "C work={work:?}; normal close={close:?}; final take={final_take:?}"
            ))
        }
    };
    let observer = armed.as_ref().expect("successful C work armed its observer");
    let (engine_id, session_id) = observer.owner_identity();
    let counts = counts.expect("successful C work joined its helper");
    report["close_succeeded"] = json!(true);
    report["closed_ns"] = json!(closed_ns);
    report["periodic_numeric_polls"] = json!(counts.runtime_polls);
    report["periodic_readiness_polls"] = json!(counts.readiness_polls);
    report["observation"] = json!({"engine_id":engine_id,"session_id":session_id,
        "records_path":paths.records,"runtime_path":paths.runtime,"pending_path":paths.pending,"wal_path":paths.wal,
        "take_count":counts.takes,"dropped_records":counts.dropped,"issues":counts.issues,
        "runtime_samples":counts.runtime_polls+1,"readiness_samples":counts.readiness_polls+1,
        "wal_samples":counts.wal_samples,"helpers_joined_ns":joined_ns,"final_take_after_close":true});
    if counts.dropped != 0 || !counts.issues.is_empty() {
        report["complete"] = json!(false);
        report["reason"] = "C bounded observation lost required data".into();
    }
    if Instant::now() >= cell.deadline {
        return Err("C whole-cell deadline reached including streams and normal close".into());
    }
    Ok(report)
}
