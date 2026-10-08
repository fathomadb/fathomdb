//! A damaged current-schema edge index must not produce a successful partial search.
#![cfg(any(debug_assertions, feature = "test-hooks"))]

use std::sync::Arc;

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("endpoint {id}"),
        source_id: SourceId::new("slice135:edge-index-fault").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn missing_current_schema_edge_index_must_report_storage_error() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("edge-index-fault{SQLITE_SUFFIX}"));
    let engine =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap()
            .engine;
    engine
        .write(&[
            node("from"),
            node("to"),
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "from".into(),
                to: "to".into(),
                source_id: SourceId::new("slice135:edge-index-fault").unwrap(),
                logical_id: Some("edge".into()),
                body: Some("slice135edgeindexsentinel".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    engine.drain(10_000).unwrap();
    let control = engine.search_text_only("slice135edgeindexsentinel").unwrap();
    assert_eq!(control.results.len(), 1);
    assert_eq!(control.results[0].body, "slice135edgeindexsentinel");

    engine.execute_for_test("DROP TABLE search_index_edges").unwrap();
    let result = engine.search_text_only("slice135edgeindexsentinel");
    assert!(
        matches!(result, Err(EngineError::Storage)),
        "missing current-schema edge index must fail closed: {result:?}"
    );
}
