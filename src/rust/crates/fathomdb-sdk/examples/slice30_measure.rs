//! Current Slice30 engine/SDK measurement adapter; P remains provider-free. See the committed
//! dev/experiments/0.8.29-slice30-measurements/README.md for commands and scope.

use fathomdb_engine::{EmbedderChoice, Engine as CoreEngine, ReadView, SearchFilter};
use fathomdb_sdk::{
    Engine as SdkEngine, InitialState, OpenOptions, PreparedWrite, SourceId, TextSearchOptions,
};
use serde_json::{json, Value};
use std::path::Path;

fn g_provider_choice(provider: &str) -> EmbedderChoice {
    match provider {
        "none" => EmbedderChoice::None,
        "default" => EmbedderChoice::Default,
        _ => panic!("unknown G provider choice"),
    }
}

enum Adapter {
    Core(Box<CoreEngine>),
    Sdk(Box<SdkEngine>),
}

impl Adapter {
    fn open(adapter: &str, path: &Path) -> Self {
        match adapter {
            "engine" => Self::Core(Box::new(
                CoreEngine::open_with_choice(path, EmbedderChoice::None)
                    .expect("open core without provider")
                    .engine,
            )),
            "sdk" => Self::Sdk(Box::new(
                SdkEngine::open(path, OpenOptions::default()).expect("open SDK without provider"),
            )),
            _ => panic!("unknown adapter"),
        }
    }

    fn open_g(adapter: &str, path: &Path, provider: &str) -> Self {
        let choice = g_provider_choice(provider);
        match adapter {
            "engine" => Self::Core(Box::new(
                CoreEngine::open_with_choice(path, choice).expect("G core open").engine,
            )),
            "sdk" => Self::Sdk(Box::new(
                SdkEngine::open(
                    path,
                    OpenOptions {
                        use_default_embedder: matches!(choice, EmbedderChoice::Default),
                        ..OpenOptions::default()
                    },
                )
                .expect("G SDK open"),
            )),
            _ => panic!("unknown G adapter"),
        }
    }

    #[cfg(feature = "slice30-observer")]
    fn begin_observation(&self) -> fathomdb_engine::Slice30ObservationGuard {
        match self {
            Self::Core(engine) => {
                engine.begin_slice30_observation_for_test(512).expect("core observer")
            }
            Self::Sdk(engine) => {
                engine.begin_slice30_observation_for_test(512).expect("SDK observer")
            }
        }
    }

    fn write(&self, batch: &[PreparedWrite]) -> u64 {
        match self {
            Self::Core(engine) => engine.write(batch).expect("core write").cursor,
            Self::Sdk(engine) => engine.write(batch).expect("SDK write").cursor,
        }
    }

    fn close(&self) {
        match self {
            Self::Core(engine) => engine.close().expect("core close"),
            Self::Sdk(engine) => engine.close().expect("SDK close"),
        }
    }
}

fn node(id: &str, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new("test:slice30").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn observe(engine: &Adapter, query: &str, kind: Option<&str>) -> Value {
    let filter = kind.map(|kind| {
        let mut filter = SearchFilter::default();
        filter.kind = Some(kind.into());
        filter
    });
    let started = std::time::Instant::now();
    let result = match engine {
        Adapter::Core(engine) => engine
            .search_text_only_filtered_with_snippets(query, filter, &ReadView::default(), 10, None)
            .map_err(|error| format!("{error:?}")),
        Adapter::Sdk(engine) => {
            let mut options = TextSearchOptions::default();
            options.filter = filter;
            engine.search_text_only(query, options).map_err(|error| format!("{error:?}"))
        }
    };
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).expect("bounded sample duration");
    match result {
        Ok(result) => json!({"ok": true, "elapsed_ns": elapsed_ns,
            "ids": result.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>(),
            "bodies": result.results.iter().map(|hit| &hit.body).collect::<Vec<_>>(),
            "returned_hits": result.results.len(), "projection_cursor": result.projection_cursor,
            "soft_fallback": format!("{:?}", result.soft_fallback), "degradations": format!("{:?}", result.degradations),
            "search_snapshot": null, "snapshot_observation": "unavailable"}),
        Err(error) => json!({"ok": false, "elapsed_ns": elapsed_ns, "ids": [], "error": error}),
    }
}

fn observe_g_route(engine: &Adapter, route: &str, query: &str) -> Value {
    // Keep dispatch allocation-free inside the outer route wall interval.
    #[allow(clippy::large_enum_variant)]
    enum Output {
        Search(fathomdb_engine::SearchResult),
        Embed(Vec<f32>),
    }
    let started = std::time::Instant::now();
    let result: fathomdb_sdk::Result<Output> = match (engine, route) {
        (Adapter::Core(engine), "text") => {
            engine.search_text_only(query).map(Output::Search).map_err(Into::into)
        }
        (Adapter::Sdk(engine), "text") => {
            engine.search_text_only(query, TextSearchOptions::default()).map(Output::Search)
        }
        (Adapter::Core(engine), "hybrid") => {
            engine.search(query).map(Output::Search).map_err(Into::into)
        }
        (Adapter::Sdk(engine), "hybrid") => {
            engine.search(query, fathomdb_sdk::SearchOptions::default()).map(Output::Search)
        }
        (Adapter::Core(engine), "embed") => {
            engine.embed_text(query).map(Output::Embed).map_err(Into::into)
        }
        (Adapter::Sdk(engine), "embed") => engine.embed(query).map(Output::Embed),
        _ => panic!("unknown G route"),
    };
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    match result {
        Ok(Output::Search(result)) => json!({"route":route,"ok":true,"elapsed_ns":elapsed_ns,
            "ids":result.results.iter().map(|hit|hit.id.to_prefixed()).collect::<Vec<_>>(),
            "bodies":result.results.iter().map(|hit|&hit.body).collect::<Vec<_>>(),
            "returned_hits":result.results.len(),"projection_cursor":result.projection_cursor,
            "soft_fallback":format!("{:?}",result.soft_fallback),
            "degradations":format!("{:?}",result.degradations)}),
        Ok(Output::Embed(vector)) => json!({"route":route,"ok":true,"elapsed_ns":elapsed_ns,
            "dimension":vector.len(),"finite":vector.iter().all(|x|x.is_finite())}),
        Err(error) => json!({"route":route,"ok":false,"elapsed_ns":elapsed_ns,
            "error_kind":error.kind().name(),"error":format!("{error:?}")}),
    }
}

fn g_fixture_observation(engine: &Adapter, path: &Path) -> Value {
    let readiness = match engine {
        Adapter::Core(engine) => {
            engine.read_embedding_readiness().map_err(fathomdb_sdk::Error::from)
        }
        Adapter::Sdk(engine) => fathomdb_sdk::read::embedding_readiness(engine),
    };
    let generation = match engine {
        Adapter::Core(engine) => {
            engine.read_projection_generation_status().map_err(fathomdb_sdk::Error::from)
        }
        Adapter::Sdk(engine) => fathomdb_sdk::read::projection_generation_status(engine),
    };
    let (readiness, generation) = match (readiness, generation) {
        (Ok(readiness), Ok(generation)) => (readiness, generation),
        (readiness, generation) => {
            return json!({"qualified_dense_fixture":false,
            "readiness_error":format!("{readiness:?}"),"generation_error":format!("{generation:?}")})
        }
    };
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open owned G fixture for separate untimed counts");
    let count = |sql| connection.query_row(sql, [], |row| row.get::<_, u64>(0)).ok();
    let natural = count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND superseded_at IS NULL AND state='active'");
    let vectors = count("SELECT COUNT(*) FROM _fathomdb_vector_rows");
    let spans = count("SELECT COUNT(*) FROM _fathomdb_vector_spans");
    let physical = count("SELECT COUNT(*) FROM vector_default");
    let format: Option<u32> =
        connection.pragma_query_value(None, "user_version", |row| row.get(0)).ok();
    let qualified = readiness.usable_embedder
        && readiness.pending_count == 0
        && readiness.state == fathomdb_engine::EmbeddingReadinessState::Ready
        && generation.readiness == fathomdb_engine::ProjectionReadinessV1::Ready
        && generation.failed_count == 0
        && natural.is_some_and(|n| n > 0)
        && vectors == natural
        && spans == natural
        && physical == natural
        && format == Some(37);
    json!({"natural_records":natural,"vector_rows":vectors,"span_rows":spans,
        "physical_vector_rows":physical,"format":format,
        "pending_count":readiness.pending_count,"readiness":readiness.state.as_str(),
        "usable_embedder":readiness.usable_embedder,"generation_id":generation.generation_id.as_str(),
        "generation_readiness":generation.readiness.as_str(),"generation_pending":generation.pending_count,
        "generation_failed":generation.failed_count,"qualified_dense_fixture":qualified,
        "observation_scope":"adjacent public readiness/generation and separate count transactions; not a search snapshot"})
}

fn g_readiness_observation(engine: &Adapter) -> Value {
    let started = std::time::Instant::now();
    let result = match engine {
        Adapter::Core(engine) => engine.read_embedding_readiness().map_err(Into::into),
        Adapter::Sdk(engine) => fathomdb_sdk::read::embedding_readiness(engine),
    };
    let elapsed_ns = u64::try_from(started.elapsed().as_nanos()).unwrap();
    match result {
        Ok(ready) => json!({"available":true,"elapsed_ns":elapsed_ns,
            "pending_count":ready.pending_count,"usable_embedder":ready.usable_embedder,
            "state":ready.state.as_str(),"observation_scope":"adjacent public readiness; not query snapshot"}),
        Err(error) => json!({"available":false,"elapsed_ns":elapsed_ns,
            "pending_count":null,"usable_embedder":null,"state":null,
            "error_kind":error.kind().name(),"error":format!("{error:?}"),
            "observation_scope":"adjacent public readiness; not query snapshot"}),
    }
}

struct Schedule {
    searches: Vec<Value>,
    batches: Vec<Value>,
    probes: Vec<Value>,
}

struct GSchedule {
    samples: usize,
    batch_size: usize,
    searches_per_batch: usize,
}

fn g_point_probe(engine: &Adapter, id: &str, expected_body: &str) -> Value {
    let start = std::time::Instant::now();
    let result = match engine {
        Adapter::Core(engine) => engine.read_get(id, &ReadView::default()).map_err(Into::into),
        Adapter::Sdk(engine) => fathomdb_sdk::read::get(engine, id, None),
    };
    let elapsed_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    match result {
        Ok(Some(record)) => json!({"elapsed_ns":elapsed_ns,"body":record.body,
            "write_cursor":record.write_cursor,"current":record.logical_id==id && record.body==expected_body}),
        Ok(None) => json!({"elapsed_ns":elapsed_ns,"body":null,"current":false}),
        Err(error) => json!({"elapsed_ns":elapsed_ns,"current":false,"error":format!("{error:?}")}),
    }
}

#[cfg(test)]
fn run_g_schedule_observed(
    engine: &Adapter,
    route: &str,
    queries: &[String],
    original_bodies: &[String],
    schedule: GSchedule,
    observer: &mut dyn FnMut(&str, &Value),
) -> Schedule {
    run_g_schedule_with_calls(
        engine,
        route,
        queries,
        original_bodies,
        schedule,
        observer,
        &mut |route, query, _phase| (observe_g_route(engine, route, query), None),
    )
}

type GRouteCall<'a> = dyn FnMut(&str, &str, &str) -> (Value, Option<Value>) + 'a;

fn run_g_schedule_with_calls(
    engine: &Adapter,
    route: &str,
    queries: &[String],
    original_bodies: &[String],
    schedule: GSchedule,
    observer: &mut dyn FnMut(&str, &Value),
    call: &mut GRouteCall<'_>,
) -> Schedule {
    assert!(!queries.is_empty() && schedule.searches_per_batch > 0);
    assert!(original_bodies.len() >= 10);
    assert!(schedule.batch_size == 1 || schedule.batch_size == 10);
    let mut output = Schedule { searches: vec![], batches: vec![], probes: vec![] };
    for sample in 0..schedule.samples {
        if sample % schedule.searches_per_batch == 0 {
            let cohort = sample / schedule.searches_per_batch;
            let slots =
                if schedule.batch_size == 1 { vec![cohort % 10] } else { (0..10).collect() };
            let replacements = slots
                .iter()
                .map(|slot| {
                    let id = format!("s30doc-{slot}");
                    let token = format!("s30gmarker{slot}revision{cohort}");
                    let body = format!("{} {token}", original_bodies[*slot]);
                    (id, token, body)
                })
                .collect::<Vec<_>>();
            let writes =
                replacements.iter().map(|(id, _, body)| node(id, "doc", body)).collect::<Vec<_>>();
            let start = std::time::Instant::now();
            let cursor = engine.write(&writes);
            let write_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
            let mut batch = json!({"before_sample":sample,"cohort_index":cohort,"rows":writes.len(),
                "write_ns":write_ns,"after_write_cursor":cursor,
                "affected_ids":replacements.iter().map(|(id,_,_)|id).collect::<Vec<_>>(),
                "revision_tokens":replacements.iter().map(|(_,token,_)|token).collect::<Vec<_>>(),
                "authority_observation":"governed write receipt only; not a search snapshot"});
            batch["adjacent_readiness"] = g_readiness_observation(engine);
            observer("batches", &batch);
            output.batches.push(batch);
            for (id, token, body) in replacements {
                let point = g_point_probe(engine, &id, &body);
                let (text, trace) = call("text", &token, "marker_probe");
                if let Some(mut trace) = trace {
                    trace["before_sample"] = json!(sample);
                    trace["cohort_index"] = json!(cohort);
                    observer("observations", &trace);
                }
                let current = point["current"] == true
                    && text["ids"] == json!([format!("l:{id}")])
                    && text["bodies"] == json!([body]);
                let probe = json!({"before_sample":sample,"cohort_index":cohort,
                    "logical_id":id,"revision_token":token,"point_probe":point,
                    "text_probe":text,"marker_current":current});
                observer("g-marker-probes", &probe);
                output.probes.push(probe);
            }
        }
        let (mut row, trace) = call(route, &queries[sample % queries.len()], "measured");
        if let Some(mut trace) = trace {
            trace["sample_index"] = json!(sample);
            trace["query_index"] = json!(sample % queries.len());
            observer("observations", &trace);
        }
        row["sample_index"] = json!(sample);
        row["query_index"] = json!(sample % queries.len());
        observer("samples", &row);
        output.searches.push(row);
    }
    output
}

#[cfg(test)]
fn run_schedule(
    engine: &Adapter,
    queries: &[String],
    samples: usize,
    batch_size: usize,
    searches_per_batch: usize,
) -> Schedule {
    run_schedule_observed(engine, queries, samples, batch_size, searches_per_batch, &mut |_, _| {})
}

fn run_schedule_observed(
    engine: &Adapter,
    queries: &[String],
    samples: usize,
    batch_size: usize,
    searches_per_batch: usize,
    observer: &mut dyn FnMut(&str, &Value),
) -> Schedule {
    assert!(!queries.is_empty() && searches_per_batch > 0);
    assert!(batch_size == 1 || batch_size == 10);
    let mut output = Schedule { searches: vec![], batches: vec![], probes: vec![] };
    for sample in 0..samples {
        if sample % searches_per_batch == 0 {
            let batch = sample / searches_per_batch;
            let slots: Vec<usize> =
                if batch_size == 1 { vec![batch % 10] } else { (0..10).collect() };
            let writes: Vec<_> = slots
                .iter()
                .map(|slot| {
                    node(
                        &format!("s30marker-{slot}"),
                        "note",
                        &format!("s30markercurrent{slot} revision{batch}"),
                    )
                })
                .collect();
            let start = std::time::Instant::now();
            let cursor = engine.write(&writes);
            output.batches.push(json!({"before_sample": sample, "rows": writes.len(),
                "write_ns": u64::try_from(start.elapsed().as_nanos()).unwrap(),
                "after_write_cursor": cursor, "visibility_generation": null,
                "authority_observation": "write receipt only; search snapshot unavailable"}));
            observer("batches", output.batches.last().unwrap());
            for slot in slots {
                let mut probe = observe(engine, &format!("s30markercurrent{slot}"), Some("note"));
                probe["marker_current"] = json!(
                    probe["ids"] == json!([format!("l:s30marker-{slot}")])
                        && probe["bodies"]
                            == json!([format!("s30markercurrent{slot} revision{batch}")])
                );
                probe["before_sample"] = json!(sample);
                observer("marker-probes", &probe);
                output.probes.push(probe);
            }
        }
        let mut result = observe(engine, &queries[sample % queries.len()], Some("doc"));
        result["sample_index"] = json!(sample);
        result["query_index"] = json!(sample % queries.len());
        observer("samples", &result);
        output.searches.push(result);
    }
    output
}

fn raw_match_count(path: &Path, query: &str, kind: Option<&str>) -> u64 {
    let expression = fathomdb_query::compile_text_query(query).match_expression;
    let connection =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("open own fixture for untimed match counts");
    if let Some(kind) = kind {
        connection
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1 AND kind=?2",
                rusqlite::params![expression, kind],
                |row| row.get(0),
            )
            .expect("filtered raw FTS count")
    } else {
        connection
            .query_row(
                "SELECT COUNT(*) FROM search_index WHERE search_index MATCH ?1",
                [expression],
                |row| row.get(0),
            )
            .expect("raw FTS count")
    }
}

fn read_jsonl(path: &str) -> Vec<String> {
    std::fs::read_to_string(path)
        .expect("read owned JSONL input")
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON string line"))
        .collect()
}

const CHECKPOINT_RECORDS: u64 = 16;

struct RawStream {
    writer: std::io::BufWriter<std::fs::File>,
    records: u64,
    bytes: u64,
    pending: u64,
    checkpoints: u64,
}

struct Recorder {
    streams: std::collections::BTreeMap<&'static str, RawStream>,
    elapsed_ns: u64,
    checkpoint_ns: u64,
}

impl Recorder {
    fn new(out: &Path) -> Self {
        let streams = ["samples", "warmups", "batches", "marker-probes", "matches"]
            .into_iter()
            .map(|name| {
                (
                    name,
                    RawStream {
                        writer: std::io::BufWriter::new(
                            std::fs::File::create(out.join(format!("{name}.jsonl")))
                                .expect("create raw output"),
                        ),
                        records: 0,
                        bytes: 0,
                        pending: 0,
                        checkpoints: 0,
                    },
                )
            })
            .collect();
        Self { streams, elapsed_ns: 0, checkpoint_ns: 0 }
    }

    fn new_g(out: &Path) -> Self {
        let mut recorder = Self::new(out);
        for name in ["observations", "g-marker-probes", "first-calls"] {
            recorder.streams.insert(
                name,
                RawStream {
                    writer: std::io::BufWriter::new(
                        std::fs::File::create(out.join(format!("{name}.jsonl")))
                            .expect("create G raw output"),
                    ),
                    records: 0,
                    bytes: 0,
                    pending: 0,
                    checkpoints: 0,
                },
            );
        }
        recorder
    }

    fn checkpoint(stream: &mut RawStream) -> u64 {
        use std::io::Write;
        if stream.pending == 0 {
            return 0;
        }
        let started = std::time::Instant::now();
        stream.writer.flush().expect("checkpoint raw records");
        stream.pending = 0;
        stream.checkpoints += 1;
        u64::try_from(started.elapsed().as_nanos()).unwrap()
    }

    fn emit(&mut self, name: &str, row: &Value) {
        use std::io::Write;
        let started = std::time::Instant::now();
        let stream = self.streams.get_mut(name).expect("known raw stream");
        let mut encoded = serde_json::to_vec(row).expect("encode raw record");
        encoded.push(b'\n');
        stream.writer.write_all(&encoded).expect("append raw record");
        stream.records += 1;
        stream.bytes += u64::try_from(encoded.len()).unwrap();
        stream.pending += 1;
        if stream.pending == CHECKPOINT_RECORDS {
            self.checkpoint_ns += Self::checkpoint(stream);
        }
        self.elapsed_ns += u64::try_from(started.elapsed().as_nanos()).unwrap();
    }

    fn flush_all(&mut self) {
        let started = std::time::Instant::now();
        for stream in self.streams.values_mut() {
            self.checkpoint_ns += Self::checkpoint(stream);
        }
        self.elapsed_ns += u64::try_from(started.elapsed().as_nanos()).unwrap();
    }

    fn totals(&self) -> [u64; 5] {
        [
            self.streams.values().map(|s| s.records).sum(),
            self.streams.values().map(|s| s.bytes).sum(),
            self.streams.values().map(|s| s.checkpoints).sum(),
            self.elapsed_ns,
            self.checkpoint_ns,
        ]
    }

    fn statistics_since(&self, before: [u64; 5]) -> Value {
        let after = self.totals();
        json!({"records":after[0]-before[0],"encoded_bytes":after[1]-before[1],
            "checkpoints":after[2]-before[2],"persistence_ns":after[3]-before[3],
            "checkpoint_ns":after[4]-before[4],"checkpoint_records":CHECKPOINT_RECORDS,
            "policy":"per stream every16 records and phase/pass boundaries; flush without fsync",
            "retention":"between completed checkpoints at most15 tail records per stream are uncheckpointed; an interrupted pending flush may be partial"})
    }
}

fn run_cell(config: &Value, out: &Path) -> Value {
    run_cell_observed(config, out, &mut |_, _| {})
}

fn run_cell_observed(config: &Value, out: &Path, observer: &mut dyn FnMut(&str, &Value)) -> Value {
    let number =
        |key: &str| usize::try_from(config[key].as_u64().expect(key)).expect("usize bound");
    let n = number("n");
    let warmup = number("warmup");
    let samples = number("samples");
    let corpus = read_jsonl(config["corpus"].as_str().expect("corpus path"));
    let queries = read_jsonl(config["queries"].as_str().expect("queries path"));
    assert!(n > 0 && corpus.len() >= n && !queries.is_empty() && samples > 0);
    let filtered = config["family"] == "P2";
    assert!(filtered || config["family"] == "P1");
    std::fs::create_dir_all(out).expect("create owned cell output");
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "measurement store must be fresh");
    let open_started = std::time::Instant::now();
    let seed = Adapter::open("sdk", &store);
    let seed_open_ns = u64::try_from(open_started.elapsed().as_nanos()).unwrap();
    let write_started = std::time::Instant::now();
    for start in (0..n).step_by(1024) {
        let batch: Vec<_> = corpus[start..n.min(start + 1024)]
            .iter()
            .enumerate()
            .map(|(offset, body)| node(&format!("s30doc-{}", start + offset), "doc", body))
            .collect();
        seed.write(&batch);
    }
    if filtered {
        let markers: Vec<_> = (0..10)
            .map(|slot| node(&format!("s30marker-{slot}"), "note", "initial marker"))
            .collect();
        seed.write(&markers);
    }
    let seed_write_ns = u64::try_from(write_started.elapsed().as_nanos()).unwrap();
    seed.close();
    drop(seed);
    let mut rows = Vec::new();
    let mut warmups = Vec::new();
    let mut batches = Vec::new();
    let mut probes = Vec::new();
    let mut passes = Vec::new();
    let mut recorder = Recorder::new(out);
    for adapter in config["adapters"].as_array().expect("adapter list") {
        let name = adapter.as_str().expect("adapter name");
        let opening = std::time::Instant::now();
        let engine = Adapter::open(name, &store);
        let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
        let persistence_before = recorder.totals();
        for sample in 0..warmup {
            let mut row =
                observe(&engine, &queries[sample % queries.len()], filtered.then_some("doc"));
            row["adapter"] = json!(name);
            row["query_index"] = json!(sample % queries.len());
            recorder.emit("warmups", &row);
            observer("warmups", &row);
            warmups.push(row);
        }
        recorder.flush_all();
        let loop_started = std::time::Instant::now();
        let mut measured = {
            let mut publish = |stream: &str, row: &Value| {
                let mut record = row.clone();
                record["adapter"] = json!(name);
                record["repeat"] = config["repeat"].clone();
                recorder.emit(stream, &record);
                observer(stream, &record);
            };
            if filtered {
                let schedule = run_schedule_observed(
                    &engine,
                    &queries,
                    samples,
                    number("batch_size"),
                    number("searches_per_batch"),
                    &mut publish,
                );
                batches.extend(schedule.batches);
                probes.extend(schedule.probes);
                schedule.searches
            } else {
                (0..samples)
                    .map(|sample| {
                        let mut row = observe(&engine, &queries[sample % queries.len()], None);
                        row["sample_index"] = json!(sample);
                        row["query_index"] = json!(sample % queries.len());
                        publish("samples", &row);
                        row
                    })
                    .collect()
            }
        };
        recorder.flush_all();
        let wall_ns = u64::try_from(loop_started.elapsed().as_nanos()).unwrap();
        for (sample, row) in measured.iter_mut().enumerate() {
            row["adapter"] = json!(name);
            row["repeat"] = config["repeat"].clone();
            row["sample_index"] = json!(sample);
            row["query_index"] = json!(sample % queries.len());
        }
        rows.extend(measured);
        let closing = std::time::Instant::now();
        engine.close();
        drop(engine);
        passes.push(json!({"adapter":name,"wall_ns":wall_ns,"open_ns":open_ns,
            "close_ns":u64::try_from(closing.elapsed().as_nanos()).unwrap(),
            "persistence_warmup_and_timed":recorder.statistics_since(persistence_before),
            "throughput_scope":if filtered {"natural searches over write/probe/search/serialization/checkpoint loop"} else {"natural search/serialization/checkpoint loop"}}));
    }
    for (index, query) in queries.iter().enumerate() {
        let row = json!({"query_index":index,
            "raw_fts_matches":raw_match_count(&store, query, filtered.then_some("doc"))});
        recorder.emit("matches", &row);
        observer("matches", &row);
    }
    recorder.flush_all();
    let connection =
        rusqlite::Connection::open_with_flags(&store, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .expect("read own post-cell counts");
    let count = |sql| connection.query_row(sql, [], |row| row.get::<_, u64>(0)).unwrap();
    let natural =
        count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='doc' AND superseded_at IS NULL");
    let markers =
        count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='note' AND superseded_at IS NULL");
    let vectors = count("SELECT COUNT(*) FROM _fathomdb_vector_rows");
    let format: u32 =
        connection.pragma_query_value(None, "user_version", |row| row.get(0)).unwrap();
    let complete = natural == n as u64
        && vectors == 0
        && format == 37
        && markers == if filtered { 10 } else { 0 }
        && rows.iter().chain(&warmups).all(|row| row["ok"] == true)
        && probes.iter().all(|row| row["marker_current"] == true);
    let result = json!({"complete":complete,"sample_count":rows.len(),"warmup_count":warmups.len(),
        "filtered":filtered,"natural_records":natural,"marker_records":markers,"vector_rows":vectors,
        "format":format,"query_count":queries.len(),"batch_count":batches.len(),"marker_probe_count":probes.len(),
        "seed_open_ns":seed_open_ns,"seed_write_ns":seed_write_ns,"passes":passes,
        "raw_persistence":recorder.statistics_since([0; 5]),
        "snapshot_observation":"unavailable; write receipts are not search snapshot authority",
        "cache_note":"P2 marker proof calls intervene between each write and natural searches; OS caches uncontrolled"});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

#[cfg(feature = "slice30-observer")]
type GObservation = Option<fathomdb_engine::Slice30ObservationGuard>;
#[cfg(not(feature = "slice30-observer"))]
struct GObservation;

fn begin_g_observation(engine: &Adapter, enabled: bool) -> GObservation {
    #[cfg(feature = "slice30-observer")]
    {
        enabled.then(|| engine.begin_observation())
    }
    #[cfg(not(feature = "slice30-observer"))]
    {
        let _ = engine;
        assert!(!enabled, "G diagnostic collection requires slice30-observer build");
        GObservation
    }
}

fn observe_g_operation(
    engine: &Adapter,
    route: &str,
    query: &str,
    operation: u64,
    observation: &GObservation,
) -> (Value, Option<Value>) {
    #[cfg(feature = "slice30-observer")]
    let mut row = if let Some(observation) = observation {
        observation.with_operation(operation, || observe_g_route(engine, route, query))
    } else {
        observe_g_route(engine, route, query)
    };
    #[cfg(not(feature = "slice30-observer"))]
    let mut row = {
        let _ = observation;
        observe_g_route(engine, route, query)
    };
    row["operation_id"] = json!(operation);
    #[cfg(feature = "slice30-observer")]
    let trace = observation.as_ref().map(|observation| {
        serde_json::to_value(observation.take()).expect("encode actual G batch")
    });
    #[cfg(not(feature = "slice30-observer"))]
    let trace = None;
    (row, trace)
}

fn run_g_read(config: &Value, out: &Path) -> Value {
    let provider = config["provider"].as_str().expect("G provider");
    let negative = config["negative_control"] == true;
    assert_eq!(provider, if negative { "none" } else { "default" });
    if !negative {
        assert_eq!(
            std::env::var("AGENT_LONG").as_deref(),
            Ok("1"),
            "Default runs require long opt-in"
        );
    }
    let store = Path::new(config["store"].as_str().expect("existing G store"));
    assert!(store.is_file());
    assert!(!out.exists(), "G read output must be fresh");
    std::fs::create_dir_all(out).expect("owned G read output");
    let queries = read_jsonl(config["queries"].as_str().expect("G queries"));
    assert!(!queries.is_empty());
    let number = |key: &str| usize::try_from(config[key].as_u64().expect("G call count")).unwrap();
    let samples = number("samples");
    let warmup = number("warmup");
    assert!(samples > 0);
    let observed = config["observe"] == true;
    let record_first = config
        .get("record_first_call")
        .map(|value| value.as_bool().expect("boolean G first-call policy"))
        .unwrap_or(true);
    let mut recorder = Recorder::new_g(out);
    let mut rows = Vec::new();
    let mut warmups = Vec::new();
    let mut first_calls = Vec::new();
    let mut traces = Vec::new();
    let mut fixtures = Vec::new();
    let mut passes = Vec::new();
    let mut operation = 1_u64;
    for adapter in config["adapters"].as_array().expect("G adapters") {
        let name = adapter.as_str().unwrap();
        let opening = std::time::Instant::now();
        let engine = Adapter::open_g(name, store, provider);
        let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
        let fixture = g_fixture_observation(&engine, store);
        assert!(
            negative || fixture["qualified_dense_fixture"] == true,
            "G read requires selected qualified dense store"
        );
        fixtures.push(fixture);
        let observation = begin_g_observation(&engine, observed);
        for route in config["routes"].as_array().expect("G routes") {
            let route = route.as_str().unwrap();
            let mut publish = |stream: &'static str, phase: &str, index: usize| {
                let (mut row, trace) = observe_g_operation(
                    &engine,
                    route,
                    &queries[index % queries.len()],
                    operation,
                    &observation,
                );
                row["adapter"] = json!(name);
                row["repeat"] = config["repeat"].clone();
                row["sample_index"] = json!(index);
                row["query_index"] = json!(index % queries.len());
                row["phase"] = json!(phase);
                recorder.emit(stream, &row);
                if let Some(mut trace) = trace {
                    trace["adapter"] = json!(name);
                    trace["route"] = json!(route);
                    trace["phase"] = json!(phase);
                    trace["sample_index"] = json!(index);
                    trace["operation_id"] = json!(operation);
                    recorder.emit("observations", &trace);
                    traces.push(trace);
                }
                operation += 1;
                row
            };
            if record_first {
                first_calls.push(publish("first-calls", "first_call", 0));
            }
            for index in 0..warmup {
                warmups.push(publish("warmups", "warmup", index));
            }
            // End phase checkpoints before starting the measured loop wall.
            recorder.flush_all();
            let measured = std::time::Instant::now();
            for index in 0..samples {
                let (mut row, trace) = observe_g_operation(
                    &engine,
                    route,
                    &queries[index % queries.len()],
                    operation,
                    &observation,
                );
                row["adapter"] = json!(name);
                row["repeat"] = config["repeat"].clone();
                row["sample_index"] = json!(index);
                row["query_index"] = json!(index % queries.len());
                row["phase"] = json!("measured");
                recorder.emit("samples", &row);
                if let Some(mut trace) = trace {
                    trace["adapter"] = json!(name);
                    trace["route"] = json!(route);
                    trace["phase"] = json!("measured");
                    trace["sample_index"] = json!(index);
                    trace["operation_id"] = json!(operation);
                    recorder.emit("observations", &trace);
                    traces.push(trace);
                }
                operation += 1;
                rows.push(row);
            }
            recorder.flush_all();
            passes.push(json!({"adapter":name,"route":route,"open_ns":open_ns,
                "wall_ns":u64::try_from(measured.elapsed().as_nanos()).unwrap(),
                "requests":samples,"collector":observed,
                "throughput_scope":"natural route loop includes recording/checkpoint I/O and enabled diagnostics"}));
        }
        #[cfg(feature = "slice30-observer")]
        drop(observation);
        engine.close();
        drop(engine);
    }
    recorder.flush_all();
    let outcomes_ok = rows.iter().chain(&warmups).chain(&first_calls).all(|row| {
        if negative && row["route"] == "embed" {
            row["ok"] == false && row["error_kind"] == "EmbedderNotConfiguredError"
        } else {
            row["ok"] == true
                && (row["route"] != "embed" || (row["dimension"] == 384 && row["finite"] == true))
        }
    });
    let attribution_ok = !observed
        || (traces.len() == rows.len() + warmups.len() + first_calls.len()
            && traces.iter().all(|trace| {
                trace["dropped_records"] == 0
                    && trace["issues"] == json!([])
                    && trace["records"].as_array().is_some_and(|records| !records.is_empty())
            }));
    let result = json!({"complete":outcomes_ok && attribution_ok,"negative_control":negative,
        "sample_count":rows.len(),"warmup_count":warmups.len(),"first_call_count":first_calls.len(),
        "record_first_call":record_first,
        "samples":rows,"warmups":warmups,"first_calls":first_calls,
        "observation_batches":traces,"passes":passes,"fixtures":fixtures,
        "raw_persistence":recorder.statistics_since([0;5]),
        "cache_note":"readiness and any configured first-call/warmup probes precede timed calls; OS caches uncontrolled; collector overhead uncorrected"});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

fn run_g_write(config: &Value, out: &Path) -> Value {
    let provider = config["provider"].as_str().expect("G provider");
    let negative = config["negative_control"] == true;
    assert_eq!(provider, if negative { "none" } else { "default" });
    if !negative {
        assert_eq!(
            std::env::var("AGENT_LONG").as_deref(),
            Ok("1"),
            "Default runs require long opt-in"
        );
    }
    let adapters = config["adapters"].as_array().expect("G-W adapter");
    let routes = config["routes"].as_array().expect("G-W route");
    assert_eq!(adapters.len(), 1, "G-W requires one adapter per fresh matching store");
    assert_eq!(routes.len(), 1, "G-W requires one route per fresh matching store");
    let name = adapters[0].as_str().unwrap();
    let route = routes[0].as_str().unwrap();
    let store = Path::new(config["store"].as_str().expect("selected G-W store"));
    assert!(store.is_file());
    assert!(!out.exists(), "G-W output must be fresh");
    std::fs::create_dir_all(out).expect("owned G-W output");
    let queries = read_jsonl(config["queries"].as_str().expect("G-W queries"));
    let originals = read_jsonl(config["corpus"].as_str().expect("original natural corpus"));
    let number =
        |key: &str| usize::try_from(config[key].as_u64().expect("G-W call count")).unwrap();
    let samples = number("samples");
    let warmup = number("warmup");
    assert!(samples > 0 && !queries.is_empty() && originals.len() >= 10);
    let observed = config["observe"] == true;
    let opening = std::time::Instant::now();
    let engine = Adapter::open_g(name, store, provider);
    let open_ns = u64::try_from(opening.elapsed().as_nanos()).unwrap();
    let before = g_fixture_observation(&engine, store);
    assert!(
        negative || before["qualified_dense_fixture"] == true,
        "G-W needs a qualified initial dense store"
    );
    let observation = begin_g_observation(&engine, observed);
    let mut recorder = Recorder::new_g(out);
    let mut operation = 1_u64;
    let mut warmups = Vec::new();
    let mut traces = Vec::new();
    for index in 0..warmup {
        let (mut row, trace) = observe_g_operation(
            &engine,
            route,
            &queries[index % queries.len()],
            operation,
            &observation,
        );
        row["adapter"] = json!(name);
        row["repeat"] = config["repeat"].clone();
        row["sample_index"] = json!(index);
        row["query_index"] = json!(index % queries.len());
        row["phase"] = json!("warmup");
        recorder.emit("warmups", &row);
        warmups.push(row);
        if let Some(mut trace) = trace {
            trace["adapter"] = json!(name);
            trace["route"] = json!(route);
            trace["phase"] = json!("warmup");
            trace["operation_id"] = json!(operation);
            recorder.emit("observations", &trace);
            traces.push(trace);
        }
        operation += 1;
    }
    recorder.flush_all();
    let measured = std::time::Instant::now();
    let schedule = run_g_schedule_with_calls(
        &engine,
        route,
        &queries,
        &originals,
        GSchedule {
            samples,
            batch_size: number("batch_size"),
            searches_per_batch: number("searches_per_batch"),
        },
        &mut |stream, row| {
            let mut row = row.clone();
            row["adapter"] = json!(name);
            row["repeat"] = config["repeat"].clone();
            if stream == "observations" {
                traces.push(row.clone());
            }
            recorder.emit(stream, &row);
        },
        &mut |called_route, query, phase| {
            let (mut row, trace) =
                observe_g_operation(&engine, called_route, query, operation, &observation);
            row["adapter"] = json!(name);
            row["repeat"] = config["repeat"].clone();
            row["phase"] = json!(phase);
            let trace = trace.map(|mut trace| {
                trace["adapter"] = json!(name);
                trace["route"] = json!(called_route);
                trace["phase"] = json!(phase);
                trace["operation_id"] = json!(operation);
                trace
            });
            operation += 1;
            (row, trace)
        },
    );
    recorder.flush_all();
    let wall_ns = u64::try_from(measured.elapsed().as_nanos()).unwrap();
    let after = g_fixture_observation(&engine, store);
    #[cfg(feature = "slice30-observer")]
    drop(observation);
    let closing = std::time::Instant::now();
    engine.close();
    drop(engine);
    let close_ns = u64::try_from(closing.elapsed().as_nanos()).unwrap();
    let outcomes_ok = schedule.searches.iter().chain(&warmups).all(|row| {
        if negative && row["route"] == "embed" {
            row["ok"] == false && row["error_kind"] == "EmbedderNotConfiguredError"
        } else {
            row["ok"] == true
                && (row["route"] != "embed" || (row["dimension"] == 384 && row["finite"] == true))
        }
    });
    let attribution_ok = !observed
        || (traces.len() == warmups.len() + schedule.searches.len() + schedule.probes.len()
            && traces.iter().all(|trace| {
                trace["dropped_records"] == 0
                    && trace["issues"] == json!([])
                    && trace["records"].as_array().is_some_and(|records| !records.is_empty())
            }));
    let complete = outcomes_ok
        && attribution_ok
        && schedule.probes.iter().all(|probe| probe["marker_current"] == true)
        && schedule.batches.iter().all(|batch| batch["adjacent_readiness"]["available"] == true)
        && before["natural_records"] == after["natural_records"];
    let result = json!({"complete":complete,"negative_control":negative,
        "sample_count":schedule.searches.len(),"warmup_count":warmups.len(),"first_call_count":0,
        "batch_count":schedule.batches.len(),"batches":schedule.batches,"probes":schedule.probes,
        "samples":schedule.searches,"warmups":warmups,"first_calls":[],"observation_batches":traces,
        "before_workload":before,"after_workload":after,"open_ns":open_ns,"close_ns":close_ns,
        "passes":[{"adapter":name,"route":route,"requests":samples,"wall_ns":wall_ns,
            "throughput_scope":"natural route calls over write/readiness/point+FTS probe/search/recording/checkpoint loop"}],
        "raw_persistence":recorder.statistics_since([0;5]),
        "cache_note":"adjacent public readiness and separate point/FTS probes intervene after each governed cohort; warmups precede writes; no interbatch drain or observed-worker-overlap claim"});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

fn g_projection_declared(engine: &Adapter) -> bool {
    let specs = match engine {
        Adapter::Core(engine) => engine.read_projections().map_err(Into::into),
        Adapter::Sdk(engine) => fathomdb_sdk::read::projections(engine),
    }
    .expect("read actual G projection declaration");
    specs.iter().any(|spec| {
        spec.name == "doc"
            && spec.vector.is_some()
            && spec.roles.contains(&fathomdb_engine::ProjectionRole::Searchable)
    })
}

fn g_remaining_ms(started: std::time::Instant, allowance_ms: u64) -> u64 {
    std::time::Duration::from_millis(allowance_ms)
        .checked_sub(started.elapsed())
        .map(|remaining| u64::try_from(remaining.as_millis()).unwrap())
        .unwrap_or(0)
}

fn g_seed_cap_refusal(out: &Path, started: std::time::Instant, allowance_ms: u64) -> Value {
    let partial = out.join("store.sqlite").exists();
    std::fs::create_dir_all(out).expect("retain G cap refusal metadata");
    let result = json!({"complete":false,"qualified_dense_fixture":false,
        "status":if partial {"PARTIAL_CAP_TIME"} else {"NOT_RUN_CAP_TIME"},
        "admitted_allowance_ms":allowance_ms,"remaining_cell_ms":0,
        "driver_wall_ns":u64::try_from(started.elapsed().as_nanos()).unwrap(),
        "budget_scope":"driver entry includes input/output preparation; outer controller owns whole-cell startup/configuration/final reporting"});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

fn run_g_seed(config: &Value, out: &Path) -> Value {
    let driver_started = std::time::Instant::now();
    let allowance_ms = match config.get("remaining_cell_ms") {
        Some(value) => value.as_u64().expect("numeric remaining G cell allowance"),
        None => {
            assert_eq!(
                config["negative_control"], true,
                "positive G driver needs the controller's remaining allowance"
            );
            900_000
        }
    };
    assert!(allowance_ms <= 900_000, "G allowance cannot expand the fixed whole-cell cap");
    if g_remaining_ms(driver_started, allowance_ms) == 0 {
        return g_seed_cap_refusal(out, driver_started, allowance_ms);
    }
    let provider = config["provider"].as_str().expect("G provider");
    let negative = config["negative_control"] == true;
    assert_eq!(provider, if negative { "none" } else { "default" });
    if !negative {
        assert_eq!(
            std::env::var("AGENT_LONG").as_deref(),
            Ok("1"),
            "Default runs require the explicit long-run opt-in"
        );
    }
    let adapter = config["adapter"].as_str().expect("G seed adapter");
    let corpus = read_jsonl(config["corpus"].as_str().expect("corpus path"));
    let queries = read_jsonl(config["queries"].as_str().expect("queries path"));
    let n = usize::try_from(config["n"].as_u64().expect("G seed n")).unwrap();
    assert!(n > 0 && n <= corpus.len() && !queries.is_empty());
    if g_remaining_ms(driver_started, allowance_ms) == 0 {
        return g_seed_cap_refusal(out, driver_started, allowance_ms);
    }
    std::fs::create_dir_all(out).expect("owned G seed output");
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "G seed store must be fresh");
    if g_remaining_ms(driver_started, allowance_ms) == 0 {
        return g_seed_cap_refusal(out, driver_started, allowance_ms);
    }
    let start = std::time::Instant::now();
    let engine = Adapter::open_g(adapter, &store, provider);
    let open_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    let spec = fathomdb_engine::ProjectionSpec {
        name: "doc".into(),
        roles: [fathomdb_engine::ProjectionRole::Searchable].into_iter().collect(),
        fts: None,
        vector: Some(fathomdb_engine::ProjectionVector::default()),
        source: None,
    };
    let start = std::time::Instant::now();
    match &engine {
        Adapter::Core(engine) => engine.configure_projections(&[spec], &[]).map_err(Into::into),
        Adapter::Sdk(engine) => engine.configure_projections(&[spec], &[]),
    }
    .expect("configure public G doc vector projection");
    let configure_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    let projection_declared = g_projection_declared(&engine);
    let start = std::time::Instant::now();
    let mut seed_batches = Vec::new();
    for begin in (0..n).step_by(1000) {
        if g_remaining_ms(driver_started, allowance_ms) == 0 {
            engine.close();
            drop(engine);
            return g_seed_cap_refusal(out, driver_started, allowance_ms);
        }
        let writes = corpus[begin..n.min(begin + 1000)]
            .iter()
            .enumerate()
            .map(|(offset, body)| node(&format!("s30doc-{}", begin + offset), "doc", body))
            .collect::<Vec<_>>();
        let cursor = engine.write(&writes);
        seed_batches.push(json!({"rows":writes.len(),"after_write_cursor":cursor}));
    }
    let write_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    let remaining_ms = g_remaining_ms(driver_started, allowance_ms);
    if remaining_ms == 0 {
        engine.close();
        drop(engine);
        return g_seed_cap_refusal(out, driver_started, allowance_ms);
    }
    let start = std::time::Instant::now();
    match &engine {
        Adapter::Core(engine) => engine.drain(remaining_ms).map_err(Into::into),
        Adapter::Sdk(engine) => engine.drain(remaining_ms),
    }
    .expect("drain within remaining G seed whole-cell budget");
    let drain_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    let before_close = g_fixture_observation(&engine, &store);
    let start = std::time::Instant::now();
    engine.close();
    drop(engine);
    let close_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    if g_remaining_ms(driver_started, allowance_ms) == 0 {
        return g_seed_cap_refusal(out, driver_started, allowance_ms);
    }
    let start = std::time::Instant::now();
    let reopened = Adapter::open_g(adapter, &store, provider);
    let reopen_ns = u64::try_from(start.elapsed().as_nanos()).unwrap();
    let lexical = observe_g_route(&reopened, "text", &queries[0]);
    let after_reopen = g_fixture_observation(&reopened, &store);
    let projection_declared_after_reopen = g_projection_declared(&reopened);
    reopened.close();
    drop(reopened);
    let driver_wall_ns = u64::try_from(driver_started.elapsed().as_nanos()).unwrap();
    let qualified = before_close["qualified_dense_fixture"] == true
        && after_reopen["qualified_dense_fixture"] == true;
    let complete = projection_declared
        && projection_declared_after_reopen
        && before_close["natural_records"] == n
        && after_reopen["natural_records"] == n
        && lexical["ok"] == true
        && driver_wall_ns <= allowance_ms * 1_000_000
        && (negative || qualified);
    let result = json!({"complete":complete,
        "control":if negative {"negative_no_provider"} else {"default_dense_seed"},
        "requested_provider":provider,"adapter":adapter,"natural_records":before_close["natural_records"],
        "projection_declared":projection_declared,"projection_declared_after_reopen":projection_declared_after_reopen,
        "configure_ns":configure_ns,"drain_ns":drain_ns,"driver_wall_ns":driver_wall_ns,"admitted_allowance_ms":allowance_ms,
        "budget_scope":"driver entry includes input/output preparation; outer controller owns whole-cell startup/configuration/final reporting",
        "open_ns":open_ns,"write_ns":write_ns,"seed_batches":seed_batches,
        "close_ns":close_ns,"reopen_ns":reopen_ns,
        "before_close":before_close,"after_reopen":after_reopen,"lexical_probe":lexical,
        "qualified_dense_fixture":qualified});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

fn run_g_control(config: &Value, out: &Path) -> Value {
    assert_eq!(config["provider"], "none", "G-control is only a negative readiness control");
    let corpus = read_jsonl(config["corpus"].as_str().expect("corpus path"));
    let queries = read_jsonl(config["queries"].as_str().expect("queries path"));
    let n = usize::try_from(config["n"].as_u64().expect("n")).unwrap();
    assert!(n > 0 && n <= corpus.len() && !queries.is_empty());
    std::fs::create_dir_all(out).expect("owned control output");
    let store = out.join("store.sqlite");
    assert!(!store.exists(), "control store must be fresh");
    let seed = Adapter::open_g("sdk", &store, config["provider"].as_str().unwrap());
    seed.write(
        &corpus[..n]
            .iter()
            .enumerate()
            .map(|(index, body)| node(&format!("s30doc-{index}"), "doc", body))
            .collect::<Vec<_>>(),
    );
    seed.close();
    drop(seed);
    let mut rows = Vec::new();
    let mut traces = Vec::new();
    let mut fixtures = Vec::new();
    let mut nested = Vec::new();
    let mut recorder = Recorder::new_g(out);
    let mut operation = 1_u64;
    for adapter in ["engine", "sdk"] {
        let engine = Adapter::open_g(adapter, &store, config["provider"].as_str().unwrap());
        #[cfg(feature = "slice30-observer")]
        let observation = (config["observe"] == true).then(|| engine.begin_observation());
        for route in ["text", "hybrid", "embed"] {
            #[cfg(feature = "slice30-observer")]
            let mut row = if let Some(observation) = &observation {
                observation
                    .with_operation(operation, || observe_g_route(&engine, route, &queries[0]))
            } else {
                observe_g_route(&engine, route, &queries[0])
            };
            #[cfg(not(feature = "slice30-observer"))]
            let mut row = observe_g_route(&engine, route, &queries[0]);
            row["adapter"] = json!(adapter);
            row["operation_id"] = json!(operation);
            recorder.emit("samples", &row);
            rows.push(row);
            let mut trace = json!({"records":[],"dropped_records":null,
                "issues":["collector_disabled_or_feature_absent"],"collection":"unavailable"});
            #[cfg(feature = "slice30-observer")]
            if let Some(observation) = &observation {
                trace = serde_json::to_value(observation.take())
                    .expect("encode actual observation batch");
                trace["collection"] = json!("enabled");
            }
            trace["adapter"] = json!(adapter);
            trace["route"] = json!(route);
            trace["operation_id"] = json!(operation);
            recorder.emit("observations", &trace);
            traces.push(trace);
            operation += 1;
        }
        let mut nested_trace = json!({"records":[],"dropped_records":null,
            "issues":["nested_control_not_observed"],"collection":"unavailable"});
        #[cfg(feature = "slice30-observer")]
        if config["nested_control"] == true {
            if let Some(observation) = &observation {
                observation.with_operation(7001, || {
                    observe_g_route(&engine, "text", &queries[0]);
                    let contained = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        observation.with_operation(7002, || {
                            observe_g_route(&engine, "text", &queries[0]);
                            panic!("contained G nested operation unwind");
                        });
                    }));
                    assert!(contained.is_err(), "nested control must exercise unwind restoration");
                    observe_g_route(&engine, "text", &queries[0]);
                });
                nested_trace = serde_json::to_value(observation.take())
                    .expect("encode actual nested observation batch");
                nested_trace["collection"] = json!("enabled");
            }
        }
        nested_trace["adapter"] = json!(adapter);
        nested.push(nested_trace);
        #[cfg(feature = "slice30-observer")]
        drop(observation);
        fixtures.push(g_fixture_observation(&engine, &store));
        engine.close();
        drop(engine);
    }
    recorder.flush_all();
    let complete = rows.iter().all(|row| {
        if row["route"] == "embed" {
            row["ok"] == false && row["error_kind"] == "EmbedderNotConfiguredError"
        } else {
            row["ok"] == true
        }
    });
    let complete = complete
        && (config["observe"] != true
            || traces.iter().all(|trace| {
                trace["collection"] == "enabled"
                    && trace["dropped_records"] == 0
                    && trace["issues"] == json!([])
                    && trace["records"].as_array().is_some_and(|records| !records.is_empty())
            }))
        && (config["nested_control"] != true
            || nested.iter().all(|trace| {
                trace["collection"] == "enabled"
                    && trace["dropped_records"] == 0
                    && trace["issues"] == json!([])
                    && trace["records"].as_array().is_some_and(|records| !records.is_empty())
            }));
    let result = json!({"complete":complete,"control":"negative_no_provider",
        "qualified_dense_fixture":false,"samples":rows,"observation_batches":traces,
        "nested_controls":nested,"fixtures":fixtures,"format":fixtures[0]["format"],
        "raw_persistence":recorder.statistics_since([0;5])});
    std::fs::write(out.join("result.json"), serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    result
}

fn main() {
    let args: Vec<_> = std::env::args().collect();
    assert_eq!(args.len(), 3, "usage: slice30_measure cell.json owned-output-directory");
    let config: Value = serde_json::from_slice(&std::fs::read(&args[1]).expect("read frozen cell"))
        .expect("cell JSON");
    let result = if config["family"] == "G-control" {
        run_g_control(&config, Path::new(&args[2]))
    } else if config["family"] == "G-seed" {
        run_g_seed(&config, Path::new(&args[2]))
    } else if config["family"] == "G-R" || config["family"] == "G-O" {
        run_g_read(&config, Path::new(&args[2]))
    } else if config["family"] == "G-W" {
        run_g_write(&config, Path::new(&args[2]))
    } else {
        run_cell(&config, Path::new(&args[2]))
    };
    println!("{result}");
    if result["complete"] != true {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupted_cell_retains_completed_raw_checkpoints() {
        const CHILD_DIRECTORY: &str = "FATHOMDB_SLICE30_RETENTION_CHILD_DIRECTORY";
        if let Some(directory) = std::env::var_os(CHILD_DIRECTORY) {
            let directory = std::path::PathBuf::from(directory);
            let config: Value =
                serde_json::from_slice(&std::fs::read(directory.join("cell.json")).unwrap())
                    .unwrap();
            let mut observed = 0;
            run_cell_observed(&config, &directory.join("out"), &mut |stream, _| {
                if stream == "samples" {
                    observed += 1;
                    if observed == 16 {
                        std::fs::write(directory.join("ready"), b"16 completed observations")
                            .unwrap();
                        std::thread::sleep(std::time::Duration::from_secs(30));
                        panic!("parent did not stop its retention child");
                    }
                }
            });
            panic!("retention child unexpectedly completed");
        }
        struct OwnedChild(std::process::Child);
        impl Drop for OwnedChild {
            fn drop(&mut self) {
                let _ = self.0.kill();
                self.0.wait().expect("reap only owned retention child");
            }
        }
        let records = |path: &Path| -> Vec<Value> {
            std::fs::read_to_string(path)
                .unwrap_or_default()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect()
        };
        for family in ["P1", "P2"] {
            let directory = tempfile::tempdir().unwrap();
            let corpus = directory.path().join("corpus.jsonl");
            let queries = directory.path().join("queries.jsonl");
            std::fs::write(&corpus, "\"alphaonly evidence\"\n\"betaonly evidence\"\n").unwrap();
            std::fs::write(&queries, "\"alphaonly\"\n\"betaonly\"\n").unwrap();
            let config = json!({"family":family,"n":2,"adapters":["sdk"],"warmup":4,"samples":64,
                "batch_size":1,"searches_per_batch":1,"repeat":0,"corpus":corpus,"queries":queries});
            std::fs::write(
                directory.path().join("cell.json"),
                serde_json::to_vec(&config).unwrap(),
            )
            .unwrap();
            let log = std::fs::File::create(directory.path().join("child.log")).unwrap();
            let child = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "tests::interrupted_cell_retains_completed_raw_checkpoints",
                    "--nocapture",
                ])
                .env(CHILD_DIRECTORY, directory.path())
                .stdout(log.try_clone().unwrap())
                .stderr(log)
                .spawn()
                .unwrap();
            let child = OwnedChild(child);
            let started = std::time::Instant::now();
            let ready = directory.path().join("ready");
            while !ready.exists() && started.elapsed() < std::time::Duration::from_secs(20) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            let acknowledged = ready.exists();
            drop(child);
            assert!(
                acknowledged,
                "actual 16-observation rendezvous absent: {}",
                std::fs::read_to_string(directory.path().join("child.log")).unwrap()
            );
            let out = directory.path().join("out");
            let samples = records(&out.join("samples.jsonl"));
            assert_eq!(samples.len(), 16, "{family}: completed raw checkpoint lost");
            assert_eq!(records(&out.join("warmups.jsonl")).len(), 4);
            assert!(!out.join("result.json").exists());
            for (index, row) in samples.iter().enumerate() {
                assert_eq!(row["ids"], json!([format!("l:s30doc-{}", index % 2)]));
                assert_eq!(row["adapter"], "sdk");
                assert_eq!(row["query_index"], index % 2);
                assert_eq!(row["sample_index"], index);
                assert!(row["elapsed_ns"].as_u64().unwrap() > 0);
            }
            if family == "P2" {
                assert_eq!(records(&out.join("batches.jsonl")).len(), 16);
                let probes = records(&out.join("marker-probes.jsonl"));
                assert_eq!(probes.len(), 16);
                assert!(probes.iter().all(|row| row["marker_current"] == true));
            }
        }
    }

    #[test]
    fn observation_records_real_text_identity_across_reopen() {
        for name in ["engine", "sdk"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("literal.sqlite");
            let engine = Adapter::open(name, &path);
            engine.write(&[
                node("alpha", "doc", "alphaonly evidence"),
                node("beta", "doc", "betaonly evidence"),
            ]);
            let result = observe(&engine, "alphaonly", None);
            assert_eq!(result["ids"], json!(["l:alpha"]), "{name}: {result}");
            assert_eq!(result["ok"], true);
            assert!(result["elapsed_ns"].as_u64().unwrap() > 0);
            engine.close();
            drop(engine);
            let reopened = Adapter::open(name, &path);
            assert_eq!(observe(&reopened, "betaonly", None)["ids"], json!(["l:beta"]));
            reopened.close();
        }
    }

    #[test]
    fn schedule_observes_commits_search_counts_and_current_marker_bodies() {
        let cases = [
            (1, 1, 4, vec![0, 1, 2, 3], 4, 3, "s30markercurrent3 revision3"),
            (10, 10, 20, vec![0, 10], 20, 9, "s30markercurrent9 revision1"),
            (1, 100, 200, vec![0, 100], 2, 1, "s30markercurrent1 revision1"),
        ];
        for (batch_size, ratio, samples, starts, probe_count, last_slot, expected_body) in cases {
            let directory = tempfile::tempdir().unwrap();
            let engine = Adapter::open("sdk", &directory.path().join("schedule.sqlite"));
            engine.write(&[node("natural", "doc", "naturalalpha")]);
            for slot in 0..10 {
                engine.write(&[node(&format!("s30marker-{slot}"), "note", "initial marker")]);
            }
            let result =
                run_schedule(&engine, &["naturalalpha".into()], samples, batch_size, ratio);
            assert_eq!(result.searches.len(), samples);
            assert!(result.searches.iter().all(|row| row["ids"] == json!(["l:natural"])));
            assert_eq!(
                result
                    .batches
                    .iter()
                    .map(|row| row["before_sample"].as_u64().unwrap())
                    .collect::<Vec<_>>(),
                starts
            );
            assert_eq!(result.probes.len(), probe_count);
            assert!(result.probes.iter().all(|row| row["ok"] == true));
            assert_eq!(
                observe(&engine, &format!("s30markercurrent{last_slot}"), Some("note"))["bodies"],
                json!([expected_body])
            );
            engine.close();
        }
    }

    #[test]
    fn match_observer_uses_product_compiler_and_keeps_raw_count_separate() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("matches.sqlite");
        let engine = Adapter::open("sdk", &path);
        for index in 0..12 {
            engine.write(&[node(
                &format!("alpha-{index}"),
                "doc",
                &format!("alpha evidence{index}"),
            )]);
        }
        engine
            .write(&[node("marker", "note", "alpha marker"), node("stopword", "doc", "the only")]);
        let query = "The ALPHA, alpha?";
        assert_eq!(raw_match_count(&path, query, None), 13);
        assert_eq!(raw_match_count(&path, query, Some("doc")), 12);
        let result = observe(&engine, query, Some("doc"));
        assert_eq!(result["returned_hits"], 10);
        assert!(result["ids"]
            .as_array()
            .unwrap()
            .iter()
            .all(|id| id.as_str().unwrap().starts_with("l:alpha-")));
        engine.close();
    }

    #[test]
    fn cell_keeps_warmups_separate_and_seeds_one_fresh_natural_store() {
        let directory = tempfile::tempdir().unwrap();
        let corpus = directory.path().join("corpus.jsonl");
        let queries = directory.path().join("queries.jsonl");
        std::fs::write(&corpus, "\"alphaonly evidence\"\n\"betaonly evidence\"\n").unwrap();
        std::fs::write(&queries, "\"alphaonly\"\n\"betaonly\"\n").unwrap();
        let out = directory.path().join("out");
        let config = json!({"family":"P1", "n":2, "adapters":["engine","sdk"],
            "warmup":3, "samples":4, "repeat":0, "corpus":corpus, "queries":queries});
        let result = run_cell(&config, &out);
        assert_eq!(result["sample_count"], 8);
        assert_eq!(result["warmup_count"], 6);
        assert_eq!(result["filtered"], false);
        assert_eq!(result["natural_records"], 2);
        let rows: Vec<Value> = std::fs::read_to_string(out.join("samples.jsonl"))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(rows.len(), 8);
        assert_eq!(
            rows.iter().map(|row| row["ids"].clone()).collect::<Vec<_>>(),
            vec![
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"]),
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"]),
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"]),
                json!(["l:s30doc-0"]),
                json!(["l:s30doc-1"])
            ]
        );
        let connection = rusqlite::Connection::open_with_flags(
            out.join("store.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, u64>(0))
                .unwrap(),
            2
        );
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |row| row
                    .get::<_, u64>(0))
                .unwrap(),
            0
        );
        drop(connection);
        assert!(std::panic::catch_unwind(|| run_cell(&config, &out)).is_err());
    }

    #[test]
    fn cell_labels_filtered_marker_schedule_separately_from_p1() {
        let directory = tempfile::tempdir().unwrap();
        let corpus = directory.path().join("corpus.jsonl");
        let queries = directory.path().join("queries.jsonl");
        std::fs::write(&corpus, "\"naturalalpha\"\n\"naturalbeta\"\n").unwrap();
        std::fs::write(&queries, "\"naturalalpha\"\n").unwrap();
        let out = directory.path().join("out");
        let config = json!({"family":"P2", "n":2, "adapters":["sdk"], "batch_size":1,
            "searches_per_batch":1, "warmup":3, "samples":4, "repeat":0,
            "corpus":corpus, "queries":queries});
        let result = run_cell(&config, &out);
        assert_eq!(result["sample_count"], 4);
        assert_eq!(result["warmup_count"], 3);
        assert_eq!(result["filtered"], true);
        assert_eq!(result["natural_records"], 2);
        assert_eq!(result["marker_records"], 10);
        assert_eq!(result["batch_count"], 4);
        assert_eq!(result["marker_probe_count"], 4);
        assert_eq!(result["complete"], true);
    }
}

#[cfg(test)]
mod g_route_tests {
    use super::*;

    #[test]
    fn g_routes_reach_the_actual_text_hybrid_and_embed_boundaries() {
        for adapter in ["engine", "sdk"] {
            let directory = tempfile::tempdir().unwrap();
            let engine = Adapter::open(adapter, &directory.path().join("g-routes.sqlite"));
            engine.write(&[node("alpha", "doc", "alpha needle"), node("beta", "doc", "beta")]);
            let embedded = observe_g_route(&engine, "embed", "alpha");
            assert_eq!(
                embedded["ok"], false,
                "embed must reach the absent-provider boundary, not return a lexical search"
            );
            assert_eq!(embedded["error_kind"], "EmbedderNotConfiguredError");
            assert_eq!(embedded["route"], "embed");
            let text = observe_g_route(&engine, "text", "needle");
            assert_eq!(text["ids"], json!(["l:alpha"]));
            assert_eq!(text["degradations"], "[]");
            assert_eq!(text["route"], "text");
            let hybrid = observe_g_route(&engine, "hybrid", "needle");
            assert_eq!(hybrid["ids"], json!(["l:alpha"]));
            assert!(hybrid["degradations"].as_str().unwrap().contains("EmbedderAbsent"));
            assert_eq!(hybrid["route"], "hybrid");
            engine.close();
        }
    }
}

#[cfg(test)]
mod g_fixture_tests {
    use super::*;

    #[test]
    fn g_fixture_reports_real_pending_and_physical_counts_without_false_readiness() {
        for adapter in ["engine", "sdk"] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("g-fixture.sqlite");
            let engine = Adapter::open(adapter, &path);
            let spec = fathomdb_sdk::ProjectionSpec {
                name: "doc".into(),
                roles: [fathomdb_sdk::ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(fathomdb_sdk::ProjectionVector::default()),
                source: None,
            };
            match &engine {
                Adapter::Core(engine) => {
                    engine.configure_projections(&[spec], &[]).unwrap();
                }
                Adapter::Sdk(engine) => {
                    engine.configure_projections(&[spec], &[]).unwrap();
                }
            }
            engine.write(&[node("alpha", "doc", "alpha needle"), node("beta", "doc", "beta")]);
            let edge = |id: &str, body: &str| PreparedWrite::Edge {
                kind: "relates_to".into(),
                from: "alpha".into(),
                to: "beta".into(),
                logical_id: Some(id.into()),
                body: Some(body.into()),
                source_id: SourceId::new("test:slice30-g-fixture").unwrap(),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            };
            engine.write(&[
                edge("edge-one", "first deferred relation"),
                edge("edge-two", "second deferred relation"),
            ]);
            let observed = g_fixture_observation(&engine, &path);
            assert_eq!(
                observed["natural_records"], 2,
                "G fixture must report actual current canonical rows"
            );
            assert_eq!(observed["pending_count"], 2);
            assert_eq!(observed["readiness"], "blocked");
            assert_eq!(observed["usable_embedder"], false);
            assert_eq!(observed["vector_rows"], 0);
            assert_eq!(observed["span_rows"], 0);
            assert_eq!(observed["format"], 37);
            assert!(observed["generation_id"].as_str().unwrap().starts_with("pgen1:"));
            assert_eq!(observed["qualified_dense_fixture"], false);
            engine.close();
            let reopened = Adapter::open(adapter, &path);
            assert_eq!(observe_g_route(&reopened, "text", "needle")["ids"], json!(["l:alpha"]));
            assert_eq!(g_fixture_observation(&reopened, &path)["pending_count"], 2);
            reopened.close();
        }
    }
}

#[cfg(test)]
mod g_schedule_tests {
    use super::*;

    #[test]
    fn natural_replacements_are_two_actual_cohorts_with_separate_current_probes() {
        for name in ["engine", "sdk"] {
            for route in ["text", "hybrid", "embed"] {
                for batch_size in [1, 10] {
                    let directory = tempfile::tempdir().unwrap();
                    let path = directory.path().join("g-schedule.sqlite");
                    let engine = Adapter::open(name, &path);
                    let bodies =
                        (0..10).map(|slot| format!("needle body{slot}")).collect::<Vec<_>>();
                    engine.write(
                        &bodies
                            .iter()
                            .enumerate()
                            .map(|(slot, body)| node(&format!("s30doc-{slot}"), "doc", body))
                            .collect::<Vec<_>>(),
                    );
                    let mut order = Vec::new();
                    let output = run_g_schedule_observed(
                        &engine,
                        route,
                        &["needle".into()],
                        &bodies,
                        GSchedule { samples: 4, batch_size, searches_per_batch: 2 },
                        &mut |stream, _| order.push(stream.to_string()),
                    );
                    assert_eq!(output.batches.len(), 2, "missing actual G replacement cohorts");
                    assert_eq!(
                        output
                            .batches
                            .iter()
                            .map(|row| row["before_sample"].as_u64().unwrap())
                            .collect::<Vec<_>>(),
                        [0, 2]
                    );
                    assert!(output
                        .batches
                        .iter()
                        .all(|row| row["rows"] == batch_size
                            && row["write_ns"].as_u64().unwrap() > 0));
                    assert!(
                        output.batches[0]["after_write_cursor"].as_u64().unwrap()
                            < output.batches[1]["after_write_cursor"].as_u64().unwrap()
                    );
                    assert_eq!(output.searches.len(), 4);
                    for (index, row) in output.searches.iter().enumerate() {
                        assert_eq!(row["route"], route);
                        assert_eq!(row["sample_index"], index);
                        assert_eq!(row["query_index"], 0);
                        assert_eq!(row["ok"], route != "embed");
                        if route == "embed" {
                            assert_eq!(row["error_kind"], "EmbedderNotConfiguredError");
                        } else {
                            assert_eq!(row["returned_hits"], 10);
                        }
                    }
                    assert_eq!(output.probes.len(), 2 * batch_size);
                    for (index, probe) in output.probes.iter().enumerate() {
                        let cohort = index / batch_size;
                        let slot = if batch_size == 1 { cohort } else { index % 10 };
                        let token = format!("s30gmarker{slot}revision{cohort}");
                        let body = format!("needle body{slot} {token}");
                        assert_eq!(probe["before_sample"], cohort * 2);
                        assert_eq!(probe["logical_id"], format!("s30doc-{slot}"));
                        assert_eq!(probe["revision_token"], token);
                        assert_eq!(probe["point_probe"]["body"], body);
                        assert_eq!(probe["point_probe"]["current"], true);
                        assert!(probe["point_probe"]["elapsed_ns"].as_u64().unwrap() > 0);
                        assert_eq!(probe["text_probe"]["ids"], json!([format!("l:s30doc-{slot}")]));
                        assert_eq!(probe["text_probe"]["bodies"], json!([body]));
                        assert!(probe["text_probe"]["elapsed_ns"].as_u64().unwrap() > 0);
                        assert_eq!(probe["marker_current"], true);
                    }
                    let mut expected = Vec::new();
                    for _ in 0..2 {
                        expected.push("batches".to_string());
                        expected.extend(vec!["g-marker-probes".to_string(); batch_size]);
                        expected.extend(vec!["samples".to_string(); 2]);
                    }
                    assert_eq!(order, expected);
                    engine.close();
                    drop(engine);
                    let connection = rusqlite::Connection::open(&path).unwrap();
                    let count =
                        |sql| connection.query_row(sql, [], |row| row.get::<_, u64>(0)).unwrap();
                    assert_eq!(count("SELECT COUNT(*) FROM canonical_nodes WHERE superseded_at IS NULL AND state='active' AND kind='doc'"), 10);
                    assert_eq!(
                        count("SELECT COUNT(*) FROM canonical_nodes"),
                        10 + 2 * batch_size as u64
                    );
                    assert_eq!(count("SELECT COUNT(*) FROM canonical_nodes WHERE kind='note'"), 0);
                    assert_eq!(count("SELECT COUNT(*) FROM _fathomdb_vector_rows"), 0);
                    assert_eq!(count("PRAGMA user_version"), 37);
                }
            }
        }
    }
}

#[cfg(test)]
mod g_provider_tests {
    use super::*;

    #[test]
    fn g_default_selection_forwards_the_actual_default_choice_without_starting_a_model() {
        assert!(matches!(g_provider_choice("none"), EmbedderChoice::None));
        assert!(
            matches!(g_provider_choice("default"), EmbedderChoice::Default),
            "G requested Default must not silently select no provider"
        );
    }
}
