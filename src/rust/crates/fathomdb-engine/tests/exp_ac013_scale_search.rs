//! AC-013 scale-search probe (`dev/experiments/ac013-scale-search/`).
//!
//! Report-only, `#[ignore]`. It extends `exp_ac013_vector_reads.rs` (same
//! synthetic generator, seeds, `VaryingEmbedder`, batch size and vector kind,
//! and the same `results_digest` / `ids_digest` formulas) to explain where a
//! hybrid search spends its time as the store grows. It prints one
//! `AC013_SCALE {json}` line with, per pass:
//!
//! - `p50_ms` / `p99_ms` / `mean_ms` over the timed searches, after a warm-up
//!   pass of the same queries; reader `CACHE_HIT` / `CACHE_MISS` per search;
//!   `soft_fallback_vector` (searches whose vector arm was declined);
//! - `digest` (every hit's `write_cursor`, branch and score bits, FNV-1a) and
//!   `ids_digest` (`write_cursor` only). The hybrid pass's digests are the
//!   vector-read probe's `results_digest` / `ids_digest`;
//! - `trace` (only with `FATHOMDB_PERF_PAGE_TRACE=1` at open): per SQL
//!   statement, page requests, misses and elapsed time per search, inclusive
//!   and exclusive of nested statements (`src/page_trace.rs`). The tracer's
//!   own callbacks add time per statement, so take latency from untraced runs
//!   and use the traced time for attribution.
//!
//! and once: `matches` (the text arm's FTS match count per distinct query:
//! min, p50, p90, max, mean, and as a fraction of the store), seed and drain
//! time, store size.
//!
//! Passes (`AC013_PASSES`, default `hybrid,text`):
//!
//! - `hybrid`: `Engine::search` (the AC-013 gate's call);
//! - `text`: `Engine::search_text_only` on the same queries;
//! - `vector`: `Engine::search` on queries whose tokens match no document
//!   (`zqv<i>`), so the text arm returns nothing and the search is the
//!   eligibility checks, the vector stage and hydration.
//!
//! ```text
//! FATHOMDB_PERF_PAGE_TRACE=1 cargo test --release -p fathomdb-engine \
//!   --features test-hooks --test exp_ac013_scale_search -- --ignored --nocapture
//! ```
//!
//! Env knobs:
//!
//! - `AC013_CORPUS_N` (default 10,000), `AC013_VECTOR_DIM` (768),
//!   `AC013_SAMPLES` (searches per pass, warm-up and timed; 1,000);
//! - `AC013_CORPUS_FILE` / `AC013_QUERY_FILE`: JSONL files of strings
//!   (`dev/experiments/ac013-scale-search/lme_corpus.py`) used instead of the
//!   synthetic generator. The corpus is the first `AC013_CORPUS_N` lines; the
//!   queries are cycled to `AC013_SAMPLES`;
//! - `AC013_SUPERSEDE_N`: after the drain, re-write that many seeded documents
//!   under the same `logical_id` with a changed body (a supersession, as an
//!   agent-memory store accumulates), then drain again;
//! - `AC013_PASSES`, `AC013_TRACE_TOP` (statements kept per trace, 40),
//!   `AC013_RESULTS_OUT` (per-query hybrid hit lines), and the engine's
//!   `FATHOMDB_PERF_*` experiment knobs.

#![cfg(feature = "test-hooks")]

use std::io::{BufRead as _, Write as _};
use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    take_page_trace_for_test, CacheStatusReply, Engine, InitialState, PreparedWrite, SearchResult,
    SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const CORPUS_SEED: u64 = 0x0AC0_13D0_13D0;
const QUERY_SEED: u64 = 0x0AC0_130D_EC0D_E000;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name).ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

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

fn read_jsonl(path: &str, limit: usize) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap_or_else(|e| panic!("open {path}: {e}"));
    std::io::BufReader::new(file)
        .lines()
        .take(limit)
        .map(|line| serde_json::from_str::<String>(&line.expect("read line")).expect("json string"))
        .collect()
}

fn digest(texts: &[String]) -> String {
    let mut h = Fnv::new();
    for text in texts {
        h.bytes(text.as_bytes());
        h.bytes(&[0]);
    }
    format!("{:016x}", h.0)
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

fn trace_json(top: usize, searches: usize) -> serde_json::Value {
    let rows = take_page_trace_for_test();
    let n = searches as f64;
    let total_excl: i64 = rows.iter().map(|r| r.exclusive_hit + r.exclusive_miss).sum();
    let total_excl_miss: i64 = rows.iter().map(|r| r.exclusive_miss).sum();
    let total_excl_ns: u64 = rows.iter().map(|r| r.exclusive_ns).sum();
    let mut by_time = rows.clone();
    by_time.sort_by_key(|r| std::cmp::Reverse(r.exclusive_ns));
    serde_json::json!({
        "exclusive_requests_per_search": total_excl as f64 / n,
        "exclusive_miss_per_search": total_excl_miss as f64 / n,
        "exclusive_ms_per_search": total_excl_ns as f64 / 1e6 / n,
        "statements": by_time.iter().take(top).map(|r| serde_json::json!({
            "sql": r.sql,
            "runs_per_search": r.runs as f64 / n,
            "nested_runs_per_search": r.nested_runs as f64 / n,
            "excl_requests_per_search": (r.exclusive_hit + r.exclusive_miss) as f64 / n,
            "excl_miss_per_search": r.exclusive_miss as f64 / n,
            "incl_requests_per_search": (r.inclusive_hit + r.inclusive_miss) as f64 / n,
            "incl_miss_per_search": r.inclusive_miss as f64 / n,
            "excl_ms_per_search": r.exclusive_ns as f64 / 1e6 / n,
            "incl_ms_per_search": r.inclusive_ns as f64 / 1e6 / n,
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

fn node(body: String, logical_id: Option<String>) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body,
        source_id: SourceId::new("test:fixture").expect("test source id"),
        logical_id,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

/// The text arm's FTS match count per distinct query, on a read-only
/// connection, with the engine's own query compiler.
fn match_counts(path: &std::path::Path, queries: &[String], n: usize) -> serde_json::Value {
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("open read-only");
    let mut distinct: Vec<&String> = queries.iter().collect();
    distinct.sort();
    distinct.dedup();
    let mut statement = connection
        .prepare("SELECT count(*) FROM search_index WHERE search_index MATCH ?1")
        .unwrap();
    let mut counts: Vec<i64> = Vec::with_capacity(distinct.len());
    let mut terms = 0usize;
    for q in &distinct {
        let compiled = fathomdb_query::compile_text_query(q.as_str());
        terms += compiled.terms.len();
        if compiled.match_expression.is_empty() {
            counts.push(0);
            continue;
        }
        counts.push(statement.query_row([&compiled.match_expression], |r| r.get(0)).unwrap());
    }
    counts.sort_unstable();
    let at = |p: f64| counts[((counts.len() as f64 * p) as usize).min(counts.len() - 1)];
    let mean = counts.iter().sum::<i64>() as f64 / counts.len() as f64;
    serde_json::json!({
        "distinct_queries": counts.len(),
        "terms_mean": terms as f64 / counts.len() as f64,
        "min": counts[0], "p50": at(0.5), "p90": at(0.9), "max": counts[counts.len() - 1],
        "mean": mean,
        "frac_p50": at(0.5) as f64 / n as f64,
        "frac_mean": mean / n as f64,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum Pass {
    Hybrid,
    Text,
    Vector,
}

fn run_pass(
    engine: &Engine,
    pass: Pass,
    queries: &[String],
    tracing: bool,
    top: usize,
    out: &mut Option<std::io::BufWriter<std::fs::File>>,
) -> serde_json::Value {
    let search = |q: &str| -> SearchResult {
        match pass {
            Pass::Text => engine.search_text_only(q).expect("text search"),
            _ => engine.search(q).expect("search"),
        }
    };
    for q in queries {
        search(q);
    }
    let _ = take_page_trace_for_test();
    let label = if pass == Pass::Text { "text" } else { "hybrid" };
    let before = engine.cache_status_per_worker_for_test(label);
    let mut samples = Vec::with_capacity(queries.len());
    let mut results = Fnv::new();
    let mut ids = Fnv::new();
    let mut fallbacks = 0usize;
    let mut hits = 0usize;
    for q in queries {
        let t = Instant::now();
        let result = search(q);
        samples.push(t.elapsed());
        fallbacks += usize::from(result.soft_fallback.is_some());
        hits += result.results.len();
        let mut line = String::new();
        for hit in &result.results {
            results.bytes(&hit.write_cursor.to_le_bytes());
            results.bytes(format!("{:?}", hit.branch).as_bytes());
            results.bytes(&hit.score.to_bits().to_le_bytes());
            ids.bytes(&hit.write_cursor.to_le_bytes());
            if out.is_some() {
                line.push_str(&format!(
                    "{}:{:?}:{:016x} ",
                    hit.write_cursor,
                    hit.branch,
                    hit.score.to_bits()
                ));
            }
        }
        results.bytes(b"\n");
        ids.bytes(b"\n");
        if pass == Pass::Hybrid {
            if let Some(out) = out.as_mut() {
                writeln!(out, "{line}").expect("write results");
            }
        }
    }
    let after = engine.cache_status_per_worker_for_test(label);
    let nq = queries.len() as f64;
    let (h0, m0) = sums(&before);
    let (h1, m1) = sums(&after);
    serde_json::json!({
        "p50_ms": ms(percentile_ceil(&samples, 50, 100)),
        "p99_ms": ms(percentile_ceil(&samples, 99, 100)),
        "mean_ms": ms(samples.iter().sum::<Duration>()) / nq,
        "cache_hit_per_search": (h1 - h0) as f64 / nq,
        "cache_miss_per_search": (m1 - m0) as f64 / nq,
        "soft_fallback_searches": fallbacks,
        "hits_per_search": hits as f64 / nq,
        "digest": format!("{:016x}", results.0),
        "ids_digest": format!("{:016x}", ids.0),
        "trace": tracing.then(|| trace_json(top, queries.len())),
    })
}

#[test]
#[ignore = "report-only AC-013 scale-search probe; run with --ignored in --release"]
fn ac013_scale_search_probe() {
    let n = env_usize("AC013_CORPUS_N", 10_000);
    let dim = env_usize("AC013_VECTOR_DIM", 768) as u32;
    let samples = env_usize("AC013_SAMPLES", 1_000);
    let top = env_usize("AC013_TRACE_TOP", 40);
    let supersede = env_usize("AC013_SUPERSEDE_N", 0);
    let tracing = std::env::var_os("FATHOMDB_PERF_PAGE_TRACE").is_some();
    let passes = std::env::var("AC013_PASSES").unwrap_or_else(|_| "hybrid,text".to_string());
    let corpus_file = std::env::var("AC013_CORPUS_FILE").ok();
    let (corpus, queries, corpus_name) = match corpus_file.as_deref() {
        Some(path) => {
            let corpus = read_jsonl(path, n);
            assert_eq!(corpus.len(), n, "{path} has fewer than {n} documents");
            let qpath = std::env::var("AC013_QUERY_FILE").expect("AC013_QUERY_FILE");
            let pool = read_jsonl(&qpath, usize::MAX);
            let queries: Vec<String> = pool.iter().cycle().take(samples).cloned().collect();
            (corpus, queries, path.to_string())
        }
        None => (bodies(CORPUS_SEED, n), bodies(QUERY_SEED, samples), "synthetic".to_string()),
    };
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac013_scale{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(VaryingEmbedder::new(dim)))
        .expect("open");
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").expect("vector kind");

    let started = Instant::now();
    for (start, chunk) in (0..).step_by(1024).zip(corpus.chunks(1024)) {
        let batch: Vec<PreparedWrite> = chunk
            .iter()
            .enumerate()
            .map(|(i, body)| {
                let lid = (start + i < supersede).then(|| format!("sup-{}", start + i));
                node(body.clone(), lid)
            })
            .collect();
        engine.write(&batch).expect("seed write");
    }
    let seed_write = started.elapsed();
    let drain_started = Instant::now();
    engine.drain(3_600_000).expect("drain");
    if supersede > 0 {
        let batch: Vec<PreparedWrite> = (0..supersede)
            .map(|i| node(format!("{} revised", corpus[i]), Some(format!("sup-{i}"))))
            .collect();
        engine.write(&batch).expect("supersede write");
        engine.drain(3_600_000).expect("drain after supersede");
    }
    let drain = drain_started.elapsed();
    let vector_rows = engine.vector_row_count_for_test().expect("vector rows");
    assert_eq!(vector_rows, (n + supersede) as u64);
    let matches = match_counts(&path, &queries, n);

    let mut out = std::env::var_os("AC013_RESULTS_OUT")
        .map(|p| std::io::BufWriter::new(std::fs::File::create(p).expect("results out")));
    let vector_queries: Vec<String> = (0..queries.len()).map(|i| format!("zqv{i}")).collect();
    let mut report = serde_json::Map::new();
    for pass in passes.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let value = match pass {
            "hybrid" => run_pass(engine, Pass::Hybrid, &queries, tracing, top, &mut out),
            "text" => run_pass(engine, Pass::Text, &queries, tracing, top, &mut out),
            "vector" => run_pass(engine, Pass::Vector, &vector_queries, tracing, top, &mut out),
            other => panic!("unknown pass {other}"),
        };
        report.insert(pass.to_string(), value);
    }
    let page_count: i64 = {
        let c = rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .expect("open read-only");
        c.query_row("PRAGMA page_count", [], |r| r.get(0)).unwrap()
    };
    opened.engine.close().expect("close");
    let experiments: serde_json::Map<String, serde_json::Value> = std::env::vars()
        .filter(|(k, _)| k.starts_with("FATHOMDB_PERF_"))
        .map(|(k, v)| (k, serde_json::Value::String(v)))
        .collect();
    let full = serde_json::json!({
        "n": n,
        "dim": dim,
        "samples": samples,
        "corpus": corpus_name,
        "corpus_digest": digest(&corpus),
        "query_digest": digest(&queries),
        "supersede_n": supersede,
        "seed_write_ms": ms(seed_write),
        "drain_ms": ms(drain),
        "page_count": page_count,
        "matches": matches,
        "passes": report,
        "page_trace": tracing,
        "env": experiments,
    });
    eprintln!("AC013_SCALE {full}");
}
