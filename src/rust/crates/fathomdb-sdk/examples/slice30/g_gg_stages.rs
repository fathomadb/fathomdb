//! Shared private GG measured caller; explicit query/view, same API and observer clocks.

use super::{g_observer, CoreObserver, Recorder};
use fathomdb_engine::ReadView;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::time::Instant;

pub(super) fn measure_stage(
    engine: &fathomdb_engine::Engine,
    config: &Value,
    directory: &Path,
    stage: (&str, i64, &str),
    observer: CoreObserver,
) {
    let (stage, as_of, query) = stage;
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
