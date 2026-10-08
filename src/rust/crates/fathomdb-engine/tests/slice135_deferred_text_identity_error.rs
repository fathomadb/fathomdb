//! Deferred text-hit identity hydration must report malformed canonical data.

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineError, InitialState, PreparedWrite, SoftFallbackBranch, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice135-deferred-identity", "rev-a", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0])
    }
}

#[test]
fn malformed_deferred_source_must_fail_instead_of_erasing_hit_provenance() {
    let directory = TempDir::new().unwrap();
    let path = directory.path().join(format!("deferred-identity-error{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "needle source passage".into(),
            source_id: SourceId::new("slice135:deferred-identity").unwrap(),
            logical_id: Some("deferred-target".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let control = opened.engine.search("needle").unwrap();
    assert_eq!(control.results.len(), 1);
    assert_eq!(control.results[0].branch, SoftFallbackBranch::Text);
    assert_eq!(control.results[0].source_id.as_deref(), Some("slice135:deferred-identity"),);
    opened.engine.close().unwrap();
    drop(opened);

    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE canonical_nodes SET source_id = x'FF' WHERE logical_id = 'deferred-target'",
                [],
            )
            .unwrap(),
        1,
    );
    let stored_type: String = connection
        .query_row(
            "SELECT typeof(source_id) FROM canonical_nodes WHERE logical_id = 'deferred-target'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored_type, "blob");
    drop(connection);

    let reopened = Engine::open_with_embedder_for_test(&path, Arc::new(FixedEmbedder)).unwrap();
    let observed = reopened.engine.search("needle");
    assert!(
        matches!(observed, Err(EngineError::Storage)),
        "malformed provenance must not yield a successful hit: {observed:?}",
    );
}
