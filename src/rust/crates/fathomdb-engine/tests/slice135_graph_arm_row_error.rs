//! A graph traversal row error must not turn a reachable result into a
//! successful partial search response.

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, SoftFallbackBranch, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use std::sync::Arc;
use tempfile::TempDir;

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice135:graph-row-error").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn malformed_graph_edge_source_must_not_silently_drop_reachable_node() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("graph-row-error{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap();
    opened
        .engine
        .write(&[
            node("anchor", "slice135graphanchorprobe"),
            node("neighbor", "reachable neighbor record"),
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "anchor".into(),
                to: "neighbor".into(),
                source_id: SourceId::new("slice135:graph-row-error").unwrap(),
                logical_id: Some("edge".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let control =
        opened.engine.search_reranked("slice135graphanchorprobe", None, 0, true, 0.3, 0).unwrap();
    assert!(control
        .results
        .iter()
        .any(|hit| { hit.id.value == "neighbor" && hit.branch == SoftFallbackBranch::GraphArm }));
    opened.engine.close().unwrap();

    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .execute("UPDATE canonical_edges SET source_id=x'ff' WHERE logical_id='edge'", []),
        Ok(1)
    );
    let physical: String = connection
        .query_row(
            "SELECT typeof(source_id) FROM canonical_edges WHERE logical_id='edge'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(physical, "blob");
    drop(connection);

    let reopened =
        Engine::open_with_choice(&path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
            .unwrap()
            .engine;
    let observed = reopened.search_reranked("slice135graphanchorprobe", None, 0, true, 0.3, 0);
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "malformed traversed edge must report storage failure: {observed:?}"
    );
}
