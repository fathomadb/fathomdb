//! AC-013 drain probe (`dev/experiments/ac013-seed-drain/`).
//!
//! Report-only. Seeds the `ac_013_vector_retrieval_latency` corpus (same
//! generator, corpus seed, `VaryingEmbedder`, batch of 1,024 and vector kind
//! `doc` as `perf_gates.rs` and `exp_ac013_seed_connections.rs`), then times
//! the projection drain and says where it goes. It runs no search and asserts
//! only that every row was projected.
//!
//! ```text
//! cargo test --release -p fathomdb-engine --features test-hooks \
//!   --test exp_ac013_seed_drain -- --ignored --nocapture
//! ```
//!
//! Env knobs:
//!
//! - `AC013_CORPUS_N` (default 10,000), `AC013_VECTOR_DIM` (default 768).
//! - `AC013_SEED_MODE`: `held` (default; the scheduler is frozen while
//!   writing, so `drain_ms` is the projection of a full backlog alone) or
//!   `overlap` (the gate's shape).
//! - `AC013_RUNTIME_THREADS`: `scheduler_runtime_threads` (default 2).
//! - `AC013_DRAIN_TIMEOUT_MS` (default 3,600,000).
//! - `AC013_SAMPLE_MS` (default 1,000; 0 disables): interval of the drain
//!   timeline, one sample of the phase counters each tick.
//! - `AC013_DIGEST` (default 1): after close, digest the projected store
//!   (`_fathomdb_vector_rows`, `vector_default` minus `created_at`,
//!   `_fathomdb_projection_terminal`, `_fathomdb_open_state`) so a candidate
//!   can be checked for identical results.
//! - `AC013_STORE_OUT`: `VACUUM INTO` this path after the write phase (held
//!   mode, outside every timed window), for offline `EXPLAIN QUERY PLAN`.
//! - `AC013_TRACE_TOP` (default 40): statements listed per run.
//! - `FATHOMDB_PERF_PAGE_TRACE=all` (or a role list) turns on the
//!   per-statement tracer (`src/page_trace.rs`); the probe clears it before
//!   the drain, so `trace` covers the drain only. The tracer's callback costs
//!   time: take timing from untraced runs.
//! - Engine levers: `FATHOMDB_PERF_EXPERIMENTS=1` with
//!   `FATHOMDB_PERF_RUNTIME_PRAGMAS`, `FATHOMDB_PERF_VEC0_CHUNK_SIZE`, and the
//!   drain candidates listed in the experiment README.
//!
//! Output: one `AC013_DRAIN {json}` line. `write_ms`, `drain_ms`, `seed_ms`;
//! `phase` (the drain-phase counters of `src/drain_trace.rs`, plus
//! per-vector derived values); `timeline` (`t_ms`, `jobs` committed and the
//! cumulative counters at each tick); `writer` cache deltas over the write and
//! the drain; `runtime` per-connection cache counters; `trace_by_role` and
//! `trace` (top statements by exclusive page requests, with `ns`); `store`
//! sizes; `digest` of the projected store; `knobs`.

#![cfg(feature = "test-hooks")]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    drain_phase_trace_for_test, reset_drain_phase_trace_for_test, take_page_trace_for_test,
    DrainPhaseTrace, EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const CORPUS_SEED: u64 = 0x0AC0_13D0_13D0;
const BATCH: usize = 1024;

struct SeededRng {
    state: u64,
}

impl SeededRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15).wrapping_add(1) }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.state
    }

    fn next_in(&mut self, bound: usize) -> usize {
        (self.next_u64() as usize) % bound
    }

    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

fn perf_vocab() -> Vec<String> {
    (0..1024)
        .map(|i: usize| {
            let a = (b'a' + ((i / 26 / 26) % 26) as u8) as char;
            let b = (b'a' + ((i / 26) % 26) as u8) as char;
            let c = (b'a' + (i % 26) as u8) as char;
            format!("{a}{b}{c}{i:04}")
        })
        .collect()
}

fn zipf_cumulative(vocab_size: usize) -> Vec<f64> {
    let mut acc = 0.0_f64;
    (1..=vocab_size)
        .map(|k| {
            acc += 1.0_f64 / k as f64;
            acc
        })
        .collect()
}

fn zipf_index(rng: &mut SeededRng, cumulative: &[f64]) -> usize {
    let r = rng.next_f64() * cumulative[cumulative.len() - 1];
    match cumulative.binary_search_by(|w| w.partial_cmp(&r).unwrap_or(std::cmp::Ordering::Equal)) {
        Ok(idx) => idx,
        Err(idx) => idx.min(cumulative.len() - 1),
    }
}

fn synth_chunk_body(rng: &mut SeededRng, vocab: &[String], cumulative: &[f64]) -> String {
    let mut body = String::with_capacity(512);
    let token_count = 55 + rng.next_in(20);
    for i in 0..token_count {
        if i > 0 {
            body.push(' ');
        }
        body.push_str(&vocab[zipf_index(rng, cumulative)]);
    }
    body
}

fn bodies(seed: u64, count: usize) -> Vec<String> {
    let vocab = perf_vocab();
    let cumulative = zipf_cumulative(vocab.len());
    let mut rng = SeededRng::new(seed);
    (0..count).map(|_| synth_chunk_body(&mut rng, &vocab, &cumulative)).collect()
}

fn digest(texts: &[String]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for text in texts {
        for &b in text.as_bytes().iter().chain(std::iter::once(&0u8)) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{h:016x}")
}

#[derive(Clone, Debug)]
struct VaryingEmbedder {
    identity: EmbedderIdentity,
    dim: u32,
}

impl VaryingEmbedder {
    fn new(dim: u32) -> Self {
        Self { identity: EmbedderIdentity::new("varying", "perf-gates-dense", dim), dim }
    }
}

impl Embedder for VaryingEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let dim = self.dim as usize;
        let mut h: u64 = 0xcbf29ce484222325;
        for &b in text.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        if h == 0 {
            h = 0xdeadbeef_cafebabe;
        }
        let mut state = h;
        let mut v = Vec::with_capacity(dim);
        for _ in 0..dim {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            v.push(((state as u32) as i32 as f32) / (i32::MAX as f32 + 1.0));
        }
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt().max(1e-6);
        for x in &mut v {
            *x /= norm;
        }
        Ok(v)
    }
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn env_parse<T: std::str::FromStr>(name: &str) -> Option<T> {
    std::env::var(name).ok().and_then(|v| v.parse().ok())
}

fn per(numerator: f64, denominator: u64) -> Option<f64> {
    (denominator > 0).then(|| numerator / denominator as f64)
}

fn phase_json(p: &DrainPhaseTrace) -> serde_json::Value {
    let jobs = p.worker_jobs;
    serde_json::json!({
        "dispatcher_scans": p.dispatcher_scans,
        "dispatcher_scan_ms": p.dispatcher_scan_ns as f64 / 1e6,
        "dispatcher_jobs": p.dispatcher_jobs,
        "dispatcher_pages": p.dispatcher_pages,
        "dispatcher_rows_seen": p.dispatcher_rows_seen,
        "dispatcher_inflight_skipped": p.dispatcher_inflight_skipped,
        "worker_batches": p.worker_batches,
        "worker_jobs": jobs,
        "worker_embed_ms": p.worker_embed_ns as f64 / 1e6,
        "worker_gate_wait_ms": p.worker_gate_wait_ns as f64 / 1e6,
        "worker_commit_ms": p.worker_commit_ns as f64 / 1e6,
        "drain_probes": p.drain_probes,
        "drain_probe_ms": p.drain_probe_ns as f64 / 1e6,
        "drain_probe_pending": p.drain_probe_pending,
        "per_vector_us": {
            "dispatcher_scan": per(p.dispatcher_scan_ns as f64 / 1e3, jobs),
            "worker_embed": per(p.worker_embed_ns as f64 / 1e3, jobs),
            "worker_commit": per(p.worker_commit_ns as f64 / 1e3, jobs),
            "worker_gate_wait": per(p.worker_gate_wait_ns as f64 / 1e3, jobs),
            "drain_probe": per(p.drain_probe_ns as f64 / 1e3, jobs),
        },
        "rows_seen_per_scan": per(p.dispatcher_rows_seen as f64, p.dispatcher_scans),
    })
}

fn timeline_sample(t: Duration, p: &DrainPhaseTrace) -> serde_json::Value {
    serde_json::json!({
        "t_ms": ms(t).round(),
        "jobs": p.worker_jobs,
        "scans": p.dispatcher_scans,
        "scan_ms": (p.dispatcher_scan_ns as f64 / 1e6).round(),
        "rows_seen": p.dispatcher_rows_seen,
        "embed_ms": (p.worker_embed_ns as f64 / 1e6).round(),
        "commit_ms": (p.worker_commit_ns as f64 / 1e6).round(),
        "gate_wait_ms": (p.worker_gate_wait_ns as f64 / 1e6).round(),
        "probes": p.drain_probes,
        "probe_ms": (p.drain_probe_ns as f64 / 1e6).round(),
    })
}

struct Fnv(u64);

impl Fnv {
    fn new() -> Self {
        Self(0xcbf29ce484222325)
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
        self.0 ^= 0xff;
        self.0 = self.0.wrapping_mul(0x100000001b3);
    }
    fn value(&mut self, value: rusqlite::types::ValueRef<'_>) {
        use rusqlite::types::ValueRef;
        match value {
            ValueRef::Null => self.bytes(b"N"),
            ValueRef::Integer(i) => self.bytes(&i.to_le_bytes()),
            ValueRef::Real(f) => self.bytes(&f.to_bits().to_le_bytes()),
            ValueRef::Text(t) => self.bytes(t),
            ValueRef::Blob(b) => self.bytes(b),
        }
    }
    fn hex(&self) -> String {
        format!("{:016x}", self.0)
    }
}

fn digest_query(connection: &rusqlite::Connection, sql: &str) -> serde_json::Value {
    let mut statement = match connection.prepare(sql) {
        Ok(statement) => statement,
        Err(err) => return serde_json::json!({ "error": err.to_string() }),
    };
    let columns = statement.column_count();
    let mut rows = statement.query([]).expect("digest query");
    let mut hash = Fnv::new();
    let mut count = 0_u64;
    while let Some(row) = rows.next().expect("digest row") {
        for i in 0..columns {
            hash.value(row.get_ref(i).expect("digest column"));
        }
        count += 1;
    }
    serde_json::json!({ "rows": count, "fnv": hash.hex() })
}

fn store_digest(path: &std::path::Path) -> serde_json::Value {
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only");
    let open_state: Vec<(String, String)> = connection
        .prepare("SELECT key, value FROM _fathomdb_open_state ORDER BY key")
        .expect("open_state")
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .expect("open_state rows")
        .collect::<Result<_, _>>()
        .expect("open_state collect");
    serde_json::json!({
        "vector_rows": digest_query(&connection,
            "SELECT rowid, kind, write_cursor FROM _fathomdb_vector_rows ORDER BY rowid"),
        "vector_default": digest_query(&connection,
            "SELECT rowid, embedding, embedding_bin, source_type, kind, status \
             FROM vector_default ORDER BY rowid"),
        "terminal": digest_query(&connection,
            "SELECT write_cursor, state FROM _fathomdb_projection_terminal ORDER BY write_cursor"),
        "open_state": open_state,
    })
}

fn store_stats(path: &std::path::Path) -> serde_json::Value {
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only");
    let page_count: i64 = connection.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap();
    let page_size: i64 = connection.query_row("PRAGMA page_size", [], |r| r.get(0)).unwrap();
    serde_json::json!({ "page_count": page_count, "page_size": page_size })
}

fn knobs() -> serde_json::Value {
    let names = [
        "FATHOMDB_PERF_EXPERIMENTS",
        "FATHOMDB_PERF_RUNTIME_PRAGMAS",
        "FATHOMDB_PERF_WRITER_PRAGMAS",
        "FATHOMDB_PERF_VEC0_CHUNK_SIZE",
        "FATHOMDB_PERF_DRAIN_SCAN",
        "FATHOMDB_PERF_DRAIN_PROBE",
        "FATHOMDB_PERF_PAGE_TRACE",
    ];
    serde_json::Value::Object(
        names
            .iter()
            .map(|name| {
                (name.to_string(), serde_json::json!(std::env::var(name).unwrap_or_default()))
            })
            .collect(),
    )
}

#[test]
#[ignore = "report-only AC-013 drain probe; run with --ignored in --release"]
fn ac013_seed_drain_probe() {
    let n: usize = env_parse("AC013_CORPUS_N").unwrap_or(10_000);
    let dim: u32 = env_parse("AC013_VECTOR_DIM").unwrap_or(768);
    let mode = std::env::var("AC013_SEED_MODE").unwrap_or_else(|_| "held".to_string());
    assert!(mode == "held" || mode == "overlap", "AC013_SEED_MODE must be held or overlap");
    let threads: Option<u64> = env_parse("AC013_RUNTIME_THREADS");
    let drain_timeout: u64 = env_parse("AC013_DRAIN_TIMEOUT_MS").unwrap_or(3_600_000);
    let sample_ms: u64 = env_parse("AC013_SAMPLE_MS").unwrap_or(1_000);
    let want_digest = env_parse::<u8>("AC013_DIGEST").unwrap_or(1) != 0;
    let trace_top: usize = env_parse("AC013_TRACE_TOP").unwrap_or(40);
    let corpus = bodies(CORPUS_SEED, n);

    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac013_drain{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(Arc::new(VaryingEmbedder::new(dim))),
        EngineConfig { scheduler_runtime_threads: threads, ..EngineConfig::default() },
    )
    .expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let held = mode == "held";
    if held {
        engine.set_projection_scheduler_frozen_for_test(true);
    }
    reset_drain_phase_trace_for_test();
    let writer_before = engine.writer_cache_status_for_test().expect("writer status");
    let started = Instant::now();
    for chunk in corpus.chunks(BATCH) {
        let batch: Vec<PreparedWrite> = chunk
            .iter()
            .map(|body| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: body.clone(),
                source_id: SourceId::new("test:fixture").expect("test source id"),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        engine.write(&batch).expect("seed write");
    }
    let write = started.elapsed();
    let writer_after_write = engine.writer_cache_status_for_test().expect("writer status");
    let write_phase = drain_phase_trace_for_test();
    if held {
        if let Ok(out) = std::env::var("AC013_STORE_OUT") {
            let copy = rusqlite::Connection::open(&path).expect("open copy source");
            copy.execute("VACUUM INTO ?1", [out.as_str()]).expect("vacuum into");
        }
        reset_drain_phase_trace_for_test();
    }
    let _ = take_page_trace_for_test();

    let stop = Arc::new(AtomicBool::new(false));
    let timeline = Arc::new(Mutex::new(Vec::new()));
    let drain_started = Instant::now();
    let sampler = (sample_ms > 0).then(|| {
        let stop = Arc::clone(&stop);
        let timeline = Arc::clone(&timeline);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(sample_ms));
                let sample =
                    timeline_sample(drain_started.elapsed(), &drain_phase_trace_for_test());
                timeline.lock().unwrap().push(sample);
            }
        })
    });
    if held {
        engine.set_projection_scheduler_frozen_for_test(false);
    }
    engine.drain(drain_timeout).expect("drain");
    let drain = drain_started.elapsed();
    stop.store(true, Ordering::Relaxed);
    if let Some(sampler) = sampler {
        sampler.join().expect("sampler");
    }
    let phase = drain_phase_trace_for_test();
    let trace = take_page_trace_for_test();
    let writer_after_drain = engine.writer_cache_status_for_test().expect("writer status");
    assert_eq!(engine.vector_row_count_for_test().expect("vector rows"), n as u64);
    let runtime = engine.runtime_cache_status_for_test();

    let mut by_role = std::collections::BTreeMap::<&str, (u64, i64, i64, u64)>::new();
    for row in &trace {
        let entry = by_role.entry(row.role).or_default();
        entry.0 += row.runs;
        entry.1 += row.exclusive_hit;
        entry.2 += row.exclusive_miss;
        entry.3 += row.exclusive_ns;
    }
    let jobs = phase.worker_jobs.max(1);
    let report = serde_json::json!({
        "n": n,
        "dim": dim,
        "mode": mode,
        "runtime_threads": threads,
        "corpus_digest": digest(&corpus),
        "write_ms": ms(write),
        "drain_ms": ms(drain),
        "seed_ms": ms(write + drain),
        "phase": phase_json(&phase),
        "write_phase": phase_json(&write_phase),
        "timeline": *timeline.lock().unwrap(),
        "writer": {
            "write_hit": writer_after_write.0 - writer_before.0,
            "write_miss": writer_after_write.1 - writer_before.1,
            "drain_hit": writer_after_drain.0 - writer_after_write.0,
            "drain_miss": writer_after_drain.1 - writer_after_write.1,
        },
        "runtime": runtime.iter().map(|s| serde_json::json!({
            "role": s.role, "index": s.index, "cache_hit": s.cache_hit,
            "cache_miss": s.cache_miss, "rounds": s.rounds, "jobs": s.jobs,
        })).collect::<Vec<_>>(),
        "trace_by_role": by_role.iter().map(|(role, (runs, hit, miss, ns))| serde_json::json!({
            "role": role, "runs": runs, "exclusive_hit": hit, "exclusive_miss": miss,
            "exclusive_ms": *ns as f64 / 1e6,
            "requests_per_vector": (hit + miss) as f64 / jobs as f64,
            "miss_per_vector": *miss as f64 / jobs as f64,
        })).collect::<Vec<_>>(),
        "trace": trace.iter().take(trace_top).map(|row| serde_json::json!({
            "role": row.role, "sql": row.sql, "runs": row.runs, "nested_runs": row.nested_runs,
            "exclusive_hit": row.exclusive_hit, "exclusive_miss": row.exclusive_miss,
            "inclusive_hit": row.inclusive_hit, "inclusive_miss": row.inclusive_miss,
            "ms": row.ns as f64 / 1e6, "exclusive_ms": row.exclusive_ns as f64 / 1e6,
            "requests_per_run": (row.exclusive_hit + row.exclusive_miss) as f64
                / row.runs.max(1) as f64,
        })).collect::<Vec<_>>(),
        "knobs": knobs(),
    });
    opened.engine.close().expect("close");
    let mut report = report;
    report["store"] = store_stats(&path);
    if want_digest {
        report["digest"] = store_digest(&path);
    }
    eprintln!("AC013_DRAIN {report}");
}
