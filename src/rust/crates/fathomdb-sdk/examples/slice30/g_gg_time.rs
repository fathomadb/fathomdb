//! GG time-copy controls; timed diagnostic stages are separate from literal text proofs.

use super::{g_observer, g_template, g_write_schedule, CoreObserver, GAdapter, Recorder};
use fathomdb_engine::{InitialState, PreparedWrite, ReadView, SourceId};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;

fn control(engine: &fathomdb_engine::Engine, stage: &str, as_of: i64) -> Value {
    let started = Instant::now();
    let view = ReadView { valid_as_of: Some(as_of), ..ReadView::default() };
    let result = engine
        .search_text_only_filtered_with_snippets("s30ggwindow", None, &view, 10, None)
        .unwrap();
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
    let expected = if stage == "inside" { vec!["l:gg-window"] } else { Vec::new() };
    assert_eq!(ids, expected);
    json!({"stage":stage,"valid_as_of":as_of,"ids":ids,"elapsed_ns":elapsed_ns})
}

fn measure_stage(
    engine: &fathomdb_engine::Engine,
    config: &Value,
    directory: &Path,
    stage: (&str, i64),
    observer: CoreObserver,
) {
    let (stage, as_of) = stage;
    let out = directory.join(stage);
    std::fs::create_dir(&out).unwrap();
    let warmups = usize::try_from(config["warmups"].as_u64().unwrap()).unwrap();
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    assert!(samples > 0);
    let mut observation = g_observer::Observation::new(engine, observer, true);
    let mut recorder = Recorder::new(&out);
    let loop_started = Instant::now();
    let mut calls = 0;
    for (stream, count) in [("warmups", warmups), ("samples", samples)] {
        for sample in 0..count {
            let query = "s30ggwindow";
            let fingerprint: String =
                Sha256::digest(query.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
            let started = Instant::now();
            let view = ReadView { valid_as_of: Some(as_of), ..ReadView::default() };
            let result = engine
                .search_reranked_view_with_limit(query, None, 0, false, 0.3, 0, false, &view, 10)
                .unwrap();
            let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
            let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
            let row = json!({"adapter":"engine","mode":"hybrid","stage":stage,"valid_as_of":as_of,
                "ok":true,"query_id":0,"query_fingerprint":fingerprint,"sample_index":sample,"call_index":calls,
                "elapsed_ns":elapsed_ns,"ids":ids,"returned_hits":result.results.len(),
                "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations),
                "observer":observation.complete(true)});
            recorder.emit(stream, &row);
            calls += 1;
        }
        recorder.flush_all();
    }
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    let disabling = Instant::now();
    observer.0(engine, false).unwrap();
    let disable_ns = u64::try_from(disabling.elapsed().as_nanos()).unwrap();
    let proof = json!({"stage":stage,"valid_as_of":as_of,"api_calls":calls,"warmup_calls":warmups,"timed_calls":samples,
        "loop_ns":loop_ns,"observer_setup_ns":observation.setup_ns,"observer_disable_ns":disable_ns,
        "raw_persistence":recorder.statistics_since([0;5]),
        "throughput_scope":"stage warmup and timed APIs plus validation/observer collection/recording; literal controls/write/drain/open/close and observer toggles separate"});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

pub(super) fn fixture(config: &Value, out: &Path, observer: CoreObserver) {
    assert_eq!(config["templates"].as_array().unwrap().len(), 1);
    g_template::run(config, out);
    let source = out.join("case-0/template.sqlite");
    let original = Sha256::digest(std::fs::read(&source).unwrap());
    let directory = out.join("time");
    std::fs::create_dir(&directory).unwrap();
    let store = directory.join("store.sqlite");
    std::fs::copy(&source, &store).unwrap();
    assert_eq!(Sha256::digest(std::fs::read(&store).unwrap()), original);
    let adapter = GAdapter::open("engine_hybrid", &store);
    let report = adapter.report();
    assert_eq!(
        report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid.as_deref(),
        config["gpu_uuid"].as_str()
    );
    let GAdapter::Core(opened) = &adapter else { unreachable!() };
    let engine = &opened.engine;
    let baseline = g_write_schedule::current_coverage(&store, &report.default_embedder);
    let mut controls = vec![control(engine, "baseline", 100)];
    measure_stage(engine, config, &directory, ("baseline", 100), observer);
    let writing = Instant::now();
    let receipt = engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "s30ggwindow épreuve".into(),
            source_id: SourceId::new("test:slice30-gg-time").unwrap(),
            logical_id: Some("gg-window".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: Some(90),
            valid_until: Some(110),
        }])
        .unwrap();
    let write_ns = u64::try_from(writing.elapsed().as_nanos()).unwrap();
    let draining = Instant::now();
    engine.drain(5000).unwrap();
    let drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
    let pending = engine.read_embedding_readiness().unwrap().pending_count;
    assert_eq!(pending, 0);
    let after = g_write_schedule::current_coverage(&store, &report.default_embedder);
    for (stage, as_of) in [("before", 80), ("inside", 100), ("after", 120)] {
        controls.push(control(engine, stage, as_of));
        measure_stage(engine, config, &directory, (stage, as_of), observer);
    }
    adapter.close();
    assert_eq!(Sha256::digest(std::fs::read(&source).unwrap()), original);
    let proof = json!({"controls":controls,"baseline_coverage":baseline,"after_coverage":after,
        "governed_write_calls":1,"write_rows":1,"write_cursor":receipt.cursor,"write_ns":write_ns,
        "drain_ns":drain_ns,"pending":pending,"selected_cuda_uuid":report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid,
        "scope":"literal public-view controls; write/drain separate; timed hybrid stages recorded separately"});
    std::fs::write(directory.join("controls.json"), serde_json::to_vec_pretty(&proof).unwrap())
        .unwrap();
}
