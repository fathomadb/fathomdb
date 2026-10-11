//! A malformed canonical row must not silently remove a vector search hit.

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, EngineError, InitialState, PreparedWrite, SourceId};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice135-vector-hydration", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

#[test]
fn malformed_canonical_kind_must_fail_instead_of_omitting_vector_hit() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("vector-hydration-error{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "vector-only source passage".into(),
            source_id: SourceId::new("slice135:vector-hydration-error").unwrap(),
            logical_id: Some("vector-target".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let control = opened.engine.search("query without lexical overlap").unwrap();
    assert!(control.results.iter().any(|hit| hit.body == "vector-only source passage"));
    opened.engine.close().unwrap();
    drop(opened);

    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE canonical_nodes SET kind = x'FF' WHERE logical_id = 'vector-target'",
                [],
            )
            .unwrap(),
        1,
    );
    let stored_type: String = connection
        .query_row(
            "SELECT typeof(kind) FROM canonical_nodes WHERE logical_id = 'vector-target'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_type, "blob");
    drop(connection);

    let reopened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let observed = reopened.engine.search("query without lexical overlap");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "malformed canonical row must not yield incomplete search: {observed:?}",
    );
}
