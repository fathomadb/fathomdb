//! Bounded Slice30 normal G API measurement adapter; see the indexed README.

use fathomdb_embedder::EffectiveEmbedDevice;
use fathomdb_engine::{
    EmbedderChoice, Engine, InitialState, OpenReport, OpenedEngine, PreparedWrite, ProjectionRole,
    ProjectionSpec, ProjectionVector, SourceId,
};
use fathomdb_sdk::{Engine as SdkEngine, OpenOptions};
use serde_json::{json, Value};
use std::path::Path;

#[path = "slice30/raw.rs"]
mod raw;
use raw::Recorder;

enum GAdapter {
    Core(Box<OpenedEngine>),
    Sdk(Box<SdkEngine>),
}

impl GAdapter {
    fn open(mode: &str, path: &Path) -> Self {
        match mode {
            "engine_mean" | "engine_hybrid" => Self::Core(Box::new(
                Engine::open_with_choice(path, EmbedderChoice::Default).unwrap(),
            )),
            "sdk_mean" | "sdk_hybrid" => Self::Sdk(Box::new(
                SdkEngine::open(
                    path,
                    OpenOptions { use_default_embedder: true, ..OpenOptions::default() },
                )
                .unwrap(),
            )),
            _ => panic!("unsupported G population"),
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

    fn prepare_hybrid(&self, path: &Path) -> Value {
        let started = std::time::Instant::now();
        let specs = [ProjectionSpec {
            name: "s30_dense".into(),
            roles: [ProjectionRole::Searchable].into_iter().collect(),
            fts: None,
            vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
            source: None,
        }];
        match self {
            Self::Core(opened) => {
                opened.engine.configure_projections(&specs, &[]).unwrap();
            }
            Self::Sdk(engine) => {
                engine.configure_projections(&specs, &[]).unwrap();
            }
        }
        let batch: Vec<_> = [("alpha", "alphaonly evidence"), ("beta", "betaonly evidence")]
            .into_iter()
            .map(|(id, body)| PreparedWrite::Node {
                kind: "doc".into(),
                body: body.into(),
                source_id: SourceId::new("test:slice30-g-hybrid").unwrap(),
                logical_id: Some(id.into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        match self {
            Self::Core(opened) => {
                opened.engine.write(&batch).unwrap();
            }
            Self::Sdk(engine) => {
                engine.write(&batch).unwrap();
            }
        }
        let seed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        let draining = std::time::Instant::now();
        match self {
            Self::Core(opened) => opened.engine.drain(5000).unwrap(),
            Self::Sdk(engine) => engine.drain(5000).unwrap(),
        }
        let drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
        let pending = match self {
            Self::Core(opened) => opened.engine.read_embedding_readiness().unwrap().pending_count,
            Self::Sdk(engine) => {
                fathomdb_sdk::read::embedding_readiness(engine).unwrap().pending_count
            }
        };
        let connection =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let count = |sql| connection.query_row(sql, [], |row| row.get::<_, u64>(0)).unwrap();
        let rows = count(
            "SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND superseded_at IS NULL",
        );
        let vectors = count("SELECT COUNT(*) FROM vector_default");
        assert_eq!((rows, vectors, pending), (2, 2, 0));
        json!({"rows":rows,"vectors":vectors,"pending":pending,"seed_ns":seed_ns,"drain_ns":drain_ns})
    }

    fn hybrid(&self, query: &str) -> fathomdb_engine::SearchResult {
        match self {
            Self::Core(opened) => opened.engine.search_filtered_with_limit(query, None, 2).unwrap(),
            Self::Sdk(engine) => {
                let mut options = fathomdb_sdk::SearchOptions::default();
                options.limit = 2;
                engine.search(query, options).unwrap()
            }
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
    let adapter = if mode.starts_with("engine_") { "engine" } else { "sdk" };
    let is_hybrid = matches!(mode, "engine_hybrid" | "sdk_hybrid");
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    assert!(samples > 0);
    let query = config["query"].as_str().unwrap();
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "source fixture store must be fresh");
    let engine = GAdapter::open(mode, &store);
    let resolution = engine.report().embedder_device_resolution.as_ref().unwrap();
    let device = match &resolution.effective_device {
        EffectiveEmbedDevice::Cuda(device) => device,
        EffectiveEmbedDevice::Cpu => panic!("G requires the reserved CUDA provider"),
    };
    assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
    assert_eq!(device.ordinal, 0);
    let fixture = if is_hybrid { engine.prepare_hybrid(&store) } else { Value::Null };
    let mut recorder = Recorder::new(out);
    let loop_started = std::time::Instant::now();
    let mut calls = 0;
    for sample in 0..samples {
        let row = if is_hybrid {
            let started = std::time::Instant::now();
            let result = engine.hybrid(query);
            let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
            let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
            let mut ordered = ids.clone();
            ordered.sort();
            assert_eq!(ordered, ["l:alpha", "l:beta"]);
            assert!(result.soft_fallback.is_none() && result.degradations.is_empty());
            json!({"adapter":adapter,"mode":"hybrid","ok":true,"sample_index":sample,
                "query_index":0,"elapsed_ns":elapsed_ns,"ids":ids,"returned_hits":result.results.len(),
                "soft_fallback":format!("{:?}",result.soft_fallback),
                "degradations":format!("{:?}",result.degradations)})
        } else {
            let started = std::time::Instant::now();
            let vector = engine.embed(query);
            let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
            assert_eq!(vector.len(), 384);
            assert!(vector.iter().all(|x| x.is_finite()));
            let norm2 = vector.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
            assert!(norm2 > 0.0);
            json!({"adapter":adapter,"mode":"mean_embed","ok":true,
            "sample_index":sample,"query_index":0,"elapsed_ns":elapsed_ns,
            "dimension":vector.len(),"finite":true,"norm_squared":norm2})
        };
        if mode != "sdk_hybrid" {
            recorder.emit("samples", &row);
        }
        calls += 1;
    }
    recorder.flush_all();
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    engine.close();
    let proof = json!({"api_calls":calls,
        "dimension":if is_hybrid {None} else {Some(384)},
        "finite_nonzero":if is_hybrid {None} else {Some(true)},
        "fixture":fixture,"fixture_ids":if is_hybrid {json!(["l:alpha","l:beta"])} else {Value::Null},
        "pooling":if is_hybrid {format!("{adapter} hybrid API with configured mean query embedding")} else {format!("{adapter} mean API")},"selected_cuda_uuid":resolution.selected_cuda_uuid,
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
