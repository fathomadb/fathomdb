//! Private generic G single-pass caller, separate from bounded source fixture preparation.

use super::{g_template, GAdapter, Recorder};
use fathomdb_embedder::EffectiveEmbedDevice;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::io::BufRead;
use std::path::Path;

pub(super) fn run(config: &Value, out: &Path, store: &Path) {
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
    let opening = std::time::Instant::now();
    let engine = GAdapter::open(mode, store);
    let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
    let resolution = engine.report().embedder_device_resolution.as_ref().unwrap();
    assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
    assert!(
        matches!(&resolution.effective_device, EffectiveEmbedDevice::Cuda(device) if device.ordinal == 0)
    );
    assert_eq!(engine.report().default_embedder.dimension, 384);
    let adapter = match &engine {
        GAdapter::Core(_) => "engine",
        GAdapter::Sdk(_) => "sdk",
    };
    let mut recorder = Recorder::new(out);
    let loop_started = std::time::Instant::now();
    for sample_index in 0..queries.len() {
        let input = &queries[0];
        let query = input["query"].as_str().unwrap();
        let fingerprint: String =
            Sha256::digest(query.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect();
        let mut row = if mode.ends_with("_text") {
            let (result, elapsed_ns) = engine.timed_text(query, None);
            let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
            json!({"mode":"direct_text","ok":true,"elapsed_ns":elapsed_ns,"ids":ids,
                "returned_hits":result.results.len(),"kind_filter":null,
                "search_snapshot":null,"snapshot_observation":"unavailable"})
        } else if mode.ends_with("_hybrid") {
            let (result, elapsed_ns) = engine.timed_hybrid(query, 10);
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
        row["adapter"] = json!(adapter);
        row["sample_index"] = json!(sample_index);
        row["query_id"] = input["id"].clone();
        row["query_fingerprint"] = json!(fingerprint);
        recorder.emit("samples", &row);
    }
    recorder.flush_all();
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    let closing = std::time::Instant::now();
    engine.close();
    let close_ns = u64::try_from(closing.elapsed().as_nanos()).unwrap();
    let proof = json!({"mode":mode,"api_calls":queries.len(),"open_ns":open_ns,
        "close_ns":close_ns,"loop_ns":loop_ns,"store":store,
        "selected_cuda_uuid":resolution.selected_cuda_uuid,"cuda_compiled":resolution.cuda_compiled,
        "raw_persistence":recorder.statistics_since([0;5]),
        "throughput_scope":"API calls plus input identity/validation/serialization/checkpoints; open/close separate"});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

pub(super) fn fixture(config: &Value, out: &Path) {
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
        run(&pass, &directory, &store);
        let wal = std::path::PathBuf::from(format!("{}-wal", store.display()));
        assert!(std::fs::metadata(wal).map_or(true, |metadata| metadata.len() == 0));
        assert_eq!(Sha256::digest(std::fs::read(&source).unwrap()), original);
    }
}
