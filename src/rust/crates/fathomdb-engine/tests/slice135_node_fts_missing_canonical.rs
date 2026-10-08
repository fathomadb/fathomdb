//! A damaged current-schema canonical node table must not expose orphaned FTS hits.
#![cfg(any(debug_assertions, feature = "test-hooks"))]

use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[test]
fn missing_current_schema_canonical_nodes_must_not_fallback_to_orphaned_fts() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("orphaned-fts{SQLITE_SUFFIX}"));
    let engine = Engine::open_without_embedder_for_test(&path).unwrap().engine;
    engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "slice135orphanedftsprobe".into(),
            source_id: SourceId::new("slice135:orphaned-fts").unwrap(),
            logical_id: Some("document".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    engine.drain(10_000).unwrap();
    let control = engine.search_text_only("slice135orphanedftsprobe").unwrap();
    assert_eq!(control.results.len(), 1);
    assert_eq!(control.results[0].id.value, "document");

    engine.execute_for_test("DROP TABLE canonical_nodes").unwrap();
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM search_index WHERE search_index MATCH 'slice135orphanedftsprobe'"
            )
            .unwrap(),
        [1]
    );
    let observed = engine.search_text_only("slice135orphanedftsprobe");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "missing canonical authority must report storage failure: {observed:?}"
    );
}
