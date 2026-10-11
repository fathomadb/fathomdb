//! A malformed stored importance value must not look absent in an explanation.
#![cfg(feature = "test-hooks")]

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use std::sync::Arc;
use tempfile::TempDir;

#[test]
fn malformed_importance_must_not_produce_neutral_explanation() {
    let dir = TempDir::new().expect("temporary database");
    let path = dir.path().join(format!("importance-explanation{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .expect("open");
    let receipt = opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "slice135importancequery node".into(),
            source_id: SourceId::new("slice135:importance-explanation").expect("source ID"),
            logical_id: Some("importance-node".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("write node");
    opened.engine.drain(10_000).expect("drain");
    opened.engine.write_node_importance(receipt.cursor, 0.5).expect("set valid importance");
    let control = opened
        .engine
        .search_explained("slice135importancequery", None, 0, false, 0.3, 0)
        .expect("valid explained search");
    let control_hit = control
        .explanation
        .expect("explanation")
        .per_hit
        .into_iter()
        .find(|hit| hit.id == receipt.cursor)
        .expect("node explanation");
    assert_eq!(control_hit.importance, Some(0.5));

    opened
        .engine
        .execute_for_test(&format!(
            "UPDATE canonical_nodes SET importance=x'ff' WHERE write_cursor={}",
            receipt.cursor
        ))
        .expect("inject malformed importance value");
    let result = opened.engine.search_explained("slice135importancequery", None, 0, false, 0.3, 0);
    assert!(
        matches!(result, Err(EngineError::Storage)),
        "a failed importance decode must not produce a neutral explanation"
    );
    opened.engine.close().expect("close");
}
