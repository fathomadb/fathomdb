//! Slice 40 (0.8.28) Defect C — `search_projected_text` must reach its rows
//! through the FTS5 rank stream instead of sorting every match.
//!
//! The documented order ("property-FTS bm25 ascending then write cursor
//! ascending") is the oracle. It is computed by running that ORDER BY directly
//! against the database file after the engine closes. The corpus has no
//! validity windows, supersession or dependencies, so the engine's node
//! predicate admits every row and the oracle needs no copy of it; the oracle
//! asserts it sees every written row to prove that.
//!
//! Route and query-plan evidence reuse the body path's test-hooks witnesses
//! (`FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST`,
//! `FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST`) and its statement hooks
//! (`FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST`, `FATHOMDB_FTS_FAIL_STREAM_FOR_TEST`).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ProjectionFts, ProjectionRole, ProjectionSpec, ReadView,
    SearchFilter, SearchResult, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use serde_json::Value;
use tempfile::TempDir;

// A failing test poisons the lock; later tests recover it so one RED test
// does not mask the others.
static ENV_LOCK: Mutex<()> = Mutex::new(());

const ENV_VARS: [&str; 4] = [
    "FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST",
    "FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST",
    "FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST",
    "FATHOMDB_FTS_FAIL_STREAM_FOR_TEST",
];

/// Restores every hook variable this file touches when dropped.
struct EnvGuard(Vec<(&'static str, Option<String>)>);

impl EnvGuard {
    fn clear() -> Self {
        let prior = ENV_VARS
            .iter()
            .map(|key| {
                let prior = std::env::var(key).ok();
                unsafe { std::env::remove_var(key) };
                (*key, prior)
            })
            .collect();
        Self(prior)
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn unset(&self, key: &str) {
        unsafe { std::env::remove_var(key) };
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in &self.0 {
            match value {
                Some(value) => unsafe { std::env::set_var(key, value) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

const QUERY: &str = "s40proj";
const FIELD: &str = "title";

fn db_path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn spec(name: &str, role: ProjectionRole, fts: bool) -> ProjectionSpec {
    ProjectionSpec {
        name: name.to_string(),
        roles: BTreeSet::from([role]),
        fts: fts.then_some(ProjectionFts { tokenizer: None }),
        vector: None,
        source: None,
    }
}

fn node(logical_id: &str, title: &str, cohort: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: serde_json::json!({ "title": title, "cohort": cohort }).to_string(),
        source_id: SourceId::new("test:slice40-projected").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

const TITLE_A: &str = "s40proj s40proj s40proj alpha";
const TITLE_T: &str = "s40proj beta gamma delta";
const TITLE_C: &str = "s40proj one two three four five six seven";

/// Twelve matching rows in three bm25 tie groups: A (2 rows, best), T (6 rows),
/// C (4 rows, worst). Identical title text inside a group gives bit-identical
/// bm25, so a limit can fall inside a tie group. Writes interleave the groups,
/// so write-cursor order differs from bm25 order. Cohorts alternate c1/c2.
/// Two non-matching rows prove the MATCH still filters.
fn corpus() -> Vec<PreparedWrite> {
    let plan = [
        ("C0", TITLE_C),
        ("T0", TITLE_T),
        ("A0", TITLE_A),
        ("C1", TITLE_C),
        ("T1", TITLE_T),
        ("T2", TITLE_T),
        ("N0", "unrelated title words"),
        ("C2", TITLE_C),
        ("T3", TITLE_T),
        ("A1", TITLE_A),
        ("T4", TITLE_T),
        ("C3", TITLE_C),
        ("N1", "another unrelated title"),
        ("T5", TITLE_T),
    ];
    plan.iter()
        .enumerate()
        .map(|(i, (id, title))| node(id, title, if i % 2 == 0 { "c1" } else { "c2" }))
        .collect()
}

const MATCHING_ROWS: usize = 12;

fn build(dir: &TempDir, name: &str) {
    let opened = Engine::open(db_path(dir, name)).expect("open");
    opened
        .engine
        .configure_projections(
            &[
                spec(FIELD, ProjectionRole::Searchable, true),
                spec("cohort", ProjectionRole::Filterable, false),
            ],
            &[],
        )
        .expect("configure projections");
    opened.engine.write(&corpus()).expect("write corpus");
    opened.engine.drain(10_000).expect("drain");
    opened.engine.close().expect("close after build");
}

/// `(write_cursor, score, body)` in the documented order, computed by SQLite
/// directly: `ORDER BY bm25(property_search_index) ASC, p.write_cursor ASC`.
/// The engine reports `score = -bm25`.
fn oracle(path: &Path) -> Vec<(u64, f64, String)> {
    let connection = rusqlite::Connection::open(path).expect("open oracle connection");
    let match_expression = fathomdb_query::compile_text_query(QUERY).match_expression;
    let rows = {
        let mut statement = connection
            .prepare(
                "SELECT p.write_cursor, bm25(property_search_index), n.body \
                 FROM property_search_index p \
                 JOIN canonical_nodes n ON n.write_cursor = p.write_cursor \
                 WHERE p.attr_name = ?1 AND property_search_index MATCH ?2 \
                 ORDER BY bm25(property_search_index) ASC, p.write_cursor ASC",
            )
            .expect("prepare oracle");
        statement
            .query_map(rusqlite::params![FIELD, match_expression], |row| {
                Ok((row.get::<_, i64>(0)? as u64, -row.get::<_, f64>(1)?, row.get::<_, String>(2)?))
            })
            .expect("query oracle")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect oracle")
    };
    connection.close().expect("close oracle connection");
    assert_eq!(rows.len(), MATCHING_ROWS, "the oracle sees every matching row");
    // The tie groups the limits rely on: A = 0..2, T = 2..8, C = 8..12.
    let scores: Vec<f64> = rows.iter().map(|(_, score, _)| *score).collect();
    for group in [0..2, 2..8, 8..12] {
        assert!(scores[group.clone()].windows(2).all(|w| w[0] == w[1]), "tie group {group:?}");
    }
    assert!(scores[1] > scores[2] && scores[7] > scores[8], "groups are strictly ordered");
    let cursors: Vec<u64> = rows.iter().map(|(cursor, _, _)| *cursor).collect();
    let mut sorted = cursors.clone();
    sorted.sort_unstable();
    assert_ne!(cursors, sorted, "bm25 order differs from write order");
    rows
}

fn ranked(result: &SearchResult) -> Vec<(u64, f64)> {
    result.results.iter().map(|hit| (hit.write_cursor, hit.score)).collect()
}

fn expected_prefix(oracle: &[(u64, f64, String)], limit: usize) -> Vec<(u64, f64)> {
    oracle.iter().take(limit).map(|(cursor, score, _)| (*cursor, *score)).collect()
}

fn cohort_filter(cohort: &str) -> SearchFilter {
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("cohort".to_string(), cohort.to_string())];
    filter
}

/// Runs one projected search per limit on a fresh open.
fn search_limits(
    dir: &TempDir,
    name: &str,
    filter: Option<SearchFilter>,
    limits: &[usize],
) -> Vec<SearchResult> {
    let opened = Engine::open(db_path(dir, name)).expect("reopen");
    let results = limits
        .iter()
        .map(|limit| {
            opened
                .engine
                .search_projected_text_with_limit(
                    QUERY,
                    FIELD,
                    filter.clone(),
                    &ReadView::default(),
                    *limit,
                )
                .expect("projected search")
        })
        .collect();
    opened.engine.close().expect("close after search");
    results
}

fn jsonl(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSONL row"))
        .collect()
}

fn routes(path: &Path) -> Vec<String> {
    jsonl(path).into_iter().map(|row| row["route"].as_str().expect("route").to_string()).collect()
}

/// AC40-27: for limits at, inside and past the tie groups, the ordered
/// `(write_cursor, score)` list equals the documented full-sort prefix.
#[test]
fn projected_search_matches_documented_order_across_tie_boundaries() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "order");
    // 1 and 5 fall inside the A and T tie groups, 3 inside T, 10 inside C;
    // 2 and 8 are strict boundaries; 12 and 100 hold every match.
    let limits = [1, 2, 3, 5, 8, 10, 12, 100];
    let results = search_limits(&dir, "order", None, &limits);
    let oracle = oracle(&db_path(&dir, "order"));
    for (limit, result) in limits.iter().zip(&results) {
        assert_eq!(
            ranked(result),
            expected_prefix(&oracle, *limit),
            "limit {limit}: projected order must equal the documented full sort"
        );
    }
}

/// AC40-28: an eligible projected search takes the FTS5 rank stream, and its
/// query plan has no temporary ORDER BY b-tree; the forced full sort takes the
/// old statement, whose plan does (so the plan witness is not vacuous).
#[test]
fn projected_search_uses_rank_stream_without_temp_order_by_btree() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "route");
    let route_witness = dir.path().join("routes.jsonl");
    let plan_witness = dir.path().join("plans.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    env.set("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", &plan_witness.display().to_string());

    // 2 is a strict boundary (A ends); 5 falls inside the T tie group.
    search_limits(&dir, "route", None, &[2, 5]);
    assert_eq!(
        routes(&route_witness),
        vec!["projected_rank_stream_strict_boundary", "projected_rank_stream_tie_completed"],
        "each eligible projected search records the rank-stream route it took"
    );
    let plans = jsonl(&plan_witness);
    assert_eq!(plans.len(), 2, "each streamed projected search records its query plan");
    assert!(
        plans.iter().all(|row| row["uses_temp_btree_for_order_by"] == false),
        "the streamed projected statement must not sort in a temporary b-tree: {plans:?}"
    );

    env.set("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1");
    search_limits(&dir, "route", None, &[2]);
    env.unset("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
    assert_eq!(
        routes(&route_witness).last().map(String::as_str),
        Some("projected_full_sort_forced"),
        "the forced full sort records its own route"
    );
    assert_eq!(
        jsonl(&plan_witness).last().map(|row| row["uses_temp_btree_for_order_by"].clone()),
        Some(Value::Bool(true)),
        "the full-sort statement does sort in a temporary b-tree"
    );
}

/// AC40-29: when the streamed statement fails, projected search falls back to
/// the full sort, returns the documented order, and records the fallback.
#[test]
fn projected_search_falls_back_to_full_sort_when_stream_fails() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "fallback");
    let route_witness = dir.path().join("routes.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    env.set("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST", "1");

    let results = search_limits(&dir, "fallback", None, &[5]);
    env.unset("FATHOMDB_FTS_FAIL_STREAM_FOR_TEST");

    let oracle = oracle(&db_path(&dir, "fallback"));
    assert_eq!(ranked(&results[0]), expected_prefix(&oracle, 5), "fallback keeps the order");
    assert_eq!(
        routes(&route_witness),
        vec!["projected_full_sort_fallback"],
        "a failed stream records exactly one route, the fallback"
    );
}

/// AC40-30: an attribute-filtered projected search returns only in-cohort
/// rows, in the documented order of the unfiltered list restricted to the
/// cohort, for a limit smaller than the cohort's match count.
#[test]
fn projected_search_attribute_filter_keeps_cohort_and_order() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "filter");
    let limits = [1, 3, 4];
    let filtered = search_limits(&dir, "filter", Some(cohort_filter("c1")), &limits);
    let unfiltered = search_limits(&dir, "filter", None, &[100]);
    let oracle = oracle(&db_path(&dir, "filter"));

    let in_cohort: Vec<(u64, f64)> = oracle
        .iter()
        .filter(|(_, _, body)| body.contains(r#""cohort":"c1""#))
        .map(|(cursor, score, _)| (*cursor, *score))
        .collect();
    assert!(in_cohort.len() > 4, "the cohort has more matches than the largest limit");
    for (limit, result) in limits.iter().zip(&filtered) {
        assert!(
            result.results.iter().all(|hit| hit.body.contains(r#""cohort":"c1""#)),
            "limit {limit}: only in-cohort rows"
        );
        assert_eq!(
            ranked(result),
            in_cohort.iter().take(*limit).copied().collect::<Vec<_>>(),
            "limit {limit}: filtered order is the documented order restricted to the cohort"
        );
    }
    assert_eq!(ranked(&unfiltered[0]), expected_prefix(&oracle, 100), "unfiltered unchanged");
}

/// AC40-31: with more matches than `limit` and a strict boundary, exactly
/// `limit` rows come back.
#[test]
fn projected_search_returns_exactly_limit_rows_at_strict_boundaries() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let _env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "limit");
    let results = search_limits(&dir, "limit", None, &[2, 8]);
    assert_eq!(results[0].results.len(), 2, "limit 2 (A group ends) returns 2 rows");
    assert_eq!(results[1].results.len(), 8, "limit 8 (T group ends) returns 8 rows");
    assert!(oracle(&db_path(&dir, "limit")).len() > 8, "more matches than either limit");
}
