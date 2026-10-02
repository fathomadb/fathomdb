//! Standalone D27 workload test, compiled against a selected engine checkout.

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const DIMENSION: usize = 384;
const SEED: u64 = 0xd027_90c0_ffee_0001;

#[derive(Debug)]
struct DeterministicEmbedder {
    active: AtomicUsize,
    peak: AtomicUsize,
}

impl DeterministicEmbedder {
    fn new() -> Self {
        Self { active: AtomicUsize::new(0), peak: AtomicUsize::new(0) }
    }
}

impl Embedder for DeterministicEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new(
            "d27_runtime_qualification_v1",
            "seed-d02790c0ffee0001",
            DIMENSION as u32,
        )
    }

    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(active, Ordering::SeqCst);
        thread::sleep(Duration::from_millis(2));
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for byte in input.as_bytes().iter().chain(SEED.to_le_bytes().iter()) {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
        let mut vector = Vec::with_capacity(DIMENSION);
        for _ in 0..DIMENSION {
            hash ^= hash << 13;
            hash ^= hash >> 7;
            hash ^= hash << 17;
            vector.push(((hash >> 32) as u32 as f32 / u32::MAX as f32) * 2.0 - 1.0);
        }
        let norm = vector.iter().map(|x| x * x).sum::<f32>().sqrt();
        for item in &mut vector {
            *item /= norm;
        }
        self.active.fetch_sub(1, Ordering::SeqCst);
        Ok(vector)
    }
}

fn prepared(body: String, logical_id: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body,
        source_id: SourceId::new("d27:corpus").expect("source id"),
        logical_id: Some(logical_id),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn monotonic_ns(origin: Instant) -> u128 {
    origin.elapsed().as_nanos()
}

fn operation(
    engine: &Engine,
    class: &str,
    sequence: usize,
    origin: Instant,
    body: &str,
    pending: Option<&Mutex<Vec<(u64, u128)>>>,
) -> Value {
    let start_ns = monotonic_ns(origin);
    let result = engine.with_d27_foreground_owner_for_test(sequence, || match class {
        "canonical_write" => engine
            .write(&[prepared(body.to_string(), format!("d27-live-{sequence}"))])
            .map(|receipt| Some(receipt.cursor)),
        "foreground_hybrid_query" => engine.search("001 002").map(|_| None),
        "direct_embed" => engine.embed_text("d27 direct embed").map(|_| None),
        _ => panic!("unknown operation class"),
    });
    let end_ns = monotonic_ns(origin);
    match result {
        Ok(cursor) => {
            if let (Some(cursor), Some(pending)) = (cursor, pending) {
                pending.lock().expect("pending lock").push((cursor, end_ns));
            }
            json!({"class":class,"sequence":sequence,"admitted_ns":start_ns,"completed_ns":end_ns,"outcome":"completed","cursor":cursor})
        }
        Err(error) => {
            json!({"class":class,"sequence":sequence,"admitted_ns":start_ns,"completed_ns":end_ns,"outcome":format!("{error:?}"),"cursor":null})
        }
    }
}

fn epoch(
    engine: &Arc<Engine>,
    direction: &str,
    sequence: &mut usize,
    origin: Instant,
    body: &str,
    pending: Option<&Mutex<Vec<(u64, u128)>>>,
) -> Vec<Value> {
    let writes = if direction == "projection_heavy" { 4 } else { 1 };
    let queries = if direction == "projection_heavy" { 4 } else { 7 };
    let output = Mutex::new(Vec::with_capacity(10));
    let base = *sequence;
    thread::scope(|scope| {
        scope.spawn(|| {
            for offset in 0..writes {
                let result =
                    operation(engine, "canonical_write", base + offset, origin, body, pending);
                output.lock().expect("output lock").push(result);
            }
        });
        for worker in 0..4 {
            let output = &output;
            scope.spawn(move || {
                for offset in (worker..queries).step_by(4) {
                    let result = operation(
                        engine,
                        "foreground_hybrid_query",
                        base + writes + offset,
                        origin,
                        body,
                        pending,
                    );
                    output.lock().expect("output lock").push(result);
                }
            });
        }
        scope.spawn(|| {
            for offset in 0..2 {
                let result = operation(
                    engine,
                    "direct_embed",
                    base + writes + queries + offset,
                    origin,
                    body,
                    pending,
                );
                output.lock().expect("output lock").push(result);
            }
        });
    });
    *sequence += 10;
    output.into_inner().expect("output lock")
}

fn thread_count() -> usize {
    fs::read_dir("/proc/self/task").expect("thread inventory").count()
}

fn observe_projection(
    database: &std::path::Path,
    origin: Instant,
    pending: &Mutex<Vec<(u64, u128)>>,
    stop: &AtomicBool,
) -> (Vec<Value>, usize) {
    let connection =
        rusqlite::Connection::open(database).expect("projection observation connection");
    let mut statement = connection
        .prepare("SELECT EXISTS(SELECT 1 FROM _fathomdb_vector_rows WHERE write_cursor = ?1)")
        .expect("projection observation query");
    let mut seen = HashSet::new();
    let mut completions = Vec::new();
    let mut high_water = 0;
    loop {
        let snapshot = pending.lock().expect("pending lock").clone();
        high_water = high_water.max(snapshot.len().saturating_sub(seen.len()));
        for (cursor, committed_ns) in snapshot {
            if seen.contains(&cursor) {
                continue;
            }
            let exists: bool =
                statement.query_row([cursor], |row| row.get(0)).expect("projection row");
            if exists {
                seen.insert(cursor);
                completions.push(json!({"cursor":cursor,"committed_ns":committed_ns,"observed_ns":monotonic_ns(origin)}));
            }
        }
        if stop.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    (completions, high_water)
}

#[test]
#[ignore = "D27 is an explicitly invoked 10-second warm-up plus 60-second measurement"]
fn d27_runtime_qualification() {
    let smoke = std::env::var_os("D27_SMOKE").is_some();
    let direction = std::env::var("D27_DIRECTION").expect("D27_DIRECTION");
    assert!(direction == "projection_heavy" || direction == "foreground_heavy");
    let repetition: usize =
        std::env::var("D27_REPETITION").expect("D27_REPETITION").parse().expect("repetition");
    assert!((1..=3).contains(&repetition));
    let output = std::env::var("D27_RAW_OUTPUT").expect("D27_RAW_OUTPUT");
    let corpus =
        fs::read_to_string(std::env::var("D27_CORPUS").expect("D27_CORPUS")).expect("corpus");
    let records: Vec<Value> =
        corpus.lines().map(|line| serde_json::from_str(line).expect("corpus row")).collect();
    assert_eq!(records.len(), 10_000);
    let directory = TempDir::new().expect("tempdir");
    let database = directory.path().join(format!("d27{SQLITE_SUFFIX}"));
    let threads_before_open = thread_count();
    let provider = Arc::new(DeterministicEmbedder::new());
    let opened = Engine::open_with_choice_and_config(
        &database,
        EmbedderChoice::Caller(provider.clone()),
        EngineConfig { scheduler_runtime_threads: Some(2), embedder_pool_size: Some(1), ..EngineConfig::default() },
    ).expect("open");
    let engine = Arc::new(opened.engine);
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let seed_records = if smoke { &records[..100] } else { &records[..] };
    for chunk in seed_records.chunks(128) {
        let rows: Vec<PreparedWrite> = chunk
            .iter()
            .map(|row| {
                prepared(
                    row["body"].as_str().expect("body").to_string(),
                    row["logical_id"].as_str().expect("logical_id").to_string(),
                )
            })
            .collect();
        engine.write(&rows).expect("seed write");
    }
    engine.drain(120_000).expect("seed projection drain");

    let warmup_start = Instant::now();
    let mut sequence = 0;
    let warmup_seconds = if smoke { 1 } else { 10 };
    let measurement_seconds = if smoke { 1 } else { 60 };
    while warmup_start.elapsed() < Duration::from_secs(warmup_seconds) {
        epoch(&engine, &direction, &mut sequence, warmup_start, "d27 warmup document", None);
    }
    engine.drain(120_000).expect("warmup drain");
    let inventory = engine.d27_connection_inventory_for_test().expect("engine SQLite inventory");
    let origin = Instant::now();
    provider.peak.store(0, Ordering::SeqCst);
    engine.begin_d27_observation_for_test(origin);
    let pending = Mutex::new(Vec::new());
    let stop_observer = AtomicBool::new(false);
    let observer = thread::scope(|scope| {
        let observer =
            scope.spawn(|| observe_projection(&database, origin, &pending, &stop_observer));
        let mut operations = Vec::new();
        while origin.elapsed() < Duration::from_secs(measurement_seconds) {
            operations.extend(epoch(
                &engine,
                &direction,
                &mut sequence,
                origin,
                "d27 measured document",
                Some(&pending),
            ));
        }
        let admission_stop_ns = monotonic_ns(origin);
        engine.drain(120_000).expect("measured projection drain");
        let drain_end_ns = monotonic_ns(origin);
        stop_observer.store(true, Ordering::Release);
        (operations, admission_stop_ns, drain_end_ns, observer.join().expect("projection observer"))
    });
    let (
        operations,
        admission_stop_ns,
        drain_end_ns,
        (projection_completions, projection_backlog_high_water),
    ) = observer;
    let engine_thread_inventory = thread_count() - threads_before_open;
    let observation = engine.d27_observation_for_test().expect("engine D27 observation");
    let close_start_ns = monotonic_ns(origin);
    let close_result = engine.close();
    let close_end_ns = monotonic_ns(origin);
    let residual_workers_after_close = thread_count().saturating_sub(threads_before_open);
    let operation_counts = json!({
        "canonical_writes": operations.iter().filter(|op| op["class"] == "canonical_write").count(),
        "foreground_hybrid_queries": operations.iter().filter(|op| op["class"] == "foreground_hybrid_query").count(),
        "direct_embeds": operations.iter().filter(|op| op["class"] == "direct_embed").count(),
    });
    let raw = json!({
        "direction": direction,
        "repetition": repetition,
        "smoke": smoke,
        "warmup_seconds": warmup_seconds,
        "measurement_seconds": measurement_seconds,
        "measurement_elapsed_ns": admission_stop_ns,
        "epoch_size": 10,
        "operation_counts": operation_counts,
        "operations": operations,
        "projection_completions": projection_completions,
        "projection_backlog_high_water": projection_backlog_high_water,
        "projection_drain_end_ns": drain_end_ns,
        "provider_peak_concurrency": provider.peak.load(Ordering::SeqCst),
        "configuration_observation": observation.configuration_observation,
        "projection_admission_observation": observation.projection_admission_observation,
        "embed_dispatch_events": observation.embed_dispatch_events,
        "connection_inventory": inventory,
        "engine_thread_inventory": engine_thread_inventory,
        "residual_workers_after_close": residual_workers_after_close,
        "close_start_ns": close_start_ns,
        "close_end_ns": close_end_ns,
        "close_result": format!("{close_result:?}"),
    });
    fs::write(output, serde_json::to_vec_pretty(&raw).expect("json")).expect("raw output");
}
