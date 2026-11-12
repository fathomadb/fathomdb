//! Bounded Slice30 mean-embedding measurement adapter; see the indexed README.

use fathomdb_embedder::EffectiveEmbedDevice;
use fathomdb_engine::{EmbedderChoice, Engine, OpenReport, OpenedEngine};
use fathomdb_sdk::{Engine as SdkEngine, OpenOptions};
use serde_json::{json, Value};
use std::path::Path;

#[path = "slice30/raw.rs"]
mod raw;
use raw::Recorder;

enum MeanAdapter {
    Core(Box<OpenedEngine>),
    Sdk(Box<SdkEngine>),
}

impl MeanAdapter {
    fn open(mode: &str, path: &Path) -> Self {
        match mode {
            "engine_mean" => Self::Core(Box::new(
                Engine::open_with_choice(path, EmbedderChoice::Default).unwrap(),
            )),
            "sdk_mean" => Self::Sdk(Box::new(
                SdkEngine::open(
                    path,
                    OpenOptions { use_default_embedder: true, ..OpenOptions::default() },
                )
                .unwrap(),
            )),
            _ => panic!("unsupported mean population"),
        }
    }

    fn report(&self) -> &OpenReport {
        match self {
            Self::Core(opened) => &opened.report,
            Self::Sdk(engine) => engine.open_report(),
        }
    }

    fn embed(&self, query: &str) -> Vec<f32> {
        match self {
            Self::Core(opened) => opened.engine.embed_text(query).unwrap(),
            Self::Sdk(engine) => engine.embed(query).unwrap(),
        }
    }

    fn close(&self) {
        match self {
            Self::Core(opened) => opened.engine.close().unwrap(),
            Self::Sdk(engine) => engine.close().unwrap(),
        }
    }
}

fn run(config: &Value, out: &Path) {
    let mode = config["mode"].as_str().unwrap();
    let adapter = if mode == "engine_mean" { "engine" } else { "sdk" };
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    assert!(samples > 0);
    let query = config["query"].as_str().unwrap();
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "source fixture store must be fresh");
    let engine = MeanAdapter::open(mode, &store);
    let resolution = engine.report().embedder_device_resolution.as_ref().unwrap();
    let device = match &resolution.effective_device {
        EffectiveEmbedDevice::Cuda(device) => device,
        EffectiveEmbedDevice::Cpu => panic!("G requires the reserved CUDA provider"),
    };
    assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
    assert_eq!(device.ordinal, 0);
    let mut recorder = Recorder::new(out);
    let loop_started = std::time::Instant::now();
    let mut calls = 0;
    for sample in 0..samples {
        let started = std::time::Instant::now();
        let vector = engine.embed(query);
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        assert_eq!(vector.len(), 384);
        assert!(vector.iter().all(|x| x.is_finite()));
        let norm2 = vector.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
        assert!(norm2 > 0.0);
        let row = json!({"adapter":adapter,"mode":"mean_embed","ok":true,
            "sample_index":sample,"query_index":0,"elapsed_ns":elapsed_ns,
            "dimension":vector.len(),"finite":true,"norm_squared":norm2});
        if mode == "engine_mean" {
            recorder.emit("samples", &row);
        }
        calls += 1;
    }
    recorder.flush_all();
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    engine.close();
    let proof = json!({"api_calls":calls,"dimension":384,"finite_nonzero":true,
        "pooling":format!("{adapter} mean API"),"selected_cuda_uuid":resolution.selected_cuda_uuid,
        "cuda_compiled":resolution.cuda_compiled,"cuda_ordinal":device.ordinal,
        "driver_version":device.driver_version,"cuda_toolkit_version":device.cuda_toolkit_version,
        "cuda_libraries":format!("{:?}",device.cuda_libraries),
        "raw_persistence":recorder.statistics_since([0;5]),"loop_ns":loop_ns,
        "throughput_scope":"API calls plus validation/serialization/checkpoint loop; open/close separate"});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30_g_mean cell.json owned-output");
    let config = serde_json::from_slice(&std::fs::read(&args[1]).unwrap()).unwrap();
    run(&config, Path::new(&args[2]));
}
