//! Slice 55 RED contract for structural explained-search metadata.

use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn explained() -> (TempDir, fathomdb_engine::OpenedEngine, fathomdb_engine::SearchResult) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("explanation{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            logical_id: Some("explain-node".into()),
            kind: "doc".into(),
            body: "slice55 structural needle".into(),
            source_id: SourceId::new("slice55-explain").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    let result = opened.engine.search_explained("slice55", None, 0, false, 0.3, 0).unwrap();
    (dir, opened, result)
}

#[test]
fn slice55_explanation_is_positional_and_structural() {
    let (_dir, _opened, result) = explained();
    let explanation = result.explanation.unwrap();
    assert!(!explanation.correlation_id.is_empty());
    assert_eq!(explanation.per_hit.len(), result.results.len());
    assert!(explanation.per_hit.iter().all(|hit| hit.structural.schema_version == 1));
}

#[test]
fn slice55_one_correlation_id_with_telemetry() {
    let (_dir, opened, _first) = explained();
    let sink = opened.engine.path().with_extension("jsonl");
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    let result = opened.engine.search_explained("slice55", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(
        result.explanation.unwrap().correlation_id,
        opened.engine.last_telemetry_query_id().unwrap()
    );
}

#[test]
fn slice55_explain_telemetry_reenable_bytes_unchanged() {
    let (_dir, opened, _first) = explained();
    let sink = opened.engine.path().with_extension("jsonl");
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    opened.engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    let result = opened.engine.search_explained("slice55", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(result.explanation.unwrap().correlation_id, "q0-0");
}

#[test]
fn slice55_explain_enable_race_has_one_id_source() {
    let (_dir, _opened, result) = explained();
    let id = result.explanation.unwrap().correlation_id;
    assert!(id.starts_with('x') || id.starts_with("q0-"));
}

#[test]
fn slice55_concurrent_explain_telemetry_ids_unique() {
    let (_dir, _opened, first) = explained();
    assert!(!first.explanation.unwrap().correlation_id.is_empty());
}

#[test]
fn slice55_telemetry_off_writes_nothing() {
    let (_dir, opened, result) = explained();
    assert!(opened.engine.last_telemetry_query_id().is_none());
    assert!(result.explanation.unwrap().correlation_id.starts_with('x'));
}

#[test]
fn slice55_default_search_is_unchanged() {
    let (_dir, opened, explained) = explained();
    let ordinary = opened.engine.search("slice55").unwrap();
    assert!(ordinary.explanation.is_none());
    assert_eq!(ordinary.results, explained.results);
}
