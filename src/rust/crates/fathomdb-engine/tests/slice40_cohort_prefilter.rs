//! Slice 40 (0.8.28) Unit H3 — `search_projected_text` with a selective
//! attribute filter prefilters the FTS5 rank stream on the attribute cohort.
//!
//! The cohort route is the Defect C rank stream plus
//! `p.write_cursor IN (cohort)`, chosen when the smallest attribute pair's
//! bounded count is at most `T` = 256. The attribute `EXISTS` already implies
//! the conjunct, so results (ids, score bits, order) must equal the Defect C
//! stream's and the forced full sort's. The documented order is the oracle:
//! bm25 ascending, then write cursor ascending.
//!
//! Witnesses (`test-hooks`): the route witness, the query-plan witness with the
//! plan's detail rows, `FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST` (`on` ignores
//! `T`, `off` never prefilters) and `FATHOMDB_FTS_FAIL_COHORT_FOR_TEST`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ProjectionFts, ProjectionRole, ProjectionSpec, ReadView,
    SearchFilter, SearchResult, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use serde_json::Value;
use tempfile::TempDir;

// A failing test poisons the lock; later tests recover it so one RED test
// does not mask the others.
static ENV_LOCK: Mutex<()> = Mutex::new(());

const ENV_VARS: [&str; 6] = [
    "FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST",
    "FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST",
    "FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST",
    "FATHOMDB_FTS_FAIL_STREAM_FOR_TEST",
    "FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST",
    "FATHOMDB_FTS_FAIL_COHORT_FOR_TEST",
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

const QUERY: &str = "s40coh";
const FIELD: &str = "title";
/// `COHORT_PREFILTER_MAX_ROWS` (REQ40-H7), fixed before measurement.
const T: usize = 256;

const TITLE_A: &str = "s40coh s40coh s40coh alpha";
const TITLE_T: &str = "s40coh beta gamma delta";
const TITLE_C: &str = "s40coh one two three four five six seven";
const TITLE_N: &str = "unrelated title words";

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

fn node(logical_id: &str, kind: &str, title: &str, cohort: &str, tag: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.to_string(),
        body: serde_json::json!({ "title": title, "cohort": cohort, "tag": tag }).to_string(),
        source_id: SourceId::new("test:slice40-cohort").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn doc(logical_id: &str, title: &str, cohort: &str) -> PreparedWrite {
    node(logical_id, "doc", title, cohort, "common")
}

/// The Defect C shape: twelve matching rows in three bm25 tie groups, A (2,
/// best), T (6), C (4, worst), written interleaved so write order differs from
/// bm25 order, plus two non-matching rows. Cohorts alternate c1/c2, so each
/// cohort holds all three tie groups.
fn tie_corpus() -> Vec<PreparedWrite> {
    let plan = [
        ("C0", TITLE_C),
        ("T0", TITLE_T),
        ("A0", TITLE_A),
        ("C1", TITLE_C),
        ("T1", TITLE_T),
        ("T2", TITLE_T),
        ("N0", TITLE_N),
        ("C2", TITLE_C),
        ("T3", TITLE_T),
        ("A1", TITLE_A),
        ("T4", TITLE_T),
        ("C3", TITLE_C),
        ("N1", TITLE_N),
        ("T5", TITLE_T),
    ];
    plan.iter()
        .enumerate()
        .map(|(i, (id, title))| doc(id, title, if i % 2 == 0 { "c1" } else { "c2" }))
        .collect()
}

fn build(dir: &TempDir, name: &str, writes: &[PreparedWrite]) {
    let opened = Engine::open(db_path(dir, name)).expect("open");
    opened
        .engine
        .configure_projections(
            &[
                spec(FIELD, ProjectionRole::Searchable, true),
                spec("cohort", ProjectionRole::Filterable, false),
                spec("tag", ProjectionRole::Filterable, false),
            ],
            &[],
        )
        .expect("configure projections");
    for chunk in writes.chunks(500) {
        opened.engine.write(chunk).expect("write corpus");
    }
    opened.engine.drain(30_000).expect("drain");
    opened.engine.close().expect("close after build");
}

fn filter(pairs: &[(&str, &str)]) -> SearchFilter {
    let mut filter = SearchFilter::default();
    filter.attributes = pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    filter
}

/// One projected search per limit on a fresh open.
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

/// `(write_cursor, score bits)`: equality here is bit-identical scores.
fn ranked(result: &SearchResult) -> Vec<(u64, u64)> {
    result.results.iter().map(|hit| (hit.write_cursor, hit.score.to_bits())).collect()
}

/// The documented order, computed by SQLite directly over rows whose body
/// carries every `(name, value)` pair. The corpora have no validity windows or
/// dependencies; superseded versions are excluded by `superseded_at IS NULL`.
fn oracle(path: &Path, pairs: &[(&str, &str)], kind: Option<&str>) -> Vec<(u64, u64)> {
    let connection = rusqlite::Connection::open(path).expect("open oracle connection");
    let match_expression = fathomdb_query::compile_text_query(QUERY).match_expression;
    let rows = {
        let mut statement = connection
            .prepare(
                "SELECT p.write_cursor, bm25(property_search_index), n.body, n.kind \
                 FROM property_search_index p \
                 JOIN canonical_nodes n ON n.write_cursor = p.write_cursor \
                 WHERE p.attr_name = ?1 AND property_search_index MATCH ?2 \
                   AND n.superseded_at IS NULL \
                 ORDER BY bm25(property_search_index) ASC, p.write_cursor ASC",
            )
            .expect("prepare oracle");
        statement
            .query_map(rusqlite::params![FIELD, match_expression], |row| {
                Ok((
                    row.get::<_, i64>(0)? as u64,
                    -row.get::<_, f64>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })
            .expect("query oracle")
            .collect::<rusqlite::Result<Vec<_>>>()
            .expect("collect oracle")
    };
    connection.close().expect("close oracle connection");
    rows.into_iter()
        .filter(|(_, _, body, row_kind)| {
            let body: Value = serde_json::from_str(body).expect("body json");
            pairs.iter().all(|(k, v)| body[*k] == *v) && kind.is_none_or(|kind| kind == row_kind)
        })
        .map(|(cursor, score, _, _)| (cursor, score.to_bits()))
        .collect()
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

fn reset(path: &Path) {
    let _ = std::fs::remove_file(path);
}

/// True iff the plan's detail rows show the cohort list subquery read from the
/// attribute name-value index: a `LIST SUBQUERY` row followed by its search of
/// `canonical_attributes_name_value_idx`.
fn plan_has_cohort_list_subquery(plan: &Value) -> bool {
    let details: Vec<&str> = plan["details"]
        .as_array()
        .map(|rows| rows.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    details.windows(2).any(|pair| {
        pair[0].starts_with("LIST SUBQUERY")
            && pair[1].contains("canonical_attributes_name_value_idx")
    })
}

/// AC40-84: a cohort of at most `T` rows takes the cohort route; its plan
/// reads the cohort through a list subquery on the name-value index and does
/// not sort in a temporary b-tree.
#[test]
fn small_cohort_takes_the_cohort_prefilter_route() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "route", &tie_corpus());
    let route_witness = dir.path().join("routes.jsonl");
    let plan_witness = dir.path().join("plans.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    env.set("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", &plan_witness.display().to_string());

    // In c1 the order is A0 | T1 T3 T4 | C0: limit 1 ends on a strict
    // boundary, limit 2 falls inside the T tie group.
    let results = search_limits(&dir, "route", Some(filter(&[("cohort", "c1")])), &[1, 2]);
    assert_eq!(
        routes(&route_witness),
        vec!["projected_cohort_stream_strict_boundary", "projected_cohort_stream_tie_completed"],
        "a seven-row cohort takes the cohort-prefiltered rank stream"
    );
    let plans = jsonl(&plan_witness);
    assert_eq!(plans.len(), 2, "each cohort search records the cohort statement's plan: {plans:?}");
    for plan in &plans {
        assert_eq!(plan["uses_temp_btree_for_order_by"], false, "no temp b-tree: {plan:?}");
        assert!(
            plan_has_cohort_list_subquery(plan),
            "LIST SUBQUERY on the name-value index: {plan:?}"
        );
    }
    let oracle = oracle(&db_path(&dir, "route"), &[("cohort", "c1")], None);
    assert_eq!(ranked(&results[0]), oracle[..1].to_vec());
    assert_eq!(ranked(&results[1]), oracle[..2].to_vec(), "tie completion, then the limit");
}

/// AC40-85: the cohort route, the Defect C stream (prefilter forced off) and
/// the forced full sort return the same ids, score bits and order at every
/// limit, for both cohorts. Each run asserts its route, so a forcing variable
/// that is ignored fails here.
#[test]
fn cohort_prefilter_results_equal_rank_stream_and_full_sort() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "equal", &tie_corpus());
    let route_witness = dir.path().join("routes.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    let limits = [1, 2, 3, 5, 8, 10, 12, 100];
    for cohort in ["c1", "c2"] {
        let pairs = [("cohort", cohort)];
        let run = |mode: &[(&str, &str)], prefix: &str| {
            reset(&route_witness);
            for (key, value) in mode {
                env.set(key, value);
            }
            let results = search_limits(&dir, "equal", Some(filter(&pairs)), &limits);
            for (key, _) in mode {
                env.unset(key);
            }
            let seen = routes(&route_witness);
            assert_eq!(
                seen.len(),
                limits.len(),
                "{cohort} {prefix}: one route per search: {seen:?}"
            );
            assert!(
                seen.iter().all(|r| r.starts_with(prefix)),
                "{cohort}: {seen:?} not all {prefix}"
            );
            results.iter().map(ranked).collect::<Vec<_>>()
        };
        let cohort_route = run(&[], "projected_cohort_stream_");
        let forced_on =
            run(&[("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", "on")], "projected_cohort_stream_");
        let rank_stream =
            run(&[("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", "off")], "projected_rank_stream_");
        let full_sort =
            run(&[("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1")], "projected_full_sort_forced");
        let oracle = oracle(&db_path(&dir, "equal"), &pairs, None);
        assert!(oracle.len() >= 5, "{cohort}: matching rows in all three tie groups");
        for (i, limit) in limits.iter().enumerate() {
            assert_eq!(
                cohort_route[i], rank_stream[i],
                "{cohort} limit {limit}: cohort vs rank stream"
            );
            assert_eq!(
                forced_on[i], rank_stream[i],
                "{cohort} limit {limit}: forced on vs rank stream"
            );
            assert_eq!(
                full_sort[i], rank_stream[i],
                "{cohort} limit {limit}: full sort vs rank stream"
            );
            assert_eq!(
                rank_stream[i],
                oracle[..(*limit).min(oracle.len())].to_vec(),
                "{cohort} limit {limit}: the documented order restricted to the cohort"
            );
        }
    }
}

/// Matching rows in cohort `t256` (exactly `T`) and `t257` (`T + 1`), in two
/// tie groups, plus the tie corpus.
fn boundary_corpus() -> Vec<PreparedWrite> {
    let mut writes = tie_corpus();
    for (cohort, size) in [("t256", T), ("t257", T + 1)] {
        for i in 0..size {
            let title = if i % 3 == 0 { TITLE_A } else { TITLE_T };
            writes.push(doc(&format!("{cohort}-{i}"), title, cohort));
        }
    }
    writes
}

/// AC40-86: `T` rows prefilter; `T + 1` rows keep the Defect C stream.
#[test]
fn cohort_of_t_rows_prefilters_and_t_plus_one_keeps_the_rank_stream() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "bound", &boundary_corpus());
    let route_witness = dir.path().join("routes.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());

    let at_t = search_limits(&dir, "bound", Some(filter(&[("cohort", "t256")])), &[10]);
    let seen = routes(&route_witness);
    assert_eq!(seen.len(), 1);
    assert!(seen[0].starts_with("projected_cohort_stream_"), "{T} rows prefilter: {seen:?}");

    reset(&route_witness);
    let above_t = search_limits(&dir, "bound", Some(filter(&[("cohort", "t257")])), &[10]);
    let seen = routes(&route_witness);
    assert_eq!(seen.len(), 1);
    assert!(
        seen[0].starts_with("projected_rank_stream_"),
        "{} rows keep the rank stream: {seen:?}",
        T + 1
    );

    let path = db_path(&dir, "bound");
    for (result, cohort) in [(&at_t[0], "t256"), (&above_t[0], "t257")] {
        let oracle = oracle(&path, &[("cohort", cohort)], None);
        assert_eq!(
            ranked(result),
            oracle[..ranked(result).len()].to_vec(),
            "{cohort}: documented order"
        );
        assert!(ranked(result).len() >= 10, "{cohort}: at least the limit");
    }
}

/// AC40-87: a failing cohort statement falls back to the Defect C stream with
/// the same results and records the fallback first; forced off records no
/// cohort route.
#[test]
fn cohort_prefilter_failure_falls_back_to_the_rank_stream() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "fail", &tie_corpus());
    let route_witness = dir.path().join("routes.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    let c1 = Some(filter(&[("cohort", "c1")]));

    env.set("FATHOMDB_FTS_FAIL_COHORT_FOR_TEST", "1");
    let failed = search_limits(&dir, "fail", c1.clone(), &[2]);
    env.unset("FATHOMDB_FTS_FAIL_COHORT_FOR_TEST");
    assert_eq!(
        routes(&route_witness),
        vec!["projected_cohort_stream_fallback", "projected_rank_stream_tie_completed"],
        "the fallback is recorded, then the Defect C route"
    );
    let oracle = oracle(&db_path(&dir, "fail"), &[("cohort", "c1")], None);
    assert_eq!(ranked(&failed[0]), oracle[..2].to_vec(), "the fallback keeps the results");

    reset(&route_witness);
    env.set("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", "off");
    let off = search_limits(&dir, "fail", c1, &[2]);
    env.unset("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST");
    assert_eq!(
        routes(&route_witness),
        vec!["projected_rank_stream_tie_completed"],
        "no cohort route"
    );
    assert_eq!(ranked(&off[0]), ranked(&failed[0]));
}

/// A cohort above `T` (`big`, 300 rows) holding a small tag (`rare`, 3 rows).
fn multi_corpus() -> Vec<PreparedWrite> {
    let mut writes = tie_corpus();
    for i in 0..300 {
        let title = if i % 4 == 0 { TITLE_A } else { TITLE_T };
        let tag = if i % 100 == 7 { "rare" } else { "common" };
        writes.push(node(&format!("big-{i}"), "doc", title, "big", tag));
    }
    writes
}

/// AC40-88: with two pairs, the smaller cohort decides; in either filter order
/// the route is the cohort route and the results equal the forced-off results.
#[test]
fn multi_attribute_filter_prefilters_on_the_smallest_cohort() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    build(&dir, "multi", &multi_corpus());
    let route_witness = dir.path().join("routes.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    for pairs in [[("cohort", "big"), ("tag", "rare")], [("tag", "rare"), ("cohort", "big")]] {
        reset(&route_witness);
        let on = search_limits(&dir, "multi", Some(filter(&pairs)), &[2, 10]);
        let seen = routes(&route_witness);
        assert!(
            seen.len() == 2 && seen.iter().all(|r| r.starts_with("projected_cohort_stream_")),
            "{pairs:?}: the 3-row tag cohort prefilters although `big` is above T: {seen:?}"
        );
        env.set("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", "off");
        let off = search_limits(&dir, "multi", Some(filter(&pairs)), &[2, 10]);
        env.unset("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST");
        for (a, b) in on.iter().zip(&off) {
            assert_eq!(ranked(a), ranked(b), "{pairs:?}: same results as the forced-off stream");
        }
        let oracle = oracle(&db_path(&dir, "multi"), &pairs, None);
        assert_eq!(oracle.len(), 3);
        assert_eq!(ranked(&on[1]), oracle, "{pairs:?}: all three rows in the documented order");
    }
    reset(&route_witness);
    search_limits(&dir, "multi", Some(filter(&[("cohort", "big")])), &[5]);
    assert!(
        routes(&route_witness)[0].starts_with("projected_rank_stream_"),
        "`big` alone is above T"
    );
}

/// AC40-89: the node predicates still apply on the cohort route: a superseded
/// version in the cohort is not returned (its attribute row is purged at
/// supersede, `write_commit.rs`), and a `kind` filter still excludes other
/// kinds.
#[test]
fn cohort_prefilter_keeps_lifecycle_and_other_filter_fields() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let dir = TempDir::new().expect("tempdir");
    let mut writes = tie_corpus();
    writes.push(node("note-0", "note", TITLE_A, "c1", "common"));
    writes.push(node("note-1", "note", TITLE_T, "c1", "common"));
    build(&dir, "life", &writes);
    {
        // Supersede A0 (cohort c1) with a version in the same cohort and a
        // worse tie group. The attribute projection is maintained at rest, so
        // the old version is excluded either way; the cohort route must not
        // bring it back.
        let opened = Engine::open(db_path(&dir, "life")).expect("reopen");
        opened.engine.write(&[doc("A0", TITLE_C, "c1")]).expect("supersede A0");
        opened.engine.drain(10_000).expect("drain");
        opened.engine.close().expect("close");
    }
    let route_witness = dir.path().join("routes.jsonl");
    env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
    let path = db_path(&dir, "life");
    let superseded_a0: i64 = {
        let connection = rusqlite::Connection::open(&path).expect("open");
        connection
            .query_row(
                "SELECT count(*) FROM canonical_nodes \
                 WHERE logical_id = 'A0' AND superseded_at IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .expect("count superseded A0 versions")
    };
    assert_eq!(superseded_a0, 1, "the first A0 version is superseded");

    let mut by_kind = filter(&[("cohort", "c1")]);
    by_kind.kind = Some("doc".to_string());
    let limits = [1, 3, 100];
    for (flt, kind) in [(filter(&[("cohort", "c1")]), None), (by_kind, Some("doc"))] {
        reset(&route_witness);
        env.set("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", "on");
        let on = search_limits(&dir, "life", Some(flt.clone()), &limits);
        env.unset("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST");
        assert!(routes(&route_witness).iter().all(|r| r.starts_with("projected_cohort_stream_")));
        env.set("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1");
        let full = search_limits(&dir, "life", Some(flt), &limits);
        env.unset("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
        let oracle = oracle(&path, &[("cohort", "c1")], kind);
        for (i, limit) in limits.iter().enumerate() {
            assert_eq!(ranked(&on[i]), ranked(&full[i]), "kind {kind:?} limit {limit}");
            assert_eq!(
                ranked(&on[i])[..],
                oracle[..ranked(&on[i]).len()],
                "kind {kind:?} limit {limit}"
            );
        }
        assert!(on[2].results.iter().all(|hit| hit.body.contains(r#""cohort":"c1""#)));
        if kind.is_some() {
            assert!(on[2].results.iter().all(|hit| hit.kind == "doc"), "kind filter applies");
        } else {
            assert!(on[2].results.iter().any(|hit| hit.kind == "note"), "notes match without it");
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 32, ..ProptestConfig::default() })]

    /// AC40-90: random corpora (at most 60 rows), cohorts, tie groups and
    /// limits: the cohort route, forced on, equals the forced full sort.
    #[test]
    fn prop_cohort_prefilter_matches_the_full_sort(
        rows in prop::collection::vec((0usize..4, 0usize..3), 1..60),
        cohort in 0usize..3,
        limit in 1usize..20,
    ) {
        let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let env = EnvGuard::clear();
        let dir = TempDir::new().expect("tempdir");
        let titles = [TITLE_A, TITLE_T, TITLE_C, TITLE_N];
        let writes: Vec<PreparedWrite> = rows
            .iter()
            .enumerate()
            .map(|(i, (group, k))| doc(&format!("r{i}"), titles[*group], &format!("k{k}")))
            .collect();
        build(&dir, "prop", &writes);
        let route_witness = dir.path().join("routes.jsonl");
        env.set("FATHOMDB_FTS_ROUTE_WITNESS_FOR_TEST", &route_witness.display().to_string());
        let target = format!("k{cohort}");
        let pairs = [("cohort", target.as_str())];

        env.set("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", "on");
        let on = search_limits(&dir, "prop", Some(filter(&pairs)), &[limit]);
        env.unset("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST");
        let seen = routes(&route_witness);
        prop_assert_eq!(seen.len(), 1);
        prop_assert!(seen[0].starts_with("projected_cohort_stream_"), "route {:?}", seen);
        env.set("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST", "1");
        let full = search_limits(&dir, "prop", Some(filter(&pairs)), &[limit]);
        env.unset("FATHOMDB_FTS_FORCE_FULL_SORT_FOR_TEST");
        prop_assert_eq!(ranked(&on[0]), ranked(&full[0]));
    }
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let index = ((sorted.len() as f64 * fraction).ceil() as usize).max(1) - 1;
    sorted[index]
}

/// Report-only (review F12, ruling Q-H3): times the cohort route and the
/// Defect C route, each forced, at c01 and c10 on a store built from the
/// `tc5-10000` input pack (`FATHOMDB_TC5_INPUTS`). Prints p50 and p95 over the
/// pack's queries after one warm pass. `T` is not retuned from it.
#[test]
#[ignore = "report-only timing; needs FATHOMDB_TC5_INPUTS"]
fn cohort_prefilter_timing_report() {
    let _lock = ENV_LOCK.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let env = EnvGuard::clear();
    let inputs = PathBuf::from(std::env::var("FATHOMDB_TC5_INPUTS").expect("FATHOMDB_TC5_INPUTS"));
    let read = |name: &str| -> Vec<Value> {
        serde_json::from_str(&std::fs::read_to_string(inputs.join(name)).expect(name)).expect(name)
    };
    let corpus = read("corpus.json");
    let queries: Vec<String> = read("queries.json")
        .iter()
        .map(|q| q["text"].as_str().expect("text").to_string())
        .collect();
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "tc5");
    {
        let opened = Engine::open(&path).expect("open");
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
    let opened = Engine::open(&path).expect("reopen");
    for cohort in ["c01", "c10"] {
        let mut flt = SearchFilter::default();
        flt.attributes = vec![("bench_cohort".to_string(), cohort.to_string())];
        for mode in ["on", "off"] {
            env.set("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST", mode);
            let run = |timed: bool| -> Vec<f64> {
                queries
                    .iter()
                    .map(|q| {
                        let started = Instant::now();
                        opened
                            .engine
                            .search_projected_text_with_limit(
                                q,
                                "text",
                                Some(flt.clone()),
                                &ReadView::default(),
                                10,
                            )
                            .expect("search");
                        if timed {
                            started.elapsed().as_secs_f64() * 1000.0
                        } else {
                            0.0
                        }
                    })
                    .collect()
            };
            run(false);
            let mut timings = run(true);
            timings.sort_by(f64::total_cmp);
            println!(
                "cohort_prefilter_timing_report cohort={cohort} prefilter={mode} queries={} p50_ms={:.3} p95_ms={:.3}",
                timings.len(),
                percentile(&timings, 0.5),
                percentile(&timings, 0.95)
            );
        }
    }
    env.unset("FATHOMDB_FTS_COHORT_PREFILTER_FOR_TEST");
    opened.engine.close().expect("close");
}
