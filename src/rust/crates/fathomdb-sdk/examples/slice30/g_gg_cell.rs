//! Private closed-template GG entry; independent time and governed copies.

use super::{g_gg_governed, g_gg_time, CoreObserver};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn run(config: &Value, out: &Path, observer: CoreObserver) {
    std::fs::create_dir_all(out).unwrap();
    let source = Path::new(config["store"].as_str().unwrap());
    g_gg_time::from_template(config, out, source, observer);
    g_gg_governed::from_template(config, out, source, observer);
    let time: Value =
        serde_json::from_slice(&std::fs::read(out.join("time/controls.json")).unwrap()).unwrap();
    let governed: Value =
        serde_json::from_slice(&std::fs::read(out.join("governed/controls.json")).unwrap())
            .unwrap();
    assert_eq!(time["selected_cuda_uuid"], governed["selected_cuda_uuid"]);
    assert_eq!(time["selected_cuda_uuid"], config["gpu_uuid"]);
    assert_eq!((time["pending"].as_u64(), governed["pending"].as_u64()), (Some(0), Some(0)));
    let proof = json!({"api_complete":true,"selected_cuda_uuid":time["selected_cuda_uuid"],
        "base_rows":time["baseline_coverage"]["current_vector_cursors"],"pending":time["pending"],
        "literal_window_ids":time["controls"][0]["ids"],
        "scope":"actual public-view/readiness/lifecycle source controls; GG stage timings separate"});
    std::fs::write(out.join("gg-control.json"), serde_json::to_vec_pretty(&proof).unwrap())
        .unwrap();
}
