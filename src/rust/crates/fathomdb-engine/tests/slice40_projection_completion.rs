use std::sync::{Arc, Barrier};

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState, PreparedWrite,
    ProjectionGenerationErrorReason, ProjectionReadinessV1, ProjectionRole, ProjectionSpec,
    ProjectionVector, ProvenancedNodeV1, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, WriteProvenanceV1,
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

fn sha256(body: &str) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(body.as_bytes()).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn seed_registered_dense_owner(engine: &Engine, suffix: &str) {
    let source_body = format!("source body {suffix}");
    let source_revision = format!("slice40-source-r-{suffix}");
    let derived_revision = format!("slice40-derived-r-{suffix}");
    engine.configure_projections(&[vector_spec()], &[]).unwrap();
    engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: source_body.clone(),
                source_id: SourceId::new(format!("source:slice40-{suffix}")).unwrap(),
                logical_id: Some(format!("slice40-source-{suffix}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new(source_revision.clone()).unwrap(),
                    SourceVersionId::new(format!("slice40-source-v-{suffix}")).unwrap(),
                ),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                kind: "doc".into(),
                body: format!("derived body {suffix}"),
                source_id: SourceId::new(format!("source:slice40-{suffix}")).unwrap(),
                logical_id: Some(format!("slice40-derived-{suffix}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new(derived_revision.clone()).unwrap(),
                    SourceVersionId::new(format!("slice40-source-v-{suffix}")).unwrap(),
                    SourceRevisionId::new(source_revision.clone()).unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(sha256(&source_body)).unwrap(),
                ),
            }),
        ])
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new(
                format!("slice40-dependency-{suffix}"),
                source_revision,
                derived_revision,
            )
            .unwrap(),
        )
        .unwrap();
    engine.drain(10_000).unwrap();
    assert_eq!(
        engine.read_projection_generation_status().unwrap().readiness,
        ProjectionReadinessV1::Ready
    );
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

#[test]
fn global_status_rejects_a_cursor_owned_by_both_node_and_edge() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("cross-owner-cursor{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let cursor = opened.engine.write(&[node("node-owner")]).unwrap().row_cursors[0];
    Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO canonical_edges(\
               write_cursor,kind,from_id,to_id,source_id,logical_id,body\
             ) VALUES(?1,'supports','a','b','slice40-cross-owner','edge-owner',NULL)",
            [cursor],
        )
        .unwrap();

    assert_generation_corruption(opened.engine.read_projection_generation_status());
}

#[test]
fn worker_publication_never_repairs_a_partial_projection_tuple() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("partial-publication{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let (ready, release) = opened.engine.pause_projection_worker_after_wal_transaction_for_test();
    let failure_reported = Arc::new(Barrier::new(2));
    let failure_release = Arc::new(Barrier::new(2));
    opened.engine.pause_projection_commit_failure_cleanup_for_test(
        Arc::clone(&failure_reported),
        Arc::clone(&failure_release),
    );
    Connection::open(&path)
        .unwrap()
        .execute("INSERT INTO _fathomdb_vector_rows(rowid,kind,write_cursor) VALUES(1,'doc',1)", [])
        .unwrap();
    let cursor = opened.engine.write(&[node("partial-before-publication")]).unwrap().row_cursors[0];
    assert_eq!(cursor, 1);
    ready.wait();
    release.wait();
    failure_reported.wait();

    let connection = Connection::open(&path).unwrap();
    let state: (u64, u64, u64) = connection
        .query_row(
            "SELECT \
               (SELECT COUNT(*) FROM _fathomdb_vector_rows WHERE rowid=1 AND write_cursor=1),\
               (SELECT COUNT(*) FROM vector_default WHERE rowid=1),\
               (SELECT COUNT(*) FROM _fathomdb_projection_terminal WHERE write_cursor=1)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(state, (1, 0, 0));

    assert_generation_corruption(opened.engine.read_projection_generation_status());
    failure_release.wait();
}

#[test]
fn generation_status_cache_invalidates_on_worker_and_external_changes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("status-cache{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let cursor = opened.engine.write(&[node("cached-owner")]).unwrap().row_cursors[0];

    let pending = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(pending.readiness, ProjectionReadinessV1::Processing);
    opened
        .engine
        .publish_projection_success_for_test(cursor, "doc", pending.generation_id)
        .unwrap();
    let ready = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(ready.readiness, ProjectionReadinessV1::Ready);

    Connection::open(&path)
        .unwrap()
        .execute("DELETE FROM vector_default WHERE rowid=?1", [cursor])
        .unwrap();
    assert_generation_corruption(opened.engine.read_projection_generation_status());
}

#[test]
fn mixed_completion_summary_is_exact_and_boundary_ordered() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("mixed-summary{SQLITE_SUFFIX}"));
    let opened = open(&path);
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let cursors = opened
        .engine
        .write(&[node("complete"), node("failed"), node("pending")])
        .unwrap()
        .row_cursors;
    let generation = opened.engine.read_projection_generation_status().unwrap().generation_id;
    opened.engine.publish_projection_success_for_test(cursors[0], "doc", generation).unwrap();
    Connection::open(&path)
        .unwrap()
        .execute(
            "INSERT INTO _fathomdb_projection_terminal(write_cursor,state) VALUES(?1,'failed')",
            [cursors[1]],
        )
        .unwrap();

    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.readiness, ProjectionReadinessV1::Degraded);
    assert_eq!(status.pending_count, 1);
    assert_eq!(status.failed_count, 1);
    assert_eq!(status.ready_through, cursors[0]);
}

#[test]
fn registered_owner_with_missing_source_link_is_corrupt() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("missing-source-link{SQLITE_SUFFIX}"));
    let opened = open(&path);
    seed_registered_dense_owner(&opened.engine, "missing-link");
    Connection::open(&path)
        .unwrap()
        .execute(
            "DELETE FROM _fathomdb_source_links WHERE artifact_revision_id='slice40-derived-r-missing-link'",
            [],
        )
        .unwrap();

    assert_generation_corruption(opened.engine.read_projection_generation_status());
}

#[test]
fn orphan_dependency_and_source_link_are_corrupt() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("orphan-dependency{SQLITE_SUFFIX}"));
    let opened = open(&path);
    seed_registered_dense_owner(&opened.engine, "orphan");
    Connection::open(&path)
        .unwrap()
        .execute(
            "DELETE FROM _fathomdb_artifact_revisions WHERE revision_id='slice40-derived-r-orphan'",
            [],
        )
        .unwrap();

    assert_generation_corruption(opened.engine.read_projection_generation_status());
}
