//! A missing fallback-probe table must not produce a successful hybrid search.

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
        EmbedderIdentity::new("slice135-soft-fallback", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

#[test]
fn missing_vector_kind_table_must_fail_instead_of_suppressing_probe_error() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("soft-fallback-error{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "needle source passage".into(),
            source_id: SourceId::new("slice135:soft-fallback").unwrap(),
            logical_id: Some("fallback-target".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    assert_eq!(opened.engine.embed_text("needle").unwrap().len(), 8);
    let control = opened.engine.search("needle").unwrap();
    assert_eq!(control.results.len(), 1);

    let connection = Connection::open(&path).unwrap();
    connection.execute("DROP TABLE _fathomdb_vector_kinds", []).unwrap();
    let remaining: i64 = connection
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE name = '_fathomdb_vector_kinds'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(remaining, 0);
    drop(connection);

    assert_eq!(opened.engine.embed_text("needle").unwrap().len(), 8);
    let observed = opened.engine.search("needle");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "missing vector-kind metadata must not yield a successful search: {observed:?}",
    );
}
