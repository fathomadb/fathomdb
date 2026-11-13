//! Bounded Slice30 normal G API measurement adapter; see the indexed README.

use fathomdb_embedder::EffectiveEmbedDevice;
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineError, InitialState, OpenReport, OpenedEngine, PreparedWrite,
    ProjectionRole, ProjectionSpec, ProjectionVector, SourceId,
};
use fathomdb_sdk::{Engine as SdkEngine, OpenOptions};
use serde_json::{json, Value};
use std::path::Path;

#[path = "../../../fathomdb-engine/examples/slice30/budget_clock.rs"]
pub(super) mod budget_clock;

#[path = "raw.rs"]
mod raw;
use raw::Recorder;

#[path = "g_template.rs"]
mod g_template;

#[path = "g_write_schedule.rs"]
mod g_write_schedule;

#[path = "g_limit.rs"]
mod g_limit;

#[path = "g_generic.rs"]
mod g_generic;

// Feature-specific method references stay in the private host, never the SDK example.
pub(super) type CoreObserver = (
    fn(&Engine, bool) -> Result<(), EngineError>,
    fn(&Engine) -> Result<Vec<Value>, EngineError>,
    fn(&Engine) -> Result<Value, EngineError>,
);

#[path = "g_gg_stages.rs"]
mod g_gg_stages;

#[path = "g_gg_governed.rs"]
mod g_gg_governed;

#[path = "g_gg_time.rs"]
mod g_gg_time;

#[path = "g_observer.rs"]
mod g_observer;

#[path = "g_observer_controls.rs"]
mod g_observer_controls;

#[path = "g_budget.rs"]
mod g_budget;

enum GAdapter {
    Core(Box<OpenedEngine>),
    Sdk(Box<SdkEngine>),
}

impl GAdapter {
    fn open(mode: &str, path: &Path) -> Self {
        match mode {
            "engine_mean" | "engine_hybrid" | "engine_text" => Self::Core(Box::new(
                Engine::open_with_choice(path, EmbedderChoice::Default).unwrap(),
            )),
            "sdk_mean" | "sdk_hybrid" | "sdk_text" => Self::Sdk(Box::new(
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

    fn hybrid(&self, query: &str, limit: usize) -> fathomdb_engine::SearchResult {
        match self {
            Self::Core(opened) => {
                opened.engine.search_filtered_with_limit(query, None, limit).unwrap()
            }
            Self::Sdk(engine) => {
                let mut options = fathomdb_sdk::SearchOptions::default();
                options.limit = limit;
                engine.search(query, options).unwrap()
            }
        }
    }

    fn inspect_text_fixture(&self, path: &Path) -> Value {
        let pending = match self {
            Self::Core(opened) => opened.engine.read_embedding_readiness().unwrap().pending_count,
            Self::Sdk(engine) => {
                fathomdb_sdk::read::embedding_readiness(engine).unwrap().pending_count
            }
        };
        let connection =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let rows: u64 = connection
            .query_row(
                "SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND superseded_at IS NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let vectors: u64 = connection
            .query_row("SELECT COUNT(*) FROM vector_default", [], |row| row.get(0))
            .unwrap();
        let bodies: Vec<(String,String)> = connection.prepare("SELECT logical_id,body FROM canonical_nodes WHERE superseded_at IS NULL ORDER BY logical_id").unwrap()
            .query_map([],|row|Ok((row.get(0)?,row.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(
            bodies,
            [
                ("alpha".to_string(), "alphaonly evidence".to_string()),
                ("beta".to_string(), "betaonly evidence".to_string())
            ]
        );
        let expression = fathomdb_query::compile_text_query("alphaonly").match_expression;
        let matches: u64 = connection
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1",
                [expression],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!((rows, vectors, pending, matches), (2, 2, 0, 1));
        json!({"rows":rows,"vectors":vectors,"pending":pending,"raw_fts_matches":matches})
    }

    fn prepare_marker_control(&self, path: &Path) -> Value {
        let writes: Vec<_> = (0..10)
            .map(|slot| PreparedWrite::Node {
                kind: "note".into(),
                body: format!("s30markercurrent{slot} revisionreset"),
                source_id: SourceId::new("test:slice30-g-schedule").unwrap(),
                logical_id: Some(format!("s30marker-{slot}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        let started = std::time::Instant::now();
        match self {
            Self::Core(opened) => {
                opened.engine.write(&writes).unwrap();
            }
            Self::Sdk(engine) => {
                engine.write(&writes).unwrap();
            }
        }
        let reset_write_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        let draining = std::time::Instant::now();
        match self {
            Self::Core(opened) => opened.engine.drain(5000).unwrap(),
            Self::Sdk(engine) => engine.drain(5000).unwrap(),
        }
        let reset_drain_ns = u64::try_from(draining.elapsed().as_nanos()).unwrap();
        let connection =
            rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
                .unwrap();
        let rows: Vec<(String,String)> = connection.prepare("SELECT logical_id,body FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL ORDER BY logical_id").unwrap().query_map([],|row| Ok((row.get(0)?,row.get(1)?))).unwrap().collect::<Result<_,_>>().unwrap();
        let expected: Vec<_> = (0..10)
            .map(|slot| {
                (format!("s30marker-{slot}"), format!("s30markercurrent{slot} revisionreset"))
            })
            .collect();
        assert_eq!(rows, expected);
        let counts: Vec<(String, u64)> = connection
            .prepare("SELECT kind,count(*) FROM _fathomdb_vector_rows GROUP BY kind ORDER BY kind")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(counts, [("doc".into(), 2), ("note".into(), 10)]);
        json!({"reset_write_calls":1,"reset_rows":10,"reset_write_ns":reset_write_ns,"reset_drain_ns":reset_drain_ns,"marker_ids":rows.iter().map(|row| &row.0).collect::<Vec<_>>(),"marker_bodies":rows.iter().map(|row| &row.1).collect::<Vec<_>>(),"doc_vectors":2,"note_vectors":10})
    }

    fn text(&self, query: &str, kind: Option<&str>) -> fathomdb_engine::SearchResult {
        let filter = kind.map(|kind| {
            let mut filter = fathomdb_engine::SearchFilter::default();
            filter.kind = Some(kind.into());
            filter
        });
        match self {
            Self::Core(opened) => opened
                .engine
                .search_text_only_filtered_with_snippets(
                    query,
                    filter,
                    &fathomdb_engine::ReadView::default(),
                    10,
                    None,
                )
                .unwrap(),
            Self::Sdk(engine) => {
                let mut options = fathomdb_sdk::TextSearchOptions::default();
                options.filter = filter;
                engine.search_text_only(query, options).unwrap()
            }
        }
    }

    fn timed_text(&self, query: &str, kind: Option<&str>) -> (fathomdb_engine::SearchResult, u64) {
        let started = std::time::Instant::now();
        let result = self.text(query, kind);
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        (result, elapsed_ns)
    }

    fn timed_hybrid(&self, query: &str, limit: usize) -> (fathomdb_engine::SearchResult, u64) {
        let started = std::time::Instant::now();
        let result = self.hybrid(query, limit);
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        (result, elapsed_ns)
    }

    fn timed_hybrid_for_kind(
        &self,
        query: &str,
        limit: usize,
        kind: &str,
    ) -> (fathomdb_engine::SearchResult, u64) {
        let started = std::time::Instant::now();
        let mut filter = fathomdb_engine::SearchFilter::default();
        filter.kind = Some(kind.into());
        let result = match self {
            Self::Core(opened) => {
                opened.engine.search_filtered_with_limit(query, Some(filter), limit).unwrap()
            }
            Self::Sdk(engine) => {
                let mut options = fathomdb_sdk::SearchOptions::default();
                options.limit = limit;
                options.filter = Some(filter.into());
                engine.search(query, options).unwrap()
            }
        };
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        (result, elapsed_ns)
    }

    fn timed_embed(&self, query: &str) -> (Vec<f32>, u64) {
        let started = std::time::Instant::now();
        let vector = self.embed(query);
        let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
        (vector, elapsed_ns)
    }

    fn close(&self) {
        match self {
            Self::Core(opened) => opened.engine.close().unwrap(),
            Self::Sdk(engine) => engine.close().unwrap(),
        }
    }
}

fn text_fixture_ack(uuid: &str) -> u64 {
    use std::io::{Read, Write};
    let started = std::time::Instant::now();
    if writeln!(std::io::stdout(), "READY {} {}", std::process::id(), uuid).is_err()
        || std::io::stdout().flush().is_err()
    {
        std::process::exit(2);
    }
    let (sender, receiver) = std::sync::mpsc::channel();
    // The reader owns no engine state and is never joined after cancellation.
    let _ack_reader = std::thread::spawn(move || {
        let mut ack = [0u8; 4];
        let valid = std::io::stdin().read_exact(&mut ack).is_ok() && ack == *b"ACK\n";
        let _ = sender.send(valid);
    });
    if !matches!(receiver.recv_timeout(std::time::Duration::from_secs(15)), Ok(true)) {
        eprintln!("manual text fixture ACK failed or exceeded15s");
        std::process::exit(2);
    }
    u64::try_from(started.elapsed().as_nanos()).unwrap()
}

pub(super) fn run(config: &Value, out: &Path) {
    run_observed(config, out, None);
}

pub(super) fn run_observed(config: &Value, out: &Path, observer: Option<CoreObserver>) {
    if config["family"] == "GG_GOVERNED_FIXTURE" {
        return g_gg_governed::fixture(
            config,
            out,
            observer.expect("GG requires the private GO artifact"),
        );
    }
    if config["family"] == "GG_TIME_FIXTURE" {
        return g_gg_time::fixture(
            config,
            out,
            observer.expect("GG requires the private GO artifact"),
        );
    }
    if config["family"] == "G1_PLAN_FIXTURE" {
        return g_generic::plan_fixture(config, out, observer);
    }
    if config["family"] == "G1_CELL" {
        return g_generic::cell(
            config,
            out,
            Path::new(config["store"].as_str().unwrap()),
            observer,
        );
    }
    if config["family"] == "G_FEED_FIXTURE" {
        return g_generic::fixture(config, out, observer);
    }
    if config["family"] == "G_PASS" {
        return g_generic::run(config, out, Path::new(config["store"].as_str().unwrap()), observer);
    }
    if config["family"] == "G_LIMIT" {
        return g_limit::run(config, out);
    }
    if config["family"] == "G_TEMPLATE" {
        return g_template::run(config, out);
    }
    let mode = config["mode"].as_str().unwrap();
    let adapter = if mode.starts_with("engine_") { "engine" } else { "sdk" };
    let is_hybrid = matches!(mode, "engine_hybrid" | "sdk_hybrid");
    let is_text = matches!(mode, "engine_text" | "sdk_text");
    let samples = usize::try_from(config["samples"].as_u64().unwrap()).unwrap();
    assert!(samples > 0);
    let query = config["query"].as_str().unwrap();
    std::fs::create_dir_all(out).unwrap();
    let store = out.join("store.sqlite");
    if is_text {
        assert!(store.exists(), "text fixture must be a fresh owned closed-source copy");
    } else {
        assert!(!store.exists(), "source fixture store must be fresh");
    }
    let engine = GAdapter::open(mode, &store);
    let resolution = engine.report().embedder_device_resolution.as_ref().unwrap();
    let device = match &resolution.effective_device {
        EffectiveEmbedDevice::Cuda(device) => device,
        EffectiveEmbedDevice::Cpu => panic!("G requires the reserved CUDA provider"),
    };
    assert_eq!(resolution.selected_cuda_uuid.as_deref(), config["gpu_uuid"].as_str());
    assert_eq!(device.ordinal, 0);
    let fixture = if is_hybrid {
        engine.prepare_hybrid(&store)
    } else if is_text {
        engine.inspect_text_fixture(&store)
    } else {
        Value::Null
    };
    let marker_control = if matches!(config["schedule"].as_str(), Some("readonly" | "write")) {
        assert!(is_text, "initial schedule fixture uses the direct-text population");
        engine.prepare_marker_control(&store)
    } else {
        assert!(config["schedule"].is_null(), "unsupported schedule");
        Value::Null
    };
    let fixture_ack_ns = if is_text && config["text_fixture_handshake"].as_bool() == Some(true) {
        text_fixture_ack(resolution.selected_cuda_uuid.as_deref().unwrap())
    } else {
        0
    };
    let mut recorder = Recorder::new(out);
    let loop_started = std::time::Instant::now();
    let mut calls = 0_usize;
    let mut scheduled_write_calls = 0;
    let mut scheduled_write_rows = 0;
    let mut marker_probes = 0;
    for sample in 0..samples {
        let record_start_ns = (config["schedule"] == "write")
            .then(|| u64::try_from(loop_started.elapsed().as_nanos()).unwrap());
        let mut row = if is_text {
            let (result, elapsed_ns) =
                engine.timed_text(query, (!marker_control.is_null()).then_some("doc"));
            let ids: Vec<_> = result.results.iter().map(|hit| hit.id.to_prefixed()).collect();
            assert_eq!(ids, ["l:alpha"]);
            json!({"adapter":adapter,"mode":"direct_text","ok":true,"sample_index":sample,
                "query_index":0,"elapsed_ns":elapsed_ns,"ids":ids,"returned_hits":result.results.len(),
                "kind_filter":if marker_control.is_null(){Value::Null}else{json!("doc")},
                "search_snapshot":null,"snapshot_observation":"unavailable"})
        } else if is_hybrid {
            let (result, elapsed_ns) = engine.timed_hybrid(query, 2);
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
            let (vector, elapsed_ns) = engine.timed_embed(query);
            assert_eq!(vector.len(), 384);
            assert!(vector.iter().all(|x| x.is_finite()));
            let norm2 = vector.iter().map(|x| f64::from(*x).powi(2)).sum::<f64>();
            assert!(norm2 > 0.0);
            json!({"adapter":adapter,"mode":"mean_embed","ok":true,
            "sample_index":sample,"query_index":0,"elapsed_ns":elapsed_ns,
            "dimension":vector.len(),"finite":true,"norm_squared":norm2})
        };
        if let Some(started) = record_start_ns {
            row["record_start_ns"] = json!(started);
            row["record_ready_ns"] =
                json!(u64::try_from(loop_started.elapsed().as_nanos()).unwrap());
        }
        recorder.emit("samples", &row);
        calls += 1;
        if config["schedule"] == "write" && calls.is_multiple_of(10) {
            let cohort = g_write_schedule::run(&engine, &store, calls, &mut recorder, loop_started);
            scheduled_write_calls += 1;
            scheduled_write_rows += cohort.rows;
            marker_probes += cohort.probes;
        }
    }
    recorder.flush_all();
    let loop_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
    engine.close();
    let schedule = if marker_control.is_null() {
        Value::Null
    } else {
        json!({"mode":config["schedule"],"natural_calls":calls,"natural_kind_filter":"doc",
            "scheduled_write_calls":scheduled_write_calls,"scheduled_write_rows":scheduled_write_rows,"marker_probes":marker_probes})
    };
    let proof = json!({"api_calls":calls,"schedule":schedule,
        "dimension":if is_hybrid || is_text {None} else {Some(384)},
        "finite_nonzero":if is_hybrid || is_text {None} else {Some(true)},
        "marker_preparation":marker_control,"fixture":fixture,"fixture_ack_ns":fixture_ack_ns,"fixture_ids":if is_text {json!(["l:alpha"])} else if is_hybrid {json!(["l:alpha","l:beta"])} else {Value::Null},
        "pooling":if is_text {format!("{adapter} text-only API; query embedding not requested")} else if is_hybrid {format!("{adapter} hybrid API with configured mean query embedding")} else {format!("{adapter} mean API")},"selected_cuda_uuid":resolution.selected_cuda_uuid,
        "cuda_compiled":resolution.cuda_compiled,"cuda_ordinal":device.ordinal,
        "driver_version":device.driver_version,"cuda_toolkit_version":device.cuda_toolkit_version,
        "cuda_libraries":format!("{:?}",device.cuda_libraries),
        "raw_persistence":recorder.statistics_since([0;5]),"loop_ns":loop_ns,
        "throughput_scope":if config["schedule"] == "write" {"natural APIs plus scheduled writes/drains/probes/current verification/serialization/phase checkpoints; reset/open/close separate"} else {"API calls plus validation/serialization/checkpoint loop; open/close separate"}});
    std::fs::write(out.join("api-proof.json"), serde_json::to_vec_pretty(&proof).unwrap()).unwrap();
}
