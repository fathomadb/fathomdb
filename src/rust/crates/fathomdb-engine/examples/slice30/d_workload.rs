//! Deterministic-provider query caller for owned D source fixtures.

use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ReadView, SourceId, TestEmbedderChoice,
};
use rusqlite::{Connection, OpenFlags};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::dense_template;
use super::varying_embedder::VaryingEmbedder;

pub(super) fn run(config: &Value, out: &Path) {
    let _ = run_inner(config, out);
}

fn drain(engine: &Engine, config: &Value, out: &Path) -> Result<(), ()> {
    super::native_budget::phase(config, out, "drain")?;
    let mut report = super::native_budget::drain(engine, config).unwrap();
    if report["drain_calls"] != 1 {
        report["phase"] = json!("drain");
        std::fs::write(out.join("budget.json"), serde_json::to_vec_pretty(&report).unwrap())
            .unwrap();
        return Err(());
    }
    super::native_budget::phase(config, out, "drain")
}

fn prepare_template(config: &Value, out: &Path, store: &Path) -> Result<(), ()> {
    if !dense_template::run(config, &out.join("template")) {
        std::fs::copy(out.join("template/budget.json"), out.join("budget.json")).unwrap();
        return Err(());
    }
    std::fs::copy(out.join("template/template.sqlite"), store).unwrap();
    Ok(())
}

fn run_inner(config: &Value, out: &Path) -> Result<(), ()> {
    let origin = Instant::now();
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("query.sqlite");
    assert!(!store.exists());
    let empty_start = config["empty_start"].as_bool().unwrap_or(false);
    super::native_budget::phase(config, out, "entry")?;
    if !empty_start {
        prepare_template(config, out, &store)?;
    }
    super::native_budget::query_open(config, out)?;
    let opened = Engine::open_with_test_choice_for_test(
        &store,
        TestEmbedderChoice::Caller(Arc::new(VaryingEmbedder)),
    )
    .unwrap();
    let engine = &opened.engine;
    if empty_start {
        super::native_budget::phase(config, out, "configure")?;
        dense_template::configure(engine);
        let start_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
        drain(engine, config, out)?;
        let readiness = engine.read_embedding_readiness().unwrap();
        let connection =
            Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let mut initial = json!({"start_ns":start_ns,"pending_count":readiness.pending_count,
            "usable_embedder":readiness.usable_embedder});
        for table in
            ["canonical_nodes", "vector_default", "_fathomdb_vector_rows", "_fathomdb_vector_spans"]
        {
            let count: u64 = connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0);
            initial[table] = json!(count);
        }
        initial["profile"] = connection.query_row("SELECT name,revision,dimension FROM _fathomdb_embedder_profiles WHERE profile='default'", [], |r|Ok(json!({"name":r.get::<_,String>(0)?,"revision":r.get::<_,String>(1)?,"dimension":r.get::<_,u32>(2)?}))).unwrap();
        drop(connection);
        engine.begin_d27_observation_for_test(origin);
        let configuration = engine.d27_observation_for_test().unwrap().configuration_observation;
        initial["projection_workers"] = json!(configuration.scheduler_runtime_threads);
        initial["provider_workers"] = json!(configuration.embedder_pool_size);
        initial["end_ns"] = json!(u64::try_from(origin.elapsed().as_nanos()).unwrap());
        std::fs::write(out.join("initial.json"), serde_json::to_vec_pretty(&initial).unwrap())
            .unwrap();
    }
    assert_eq!(engine.read_embedding_readiness().unwrap().pending_count, 0);
    enable_observation(engine, config);
    let mut output = std::fs::File::create(out.join("queries.jsonl")).unwrap();
    let warmups = usize::try_from(config["warmups"].as_u64().unwrap()).unwrap();
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    for sample in 0..warmups {
        super::native_budget::phase(config, out, "warmup")?;
        write_row(
            &mut output,
            &query_row(engine, origin, "warmup", sample, request(config, sample)),
        );
    }
    if let Some(records) = config["write_records"].as_array() {
        let cohort = usize::try_from(config["cohort"].as_u64().unwrap()).unwrap();
        assert!(cohort > 0);
        let writes = std::thread::scope(|scope| {
            let (start_readiness, readiness_ready) = std::sync::mpsc::sync_channel(1);
            let (stop_readiness, readiness_stop) = std::sync::mpsc::sync_channel(1);
            let readiness = if config["readiness"].as_bool().unwrap_or(false) {
                Some(scope.spawn(move || {
                    readiness_loop(engine, origin, readiness_ready, readiness_stop, config)
                }))
            } else {
                None
            };
            let (release, ready) = std::sync::mpsc::sync_channel(1);
            let writer = scope.spawn(move || {
                let launch_ns: u64 = ready.recv_timeout(Duration::from_secs(5)).unwrap();
                let mut writes = Vec::new();
                for rows in records.chunks(cohort) {
                    if super::native_budget::bounds(config)
                        .map_or(true, |bound| bound.is_some_and(|(now, limit)| now >= limit))
                    {
                        break;
                    }
                    let batch: Vec<_> = rows.iter().map(|row| PreparedWrite::Node {
                        kind: row["kind"].as_str().unwrap().into(),
                        body: row["body"].as_str().unwrap().into(),
                        source_id: SourceId::new("test:slice30-d-overlap").unwrap(),
                        logical_id: Some(row["id"].as_str().unwrap().into()),
                        state: InitialState::Active,
                        reason: None, valid_from: None, valid_until: None,
                    }).collect();
                    let start_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
                    let receipt = engine.write(&batch).unwrap();
                    let end_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
                    writes.push(json!({"ok":true,"launch_ns":launch_ns,"start_ns":start_ns,"end_ns":end_ns,
                        "rows":batch.len(),"cursor":receipt.cursor,"row_cursors":receipt.row_cursors}));
                }
                writes
            });
            let launch_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
            if readiness.is_some() {
                start_readiness.send(()).unwrap();
            }
            release.send(launch_ns).unwrap();
            for sample in 0..samples {
                if super::native_budget::phase(config, out, "timed").is_err() {
                    break;
                }
                write_row(
                    &mut output,
                    &query_row(engine, origin, "timed", sample, request(config, sample)),
                );
            }
            let writes = writer.join().unwrap();
            if let Some(readiness) = readiness {
                let _ = stop_readiness.send(());
                let reads = readiness.join().unwrap();
                std::fs::write(
                    out.join("readiness.json"),
                    serde_json::to_vec_pretty(&reads).unwrap(),
                )
                .unwrap();
            }
            writes
        });
        std::fs::write(out.join("writes.json"), serde_json::to_vec_pretty(&writes).unwrap())
            .unwrap();
        drain(engine, config, out)?;
    } else {
        for sample in 0..samples {
            super::native_budget::phase(config, out, "timed")?;
            write_row(
                &mut output,
                &query_row(engine, origin, "timed", sample, request(config, sample)),
            );
        }
    }
    output.flush().unwrap();
    super::native_budget::phase(config, out, "collect")?;
    let observation = observation_report(engine, origin, config);
    std::fs::write(
        out.join("worker-observer.json"),
        serde_json::to_vec_pretty(&observation).unwrap(),
    )
    .unwrap();
    engine.close().unwrap();
    super::native_budget::phase(config, out, "complete")?;
    std::fs::write(out.join("complete.json"), b"{\"complete\":true}\n").unwrap();
    Ok(())
}

pub(super) fn write_row(output: &mut std::fs::File, row: &Value) {
    serde_json::to_writer(&mut *output, row).unwrap();
    output.write_all(b"\n").unwrap();
}

pub(super) fn request(config: &Value, sample: usize) -> Option<&Value> {
    let queries = config["queries"].as_array()?;
    assert!(!queries.is_empty());
    Some(&queries[sample % queries.len()])
}

pub(super) fn query_row(
    engine: &Engine,
    origin: Instant,
    phase: &str,
    sample: usize,
    request: Option<&Value>,
) -> Value {
    let query = request.map_or("record", |r| r["query"].as_str().unwrap());
    let fingerprint = request.map(|_| {
        Sha256::digest(query.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()
    });

    let start_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
    let started = Instant::now();
    let result = engine
        .search_reranked_view_with_limit(
            query,
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
        )
        .unwrap();
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let end_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
    let mut row = json!({"phase":phase,"sample_index":sample,"query":query,
    "start_ns":start_ns,"end_ns":end_ns,"elapsed_ns":elapsed_ns,"ok":true,
    "ids":result.results.iter().map(|hit|hit.id.to_prefixed()).collect::<Vec<_>>(),
    "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations)});
    if let Some(request) = request {
        row["query_id"] = request["id"].clone();
        row["query_sha256"] = json!(fingerprint.unwrap());
    }
    row
}

fn unavailable(reason: &str) -> Value {
    json!({"available":false,"reason":reason,"records":null,"dropped":null,
        "query_records":null,"query_dropped":null,"attribution_complete":null})
}

#[cfg(not(feature = "slice30-measurement"))]
pub(super) fn observation_report(_engine: &Engine, _origin: Instant, _config: &Value) -> Value {
    unavailable("feature_absent")
}

#[cfg(feature = "slice30-measurement")]
pub(super) fn observation_report(engine: &Engine, origin: Instant, config: &Value) -> Value {
    if !config["observer"].as_bool().unwrap_or(false) {
        return unavailable("observer_disabled");
    }
    let expected = config["write_records"]
        .as_array()
        .or_else(|| config["records"].as_array())
        .map_or(0, Vec::len) as u64;
    let allowance = super::native_budget::bounds(config)
        .unwrap()
        .map_or(Duration::from_secs(5), |(now, limit)| {
            Duration::from_nanos(limit.saturating_sub(now)).min(Duration::from_secs(5))
        });
    let deadline = Instant::now() + allowance;
    let mut records = Vec::new();
    let dropped;
    loop {
        let report = engine.take_slice30_projection_measurements_for_test(origin).unwrap();
        assert_eq!(report["capacity"], 32768);
        records.extend(report["records"].as_array().unwrap().iter().cloned());
        let observed: u64 = records.iter().map(|r| r["job_count"].as_u64().unwrap()).sum();
        if observed >= expected || Instant::now() >= deadline {
            dropped = report["dropped"].as_u64().unwrap();
            break;
        }
        std::thread::sleep(
            Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    let query_records = engine.take_slice30_measurements_for_test().unwrap();
    let query_stats = engine.slice30_measurement_stats_for_test().unwrap();
    let query_dropped = query_stats["dropped"].as_u64().unwrap();
    let query_count = config["warmups"].as_u64().unwrap() + config["samples"].as_u64().unwrap();
    let observed: u64 = records.iter().map(|r| r["job_count"].as_u64().unwrap()).sum();
    let complete = dropped == 0
        && query_dropped == 0
        && !records.is_empty()
        && observed == expected
        && query_records.len() as u64 == query_count
        && records.iter().all(|r| {
            r["clock_issue"].is_null()
                && r["start_ns"].as_u64().is_some()
                && r["end_ns"].as_u64().is_some()
                && r["cleanup"] == "confirmed"
                && r["commit_outcome"] == "ok"
        });
    json!({"available":true,"reason":null,"capacity":32768,"dropped":dropped,
        "records":records,"query_records":query_records,"query_dropped":query_dropped,
        "attribution_complete":complete,"collected_at_ns":u64::try_from(origin.elapsed().as_nanos()).unwrap()})
}

pub(super) fn readiness_loop(
    engine: &Engine,
    origin: Instant,
    start: std::sync::mpsc::Receiver<()>,
    stop: std::sync::mpsc::Receiver<()>,
    _config: &Value,
) -> Vec<Value> {
    start.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut reads = Vec::new();
    loop {
        let started = Instant::now();
        let start_ns = u64::try_from(started.duration_since(origin).as_nanos()).unwrap();
        let result = engine.read_embedding_readiness();
        let end_ns = u64::try_from(origin.elapsed().as_nanos()).unwrap();
        reads.push(match result {
            Ok(readiness) => json!({"ok":true,"start_ns":start_ns,"end_ns":end_ns,
            "pending_count":readiness.pending_count,"usable_embedder":readiness.usable_embedder,
            "state":format!("{:?}",readiness.state)}),
            Err(error) => json!({"ok":false,"start_ns":start_ns,"end_ns":end_ns,
            "pending_count":null,"error":format!("{error:?}")}),
        });
        let until_next =
            (started + Duration::from_millis(100)).saturating_duration_since(Instant::now());
        match stop.recv_timeout(until_next) {
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => (),
            _ => break,
        }
    }
    reads
}

pub(super) fn enable_observation(engine: &Engine, config: &Value) {
    #[cfg(feature = "slice30-measurement")]
    if config["observer"].as_bool().unwrap_or(false) {
        engine.slice30_observe_projection_workers_for_test(true).unwrap();
        engine.slice30_observe_queries_for_test(true).unwrap();
    }
    #[cfg(not(feature = "slice30-measurement"))]
    let _ = (engine, config);
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use proptest::prelude::*;

    fn failed_preparation(rows: usize) {
        let root = tempfile::Builder::new().prefix("s30-template-stop-").tempdir().unwrap().keep();
        let records: Vec<_> = (0..rows).map(|index|json!({"id":format!("prepared-{index}"),"kind":"doc","body":format!("real prepared é {index}")})).collect();
        let healthy = root.join("healthy");
        let healthy_copy = healthy.join("query.sqlite");
        assert!(prepare_template(&json!({"records":records}), &healthy, &healthy_copy).is_ok());
        assert!(healthy.join("template/template.json").exists());
        let verify = |path: &Path, expected: usize| {
            let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
            let count: usize =
                db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
            assert_eq!(
                count, expected,
                "expired D preparation committed rows before checking its next phase"
            );
            let dimension: usize = db
                .query_row(
                    "SELECT dimension FROM _fathomdb_embedder_profiles WHERE profile='default'",
                    [],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(dimension, 768);
        };
        verify(&healthy_copy, rows);
        let failed = root.join("refused");
        let target = failed.join("query.sqlite");
        let result = prepare_template(
            &json!({"records":records,"work_deadline_ns":0,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX}),
            &failed,
            &target,
        );
        let source = failed.join("template/template.sqlite");
        verify(&source, 0);
        assert!(!failed.join("template/template.json").exists());
        let refusal: Value =
            serde_json::from_slice(&std::fs::read(failed.join("template/budget.json")).unwrap())
                .unwrap();
        assert_eq!(refusal["phase"], "configure");
        assert_eq!(refusal["disposition"], "deadline_refused");
        println!(
            "actual unqualified template {} rows={rows} copied={} result={result:?}",
            source.display(),
            target.exists()
        );
        assert!(!target.exists(), "failed preparation still started a downstream SQLite copy");
        assert!(result.is_err());
        let forwarded: Value =
            serde_json::from_slice(&std::fs::read(failed.join("budget.json")).unwrap()).unwrap();
        assert_eq!(forwarded, refusal);
    }

    #[test]
    fn refused_template_prevents_downstream_copy() {
        failed_preparation(2);
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:3,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn refused_preparation_commits_no_new_rows_and_never_copies(rows in 1usize..5) {failed_preparation(rows);}
    }

    fn readiness_admission(key: &str) {
        let root =
            tempfile::Builder::new().prefix("s30-readiness-budget-").tempdir().unwrap().keep();
        let store = root.join("store.sqlite");
        let origin = Instant::now();
        let engine =
            Engine::open_with_choice(&store, fathomdb_engine::EmbedderChoice::None).unwrap().engine;
        engine
            .write(&[PreparedWrite::Node {
                kind: "doc".into(),
                body: "actual readiness control é".into(),
                source_id: SourceId::new("test:s30-readiness-budget").unwrap(),
                logical_id: Some("ready-control".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            }])
            .unwrap();
        let sample = |config: &Value| {
            std::thread::scope(|scope| {
                let (start, ready) = std::sync::mpsc::sync_channel(1);
                let (stop, stopping) = std::sync::mpsc::sync_channel(1);
                let worker =
                    scope.spawn(|| readiness_loop(&engine, origin, ready, stopping, config));
                let _ = start.send(());
                std::thread::sleep(Duration::from_millis(125));
                let _ = stop.send(());
                worker.join().unwrap()
            })
        };
        let healthy = sample(&json!({}));
        assert!(!healthy.is_empty());
        assert!(healthy.iter().all(|row| row["ok"] == true
            && row["end_ns"].as_u64().unwrap() >= row["start_ns"].as_u64().unwrap()));
        let mut config = json!({"work_deadline_ns":u64::MAX,"hard_deadline_ns":u64::MAX,"shared_deadline_ns":u64::MAX});
        config[key] = json!(0);
        let expired = sample(&config);
        engine.close().unwrap();
        let db = Connection::open_with_flags(&store, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM canonical_nodes", [], |row| row.get::<_, usize>(0))
                .unwrap(),
            1
        );
        println!(
            "actual readiness budget {} key={key} completed_reads={}",
            root.display(),
            expired.len()
        );
        assert!(
            expired.is_empty(),
            "readiness sampler kept starting real getter calls beyond its original budget"
        );
    }

    #[test]
    fn expired_work_stops_actual_background_readiness() {
        readiness_admission("work_deadline_ns");
    }
    #[test]
    fn expired_hard_stops_actual_background_readiness() {
        readiness_admission("hard_deadline_ns");
    }
    #[test]
    fn expired_shared_stops_actual_background_readiness() {
        readiness_admission("shared_deadline_ns");
    }
    proptest! {
        #![proptest_config(ProptestConfig{cases:2,max_shrink_iters:0,failure_persistence:None,..ProptestConfig::default()})]
        #[test]
        fn background_readiness_never_renews_any_original_bound(key in prop::sample::select(vec!["work_deadline_ns","hard_deadline_ns","shared_deadline_ns"])) {readiness_admission(key);}
    }
}
