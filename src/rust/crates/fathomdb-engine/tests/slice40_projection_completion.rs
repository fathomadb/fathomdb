use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineError, InitialState, PreparedWrite, ProjectionGenerationErrorReason,
    ProjectionReadinessV1, ProjectionRole, ProjectionSpec, ProjectionVector, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Debug)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-completion", "v1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.25; 8])
    }
}

fn open(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_embedder_for_test(path, Arc::new(FixedEmbedder)).unwrap()
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "memory".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

fn node(logical_id: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("body for {logical_id}"),
        source_id: SourceId::new("slice40-completion-source").unwrap(),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn assert_generation_corruption(
    result: Result<fathomdb_engine::ProjectionGenerationStatusV1, EngineError>,
) {
    assert!(matches!(
        result,
        Err(EngineError::ProjectionGeneration(error))
            if error.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                && error.field_path == "/projectionGeneration"
    ));
}

#[test]
fn complete_edge_without_required_enrolment_is_corrupt() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("edge-enrolment{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "supports".into(),
            from: "a".into(),
            to: "b".into(),
            source_id: SourceId::new("slice40-edge-source").unwrap(),
            logical_id: Some("edge".into()),
            body: Some("edge evidence".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute("DELETE FROM _fathomdb_vector_kinds WHERE kind='edge_fact'", [])
        .unwrap();

    assert_generation_corruption(opened.engine.read_projection_generation_status());
}

#[test]
fn usable_runtime_cannot_leave_a_stranded_node_marker() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("usable-stranded{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let cursor = opened.engine.write(&[node("stranded")]).unwrap().row_cursors[0];
    opened.engine.drain(10_000).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection.execute("DELETE FROM _fathomdb_vector_kinds WHERE kind='doc'", []).unwrap();
    connection
        .execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=?1", [cursor])
        .unwrap();
    connection.execute("DELETE FROM vector_default WHERE rowid=?1", [cursor]).unwrap();

    assert_generation_corruption(opened.engine.read_projection_generation_status());
}

#[test]
fn missing_member_below_the_watermark_is_rediscovered_before_drain_returns() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("below-watermark{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let first_cursor = opened.engine.write(&[node("first")]).unwrap().row_cursors[0];
    opened.engine.drain(10_000).unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute("DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=?1", [first_cursor])
        .unwrap();
    connection
        .execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=?1", [first_cursor])
        .unwrap();
    connection.execute("DELETE FROM vector_default WHERE rowid=?1", [first_cursor]).unwrap();
    drop(connection);

    opened.engine.write(&[node("second")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status.pending_count, 0);
}

#[test]
fn sidecar_row_identity_must_match_the_projection_owner() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("sidecar-row-identity{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let cursor = opened.engine.write(&[node("sidecar-owner")]).unwrap().row_cursors[0];
    opened.engine.drain(10_000).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_vector_rows SET rowid=?2 WHERE write_cursor=?1",
            [cursor, cursor + 10_000],
        )
        .unwrap();

    assert_generation_corruption(opened.engine.read_projection_generation_status());
}
