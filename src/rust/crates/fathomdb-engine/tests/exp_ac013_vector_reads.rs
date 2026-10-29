//! AC-013 vector-read probe (`dev/experiments/ac013-vector-reads/`).
//!
//! Report-only. Same fixture as `exp_ac013_page_cache.rs` (a copy of the
//! `ac_013_vector_retrieval_latency` generator in `perf_gates.rs`: corpus
//! seed, query seed, `VaryingEmbedder`, batch size, vector kind). It prints
//! one `AC013_VREADS {json}` line with:
//!
//! - `hybrid` / `text_only`: p50/p99/mean ms of a timed pass after a
//!   warm-up pass, and reader `CACHE_HIT` / `CACHE_MISS` per search summed
//!   over all readers;
//! - `results_digest`: FNV-1a over every query's hits (`write_cursor`,
//!   branch, score bits) of the timed hybrid pass, so two builds can be
//!   checked for identical retrieval; `ids_digest` is the same over
//!   `write_cursor` only. `AC013_RESULTS_OUT=<path>` also writes one line
//!   per query for diffing;
//! - `store`: page count, page size, main-file bytes, WAL bytes, and the
//!   file size after a `wal_checkpoint(TRUNCATE)` at the end;
//! - `seed_write_ms`, `drain_ms`;
//! - `trace` (only with `FATHOMDB_PERF_PAGE_TRACE=1` in the environment at
//!   open): page requests per search by SQL statement over the timed hybrid
//!   pass, inclusive and exclusive of nested statements
//!   (`src/page_trace.rs` explains the attribution). `trace_text_only` is
//!   the same over the text-only pass.
//!
//! ```text
//! FATHOMDB_PERF_PAGE_TRACE=1 cargo test --release -p fathomdb-engine \
//!   --features test-hooks --test exp_ac013_vector_reads -- --ignored --nocapture
//! ```
//!
//! Env knobs: `AC013_CORPUS_N` (default 10,000), `AC013_VECTOR_DIM`
//! (default 768), `AC013_SAMPLES` (warm-up and timed searches per pass,
//! default 1,000), `AC013_RESULTS_OUT`, `AC013_TRACE_TOP` (statements kept
//! in `trace`, default 30), `FATHOMDB_PERF_PAGE_TRACE`, and the engine's
//! PRAGMA levers (`FATHOMDB_PERF_EXPERIMENTS=1` with
//! `FATHOMDB_PERF_READER_PRAGMAS`). The tracer's own overhead inflates
//! latency, so take latency from an untraced run.

#![cfg(feature = "test-hooks")]

use std::io::Write as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    take_page_trace_for_test, CacheStatusReply, Engine, InitialState, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

/// Warm-up and timed searches per pass; `AC013_SAMPLES` overrides (gate: 1,000).
fn samples() -> usize {
    std::env::var("AC013_SAMPLES").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000)
}
const CORPUS_SEED: u64 = 0x0AC0_13D0_13D0;
const QUERY_SEED: u64 = 0x0AC0_130D_EC0D_E000;

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

/// FNV-1a over the texts, each followed by a 0 byte; the MCP harness prints
/// the same digest for its copy of the generator.
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

fn percentile_ceil(samples: &[Duration], numerator: usize, denominator: usize) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[((sorted.len() * numerator).div_ceil(denominator)).saturating_sub(1)]
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

fn sums(status: &[CacheStatusReply]) -> (i64, i64) {
    status
        .iter()
        .fold((0, 0), |(h, m), r| (h + i64::from(r.cache_hit), m + i64::from(r.cache_miss)))
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
    let file_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    serde_json::json!({
        "page_count": page_count,
        "page_size": page_size,
        "db_bytes": page_count * page_size,
        "file_bytes": file_bytes,
        "wal_bytes": wal_bytes,
    })
}

fn trace_json(top: usize, searches: usize) -> serde_json::Value {
    let rows = take_page_trace_for_test();
    let n = searches as f64;
    let total_excl: i64 = rows.iter().map(|r| r.exclusive_hit + r.exclusive_miss).sum();
    let total_excl_miss: i64 = rows.iter().map(|r| r.exclusive_miss).sum();
    serde_json::json!({
        "exclusive_requests_per_search": total_excl as f64 / n,
        "exclusive_miss_per_search": total_excl_miss as f64 / n,
        "statements": rows.iter().take(top).map(|r| serde_json::json!({
            "sql": r.sql,
            "runs_per_search": r.runs as f64 / n,
            "nested_runs_per_search": r.nested_runs as f64 / n,
            "excl_requests_per_search": (r.exclusive_hit + r.exclusive_miss) as f64 / n,
            "excl_miss_per_search": r.exclusive_miss as f64 / n,
            "incl_requests_per_search": (r.inclusive_hit + r.inclusive_miss) as f64 / n,
            "incl_miss_per_search": r.inclusive_miss as f64 / n,
        })).collect::<Vec<_>>(),
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
    }
}

#[test]
#[ignore = "report-only AC-013 vector-read probe; run with --ignored in --release"]
fn ac013_vector_reads_probe() {
    let n: usize =
        std::env::var("AC013_CORPUS_N").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let dim: u32 =
        std::env::var("AC013_VECTOR_DIM").ok().and_then(|v| v.parse().ok()).unwrap_or(768);
    let top: usize =
        std::env::var("AC013_TRACE_TOP").ok().and_then(|v| v.parse().ok()).unwrap_or(30);
    let tracing = std::env::var_os("FATHOMDB_PERF_PAGE_TRACE").is_some();
    let corpus = bodies(CORPUS_SEED, n);
    let queries = bodies(QUERY_SEED, samples());
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac013_vreads{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(VaryingEmbedder::new(dim)))
        .expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");

    let started = Instant::now();
    for chunk in corpus.chunks(1024) {
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
    let seed_write = started.elapsed();
    let drain_started = Instant::now();
    engine.drain(1_800_000).expect("drain");
    let drain = drain_started.elapsed();
    assert_eq!(engine.vector_row_count_for_test().expect("vector rows"), n as u64);

    for q in &queries {
        engine.search(q).expect("warmup search");
    }
    let _ = take_page_trace_for_test();

    let before = engine.cache_status_per_worker_for_test("hybrid");
    let mut samples_h = Vec::with_capacity(queries.len());
    let mut results = Fnv::new();
    let mut ids = Fnv::new();
    let mut out = std::env::var_os("AC013_RESULTS_OUT")
        .map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("results out")));
    for q in &queries {
        let t = Instant::now();
        let result = engine.search(q).expect("hybrid search");
        samples_h.push(t.elapsed());
        let mut line = String::new();
        for hit in &result.results {
            results.bytes(&hit.write_cursor.to_le_bytes());
            results.bytes(format!("{:?}", hit.branch).as_bytes());
            results.bytes(&hit.score.to_bits().to_le_bytes());
            ids.bytes(&hit.write_cursor.to_le_bytes());
            line.push_str(&format!(
                "{}:{:?}:{:016x} ",
                hit.write_cursor,
                hit.branch,
                hit.score.to_bits()
            ));
        }
        results.bytes(b"\n");
        ids.bytes(b"\n");
        if let Some(out) = out.as_mut() {
            writeln!(out, "{line}").expect("write results");
        }
    }
    let after = engine.cache_status_per_worker_for_test("hybrid");
    let trace_hybrid = tracing.then(|| trace_json(top, queries.len()));

    let mut samples_t = Vec::with_capacity(queries.len());
    let before_t = engine.cache_status_per_worker_for_test("text");
    for q in &queries {
        let t = Instant::now();
        engine.search_text_only(q).expect("text search");
        samples_t.push(t.elapsed());
    }
    let after_t = engine.cache_status_per_worker_for_test("text");
    let trace_text = tracing.then(|| trace_json(top, queries.len()));

    let nq = queries.len() as f64;
    let pass = |s: &[Duration], b: &[CacheStatusReply], a: &[CacheStatusReply]| {
        let (h0, m0) = sums(b);
        let (h1, m1) = sums(a);
        serde_json::json!({
            "p50_ms": ms(percentile_ceil(s, 50, 100)),
            "p99_ms": ms(percentile_ceil(s, 99, 100)),
            "mean_ms": ms(s.iter().sum::<Duration>()) / nq,
            "cache_hit_per_search": (h1 - h0) as f64 / nq,
            "cache_miss_per_search": (m1 - m0) as f64 / nq,
        })
    };
    let store = store_stats(&path);
    opened.engine.close().expect("close");
    let checkpointed = {
        let connection = rusqlite::Connection::open(&path).expect("reopen for checkpoint");
        let _: (i64, i64, i64) = connection
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .expect("checkpoint");
        drop(connection);
        std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0)
    };
    let report = serde_json::json!({
        "n": n,
        "dim": dim,
        "corpus_digest": digest(&corpus),
        "query_digest": digest(&queries),
        "results_digest": format!("{:016x}", results.0),
        "ids_digest": format!("{:016x}", ids.0),
        "seed_write_ms": ms(seed_write),
        "drain_ms": ms(drain),
        "hybrid": pass(&samples_h, &before, &after),
        "text_only": pass(&samples_t, &before_t, &after_t),
        "store": store,
        "file_bytes_checkpointed": checkpointed,
        "trace": trace_hybrid,
        "trace_text_only": trace_text,
        "page_trace": tracing,
        "reader_pragmas": std::env::var("FATHOMDB_PERF_READER_PRAGMAS").unwrap_or_default(),
    });
    eprintln!("AC013_VREADS {report}");
}
