//! 0.8.23 Slice 30 — embedding-readiness contract.
//!
//! These tests use a real SQLite engine. They pin the configuration-feedback
//! boundary: a body-bearing graph edge accepted without an embedder must become
//! an immediate, typed caller outcome, never a retry-driven scheduler timeout.

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    EmbeddingOperation, EmbeddingReadinessState, Engine, EngineError, PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use std::path::Path;
use std::sync::Arc;
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

#[derive(Clone, Debug)]
struct ControlledEmbedder {
    identity: EmbedderIdentity,
    divergent: bool,
}

impl ControlledEmbedder {
    fn faithful(identity: EmbedderIdentity) -> Self {
        Self { identity, divergent: false }
    }

    fn divergent(identity: EmbedderIdentity) -> Self {
        Self { identity, divergent: true }
    }
}

impl Embedder for ControlledEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.identity.clone()
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        let mut vector = vec![0.0_f32; self.identity.dimension as usize];
        vector[if self.divergent { 1 } else { 0 }] = 1.0;
        Ok(vector)
    }
}

fn stored_default_identity(path: &Path) -> EmbedderIdentity {
    let connection = rusqlite::Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )
    .expect("open read-only");
    connection
        .query_row(
            "SELECT name, revision, dimension FROM _fathomdb_embedder_profiles WHERE profile = 'default'",
            [],
            |row| {
                Ok(EmbedderIdentity::new(
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, u32>(2)?,
                ))
            },
        )
        .expect("stored default identity")
}

fn make_probe_verdict_stale(path: &Path) {
    let connection = rusqlite::Connection::open(path).expect("open mutation connection");
    connection
        .execute(
            "UPDATE _fathomdb_open_state \
             SET value = 'deliberately-stale-slice30-refusal' \
             WHERE key = 'vector_equivalence_verified_fingerprint'",
            [],
        )
        .expect("make equivalence verdict stale");
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

#[test]
fn equivalence_refused_embedder_defers_work_without_relabelling_it_as_missing_configuration() {
    let dir = TempDir::new().expect("tempdir");
    let path = db_path(&dir, "refused_embedder");
    Engine::open(&path).expect("create profile").engine.close().expect("close profile setup");
    let identity = stored_default_identity(&path);

    {
        let opened = Engine::open_with_embedder_for_test(
            &path,
            Arc::new(ControlledEmbedder::faithful(identity.clone())),
        )
        .expect("open faithful setup runtime");
        opened
            .engine
            .configure_vector_kind_for_test("doc")
            .expect("register vector arm for equivalence preflight");
        opened.engine.close().expect("close setup runtime");
    }
    {
        let opened = Engine::open_with_embedder_for_test(
            &path,
            Arc::new(ControlledEmbedder::faithful(identity.clone())),
        )
        .expect("persist accepted equivalence baseline");
        assert!(!opened.report.dense_disabled, "fixture: faithful runtime is accepted");
        opened.engine.close().expect("close baseline runtime");
    }
    make_probe_verdict_stale(&path);
    {
        let opened = Engine::open(&path).expect("open without runtime to create pending work");
        opened.engine.write(&[body_edge()]).expect("write deferred edge");
        opened.engine.close().expect("close pending-work session");
    }

    let opened = Engine::open_with_embedder_for_test(
        &path,
        Arc::new(ControlledEmbedder::divergent(identity)),
    )
    .expect("equivalence refusal leaves the engine serviceable");
    assert!(opened.report.dense_disabled, "fixture: runtime was refused by equivalence");
    let readiness = opened.engine.read_embedding_readiness().expect("readiness");
    assert_eq!(readiness.state, EmbeddingReadinessState::Deferred);
    assert!(!readiness.usable_embedder);
    assert_eq!(readiness.pending_count, 1);
    assert!(readiness.blocked.is_none(), "a refused attached runtime is not missing configuration");
    let error =
        opened.engine.drain(0).expect_err("deferred work cannot drain while runtime is refused");
    assert!(
        !matches!(error, EngineError::EmbedderRequired(_)),
        "only an absent configured runtime is FDB_EMBEDDER_REQUIRED"
    );
}
