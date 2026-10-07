//! A malformed node FTS row must report a storage error rather than a
//! successful, incomplete search result.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use std::sync::Arc;
use tempfile::TempDir;

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice135:node-fts-error").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn malformed_node_fts_kind_must_not_silently_remove_a_committed_hit() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("node-fts-error{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap();
    let receipt = opened
        .engine
        .write(&[
            node("target", "slice135uniquenodefact"),
            node("other", "other node"),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "target".into(),
                to: "other".into(),
                source_id: SourceId::new("slice135:node-fts-error").unwrap(),
                logical_id: Some("edge".into()),
                body: Some("other edge body".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let control = opened.engine.search_text_only("slice135uniquenodefact").unwrap();
    assert!(control.projection_cursor >= receipt.cursor);
    assert!(control.results.iter().any(|hit| hit.body == "slice135uniquenodefact"));

    opened.engine.close().unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    let edge_fts_count: i64 = connection
        .query_row("SELECT count(*) FROM search_index_edges", [], |row| row.get(0))
        .unwrap();
    assert!(edge_fts_count > 0, "edge FTS must disable the node rank stream");
    assert_eq!(
        connection
            .execute("UPDATE search_index SET kind=x'ff' WHERE body='slice135uniquenodefact'", [],)
            .unwrap(),
        1
    );
    let physical: (String, i64) = connection
        .query_row(
            "SELECT typeof(kind), length(kind) FROM search_index WHERE search_index MATCH 'slice135uniquenodefact'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(physical, ("blob".to_owned(), 1));
    drop(connection);

    let reopened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap()
            .engine;
    let observed = reopened.search_text_only("slice135uniquenodefact");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "a malformed FTS row must not produce a successful incomplete search: {observed:?}"
    );
}
