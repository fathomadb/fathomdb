//! GG time-copy controls; timed diagnostic stages are separate from literal text proofs.

use super::{g_gg_stages, g_template, g_write_schedule, CoreObserver, GAdapter};
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

pub(super) fn fixture(config: &Value, out: &Path, observer: CoreObserver) {
    assert_eq!(config["templates"].as_array().unwrap().len(), 1);
    g_template::run(config, out);
    let source = out.join("case-0/template.sqlite");
    from_template(config, out, &source, observer);
}

pub(super) fn from_template(
    config: &Value,
    out: &Path,
    source: &Path,
    observer: CoreObserver,
) -> bool {
    let Ok(prepared) = g_gg_stages::prepare_store(config, out, source, "time", |path| {
        GAdapter::open("engine_hybrid", path)
    }) else {
        return false;
    };
    let g_gg_stages::PreparedStore { adapter, original, directory, store } = prepared;
    let report = adapter.report();
    assert_eq!(
        report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid.as_deref(),
        config["gpu_uuid"].as_str()
    );
    let GAdapter::Core(opened) = &adapter else { unreachable!() };
    let engine = &opened.engine;
    let baseline = g_gg_stages::verification(config, out, || {
        g_write_schedule::current_coverage(&store, &report.default_embedder)
    })
    .unwrap();
    let mut controls =
        vec![g_gg_stages::verification(config, out, || control(engine, "baseline", 100)).unwrap()];
    if !g_gg_stages::measure_stage(
        engine,
        config,
        &directory,
        ("baseline", 100, "s30ggwindow"),
        observer,
    ) {
        adapter.close();
        return false;
    }
    let Ok((receipt, write_ns)) = g_gg_stages::mutation(config, out, "write", || {
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
        (receipt, write_ns)
    }) else {
        adapter.close();
        return false;
    };
    let draining = Instant::now();
    let drain_budget = match g_gg_stages::drain(engine, config) {
        Ok(report) => report,
        Err(error) => {
            std::fs::write(
                out.join("budget.json"),
                serde_json::to_vec_pretty(&json!({
                    "phase":"drain","disposition":"phase_failed","error":error
                }))
                .unwrap(),
            )
            .unwrap();
            adapter.close();
            return false;
        }
    };
    let drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
    let pending = g_gg_stages::verification(config, out, || {
        engine.read_embedding_readiness().unwrap().pending_count
    })
    .unwrap();
    assert_eq!(pending, 0);
    let after = g_gg_stages::verification(config, out, || {
        g_write_schedule::current_coverage(&store, &report.default_embedder)
    })
    .unwrap();
    for (stage, as_of) in [("before", 80), ("inside", 100), ("after", 120)] {
        controls.push(
            g_gg_stages::verification(config, out, || control(engine, stage, as_of)).unwrap(),
        );
        if !g_gg_stages::measure_stage(
            engine,
            config,
            &directory,
            (stage, as_of, "s30ggwindow"),
            observer,
        ) {
            adapter.close();
            return false;
        }
    }
    adapter.close();
    let source_hash = g_gg_stages::verification(config, out, || {
        Sha256::digest(std::fs::read(source).unwrap()).to_vec()
    })
    .unwrap();
    assert_eq!(source_hash, original);
    let mut proof = json!({"controls":controls,"baseline_coverage":baseline,"after_coverage":after,
        "governed_write_calls":1,"write_rows":1,"write_cursor":receipt.cursor,"write_ns":write_ns,
        "drain_ns":drain_ns,"pending":pending,"selected_cuda_uuid":report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid,
        "scope":"literal public-view controls; write/drain separate; timed hybrid stages recorded separately"});
    if config["work_deadline_ns"].is_u64() {
        proof["drain_budget"] = drain_budget;
    }
    std::fs::write(directory.join("controls.json"), serde_json::to_vec_pretty(&proof).unwrap())
        .unwrap();
    true
}
