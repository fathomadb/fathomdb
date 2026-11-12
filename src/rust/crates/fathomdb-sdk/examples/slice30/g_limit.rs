//! Bounded real closed-template control for the private caller's retrieval limit.

use super::{g_template, GAdapter};
use fathomdb_embedder::EffectiveEmbedDevice;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn run(config: &Value, out: &Path) {
    assert_eq!(config["templates"].as_array().unwrap().len(), 1);
    g_template::run(config, out);
    let source = out.join("case-0/template.sqlite");
    let source_hash = Sha256::digest(std::fs::read(&source).unwrap());
    let mut rows = Vec::new();
    for mode in ["engine_hybrid", "sdk_hybrid"] {
        let directory = out.join(mode);
        std::fs::create_dir(&directory).unwrap();
        let store = directory.join("store.sqlite");
        std::fs::copy(&source, &store).unwrap();
        assert_eq!(Sha256::digest(std::fs::read(&store).unwrap()), source_hash);
        let engine = GAdapter::open(mode, &store);
        let resolution = engine.report().embedder_device_resolution.as_ref().unwrap();
        assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
        assert!(
            matches!(&resolution.effective_device, EffectiveEmbedDevice::Cuda(device) if device.ordinal == 0)
        );
        let query = config["query"].as_str().unwrap();
        let started = std::time::Instant::now();
        let result = engine.hybrid(query);
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
        rows.push(json!({"mode":mode,"query":query,"requested_limit":config["limit"],
            "ids":ids,"returned_hits":result.results.len(),"elapsed_ns":elapsed_ns,
            "soft_fallback":format!("{:?}",result.soft_fallback),"degradations":format!("{:?}",result.degradations),
            "selected_cuda_uuid":resolution.selected_cuda_uuid,"cuda_compiled":resolution.cuda_compiled}));
        engine.close();
        drop(engine);
        let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
        assert_eq!(Sha256::digest(std::fs::read(&source).unwrap()), source_hash);
    }
    std::fs::write(out.join("limit-proof.json"), serde_json::to_vec_pretty(&rows).unwrap())
        .unwrap();
}
