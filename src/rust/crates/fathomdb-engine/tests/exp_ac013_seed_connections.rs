//! AC-013 seed-time connection probe (`dev/experiments/ac013-seed-connections/`).
//!
//! Report-only. It seeds the `ac_013_vector_retrieval_latency` corpus of
//! `perf_gates.rs` (same corpus seed, generator, `VaryingEmbedder`, batch of
//! 1,024 and vector kind `doc`) and reports, per connection that writes
//! during the seed, the page-cache hits and misses: the writer, the
//! projection dispatcher and each projection worker. It runs no search and
//! asserts nothing about time.
//!
//! ```text
//! cargo test --release -p fathomdb-engine --features test-hooks \
//!   --test exp_ac013_seed_connections -- --ignored --nocapture
//! ```
//!
//! Env knobs:
//!
//! - `AC013_CORPUS_N` (default 10,000) and `AC013_VECTOR_DIM` (default 768).
//! - `AC013_SEED_MODE`: `held` (default) freezes the projection scheduler
//!   while writing, so `write_ms` is the writer alone and `drain_ms` is the
//!   projection alone, starting from a fully written store; `overlap` is the
//!   gate's shape, where projection runs during the write and `drain_ms` is
//!   only what is left after the last write returns.
//! - `AC013_RUNTIME_THREADS`: `EngineConfig::scheduler_runtime_threads`
//!   (projection worker count; unset keeps the default, 2).
//! - `AC013_DRAIN_TIMEOUT_MS` (default 1,800,000).
//! - The engine PRAGMA levers: `FATHOMDB_PERF_EXPERIMENTS=1` with
//!   `FATHOMDB_PERF_RUNTIME_PRAGMAS` (dispatcher and workers),
//!   `FATHOMDB_PERF_WRITER_PRAGMAS` and `FATHOMDB_PERF_READER_PRAGMAS`.
//!   `LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT` in a separate
//!   `CARGO_TARGET_DIR` restores the shared page-cache group.
//!
//! Output: one `AC013_SEED {json}` line. `write_ms`, `drain_ms`, `seed_ms`
//! (their sum, the gate's `seed_ms`); `writer` holds the writer's hit / miss
//! deltas over the write phase and the drain phase; `runtime` holds one entry
//! per dispatcher and worker connection with its hits, misses, `rounds`
//! (dispatcher scans or worker commit batches), `jobs`, and the derived
//! `miss_per_job` and `miss_per_round`; `worker_totals` sums the workers.
//! `runtime_cache_size` is the `cache_size` a runtime connection opens with.
//! `corpus_digest` equals `exp_ac013_page_cache.rs`'s for the same N.

#![cfg(feature = "test-hooks")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineConfig, InitialState, PreparedWrite, RuntimeCacheStatusForTest,
    SourceId,
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

fn ratio(numerator: i64, denominator: u64) -> Option<f64> {
    (denominator > 0).then(|| numerator as f64 / denominator as f64)
}

fn runtime_entry(status: &RuntimeCacheStatusForTest) -> serde_json::Value {
    serde_json::json!({
        "role": status.role,
        "index": status.index,
        "cache_hit": status.cache_hit,
        "cache_miss": status.cache_miss,
        "cache_used_bytes": status.cache_used_bytes,
        "rounds": status.rounds,
        "jobs": status.jobs,
        "miss_per_job": ratio(status.cache_miss, status.jobs),
        "miss_per_round": ratio(status.cache_miss, status.rounds),
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
    drop(connection);
    let mut wal = path.as_os_str().to_owned();
    wal.push("-wal");
    let wal_bytes = std::fs::metadata(&wal).map(|m| m.len()).unwrap_or(0);
    serde_json::json!({
        "page_count": page_count,
        "page_size": page_size,
        "db_bytes": page_count * page_size,
        "wal_bytes": wal_bytes,
    })
}

#[test]
#[ignore = "report-only AC-013 seed connection probe; run with --ignored in --release"]
fn ac013_seed_connections_probe() {
    let n: usize = env_parse("AC013_CORPUS_N").unwrap_or(10_000);
    let dim: u32 = env_parse("AC013_VECTOR_DIM").unwrap_or(768);
    let mode = std::env::var("AC013_SEED_MODE").unwrap_or_else(|_| "held".to_string());
    assert!(mode == "held" || mode == "overlap", "AC013_SEED_MODE must be held or overlap");
    let threads: Option<u64> = env_parse("AC013_RUNTIME_THREADS");
    let drain_timeout: u64 = env_parse("AC013_DRAIN_TIMEOUT_MS").unwrap_or(1_800_000);
    let corpus = bodies(CORPUS_SEED, n);

    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac013_seed{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_choice_and_config(
        &path,
        EmbedderChoice::Caller(Arc::new(VaryingEmbedder::new(dim))),
        EngineConfig { scheduler_runtime_threads: threads, ..EngineConfig::default() },
    )
    .expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let runtime_cache_size = engine.runtime_pragma_for_test("cache_size").expect("cache_size");

    let held = mode == "held";
    if held {
        engine.set_projection_scheduler_frozen_for_test(true);
    }
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
    let runtime_after_write = engine.runtime_cache_status_for_test();
    if held {
        assert!(
            runtime_after_write.iter().all(|s| s.role != "projection_worker"),
            "no worker may commit while the scheduler is held: {runtime_after_write:?}"
        );
    }
    let drain_started = Instant::now();
    if held {
        engine.set_projection_scheduler_frozen_for_test(false);
    }
    engine.drain(drain_timeout).expect("drain");
    let drain = drain_started.elapsed();
    let writer_after_drain = engine.writer_cache_status_for_test().expect("writer status");
    assert_eq!(engine.vector_row_count_for_test().expect("vector rows"), n as u64);

    let runtime = engine.runtime_cache_status_for_test();
    let workers: Vec<_> = runtime.iter().filter(|s| s.role == "projection_worker").collect();
    let worker_jobs: u64 = workers.iter().map(|w| w.jobs).sum();
    let worker_rounds: u64 = workers.iter().map(|w| w.rounds).sum();
    let worker_miss: i64 = workers.iter().map(|w| w.cache_miss).sum();
    let worker_hit: i64 = workers.iter().map(|w| w.cache_hit).sum();
    let report = serde_json::json!({
        "n": n,
        "dim": dim,
        "mode": mode,
        "runtime_threads": threads,
        "corpus_digest": digest(&corpus),
        "write_ms": ms(write),
        "drain_ms": ms(drain),
        "seed_ms": ms(write + drain),
        "runtime_cache_size": runtime_cache_size,
        "writer": {
            "write_hit": writer_after_write.0 - writer_before.0,
            "write_miss": writer_after_write.1 - writer_before.1,
            "drain_hit": writer_after_drain.0 - writer_after_write.0,
            "drain_miss": writer_after_drain.1 - writer_after_write.1,
            "miss_per_row": (writer_after_drain.1 - writer_before.1) as f64 / n as f64,
            "cache_used_bytes": writer_after_drain.2,
        },
        "runtime": runtime.iter().map(runtime_entry).collect::<Vec<_>>(),
        "worker_totals": {
            "cache_hit": worker_hit,
            "cache_miss": worker_miss,
            "rounds": worker_rounds,
            "jobs": worker_jobs,
            "miss_per_job": ratio(worker_miss, worker_jobs),
            "miss_per_round": ratio(worker_miss, worker_rounds),
            "requests_per_job": ratio(worker_hit + worker_miss, worker_jobs),
        },
        "store": store_stats(&path),
        "runtime_pragmas": std::env::var("FATHOMDB_PERF_RUNTIME_PRAGMAS").unwrap_or_default(),
        "writer_pragmas": std::env::var("FATHOMDB_PERF_WRITER_PRAGMAS").unwrap_or_default(),
        "memory_management_compiled": unsafe {
            rusqlite::ffi::sqlite3_compileoption_used(c"ENABLE_MEMORY_MANAGEMENT".as_ptr())
        },
    });
    eprintln!("AC013_SEED {report}");
    opened.engine.close().expect("close");
}
