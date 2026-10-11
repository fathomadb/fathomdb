//! A malformed edge FTS row must report a storage error rather than a
//! successful, incomplete search result.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use std::sync::Arc;
use tempfile::TempDir;

fn node(id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("node {id}"),
        source_id: SourceId::new("slice135:logic-probe").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn malformed_edge_fts_kind_must_not_silently_remove_a_committed_hit() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("logic-probe{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap();
    let receipt = opened
        .engine
        .write(&[
            node("left"),
            node("right"),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new("slice135:logic-probe").unwrap(),
                logical_id: Some("edge".into()),
                body: Some("slice135uniqueedgefact".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let control = opened.engine.search_text_only("slice135uniqueedgefact").unwrap();
    assert!(control.projection_cursor >= receipt.cursor);
    assert!(control.results.iter().any(|hit| hit.body == "slice135uniqueedgefact"));

    // FTS5 UNINDEXED columns accept SQLite dynamic types. A BLOB containing
    // invalid UTF-8 cannot decode as the String expected by the edge-hit mapper.
    opened.engine.close().unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE search_index_edges SET kind=x'ff' WHERE body='slice135uniqueedgefact'",
                [],
            )
            .unwrap(),
        1
    );
    let physical: (String, i64) = connection
        .query_row(
            "SELECT typeof(kind), length(kind) FROM search_index_edges WHERE search_index_edges MATCH 'slice135uniqueedgefact'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(physical, ("blob".to_owned(), 1), "the FTS row must still match the query");
    drop(connection);
    let reopened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap()
            .engine;
    let observed = reopened.search_text_only("slice135uniqueedgefact");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "a malformed FTS row must not produce a successful incomplete search"
    );
}
