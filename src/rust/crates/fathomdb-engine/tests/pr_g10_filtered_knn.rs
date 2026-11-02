//! Metadata-filtered KNN with native lifecycle and validity eligibility.
//!
//! `Option<SearchFilter>` prunes in the single phase-1 candidates statement;
//! Default eligibility precedes the candidate cap (Slice 70); `status` is a plain `TEXT` metadata column (aux hard-errors under a
//! KNN `WHERE`) and existing **Pack-1** shapes are refused without reshaping;
//! `status` ships an empty-string sentinel only (vec0 TEXT metadata is NOT NULL-able),
//! so filtering `status=Some("open")` prunes every row.
//! No mocking of the database.

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    vector_phase1_sql_for_test, Engine, PreparedWrite, SearchFilter, SEARCH_RERANK_LIMIT,
    TOP_K_BIT_CANDIDATES,
};
use fathomdb_schema::SQLITE_SUFFIX;
// Only used by `sentinel_refuses_simulated_pack1_without_mutation`, gated
// `#[cfg(debug_assertions)]` below for its `Engine::execute_for_test` hook.
#[cfg(debug_assertions)]
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("deterministic", "rev-a", 8)
    }
    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut v = vec![0.0_f32; 8];
        v[0] = 1.0;
        Ok(v)
    }
}

fn fixture(name: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    (dir, path)
}

#[test]
fn unfiltered_phase1_sql_applies_native_eligibility_before_cap() {
    let sql = vector_phase1_sql_for_test(None);
    assert_eq!(sql, vector_phase1_sql_for_test(Some(&SearchFilter::default())));
    let cap = sql.find(&format!("LIMIT {TOP_K_BIT_CANDIDATES}")).unwrap();
    for predicate in ["lifecycle=0", "valid_from<=?3", "valid_until>=?3"] {
        assert!(sql.find(predicate).unwrap() < cap, "{predicate} must precede KNN cap: {sql}");
    }
    assert!(sql.ends_with(&format!("LIMIT {SEARCH_RERANK_LIMIT}")));
}

#[test]
fn filtered_phase1_sql_appends_present_predicates_only() {
    // `SearchFilter` is `#[non_exhaustive]` (0.8.20 Slice 15e fix-2) — downstream
    // crates (this test crate included) cannot use a struct literal; build from
    // `default()` and assign the pub field.
    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".to_string());
    let sql = vector_phase1_sql_for_test(Some(&filter));
    assert!(sql.contains("AND kind=?3"), "present `kind` predicate appended at ?3:\n{sql}");
    assert!(!sql.contains("source_type=?"), "absent fields must not appear");
    assert!(!sql.contains("status=?"), "absent fields must not appear");
    // KNN form preserved: ORDER BY distance LIMIT top_k, no `k=`.
    assert!(sql.contains("ORDER BY distance"), "KNN order preserved");
    assert!(!sql.contains("k="), "no `k=` parameter form");
}

#[test]
fn filter_prunes_vector_candidates_by_kind() {
    let (_dir, path) = fixture("g10_kind_prune");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind doc");
    opened.engine.configure_vector_kind_for_test("note").expect("vector kind note");

    opened
        .engine
        .write(&[
            PreparedWrite::Node {
                kind: "doc".to_string(),
                body: "semantic alpha document".to_string(),
                source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
                logical_id: None,
                state: fathomdb_engine::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
            PreparedWrite::Node {
                kind: "note".to_string(),
                body: "semantic beta note".to_string(),
                source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
                logical_id: None,
                state: fathomdb_engine::InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            },
        ])
        .expect("write");
    opened.engine.drain(10_000).expect("drain");

    // Unfiltered: both kinds surface on the vector branch.
    let all = opened.engine.search_filtered("semantic", None).expect("search");
    let kinds: std::collections::BTreeSet<&str> =
        all.results.iter().map(|h| h.kind.as_str()).collect();
    assert!(kinds.contains("doc") && kinds.contains("note"), "unfiltered surfaces both kinds");

    // Filtered to kind=doc: only doc hits survive (pruned in phase 1).
    // `SearchFilter` is `#[non_exhaustive]` (0.8.20 Slice 15e fix-2) — downstream
    // crates (this test crate included) cannot use a struct literal; build from
    // `default()` and assign the pub field.
    let mut filter = SearchFilter::default();
    filter.kind = Some("doc".to_string());
    let only_doc = opened.engine.search_filtered("semantic", Some(filter)).expect("search");
    assert!(!only_doc.results.is_empty(), "doc kind still matches");
    assert!(
        only_doc.results.iter().all(|h| h.kind == "doc"),
        "kind filter prunes non-doc hits: {:?}",
        only_doc.results.iter().map(|h| h.kind.clone()).collect::<Vec<_>>()
    );

    opened.engine.close().unwrap();
}

#[test]
fn status_filter_prunes_all_because_population_is_null_only() {
    let (_dir, path) = fixture("g10_status_null");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
    opened.engine.configure_vector_kind_for_test("doc").expect("vector kind");
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_string(),
            body: "status probe document".to_string(),
            source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
            logical_id: None,
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("write");
    opened.engine.drain(10_000).expect("drain");

    // status ships NULL plumbing only: `status = 'open'` never matches.
    // `#[non_exhaustive]` (0.8.20 Slice 15e fix-2): build from `default()`.
    let mut filter = SearchFilter::default();
    filter.status = Some("open".to_string());
    let filtered = opened.engine.search_filtered("status", Some(filter)).expect("search");
    assert!(
        filtered.results.iter().all(|h| h.body != "status probe document"),
        "NULL status column never satisfies `status = ?` on the vector branch"
    );

    opened.engine.close().unwrap();
}

/// Regression that documents WHY `status` is plain `TEXT`, not a vec0 auxiliary
/// (`+status`) column: an auxiliary column cannot appear in a KNN `WHERE`
/// (sqlite-vec hard-errors), so a filtered KNN over an aux `status` fails. A
/// plain metadata `TEXT` column is constrainable under the `MATCH ... WHERE`.
// Uses `Engine::execute_for_test`, a debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn aux_status_column_hard_errors_under_knn_where() {
    let (_dir, path) = fixture("g10_aux_regression");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");

    // Build a probe table with `status` declared AUX (`+status`).
    opened
        .engine
        .execute_for_test(
            "CREATE VIRTUAL TABLE vec_aux_probe USING vec0(
                 embedding_bin bit[8],
                 +status TEXT
             );
             INSERT INTO vec_aux_probe(rowid, embedding_bin, status)
                 VALUES(1, vec_quantize_binary(vec_f32('[1,0,0,0,0,0,0,0]')), 'open');",
        )
        .expect("create aux probe");

    // A KNN MATCH that also constrains the aux column must fail.
    let knn_on_aux = opened.engine.execute_for_test(
        "SELECT rowid FROM vec_aux_probe
         WHERE embedding_bin MATCH vec_quantize_binary(vec_f32('[1,0,0,0,0,0,0,0]'))
           AND status = 'open'
         ORDER BY distance LIMIT 1",
    );
    assert!(knn_on_aux.is_err(), "aux `status` must hard-error under a KNN WHERE");

    opened.engine.close().unwrap();
}

/// 3-way shape-sentinel: an existing **Pack-1** `vector_default` (has
/// `embedding_bin`, lacks `status`) must be staged + recreated + back-filled so
/// `status` lands on reopen — the no-op-on-`embedding_bin` bug is fixed.
// Uses `Engine::execute_for_test`, a debug-build-only hook.
#[cfg(debug_assertions)]
#[test]
fn sentinel_refuses_simulated_pack1_without_mutation() {
    let (_dir, path) = fixture("g10_pack1_sentinel");

    // 1. Fresh open lands the Pack-2 shape (with status).
    {
        let opened =
            Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).expect("open");
        // 2. Rewind to a simulated Pack-1 shape on the engine connection (which
        //    has sqlite-vec loaded): drop + recreate WITHOUT status, seed a row.
        opened
            .engine
            .execute_for_test(
                "DROP TABLE vector_default;
                 CREATE VIRTUAL TABLE vector_default USING vec0(
                     embedding float[8],
                     embedding_bin bit[8],
                     source_type TEXT partition key,
                     kind TEXT,
                     created_at INTEGER
                 );
                 INSERT INTO vector_default(rowid, embedding, embedding_bin, source_type, kind, created_at)
                     VALUES(1, vec_f32('[1,0,0,0,0,0,0,0]'),
                            vec_quantize_binary(vec_f32('[1,0,0,0,0,0,0,0]')),
                            'article', 'doc', 0);",
            )
            .expect("simulate pack-1 shape");
        opened.engine.close().unwrap();
    }

    // Format 35 refuses an obsolete physical shape without changing its rows.
    let before = std::fs::read(&path).expect("read closed database");
    assert!(Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let conn = Connection::open(&path).expect("raw reopen");
    let table_sql: String = conn
        .query_row("SELECT sql FROM sqlite_master WHERE name='vector_default'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert!(!table_sql.contains("status"));
    let kind: String = conn
        .query_row("SELECT kind FROM vector_default WHERE rowid=1", [], |row| row.get(0))
        .unwrap();
    assert_eq!(kind, "doc");
}
