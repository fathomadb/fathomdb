//! Slice 40 (0.8.28) Unit I — each SQLite connection has its own page cache.
//!
//! The bundled SQLite used to be built with `SQLITE_ENABLE_MEMORY_MANAGEMENT`,
//! which puts every connection in one `pcache1` group. A reader at its own
//! `cache_size` limit then recycles the group's least recently used page,
//! which can belong to another reader, and the robbed readers stay near one
//! page until the engine is reopened. These tests pin the per-connection
//! cache: the build option is off, a heavy read stays inside its own reader's
//! budget, and the other readers keep their working sets.
//!
//! The counters come from `cache_status_per_worker_for_test`; queries are
//! pinned to readers through `next_reader_worker_index_for_test`.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use fathomdb_engine::{
    CacheStatusReply, Engine, InitialState, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use serde_json::Value;
use tempfile::TempDir;

// Under a shared page-cache group every connection in the test process
// competes for the same pages, so parallel tests would perturb each other's
// counters. A failing test poisons the lock; later tests recover it.
static CACHE_LOCK: Mutex<()> = Mutex::new(());

const READERS: usize = 8;
const WARM_DOCS: usize = 40;
const HEAVY_DOCS: usize = 100;

/// SQLite's default `cache_size` is `-2000`: a 2,000 KiB budget per
/// connection, which the production open path keeps. SQLite turns it into
/// `nMax = 2,048,000 / (page + extra)` pages, while `CACHE_USED` counts each
/// page at its full allocation (page, extra and the `PgHdr1` header), so a full
/// cache reports about 2.6% over 2,048,000 bytes; `mxPinned` lets about ten
/// more pinned pages through. 1.10× covers both. A shared-group build lets
/// the heavy reader reach about twice the budget.
const CACHE_BUDGET_BYTES: f64 = 2_048_000.0;
const CACHE_BOUND_FACTOR: f64 = 1.10;

fn db_path(dir: &TempDir) -> PathBuf {
    dir.path().join(format!("reader-cache{SQLITE_SUFFIX}"))
}

fn doc(logical_id: &str, text: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: serde_json::json!({ "text": text }).to_string(),
        source_id: SourceId::new("test:slice40-reader-cache").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

/// `words` distinct pseudo-random filler words, so bodies do not compress
/// into shared pages and each document costs its own pages.
fn filler(seed: usize, words: usize) -> String {
    (0..words)
        .map(|i| format!("w{:x}", (seed * 7919 + i * 104_729) % 1_000_003))
        .collect::<Vec<_>>()
        .join(" ")
}

fn warm_term(reader: usize) -> String {
    format!("s40iw{reader}")
}

const HEAVY_TERM: &str = "s40ih";

/// Eight warm sets of 40 documents with about 3 KB bodies, one per reader,
/// and one heavy set of 100 documents with about 40 KB bodies.
fn build(path: &Path) {
    let opened = Engine::open(path).expect("open");
    let mut writes = Vec::new();
    for reader in 0..READERS {
        for i in 0..WARM_DOCS {
            let text = format!("{} {}", warm_term(reader), filler(reader * 100 + i, 400));
            writes.push(doc(&format!("w{reader}-{i}"), text));
        }
    }
    for i in 0..HEAVY_DOCS {
        writes.push(doc(&format!("h-{i}"), format!("{HEAVY_TERM} {}", filler(10_000 + i, 5_000))));
    }
    for chunk in writes.chunks(100) {
        opened.engine.write(chunk).expect("write corpus");
    }
    opened.engine.drain(60_000).expect("drain");
    opened.engine.close().expect("close after build");
}

/// One pass: eight consecutive searches; each reader runs its own warm set.
/// Returns the ids per reader.
fn pass(engine: &Engine) -> Vec<Vec<String>> {
    let mut ids = vec![Vec::new(); READERS];
    for _ in 0..READERS {
        let reader = engine.next_reader_worker_index_for_test();
        let result =
            engine.search_text_only_with_limit(&warm_term(reader), 100).expect("warm search");
        assert_eq!(result.results.len(), WARM_DOCS, "reader {reader} hit its own warm set");
        ids[reader] = result.results.iter().map(|hit| hit.id.value.clone()).collect();
    }
    ids
}

fn misses(after: &[CacheStatusReply], before: &[CacheStatusReply]) -> Vec<i32> {
    after.iter().zip(before).map(|(a, b)| a.cache_miss - b.cache_miss).collect()
}

/// Three warm passes, then the heavy read on the next reader. Returns that
/// reader, the status after the heavy read, and the first pass's ids.
fn warm_then_heavy(engine: &Engine) -> (usize, Vec<CacheStatusReply>, Vec<Vec<String>>) {
    let first = pass(engine);
    pass(engine);
    let before_third = engine.cache_status_per_worker_for_test("before third pass");
    assert_eq!(before_third.len(), READERS);
    assert_eq!(pass(engine), first, "warm passes return the same ids");
    let after_third = engine.cache_status_per_worker_for_test("after third pass");
    assert_eq!(
        misses(&after_third, &before_third),
        vec![0; READERS],
        "precondition: each reader's warm set fits in its own cache"
    );
    let heavy_reader = engine.next_reader_worker_index_for_test();
    let heavy = engine.search_text_only_with_limit(HEAVY_TERM, 100).expect("heavy search");
    assert_eq!(heavy.results.len(), HEAVY_DOCS, "the heavy read returns the whole heavy set");
    let after_heavy = engine.cache_status_per_worker_for_test("after heavy read");
    (heavy_reader, after_heavy, first)
}

/// AC40-95: the bundled SQLite gives each connection its own page cache.
#[test]
fn bundled_sqlite_keeps_a_page_cache_per_connection() {
    let used =
        unsafe { rusqlite::ffi::sqlite3_compileoption_used(c"ENABLE_MEMORY_MANAGEMENT".as_ptr()) };
    assert_eq!(used, 0, "SQLITE_ENABLE_MEMORY_MANAGEMENT puts every connection in one page cache");
}

/// AC40-96: a read larger than one reader's cache budget stays inside that
/// reader's budget instead of taking other readers' pages.
#[test]
fn a_heavy_read_stays_inside_its_own_page_cache() {
    let _lock = CACHE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir);
    build(&path);
    let opened = Engine::open(&path).expect("reopen");
    let (heavy_reader, after_heavy, _) = warm_then_heavy(&opened.engine);
    let used = f64::from(after_heavy[heavy_reader].cache_used_bytes);
    let bound = CACHE_BUDGET_BYTES * CACHE_BOUND_FACTOR;
    assert!(
        used <= bound,
        "reader {heavy_reader} holds {used} bytes after the heavy read, above {bound}: {:?}",
        after_heavy.iter().map(|r| r.cache_used_bytes).collect::<Vec<_>>()
    );
    opened.engine.close().expect("close");
}

/// AC40-97: after the heavy read, every other reader's next pass over its own
/// working set is served from its own cache, and every pass returns the same
/// ids.
#[test]
fn other_readers_keep_their_page_caches_after_a_heavy_read() {
    let _lock = CACHE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir);
    build(&path);
    let opened = Engine::open(&path).expect("reopen");
    let (heavy_reader, after_heavy, first) = warm_then_heavy(&opened.engine);
    assert_eq!(pass(&opened.engine), first, "a page cache cannot change results");
    let after_pass = opened.engine.cache_status_per_worker_for_test("after the next pass");
    let delta = misses(&after_pass, &after_heavy);
    let robbed: Vec<(usize, i32)> = delta
        .iter()
        .enumerate()
        .filter(|(reader, miss)| *reader != heavy_reader && **miss != 0)
        .map(|(reader, miss)| (reader, *miss))
        .collect();
    assert!(
        robbed.is_empty(),
        "readers lost cached pages to reader {heavy_reader}'s heavy read (reader, misses): {robbed:?}"
    );
    opened.engine.close().expect("close");
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let index = ((sorted.len() as f64 * fraction).ceil() as usize).max(1) - 1;
    sorted[index]
}

/// Report-only (REQ40-I5): body-search latency before and after one selective
/// projected query on the same open engine, on the Defect C route and on the
/// cohort route, each on a fresh open of a store built from the `tc5-10000`
/// input pack (`FATHOMDB_TC5_INPUTS`). Prints p50 and p95 per pass and each
/// reader's cache counters. Gates nothing.
#[test]
#[ignore = "report-only timing; needs FATHOMDB_TC5_INPUTS and --release"]
fn reader_page_cache_timing_report() {
    let _lock = CACHE_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let inputs = PathBuf::from(std::env::var("FATHOMDB_TC5_INPUTS").expect("FATHOMDB_TC5_INPUTS"));
    let read = |name: &str| -> Vec<Value> {
        serde_json::from_str(&std::fs::read_to_string(inputs.join(name)).expect(name)).expect(name)
    };
    let corpus = read("corpus.json");
    let queries: Vec<String> = read("queries.json")
        .iter()
        .map(|q| q["text"].as_str().expect("query text").to_string())
        .collect();
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("tc5{SQLITE_SUFFIX}"));
    {
        let opened = Engine::open(&path).expect("open");
        let spec = |name: &str, role: ProjectionRole, fts: bool| ProjectionSpec {
            name: name.to_string(),
            roles: [role].into_iter().collect(),
            fts: fts.then_some(ProjectionFts { tokenizer: None }),
            vector: None,
            source: None,
        };
        opened
            .engine
            .configure_projections(
                &[
                    spec("text", ProjectionRole::Searchable, true),
                    spec("bench_cohort", ProjectionRole::Filterable, false),
                ],
                &[],
            )
            .expect("configure projections");
        let writes: Vec<PreparedWrite> = corpus
            .iter()
            .map(|d| PreparedWrite::Node {
                kind: "doc".to_string(),
                body: serde_json::json!({ "text": d["body"], "bench_cohort": d["cohort"] })
                    .to_string(),
                source_id: SourceId::new("test:tc5").expect("source id"),
                logical_id: Some(d["id"].as_str().expect("id").to_string()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            })
            .collect();
        for chunk in writes.chunks(1000) {
            opened.engine.write(chunk).expect("write");
        }
        opened.engine.drain(600_000).expect("drain");
        opened.engine.close().expect("close");
    }
    for (route, prefilter) in [("defect-c", "off"), ("cohort", "on")] {
        let opened = Engine::open(&path).expect("reopen");
        let engine = &opened.engine;
        let body_pass = |label: &str| {
            let before = engine.cache_status_per_worker_for_test(label);
            let mut timings: Vec<f64> = queries
                .iter()
                .map(|q| {
                    let started = Instant::now();
                    engine.search_text_only_with_limit(q, 10).expect("body search");
                    started.elapsed().as_secs_f64() * 1000.0
                })
                .collect();
            timings.sort_by(f64::total_cmp);
            let after = engine.cache_status_per_worker_for_test(label);
            println!(
                "unit-i route={route} pass={label:?} p50_ms={:.3} p95_ms={:.3} \
                 cache_used_bytes={:?} cache_miss_delta={:?}",
                percentile(&timings, 0.5),
                percentile(&timings, 0.95),
                after.iter().map(|r| r.cache_used_bytes).collect::<Vec<_>>(),
                misses(&after, &before),
            );
        };
        body_pass("before 1");
        body_pass("before 2");
        // SAFETY of the environment write: the file lock serializes the
        // tests here, and no other thread reads this variable meanwhile.
        unsafe { std::env::set_var("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", prefilter) };
        let mut filter = SearchFilter::default();
        filter.attributes = vec![("bench_cohort".to_string(), "c01".to_string())];
        engine
            .search_projected_text_with_limit(
                &queries[0],
                "text",
                Some(filter),
                &ReadView::default(),
                10,
            )
            .expect("c01 projected search");
        unsafe { std::env::remove_var("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST") };
        body_pass("after c01, 1");
        body_pass("after c01, 2");
        body_pass("after c01, 3");
        opened.engine.close().expect("close");
    }
}
