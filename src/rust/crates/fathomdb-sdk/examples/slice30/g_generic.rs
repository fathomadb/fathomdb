//! Private generic G single-pass caller, separate from bounded source fixture preparation.

use super::{
    g_observer, g_observer_controls, g_template, g_write_schedule, CoreObserver, GAdapter, Recorder,
};
use fathomdb_embedder::EffectiveEmbedDevice;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::Path;

fn observe(
    engine: &GAdapter,
    mode: &str,
    adapter: &str,
    sample_index: usize,
    input: &Value,
    kind: Option<&str>,
) -> Value {
    let query = input["query"].as_str().unwrap();
    let fingerprint: String =
        Sha256::digest(query.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
    let mut row = if mode.ends_with("_text") {
        let (result, elapsed_ns) = engine.timed_text(query, kind);
        let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
        json!({"mode":"direct_text","ok":true,"elapsed_ns":elapsed_ns,"ids":ids,
            "returned_hits":result.results.len(),"kind_filter":null,
            "search_snapshot":null,"snapshot_observation":"unavailable"})
    } else if mode.ends_with("_hybrid") {
        let (result, elapsed_ns) = match kind {
            Some(kind) => engine.timed_hybrid_for_kind(query, 10, kind),
            None => engine.timed_hybrid(query, 10),
        };
        let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
        json!({"mode":"hybrid","ok":true,"elapsed_ns":elapsed_ns,"ids":ids,
            "returned_hits":result.results.len(),"soft_fallback":format!("{:?}",result.soft_fallback),
            "degradations":format!("{:?}",result.degradations)})
    } else {
        assert!(mode.ends_with("_mean"));
        let (vector, elapsed_ns) = engine.timed_embed(query);
        assert_eq!(vector.len(), 384);
        assert!(vector.iter().all(|value| value.is_finite()));
        let norm2 = vector.iter().map(|value| f64::from(*value).powi(2)).sum::<f64>();
        assert!(norm2 > 0.0);
        json!({"mode":"mean_embed","ok":true,"elapsed_ns":elapsed_ns,"dimension":384,
            "finite":true,"norm_squared":norm2})
    };
    if kind.is_some() && !mode.ends_with("_mean") {
        row["kind_filter"] = json!(kind);
    }
    row["adapter"] = json!(adapter);
    row["sample_index"] = json!(sample_index);
    row["query_id"] = input["id"].clone();
    row["query_fingerprint"] = json!(fingerprint);
    row
}

pub(super) fn run(config: &Value, out: &Path, store: &Path, observer: Option<CoreObserver>) {
    if observer.is_some() {
        assert!(config["mode"].as_str().unwrap().starts_with("engine_"));
    }
    assert!(store.exists(), "generic pass requires its owned writable closed-template copy");
    std::fs::create_dir_all(out).unwrap();
    let queries: Vec<Value> =
        std::io::BufReader::new(std::fs::File::open(config["queries"].as_str().unwrap()).unwrap())
            .lines()
            .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
            .collect();
    assert!(!queries.is_empty());
    let ids: BTreeSet<_> = queries.iter().map(|query| query["id"].as_u64().unwrap()).collect();
    assert_eq!(ids.len(), queries.len());
    let mode = config["mode"].as_str().unwrap();
    if super::budget_clock::phase(config, out, "pass").is_err() {
        return;
    }
    let opening = std::time::Instant::now();
    let engine = GAdapter::open(mode, store);
    let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
    let resolution = engine.report().embedder_device_resolution.as_ref().unwrap();
    assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
    assert!(
        matches!(&resolution.effective_device, EffectiveEmbedDevice::Cuda(device) if device.ordinal == 0)
    );
    assert_eq!(engine.report().default_embedder.dimension, 384);
    if super::budget_clock::phase(config, out, "pass").is_err() {
        engine.close();
        return;
    }
    let adapter = match &engine {
        GAdapter::Core(_) => "engine",
        GAdapter::Sdk(_) => "sdk",
    };
    let g2 = config["population"] == "G2";
    let pass_origin = std::time::Instant::now();
    let mut reset_start_ns = None;
    let mut reset_end_ns = None;
    let reset = g2.then(|| {
        reset_start_ns = Some(u64::try_from(pass_origin.elapsed().as_nanos()).unwrap());
        let reset = g_write_schedule::reset(&engine, store, config);
        reset_end_ns = Some(u64::try_from(pass_origin.elapsed().as_nanos()).unwrap());
        reset
    });
    let mut first_warmup_record_start_ns = None;
    let mut first_timed_record_start_ns = None;
    let mut recorder = Recorder::new(out);
    let mut observation = observer.map(|callbacks| {
        let GAdapter::Core(opened) = &engine else { unreachable!() };
        g_observer::Observation::new(
            &opened.engine,
            callbacks,
            config["observer"].as_bool().unwrap_or(false),
        )
    });
    let loop_started = std::time::Instant::now();
    let mut calls = 0;
    let writing = g2 && config["schedule"] == "write";
    let mut scheduled_write_calls = 0;
    let mut scheduled_write_rows = 0;
    let mut marker_probes = 0;
    let warmup_count = config
        .get("warmups")
        .map(|value| usize::try_from(value.as_u64().unwrap()).unwrap())
        .unwrap_or(0);
    assert!(warmup_count <= queries.len());
    for (sample_index, input) in queries.iter().take(warmup_count).enumerate() {
        if super::budget_clock::phase(config, out, "warmup").is_err() {
            recorder.flush_all();
            engine.close();
            return;
        }
        let record_start_ns =
            writing.then(|| u64::try_from(pass_origin.elapsed().as_nanos()).unwrap());
        if g2 && sample_index == 0 {
            first_warmup_record_start_ns = record_start_ns
                .or_else(|| Some(u64::try_from(pass_origin.elapsed().as_nanos()).unwrap()));
        }
        let mut row = observe(&engine, mode, adapter, sample_index, input, g2.then_some("doc"));
        if let Some(observation) = observation.as_mut() {
            row["observer"] = observation.complete(!mode.ends_with("_mean"));
        }
        row["call_index"] = json!(calls);
        calls += 1;
        if let Some(started) = record_start_ns {
            row["record_start_ns"] = json!(started);
            row["record_ready_ns"] =
                json!(u64::try_from(pass_origin.elapsed().as_nanos()).unwrap());
        }
        recorder.emit("warmups", &row);
    }
    recorder.flush_all();
    let warmup_calls = calls;
    for (sample_index, input) in queries.iter().enumerate() {
        if super::budget_clock::phase(config, out, "timed").is_err() {
            recorder.flush_all();
            engine.close();
            return;
        }
        let record_start_ns =
            writing.then(|| u64::try_from(pass_origin.elapsed().as_nanos()).unwrap());
        if g2 && sample_index == 0 {
            first_timed_record_start_ns = record_start_ns
                .or_else(|| Some(u64::try_from(pass_origin.elapsed().as_nanos()).unwrap()));
        }
        let mut row = observe(&engine, mode, adapter, sample_index, input, g2.then_some("doc"));
        if let Some(observation) = observation.as_mut() {
            row["observer"] = observation.complete(!mode.ends_with("_mean"));
        }
        row["call_index"] = json!(calls);
        calls += 1;
        if let Some(started) = record_start_ns {
            row["record_start_ns"] = json!(started);
            row["record_ready_ns"] =
                json!(u64::try_from(pass_origin.elapsed().as_nanos()).unwrap());
        }
        recorder.emit("samples", &row);
        let timed_calls = sample_index + 1;
        if writing && timed_calls.is_multiple_of(10) {
            let cohort = g_write_schedule::run(
                &engine,
                store,
                timed_calls,
                &mut recorder,
                pass_origin,
                config,
            );
            scheduled_write_calls += 1;
            scheduled_write_rows += cohort.rows;
            marker_probes += cohort.probes;
        }
    }
    recorder.flush_all();
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    let closing = std::time::Instant::now();
    engine.close();
    let close_ns = u64::try_from(closing.elapsed().as_nanos()).unwrap();
    let mut proof = json!({"mode":mode,"api_calls":calls,"timed_api_calls":calls-warmup_calls,"warmup_api_calls":warmup_calls,"open_ns":open_ns,
        "close_ns":close_ns,"loop_ns":loop_ns,"store":store,
        "selected_cuda_uuid":resolution.selected_cuda_uuid,"cuda_compiled":resolution.cuda_compiled,
        "raw_persistence":recorder.statistics_since([0;5]),
        "throughput_scope":"warmup and timed API calls plus input identity/validation/serialization/checkpoints; open/close separate"});
    if let Some(reset) = reset {
        proof["g2_reset"] = reset;
        proof["g2_placement"] = json!({"reset_start_ns":reset_start_ns,"reset_end_ns":reset_end_ns,
            "first_warmup_record_start_ns":first_warmup_record_start_ns,
            "first_warmup_disposition":if warmup_count == 0 { "not_applicable" } else { "observed" },
            "first_timed_record_start_ns":first_timed_record_start_ns});
        proof["throughput_scope"] = json!("warmup and timed API calls plus input identity/validation/serialization/checkpoints; reset/open/close separate");
    }
    if writing {
        proof["schedule"] = json!({"mode":"write","natural_calls":queries.len(),
            "scheduled_write_calls":scheduled_write_calls,"scheduled_write_rows":scheduled_write_rows,
            "marker_probes":marker_probes});
        proof["throughput_scope"] = json!("warmup and timed API calls plus input identity/validation/serialization/checkpoints/scheduled writes/drain/probes/coverage; reset/open/close separate");
    }
    if let Some(observation) = observation {
        proof["observer_setup_ns"] = json!(observation.setup_ns);
        proof["throughput_scope"] = json!("warmup and timed API calls plus input identity/validation/observer collection/serialization/checkpoints; observer setup/open/close separate");
    }
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

pub(super) fn fixture(config: &Value, out: &Path, observer: Option<CoreObserver>) {
    assert_eq!(config["templates"].as_array().unwrap().len(), 1);
    g_template::run(config, out);
    let source = out.join("case-0/template.sqlite");
    let original = Sha256::digest(std::fs::read(&source).unwrap());
    for mode in config["modes"].as_array().unwrap() {
        let directory = out.join(mode.as_str().unwrap());
        std::fs::create_dir(&directory).unwrap();
        let store = directory.join("store.sqlite");
        std::fs::copy(&source, &store).unwrap();
        assert_eq!(Sha256::digest(std::fs::read(&store).unwrap()), original);
        let mut pass = config.clone();
        pass["mode"] = mode.clone();
        run(&pass, &directory, &store, observer);
        let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
        assert_eq!(Sha256::digest(std::fs::read(&source).unwrap()), original);
    }
    if config["error_controls"].as_bool() == Some(true) {
        let directory = out.join("error-controls");
        std::fs::create_dir(&directory).unwrap();
        let store = directory.join("store.sqlite");
        std::fs::copy(&source, &store).unwrap();
        assert_eq!(Sha256::digest(std::fs::read(&store).unwrap()), original);
        g_observer_controls::run(config, &directory, &store, observer);
        assert_eq!(Sha256::digest(std::fs::read(&source).unwrap()), original);
    }
}

pub(super) fn cell(config: &Value, out: &Path, store: &Path, observer: Option<CoreObserver>) {
    std::fs::create_dir_all(out).unwrap();
    let passes = config["passes"].as_array().unwrap();
    assert!(!passes.is_empty());
    let schedules = config.get("schedules").map(|value| value.as_array().unwrap());
    if let Some(schedules) = schedules {
        assert_eq!(schedules.len(), passes.len());
    }
    for (ordinal, mode) in passes.iter().enumerate() {
        let mut pass = config.clone();
        pass["mode"] = mode.clone();
        if let Some(schedules) = schedules {
            assert!(matches!(schedules[ordinal].as_str(), Some("readonly" | "write")));
            pass["schedule"] = schedules[ordinal].clone();
        }
        run(&pass, &out.join(format!("pass-{ordinal}")), store, observer);
    }
}

pub(super) fn plan_fixture(config: &Value, out: &Path, observer: Option<CoreObserver>) {
    assert_eq!(config["templates"].as_array().unwrap().len(), 1);
    g_template::run(config, out);
    let source = out.join("case-0/template.sqlite");
    let original = Sha256::digest(std::fs::read(&source).unwrap());
    let directory = out.join("cell");
    std::fs::create_dir(&directory).unwrap();
    let store = directory.join("store.sqlite");
    std::fs::copy(&source, &store).unwrap();
    assert_eq!(Sha256::digest(std::fs::read(&store).unwrap()), original);
    cell(config, &directory, &store, observer);
    let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
    assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
    assert_eq!(Sha256::digest(std::fs::read(&source).unwrap()), original);
}
