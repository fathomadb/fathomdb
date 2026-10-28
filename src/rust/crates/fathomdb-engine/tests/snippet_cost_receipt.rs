//! EP-17 snippet cost receipt: point queries and search latency per cell
//! (`dev/experiments/agent-access-snippet-cost/README.md`).
//!
//! The sweep is the ignored test `snippet_cost_receipt`; run it in release.
//! The other tests check that the measurement is correct.

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    take_slice71_search_statement_trace_for_test, Engine, InitialState, OpenedEngine,
    PreparedWrite, ReadView, SnippetOptions, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

/// Serialises the tests: they read the process-wide statement trace.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

const TERMS: [&str; 5] = ["zanzibar", "quokka", "marmot", "pangolin", "axolotl"];
const RECORD_BYTES: [usize; 3] = [2 * 1024, 10 * 1024, 50 * 1024];
const KS: [usize; 3] = [1, 3, 5];
const RECORDS: usize = 40;
const LIMIT: usize = 20;
const WARMUP: usize = 10;
const SEARCHES: usize = 200;

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("snippet-cost", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

struct Store {
    _dir: TempDir,
    opened: OpenedEngine,
}

/// A record of at least `len` bytes of filler words with the first `k`
/// terms spread evenly, term `j` at the first word boundary at or after
/// byte `(j + 1) × len / (k + 1)`, so each term sits in its own window.
fn record(len: usize, k: usize, index: usize) -> String {
    let mut body = String::with_capacity(len + 16);
    for (j, term) in TERMS[..k].iter().enumerate() {
        while body.len() < (j + 1) * len / (k + 1) {
            body.push_str("lorem ");
        }
        body.push_str(term);
        body.push(' ');
    }
    while body.len() < len {
        body.push_str("ipsum ");
    }
    body.push_str(&format!("doc{index}"));
    body
}

fn store(len: usize, k: usize, records: usize) -> Store {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("cost{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    let writes: Vec<PreparedWrite> = (0..records)
        .map(|i| PreparedWrite::Node {
            kind: "doc".into(),
            body: record(len, k, i),
            source_id: SourceId::new("test:snippet-cost").expect("source id"),
            logical_id: Some(format!("n{i}")),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        })
        .collect();
    opened.engine.write(&writes).expect("write");
    opened.engine.drain(10_000).expect("drain");
    Store { _dir: dir, opened }
}

fn point_queries() -> usize {
    take_slice71_search_statement_trace_for_test()
        .iter()
        .filter(|event| event.as_str() == "snippet_highlight")
        .count()
}

/// One measured search: its hit count, its point queries and its latency in
/// microseconds. The trace is drained before the clock starts and read after
/// it stops.
fn measure(engine: &Engine, query: &str, snippet: Option<&SnippetOptions>) -> (usize, usize, u64) {
    take_slice71_search_statement_trace_for_test();
    let started = Instant::now();
    let result = engine
        .search_text_only_view_with_snippets(query, &ReadView::default(), LIMIT, snippet)
        .expect("search");
    let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    (result.results.len(), point_queries(), micros)
}

/// Nearest-rank percentile `p` (0–100) of `values`.
fn percentile(values: &[u64], p: usize) -> u64 {
    assert!(!values.is_empty() && p <= 100);
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    let rank = (p * sorted.len()).div_ceil(100).max(1);
    sorted[rank - 1]
}

struct Cell {
    record_bytes: usize,
    k: usize,
    snippets: bool,
    hits: usize,
    point_queries: Vec<usize>,
    micros: Vec<u64>,
}

impl Cell {
    fn json(&self) -> serde_json::Value {
        serde_json::json!({
            "record_bytes": self.record_bytes,
            "k": self.k,
            "snippets": if self.snippets { "on" } else { "off" },
            "hits": self.hits,
            "point_queries_min": self.point_queries.iter().min(),
            "point_queries_max": self.point_queries.iter().max(),
            "searches": self.micros.len(),
            "p50_us": percentile(&self.micros, 50),
            "p95_us": percentile(&self.micros, 95),
        })
    }
}

fn run_cell(s: &Store, len: usize, k: usize, snippets: bool, searches: usize) -> Cell {
    let query = TERMS[..k].join(" ");
    let options = SnippetOptions::default();
    let snippet = snippets.then_some(&options);
    for _ in 0..WARMUP {
        measure(&s.opened.engine, &query, snippet);
    }
    let mut cell = Cell {
        record_bytes: len,
        k,
        snippets,
        hits: 0,
        point_queries: Vec::with_capacity(searches),
        micros: Vec::with_capacity(searches),
    };
    for _ in 0..searches {
        let (hits, queries, micros) = measure(&s.opened.engine, &query, snippet);
        cell.hits = hits;
        cell.point_queries.push(queries);
        cell.micros.push(micros);
    }
    cell
}

#[test]
fn percentile_is_nearest_rank() {
    let values: Vec<u64> = (1..=200).rev().collect();
    assert_eq!(percentile(&values, 50), 100);
    assert_eq!(percentile(&values, 95), 190);
    assert_eq!(percentile(&[7, 3, 5], 50), 5);
    assert_eq!(percentile(&[7, 3, 5], 95), 7);
    assert_eq!(percentile(&[4], 0), 4);
}

#[test]
fn records_hold_each_term_once_at_its_place() {
    for len in RECORD_BYTES {
        for k in KS {
            let body = record(len, k, 3);
            assert!(body.len() >= len && body.ends_with(" doc3"));
            for (j, term) in TERMS.iter().enumerate() {
                let found: Vec<usize> = body.match_indices(term).map(|(at, _)| at).collect();
                if j < k {
                    let at = (j + 1) * len / (k + 1);
                    assert_eq!(found.len(), 1, "{term} in {len}/{k}");
                    assert!(found[0] >= at && found[0] < at + 6, "{term} at {found:?}");
                } else {
                    assert!(found.is_empty(), "{term} in {len}/{k}");
                }
            }
        }
    }
}

// The counts a cell records are the search's own: none with snippets off,
// and one combined span query per long hit for a one-term query, which
// rule 3 never extends.
#[test]
fn a_cell_counts_the_point_queries_of_its_own_searches() {
    let _guard = lock();
    let s = store(2 * 1024, 1, 5);
    let off = run_cell(&s, 2 * 1024, 1, false, 3);
    assert_eq!((off.hits, off.point_queries, off.micros.len()), (5, vec![0; 3], 3));
    let on = run_cell(&s, 2 * 1024, 1, true, 3);
    assert_eq!((on.hits, on.point_queries, on.micros.len()), (5, vec![5; 3], 3));
}

#[test]
#[ignore = "measurement sweep; run in release, see the experiment README"]
fn snippet_cost_receipt() {
    let _guard = lock();
    let label =
        std::env::var("FATHOMDB_SNIPPET_COST_LABEL").unwrap_or_else(|_| "unlabelled".into());
    let mut cells = Vec::new();
    for len in RECORD_BYTES {
        for k in KS {
            let s = store(len, k, RECORDS);
            for snippets in [false, true] {
                let cell = run_cell(&s, len, k, snippets, SEARCHES);
                assert_eq!(cell.hits, LIMIT, "{len} bytes, k = {k}");
                eprintln!("{}", cell.json());
                cells.push(cell.json());
            }
        }
    }
    let summary = serde_json::json!({
        "label": label,
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "records": RECORDS,
        "limit": LIMIT,
        "warmup": WARMUP,
        "snippet_options": "SnippetOptions::default()",
        "cells": cells,
    });
    let text = serde_json::to_string_pretty(&summary).expect("json");
    match std::env::var("FATHOMDB_SNIPPET_COST_OUT") {
        Ok(path) => std::fs::write(&path, format!("{text}\n")).expect("write summary"),
        Err(_) => println!("{text}"),
    }
}
