//! Slice 40 (0.8.28) Unit H2 — the hybrid vector branch hydrates a fixed pool.
//!
//! `search` hydrates up to `max(limit, VECTOR_FUSION_POOL)` surviving vector
//! candidates before fusion, so for one database state `search(limit = k)` is
//! the first k of `search(limit = k')` for every k < k' <= the pool, scores
//! bit-identical. The pool is read through `vector_fusion_pool_for_test()`;
//! the H2 verdict (`vector-pool-verdict.v1.json`, harness `f9ae5c27b`) set it
//! to 10, so the prefix property is new for limits below 10.
//!
//! The embedder places every document on a line by the number in its body
//! (`v<NNN>`): vector rank is that number's order, so each row's vector rank is
//! known exactly. The query embeds at the origin of the line.

use std::collections::BTreeSet;
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    vector_fusion_pool_for_test, Engine, InitialState, PreparedWrite, SearchFilter, SearchResult,
    SoftFallbackBranch, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const QUERY: &str = "s40pool";

#[derive(Clone, Debug)]
struct LineEmbedder;

impl Embedder for LineEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-line", "rev-a", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let position = text
            .strip_prefix('v')
            .and_then(|rest| rest.get(..3))
            .and_then(|digits| digits.parse::<f32>().ok())
            .unwrap_or(0.0);
        let mut vector = vec![0.0; 8];
        vector[0] = 1.0;
        vector[1] = position * 0.001;
        Ok(vector)
    }
}

fn write(kind: &str, logical_id: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body,
        source_id: SourceId::new("test:slice40-pool").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn pool() -> usize {
    vector_fusion_pool_for_test()
}

/// The vector rank of the row that is text rank 1: inside the pool of 10 but
/// past limit 5, so a pool cut at the limit scores it differently at limits 5
/// and 10.
const X_RANK: usize = 8;

fn x_rank() -> usize {
    X_RANK
}

/// `vector_rows` rows that match the vector only (`v001`…), one row `X` at
/// vector rank `x_rank()` that is text rank 1, and nine text rows that match
/// the query far down the vector line.
fn corpus(vector_rows: usize) -> Vec<PreparedWrite> {
    let mut writes = Vec::new();
    let mut position = 0;
    for i in 1..=vector_rows {
        position += 1;
        if position == x_rank() {
            writes.push(write("doc", "X", format!("v{position:03} {QUERY} {QUERY} {QUERY} alpha")));
            position += 1;
        }
        writes.push(write("doc", &format!("V{i}"), format!("v{position:03} vectorfiller {i}")));
    }
    for i in 0..9 {
        writes.push(write("doc", &format!("T{i}"), format!("v{:03} {QUERY} beta{i}", 900 + i)));
    }
    writes
}

fn open(dir: &TempDir, writes: &[PreparedWrite]) -> fathomdb_engine::OpenedEngine {
    let path = dir.path().join(format!("pool{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(LineEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind doc");
    opened.engine.configure_vector_kind_for_test("note").expect("vector kind note");
    opened.engine.write(writes).expect("write corpus");
    opened.engine.drain(30_000).expect("drain");
    opened
}

fn ranked(result: &SearchResult) -> Vec<(String, u64)> {
    result.results.iter().map(|hit| (hit.body.clone(), hit.score.to_bits())).collect()
}

fn vector_hits(engine: &Engine, limit: usize) -> u32 {
    engine
        .search_explained_with_limit(QUERY, None, 0, false, 0.0, 0, limit)
        .expect("explained search")
        .explanation
        .expect("explanation")
        .trace
        .vector_hits
}

/// AC40-79: for k < k' <= the pool, the ids and score bits of limit k are the
/// first k of limit k'. Today the vector list is cut at the limit, so the row
/// `X` scores text-only at the smaller limit and text plus vector at the larger.
#[test]
fn hybrid_results_are_prefix_stable_up_to_the_vector_pool() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, &corpus(40));
    let engine = &opened.engine;
    let pool = pool();
    assert_eq!(pool, 10, "the corpus places X for the verdict's pool of 10");
    for k_prime in [pool] {
        let long = engine.search_with_limit(QUERY, k_prime).expect("search k'");
        for k in [1, 5, 10].into_iter().filter(|k| *k < k_prime) {
            let short = engine.search_with_limit(QUERY, k).expect("search k");
            assert_eq!(
                ranked(&short),
                ranked(&long)[..k].to_vec(),
                "limit {k} must be the first {k} of limit {k_prime} (pool {pool})"
            );
        }
    }
    opened.engine.close().expect("close");
}

/// AC40-80: `vector_hits` reports the hydrated count:
/// `min(max(limit, pool), surviving)`.
#[test]
fn vector_branch_hydrates_the_pool_below_it_and_the_limit_above_it() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, &corpus(150));
    let engine = &opened.engine;
    let pool = pool();
    assert_eq!(
        pool, 10,
        "VECTOR_FUSION_POOL is the H2 verdict's pool (vector-pool-verdict.v1.json)"
    );
    let surviving = 150 + 1 + 9;
    assert_eq!(vector_hits(engine, 5) as usize, pool.min(surviving), "below the pool: the pool");
    assert_eq!(vector_hits(engine, pool) as usize, pool.min(surviving), "at the pool");
    if pool < 100 {
        assert_eq!(vector_hits(engine, pool + 1) as usize, pool + 1, "above the pool: the limit");
    }
    opened.engine.close().expect("close");
}

/// AC40-81: fused scores keep `Σ w / (30 + rank)`, text 3 and vector 1, for a
/// body found by both branches whose vector rank is beyond the limit.
#[test]
fn fused_score_formula_is_unchanged_with_the_pool() {
    let dir = TempDir::new().expect("tempdir");
    let opened = open(&dir, &corpus(40));
    let engine = &opened.engine;
    assert_eq!(pool(), 10, "the corpus places X for the verdict's pool of 10");
    let limit = 5;
    assert!(x_rank() > limit, "the row's vector rank is beyond the limit");
    let result = engine.search_with_limit(QUERY, limit).expect("search");
    let x = result.results.iter().find(|hit| hit.body.contains("alpha")).expect("X is returned");
    // Vector term first, then text, as fusion accumulates them.
    let mut expected = 1.0 / (30.0 + x_rank() as f64);
    expected += 3.0 / (30.0 + 1.0);
    assert_eq!(
        x.score.to_bits(),
        expected.to_bits(),
        "X fuses vector rank {} and text rank 1",
        x_rank()
    );
    assert_eq!(result.results[0].body, x.body, "X ranks first");
    opened.engine.close().expect("close");
}

/// AC40-82: the `vector_stage_only` seam still returns `limit` vector hits,
/// and the filtered vector branch keeps its cohort.
#[test]
fn vector_stage_only_and_filtered_vector_search_keep_their_contracts() {
    let dir = TempDir::new().expect("tempdir");
    let mut writes = corpus(40);
    for i in 0..30 {
        writes.push(write("note", &format!("N{i}"), format!("v{:03} notefiller {i}", 100 + i)));
    }
    let opened = open(&dir, &writes);
    let engine = &opened.engine;

    engine.set_vector_stage_only_for_test(true);
    for limit in [1, 5, 10] {
        let stage = engine.search_with_limit(QUERY, limit).expect("vector stage");
        assert_eq!(stage.results.len(), limit, "the seam returns limit vector hits");
        assert!(stage.results.iter().all(|hit| hit.branch == SoftFallbackBranch::Vector));
    }
    engine.set_vector_stage_only_for_test(false);

    let mut notes = SearchFilter::default();
    notes.kind = Some("note".to_string());
    for limit in [5, 10] {
        let filtered =
            engine.search_filtered_with_limit(QUERY, Some(notes.clone()), limit).expect("filtered");
        assert_eq!(filtered.results.len(), limit);
        assert!(filtered.results.iter().all(|hit| hit.kind == "note"), "only the filtered kind");
        let bodies: BTreeSet<&str> = filtered.results.iter().map(|hit| hit.body.as_str()).collect();
        assert_eq!(bodies.len(), limit, "distinct notes");
    }
    opened.engine.close().expect("close");
}
