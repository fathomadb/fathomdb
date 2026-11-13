//! Private closed-template GG entry; source control precedes stage routing.

use super::{g_write_schedule, CoreObserver, GAdapter};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn run(config: &Value, out: &Path, _observer: CoreObserver) {
    std::fs::create_dir_all(out).unwrap();
    let store = Path::new(config["store"].as_str().unwrap());
    let adapter = GAdapter::open("engine_hybrid", store);
    let report = adapter.report();
    assert_eq!(
        report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid.as_deref(),
        config["gpu_uuid"].as_str()
    );
    let GAdapter::Core(opened) = &adapter else { unreachable!() };
    let coverage = g_write_schedule::current_coverage(store, &report.default_embedder);
    let pending = opened.engine.read_embedding_readiness().unwrap().pending_count;
    let vector = adapter.embed("s30ggwindow");
    assert_eq!(vector.len(), 384);
    assert!(vector.iter().all(|value| value.is_finite()));
    let literal = adapter.text("s30ggwindow", None);
    assert!(literal.results.is_empty());
    let proof = json!({"api_complete":true,"selected_cuda_uuid":report.embedder_device_resolution.as_ref().unwrap().selected_cuda_uuid,
        "base_rows":coverage["current_vector_cursors"],"pending":pending,"literal_window_ids":[],
        "scope":"existing real Default/open/readiness/source controls; GG stages are separate"});
    adapter.close();
    std::fs::write(out.join("gg-control.json"), serde_json::to_vec_pretty(&proof).unwrap())
        .unwrap();
}
