//! 0.8.23 Slice 30 — embedding-readiness contract.
//!
//! These tests use a real SQLite engine. They pin the configuration-feedback
//! boundary: a body-bearing graph edge accepted without an embedder must become
//! an immediate, typed caller outcome, never a retry-driven scheduler timeout.

use fathomdb_engine::{
    EmbeddingOperation, EmbeddingReadinessState, Engine, EngineError, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn body_edge() -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "relates_to".to_string(),
        from: "memex-a".to_string(),
        to: "memex-b".to_string(),
        source_id: SourceId::new("test:slice30-readiness").expect("source id"),
        logical_id: Some("memex-edge-a-b".to_string()),
        body: Some("memex-a relates to memex-b".to_string()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

#[test]
fn body_bearing_edge_without_embedder_reports_blocked_and_drain_is_immediate_typed_feedback() {
    let dir = TempDir::new().expect("tempdir");
    let opened = Engine::open(db_path(&dir, "missing_embedder")).expect("open without embedder");
    opened.engine.write(&[body_edge()]).expect("accepted deferred edge write");

    let readiness = opened.engine.read_embedding_readiness().expect("readiness");
    assert_eq!(readiness.state, EmbeddingReadinessState::Blocked);
    assert!(!readiness.usable_embedder);
    assert_eq!(readiness.pending_count, 1);
    assert_eq!(readiness.affected_kinds, vec!["edge_fact"]);
    let blocked = readiness.blocked.expect("blocked payload");
    assert_eq!(blocked.operation, EmbeddingOperation::GraphEdgeBodyProjection);
    assert_eq!(blocked.code, "FDB_EMBEDDER_REQUIRED");
    assert_eq!(blocked.documentation_url, "https://fathomdb.dev/errors/FDB_EMBEDDER_REQUIRED");

    let started = std::time::Instant::now();
    let error = opened.engine.drain(30_000).expect_err("must not wait for retry backoff");
    assert!(started.elapsed() < std::time::Duration::from_secs(1), "feedback must be immediate");
    assert!(matches!(
        error,
        EngineError::EmbedderRequired(ref required)
            if required.operation == EmbeddingOperation::GraphEdgeBodyProjection
                && required.code == "FDB_EMBEDDER_REQUIRED"
    ));
}
