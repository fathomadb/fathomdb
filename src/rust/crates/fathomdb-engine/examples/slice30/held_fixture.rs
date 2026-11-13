use fathomdb_embedder_api::Embedder;
use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ProjectionRole, ProjectionSpec, ProjectionVector,
    SourceId, TestEmbedderChoice,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::Cell;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use super::template_verify;
use super::varying_embedder::VaryingEmbedder;

struct Hold<'a> {
    engine: &'a Engine,
    active: bool,
    release_requests: &'a Cell<usize>,
}

impl<'a> Hold<'a> {
    fn freeze(engine: &'a Engine, release_requests: &'a Cell<usize>) -> Self {
        engine.set_projection_scheduler_frozen_for_test(true);
        Self { engine, active: true, release_requests }
    }

    fn release(&mut self) {
        if self.active {
            self.engine.set_projection_scheduler_frozen_for_test(false);
            self.release_requests.set(self.release_requests.get() + 1);
            self.active = false;
        }
    }
}

impl Drop for Hold<'_> {
    fn drop(&mut self) {
        self.release();
    }
}

fn release_refused(hold: Hold<'_>, out: &Path) {
    let requests = hold.release_requests;
    drop(hold);
    retain_release_count(out, requests.get());
}

fn retain_release_count(out: &Path, requests: usize) {
    let path = out.join("budget.json");
    let mut report: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    report["release_requests"] = json!(requests);
    std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

fn now(origin: Instant) -> u64 {
    u64::try_from(origin.elapsed().as_nanos()).unwrap()
}

fn timed<T>(origin: Instant, work: impl FnOnce() -> T) -> (T, Value) {
    let start = now(origin);
    let value = work();
    let end = now(origin);
    (value, json!({"start_ns":start,"end_ns":end}))
}

fn readiness(value: fathomdb_engine::EmbeddingReadiness) -> Value {
    json!({"state":value.state.as_str(),"pending_count":value.pending_count,
        "usable_embedder":value.usable_embedder,"affected_kinds":value.affected_kinds,
        "blocked":value.blocked.as_ref().map(|blocked|format!("{blocked:?}"))})
}

fn snapshot(path: &Path) -> Value {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let count = |table: &str| {
        connection
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| row.get::<_, u64>(0))
            .unwrap()
    };
    let canonical: Vec<Value> = connection.prepare("SELECT logical_id,kind,write_cursor,length(CAST(body AS BLOB)) FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY write_cursor").unwrap().query_map([], |row| Ok(json!({"id":row.get::<_,String>(0)?,"kind":row.get::<_,String>(1)?,"write_cursor":row.get::<_,u64>(2)?,"body_bytes":row.get::<_,usize>(3)?}))).unwrap().collect::<Result<_, _>>().unwrap();
    let profile: Value = connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |row| Ok(json!({"name":row.get::<_,String>(0)?,"revision":row.get::<_,String>(1)?,"dimension":row.get::<_,u32>(2)?}))).unwrap();
    json!({"canonical_rows":count("canonical_nodes"),"records":canonical,
        "mapped_vectors":count("_fathomdb_vector_rows"),"physical_vectors":count("vector_default"),
        "span_rows":count("_fathomdb_vector_spans"),"profile":profile})
}

fn observation(
    origin: Instant,
    engine: &Engine,
    path: &Path,
    config: &Value,
    out: &Path,
) -> Result<Value, ()> {
    super::native_budget::phase(config, out, "observation")?;
    let (ready, readiness_time) = timed(origin, || engine.read_embedding_readiness().unwrap());
    let ready = readiness(ready);
    super::native_budget::phase(config, out, "observation")?;
    let (sql, sql_time) = timed(origin, || snapshot(path));
    Ok(json!({"start_ns":readiness_time["start_ns"],"end_ns":sql_time["end_ns"],
        "readiness":ready,"readiness_time":readiness_time,"sql":sql,"sql_time":sql_time}))
}

fn coverage(
    origin: Instant,
    store: &Path,
    config: &Value,
    out: &Path,
) -> Result<(Value, Value), ()> {
    super::native_budget::phase(config, out, "verification")?;
    Ok(timed(origin, || template_verify::coverage(store, &VaryingEmbedder.identity())))
}

fn write_cohort(
    origin: Instant,
    engine: &Engine,
    batch: &[PreparedWrite],
    config: &Value,
    out: &Path,
) -> Result<(fathomdb_engine::WriteReceipt, Value), ()> {
    super::native_budget::phase(config, out, "write")?;
    Ok(timed(origin, || engine.write(batch).unwrap()))
}

pub(super) fn run(config: &Value, out: &Path) {
    let origin = Instant::now();
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "held fixture must be fresh");
    let (opened, opening) = timed(origin, || {
        Engine::open_with_test_choice_for_test(
            &store,
            TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
        )
        .unwrap()
    });
    let engine = &opened.engine;
    if super::native_budget::phase(config, out, "configure").is_err() {
        engine.close().unwrap();
        return;
    }
    let (_, configuring) = timed(origin, || {
        engine
            .configure_projections(
                &[ProjectionSpec {
                    name: "s30_dense".into(),
                    roles: [ProjectionRole::Searchable].into_iter().collect(),
                    fts: None,
                    vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
                    source: None,
                }],
                &[],
            )
            .unwrap()
    });
    engine.begin_d27_observation_for_test(origin);
    let configuration = engine.d27_observation_for_test().unwrap().configuration_observation;
    assert_eq!((configuration.scheduler_runtime_threads, configuration.embedder_pool_size), (2, 5));
    let Ok(empty) = observation(origin, engine, &store, config, out) else {
        engine.close().unwrap();
        return;
    };
    assert_eq!(empty["readiness"]["pending_count"], 0);
    assert_eq!(empty["sql"]["canonical_rows"], 0);
    assert_eq!(empty["sql"]["physical_vectors"], 0);
    let requests = Cell::new(0);
    let (mut hold, freezing) = timed(origin, || Hold::freeze(engine, &requests));
    let (idle_result, empty_idle) =
        timed(origin, || super::native_budget::checked_drain(engine, config, out));
    if idle_result.is_err() {
        release_refused(hold, out);
        engine.close().unwrap();
        return;
    }
    let Ok(idle) = observation(origin, engine, &store, config, out) else {
        release_refused(hold, out);
        engine.close().unwrap();
        return;
    };
    assert_eq!(idle["readiness"]["pending_count"], 0);
    assert_eq!(idle["sql"]["canonical_rows"], 0);
    super::d_workload::enable_observation(engine, config);
    let records = config["records"].as_array().unwrap();
    let cohort = usize::try_from(config["cohort"].as_u64().unwrap()).unwrap();
    assert!(!records.is_empty() && cohort > 0);
    let mut writes = Vec::new();
    let mut raw_writes = std::fs::File::create(out.join("writes.jsonl")).unwrap();
    let mut raw_observations = std::fs::File::create(out.join("observations.jsonl")).unwrap();
    for chunk in records.chunks(cohort) {
        let batch: Vec<_> = chunk
            .iter()
            .map(|row| PreparedWrite::Node {
                kind: row["kind"].as_str().unwrap().into(),
                body: row["body"].as_str().unwrap().into(),
                source_id: SourceId::new("test:s30-held").unwrap(),
                logical_id: Some(row["id"].as_str().unwrap().into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        let Ok((receipt, clock)) = write_cohort(origin, engine, &batch, config, out) else {
            release_refused(hold, out);
            engine.close().unwrap();
            return;
        };
        writes.push(json!({"clock":clock,"rows":batch.len(),"cursor":receipt.cursor,"row_cursors":receipt.row_cursors}));
        super::d_workload::write_row(&mut raw_writes, writes.last().unwrap());
    }
    let Ok(held) = observation(origin, engine, &store, config, out) else {
        release_refused(hold, out);
        engine.close().unwrap();
        return;
    };
    assert_eq!(held["readiness"]["pending_count"], records.len());
    assert_eq!(held["sql"]["physical_vectors"], 0);
    super::d_workload::write_row(
        &mut raw_observations,
        &json!({"phase":"held","observation":held}),
    );
    let after_queries = if config["queries"].as_array().is_some() {
        let mut output = std::fs::File::create(out.join("queries.jsonl")).unwrap();
        let run_phase = |output: &mut std::fs::File, phase: &str, key: &str| {
            let count = usize::try_from(config[key].as_u64().unwrap()).unwrap();
            for sample in 0..count {
                if super::native_budget::phase(config, out, phase).is_err() {
                    return false;
                }
                let row = super::d_workload::query_row(
                    engine,
                    origin,
                    phase,
                    sample,
                    super::d_workload::request(config, sample),
                );
                super::d_workload::write_row(output, &row);
            }
            true
        };
        if !run_phase(&mut output, "warmup", "warmups") {
            release_refused(hold, out);
            return;
        }
        let complete = std::thread::scope(|scope| {
            let (start, ready) = std::sync::mpsc::sync_channel(1);
            let (stop, stopping) = std::sync::mpsc::sync_channel(1);
            let sampler = if config["readiness"].as_bool().unwrap_or(false) {
                Some(scope.spawn(move || {
                    super::d_workload::readiness_loop(engine, origin, ready, stopping, config)
                }))
            } else {
                None
            };
            if sampler.is_some() {
                let _ = start.send(());
            }
            let complete = run_phase(&mut output, "timed", "samples");
            if let Some(sampler) = sampler {
                let _ = stop.send(());
                let reads = sampler.join().unwrap();
                std::fs::write(
                    out.join("readiness.json"),
                    serde_json::to_vec_pretty(&reads).unwrap(),
                )
                .unwrap();
            }
            complete
        });
        std::io::Write::flush(&mut output).unwrap();
        if !complete {
            release_refused(hold, out);
            return;
        }
        let Ok(after) = observation(origin, engine, &store, config, out) else {
            release_refused(hold, out);
            engine.close().unwrap();
            return;
        };
        Some(after)
    } else {
        None
    };
    let mode = config["release_mode"].as_str().unwrap();
    let (_, releasing) = timed(origin, || {
        match mode {
            "explicit" => hold.release(),
            "drop" => (),
            _ => panic!("unknown release mode"),
        }
        drop(hold);
    });
    let (drain_result, draining) =
        timed(origin, || super::native_budget::checked_drain(engine, config, out));
    if drain_result.is_err() {
        retain_release_count(out, requests.get());
        engine.close().unwrap();
        return;
    }
    let Ok(released) = observation(origin, engine, &store, config, out) else {
        retain_release_count(out, requests.get());
        engine.close().unwrap();
        return;
    };
    assert_eq!(released["readiness"]["pending_count"], 0);
    assert_eq!(released["sql"]["physical_vectors"], records.len());
    super::d_workload::write_row(
        &mut raw_observations,
        &json!({"phase":"released","observation":released}),
    );
    let observed = super::d_workload::observation_report(engine, origin, config);
    std::fs::write(out.join("worker-observer.json"), serde_json::to_vec_pretty(&observed).unwrap())
        .unwrap();
    let Ok((coverage, coverage_time)) = coverage(origin, &store, config, out) else {
        retain_release_count(out, requests.get());
        engine.close().unwrap();
        return;
    };
    let (_, closing) = timed(origin, || engine.close().unwrap());
    let Ok(hash) = super::native_budget::completion(config, out, || {
        let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
        let hash: String = Sha256::digest(std::fs::read(&store).unwrap())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        hash
    }) else {
        retain_release_count(out, requests.get());
        return;
    };
    let mut controls = json!({"api_complete":true,"clock_scope":"one attempt origin before open; serialization outside API clocks",
        "open":opening,"configure":configuring,"empty":empty,"freeze_request":freezing,
        "empty_idle_drain":empty_idle,"idle":idle,"writes":writes,"held":held,"release_request":releasing,
        "release_mode":mode,"release_requests":requests.get(),"released_drain":draining,"released":released,
        "coverage":coverage,"coverage_time":coverage_time,"close":closing,"closed_main_sha256":hash,
        "configured_projection_workers":configuration.scheduler_runtime_threads,"configured_provider_workers":configuration.embedder_pool_size});
    if let Some(after_queries) = after_queries {
        controls["after_queries"] = after_queries;
    }
    std::fs::write(out.join("controls.json"), serde_json::to_vec_pretty(&controls).unwrap())
        .unwrap();
    let phases: Vec<_> = [("held", &controls["held"]), ("released", &controls["released"])]
        .into_iter()
        .map(|(phase, observed)| {
            json!({"phase":phase,
            "start_ns":observed["start_ns"],"end_ns":observed["end_ns"],
            "pending_count":observed["readiness"]["pending_count"],
            "canonical_rows":observed["sql"]["canonical_rows"],
            "physical_vectors":observed["sql"]["physical_vectors"],
            "mapped_vectors":observed["sql"]["mapped_vectors"],
            "span_rows":observed["sql"]["span_rows"],"profile":observed["sql"]["profile"]})
        })
        .collect();
    std::fs::write(out.join("phases.json"), serde_json::to_vec_pretty(&phases).unwrap()).unwrap();
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn configure_admission(key: &str, count: usize) {
        let root = tempfile::Builder::new().prefix("s30-held-configure-").tempdir().unwrap().keep();
        let records: Vec<_> = (0..count).map(|i|json!({"id":format!("held-{i}"),"kind":"doc","body":format!("actual held é {i}")})).collect();
        let base = json!({"records":records,"cohort":2,"release_mode":"explicit"});
        let healthy = root.join("healthy");
        run(&base, &healthy);
        let control: Value =
            serde_json::from_slice(&std::fs::read(healthy.join("controls.json")).unwrap()).unwrap();
        assert_eq!(control["held"]["readiness"]["pending_count"], count);
        assert_eq!(control["held"]["sql"]["physical_vectors"], 0);
        assert_eq!(control["released"]["readiness"]["pending_count"], 0);
        assert_eq!(control["released"]["sql"]["physical_vectors"], count);
        assert_eq!(control["release_requests"], 1);
        let mut expired = base;
        for bound in ["work_deadline_ns", "hard_deadline_ns", "shared_deadline_ns"] {
            expired[bound] = json!(u64::MAX);
        }
        expired[key] = json!(0);
        let out = root.join("expired");
        run(&expired, &out);
        let db =
            Connection::open_with_flags(out.join("store.sqlite"), OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get::<_, usize>(0))
                .unwrap(),
            0
        );
        let budget: Value =
            serde_json::from_slice(&std::fs::read(out.join("budget.json")).unwrap()).unwrap();
        println!("actual held configure {} key={key} refusal={budget}", out.display());
        assert_eq!(
            (budget["phase"].as_str(), budget["release_requests"].as_u64().unwrap_or(0)),
            (Some("configure"), 0),
            "expired held route configured/froze work before refusing"
        );
        assert!(!out.join("controls.json").exists());
    }

    #[test]
    fn work_expiry_stops_before_held_configuration() {
        configure_admission("work_deadline_ns", 3);
    }
    #[test]
    fn hard_expiry_stops_before_held_configuration() {
        configure_admission("hard_deadline_ns", 3);
    }
    #[test]
    fn shared_expiry_stops_before_held_configuration() {
        configure_admission("shared_deadline_ns", 3);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn original_held_bounds_prevent_freezing_an_expired_route(count in 1usize..5,key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {configure_admission(key,count);}
    }

    fn write_admission(key: &str, count: usize) {
        let root = tempfile::Builder::new().prefix("s30-held-write-").tempdir().unwrap().keep();
        let origin = Instant::now();
        let store = root.join("store.sqlite");
        let engine =
            Engine::open_with_choice(&store, fathomdb_engine::EmbedderChoice::None).unwrap().engine;
        let make = |prefix: &str, n: usize| {
            (0..n)
                .map(|i| PreparedWrite::Node {
                    kind: "doc".into(),
                    body: format!("actual governed é {i}"),
                    logical_id: Some(format!("{prefix}-{i}")),
                    source_id: SourceId::new("test:s30-held-write").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                })
                .collect::<Vec<_>>()
        };
        let (receipt, clock) =
            write_cohort(origin, &engine, &make("base", 1), &json!({}), &root).unwrap();
        assert_eq!(receipt.row_cursors.len(), 1);
        assert!(clock["end_ns"].as_u64().unwrap() >= clock["start_ns"].as_u64().unwrap());
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let result = write_cohort(origin, &engine, &make("expired", count), &config, &root);
        engine.close().unwrap();
        let db = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let rows: usize =
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
        println!(
            "actual held write {} key={key} canonical={rows} accepted={}",
            root.display(),
            result.is_ok()
        );
        assert_eq!(rows, 1, "expired held cohort still committed new canonical rows");
        assert!(result.is_err());
        let report: Value =
            serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
        assert_eq!(report["phase"], "write");
        assert_eq!(report["disposition"], "deadline_refused");
    }

    #[test]
    fn work_expiry_prevents_actual_held_write() {
        write_admission("work_deadline_ns", 3);
    }
    #[test]
    fn hard_expiry_prevents_actual_held_write() {
        write_admission("hard_deadline_ns", 3);
    }
    #[test]
    fn shared_expiry_prevents_actual_held_write() {
        write_admission("shared_deadline_ns", 3);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn original_held_cohort_bounds_preserve_only_previous_commits(count in 1usize..5,key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {write_admission(key,count);}
    }

    fn observation_admission(key: &str, count: usize) {
        let root =
            tempfile::Builder::new().prefix("s30-held-observation-").tempdir().unwrap().keep();
        let origin = Instant::now();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_test_choice_for_test(
            &store,
            TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
        )
        .unwrap()
        .engine;
        super::super::dense_template::configure(&engine);
        let batch: Vec<_> = (0..count)
            .map(|i| PreparedWrite::Node {
                kind: "doc".into(),
                body: format!("actual observation é {i}"),
                logical_id: Some(format!("observed-{i}")),
                source_id: SourceId::new("test:s30-held-observation").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        engine.write(&batch).unwrap();
        engine.drain(5000).unwrap();
        let healthy = observation(origin, &engine, &store, &json!({}), &root).unwrap();
        assert_eq!(healthy["readiness"]["pending_count"], 0);
        assert_eq!(healthy["sql"]["canonical_rows"], count);
        assert_eq!(healthy["sql"]["physical_vectors"], count);
        assert_eq!(healthy["sql"]["profile"]["dimension"], 768);
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let result = observation(origin, &engine, &store, &config, &root);
        engine.close().unwrap();
        let db = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get::<_, usize>(0))
                .unwrap(),
            count
        );
        println!(
            "actual held observation {} key={key} accepted={}",
            root.display(),
            result.is_ok()
        );
        assert!(
            result.is_err(),
            "expired held observation still executed successful readiness/SQL work"
        );
        let report: Value =
            serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
        assert_eq!(report["phase"], "observation");
        assert_eq!(report["disposition"], "deadline_refused");
    }

    #[test]
    fn work_expiry_prevents_actual_held_observation() {
        observation_admission("work_deadline_ns", 3);
    }
    #[test]
    fn hard_expiry_prevents_actual_held_observation() {
        observation_admission("hard_deadline_ns", 3);
    }
    #[test]
    fn shared_expiry_prevents_actual_held_observation() {
        observation_admission("shared_deadline_ns", 3);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn original_held_observation_bounds_preserve_stored_population(count in 1usize..5,key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {observation_admission(key,count);}
    }

    fn coverage_admission(key: &str, count: usize) {
        let root = tempfile::Builder::new().prefix("s30-held-coverage-").tempdir().unwrap().keep();
        let origin = Instant::now();
        let store = root.join("store.sqlite");
        let engine = Engine::open_with_test_choice_for_test(
            &store,
            TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
        )
        .unwrap()
        .engine;
        super::super::dense_template::configure(&engine);
        let batch: Vec<_> = (0..count)
            .map(|i| PreparedWrite::Node {
                kind: "doc".into(),
                body: format!("actual coverage é {i}"),
                logical_id: Some(format!("covered-{i}")),
                source_id: SourceId::new("test:s30-held-coverage").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        engine.write(&batch).unwrap();
        engine.drain(5000).unwrap();
        engine.close().unwrap();
        let (healthy, clock) = coverage(origin, &store, &json!({}), &root).unwrap();
        assert_eq!(healthy["verified"], true);
        assert_eq!(healthy["rows"].as_array().unwrap().len(), count);
        assert_eq!(healthy["profile"]["dimension"], 768);
        assert!(clock["end_ns"].as_u64().unwrap() >= clock["start_ns"].as_u64().unwrap());
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let result = coverage(origin, &store, &config, &root);
        println!("actual held coverage {} key={key} accepted={}", root.display(), result.is_ok());
        assert!(
            result.is_err(),
            "expired held route still starts actual cursor/vector/span verification"
        );
        let report: Value =
            serde_json::from_slice(&std::fs::read(root.join("budget.json")).unwrap()).unwrap();
        assert_eq!(report["phase"], "verification");
        assert_eq!(report["disposition"], "deadline_refused");
    }

    #[test]
    fn work_expiry_prevents_actual_held_coverage() {
        coverage_admission("work_deadline_ns", 3);
    }
    #[test]
    fn hard_expiry_prevents_actual_held_coverage() {
        coverage_admission("hard_deadline_ns", 3);
    }
    #[test]
    fn shared_expiry_prevents_actual_held_coverage() {
        coverage_admission("shared_deadline_ns", 3);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn original_held_coverage_bounds_apply_to_real_closed_stores(count in 1usize..5,key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {coverage_admission(key,count);}
    }
}
