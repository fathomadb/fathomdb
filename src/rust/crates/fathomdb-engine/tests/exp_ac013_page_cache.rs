//! AC-013 page-cache probe (`dev/experiments/ac013-page-cache/`).
//!
//! Report-only. It rebuilds the `ac_013_vector_retrieval_latency` fixture of
//! `perf_gates.rs` (same corpus seed, query seed, `VaryingEmbedder`, batch
//! size and vector kind; the generator below is a copy of that file's) and
//! adds what the gate does not print: seed write and drain time apart, the
//! reader page-cache hit and miss counts per search, each reader's
//! `cache_used_bytes`, and a text-only pass on the same queries, so the
//! hybrid cost splits into its text arm and the rest. It prints one
//! `AC013_PROBE {json}` line and asserts nothing about latency.
//!
//! ```text
//! cargo test --release -p fathomdb-engine --features test-hooks \
//!   --test exp_ac013_page_cache -- --ignored --nocapture
//! ```
//!
//! Env knobs: `AC013_CORPUS_N` (default 10,000), `AC013_VECTOR_DIM` (default
//! 768), `AC013_SAMPLES` (warm-up and timed searches per pass, default 1,000),
//! `AC013_ISO` (set: add 100 heavy text documents and an isolation phase),
//! and the engine's PRAGMA levers (`FATHOMDB_PERF_EXPERIMENTS=1` with
//! `FATHOMDB_PERF_READER_PRAGMAS` / `FATHOMDB_PERF_WRITER_PRAGMAS`, e.g.
//! `cache_size=-32000`). `LIBSQLITE3_FLAGS=-DSQLITE_ENABLE_MEMORY_MANAGEMENT`
//! in a separate `CARGO_TARGET_DIR` restores the shared page-cache group.
//!
//! Reading the output: `hybrid` and `text_only` give p50/p99/mean in ms for
//! the timed pass, `cache_hit_per_search` / `cache_miss_per_search` summed
//! over all readers (SQLite `CACHE_HIT` / `CACHE_MISS` deltas divided by the
//! search count), and `cache_used_bytes_per_reader` after the pass.
//! `cache_miss_per_reader` splits the pass's misses by reader, and
//! `vm_rss_kib` is the process RSS after it. `store` / `store_before_warmup`
//! give `page_count`, `page_size`, `db_bytes` and the WAL size, so misses ×
//! page size can be compared with the store's size. With `AC013_ISO`,
//! `isolation` holds a hybrid pass, one heavy text read on the next reader,
//! another hybrid pass, and `p50_ratio` (after / before): the AC-013-shaped
//! form of REQ40-I6. Heavy docs are kind `heavy`, so the vector rows and the
//! text arm's corpus are unchanged, but the text index is larger.
//! `memory_management_compiled` is 1 when the shared-group flag is built in.
//! `corpus_digest` / `query_digest` must match the MCP harness's
//! (`fathomdb-mcp/examples/ac013_mcp.rs`) for the two to share a workload.

#![cfg(feature = "test-hooks")]

use std::sync::Arc;
use std::time::{Duration, Instant};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{CacheStatusReply, Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

/// Warm-up and timed searches per pass; `AC013_SAMPLES` overrides (gate: 1,000).
fn samples() -> usize {
    std::env::var("AC013_SAMPLES").ok().and_then(|v| v.parse().ok()).unwrap_or(1_000)
}
const HEAVY_DOCS: usize = 100;
const HEAVY_TERM: &str = "ac013heavy";
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

/// About 40 KB of distinct filler words, so heavy bodies cost their own pages.
fn heavy_filler(doc: usize) -> String {
    (0..5_000)
        .map(|i| format!("h{:x}", (doc * 7919 + i * 104_729) % 1_000_003))
        .collect::<Vec<_>>()
        .join(" ")
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

/// The store's size at measurement time: main-file pages, page size, and the
/// WAL file's bytes and approximate frame count, read from a separate
/// read-only connection and the file system.
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
        "wal_frames_approx": wal_bytes.saturating_sub(32) / (page_size as u64 + 24),
    })
}

/// This process's resident set size in KiB (`VmRSS`), or -1 if unreadable.
fn vm_rss_kib() -> i64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1).and_then(|v| v.parse().ok()))
        })
        .unwrap_or(-1)
}

/// One timed pass: per-search latency and the reader cache deltas over it.
fn timed_pass(
    engine: &Engine,
    queries: &[String],
    label: &str,
    run: impl Fn(&Engine, &str),
) -> serde_json::Value {
    let before = engine.cache_status_per_worker_for_test(label);
    let mut samples = Vec::with_capacity(queries.len());
    for q in queries {
        let started = Instant::now();
        run(engine, q);
        samples.push(started.elapsed());
    }
    let after = engine.cache_status_per_worker_for_test(label);
    let (h0, m0) = sums(&before);
    let (h1, m1) = sums(&after);
    let n = queries.len() as f64;
    serde_json::json!({
        "p50_ms": ms(percentile_ceil(&samples, 50, 100)),
        "p99_ms": ms(percentile_ceil(&samples, 99, 100)),
        "mean_ms": ms(samples.iter().sum::<Duration>()) / n,
        "cache_hit_per_search": (h1 - h0) as f64 / n,
        "cache_miss_per_search": (m1 - m0) as f64 / n,
        "cache_used_bytes_per_reader": after.iter().map(|r| r.cache_used_bytes).collect::<Vec<_>>(),
        "cache_miss_per_reader": after
            .iter()
            .zip(&before)
            .map(|(a, b)| a.cache_miss - b.cache_miss)
            .collect::<Vec<_>>(),
        "vm_rss_kib": vm_rss_kib(),
    })
}

#[test]
#[ignore = "report-only AC-013 page-cache probe; run with --ignored in --release"]
fn ac013_page_cache_probe() {
    let n: usize =
        std::env::var("AC013_CORPUS_N").ok().and_then(|v| v.parse().ok()).unwrap_or(10_000);
    let dim: u32 =
        std::env::var("AC013_VECTOR_DIM").ok().and_then(|v| v.parse().ok()).unwrap_or(768);
    let corpus = bodies(CORPUS_SEED, n);
    let queries = bodies(QUERY_SEED, samples());
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("ac013_probe{SQLITE_SUFFIX}"));
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
    // AC013_ISO=1: also write 100 documents with ~40 KB bodies under a term
    // no corpus body or query contains, as kind `heavy` (not embedded). The
    // isolation phase below reads all of them in one text search.
    let iso = std::env::var_os("AC013_ISO").is_some();
    if iso {
        let heavy: Vec<PreparedWrite> = (0..HEAVY_DOCS)
            .map(|i| PreparedWrite::Node {
                kind: "heavy".to_string(),
                body: format!("{HEAVY_TERM} {}", heavy_filler(i)),
                source_id: SourceId::new("test:fixture").expect("test source id"),
                logical_id: None,
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        for chunk in heavy.chunks(20) {
            engine.write(chunk).expect("heavy write");
        }
    }
    let seed_write = started.elapsed();
    let drain_started = Instant::now();
    engine.drain(1_800_000).expect("drain");
    let drain = drain_started.elapsed();
    assert_eq!(engine.vector_row_count_for_test().expect("vector rows"), n as u64);

    let stats_before = store_stats(&path);
    let warm_before = engine.cache_status_per_worker_for_test("warmup");
    for q in &queries {
        engine.search(q).expect("warmup search");
    }
    let warm_after = engine.cache_status_per_worker_for_test("warmup");
    let hybrid = timed_pass(engine, &queries, "hybrid", |e, q| {
        e.search(q).expect("hybrid search");
    });
    let text = timed_pass(engine, &queries, "text", |e, q| {
        e.search_text_only(q).expect("text search");
    });
    // The AC-013-shaped form of REQ40-I6: a hybrid pass, one heavy text read
    // on the next reader, then another hybrid pass. The ratio is the after
    // pass's p50 over the before pass's.
    let isolation = iso.then(|| {
        let before = timed_pass(engine, &queries, "iso before", |e, q| {
            e.search(q).expect("hybrid search");
        });
        let heavy_reader = engine.next_reader_worker_index_for_test();
        let heavy = engine.search_text_only_with_limit(HEAVY_TERM, 100).expect("heavy search");
        assert_eq!(heavy.results.len(), HEAVY_DOCS, "the heavy read returns every heavy doc");
        let after_heavy = engine.cache_status_per_worker_for_test("after heavy");
        let after = timed_pass(engine, &queries, "iso after", |e, q| {
            e.search(q).expect("hybrid search");
        });
        let ratio = after["p50_ms"].as_f64().unwrap() / before["p50_ms"].as_f64().unwrap();
        serde_json::json!({
            "heavy_reader": heavy_reader,
            "cache_used_bytes_after_heavy": after_heavy
                .iter()
                .map(|r| r.cache_used_bytes)
                .collect::<Vec<_>>(),
            "before": before,
            "after": after,
            "p50_ratio": ratio,
        })
    });
    let (wh0, wm0) = sums(&warm_before);
    let (wh1, wm1) = sums(&warm_after);
    let report = serde_json::json!({
        "n": n,
        "dim": dim,
        "corpus_digest": digest(&corpus),
        "query_digest": digest(&queries),
        "store_before_warmup": stats_before,
        "seed_write_ms": ms(seed_write),
        "drain_ms": ms(drain),
        "warmup_cache_hit_per_search": (wh1 - wh0) as f64 / samples() as f64,
        "warmup_cache_miss_per_search": (wm1 - wm0) as f64 / samples() as f64,
        "store": store_stats(&path),
        "hybrid": hybrid,
        "text_only": text,
        "isolation": isolation,
        "reader_pragmas": std::env::var("FATHOMDB_PERF_READER_PRAGMAS").unwrap_or_default(),
        "writer_pragmas": std::env::var("FATHOMDB_PERF_WRITER_PRAGMAS").unwrap_or_default(),
        "memory_management_compiled": unsafe {
            rusqlite::ffi::sqlite3_compileoption_used(c"ENABLE_MEMORY_MANAGEMENT".as_ptr())
        },
    });
    eprintln!("AC013_PROBE {report}");
    opened.engine.close().expect("close");
}
