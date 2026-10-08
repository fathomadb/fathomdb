//! A damaged vector rerank row must not silently remove a committed candidate.

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
        EmbedderIdentity::new("slice135-vector-row", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

#[test]
fn malformed_vector_rerank_value_must_fail_instead_of_omitting_a_hit() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("vector-row-error{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "vector-only source passage".into(),
            source_id: SourceId::new("slice135:vector-row-error").unwrap(),
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
    // Corrupt only the full-precision rerank chunk; binary KNN remains queryable.
    assert_eq!(
        connection
            .execute("UPDATE vector_default_vector_chunks00 SET vectors = x'00'", [],)
            .unwrap(),
        1,
    );
    let physical: i64 = connection
        .query_row("SELECT length(vectors) FROM vector_default_vector_chunks00", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(physical, 1);
    drop(connection);

    let reopened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let observed = reopened.engine.search("query without lexical overlap");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "damaged vector row must not yield a successful incomplete search: {observed:?}",
    );
}
