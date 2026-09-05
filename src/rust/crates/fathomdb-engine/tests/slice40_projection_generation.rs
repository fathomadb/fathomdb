use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    Engine, EngineError, InitialState, PreparedWrite, ProjectionFts,
    ProjectionGenerationErrorReason, ProjectionGenerationOriginV1, ProjectionReadinessV1,
    ProjectionRole, ProjectionRuntimeStateV1, ProjectionSpec, ProjectionVector, SourceId,
};
use fathomdb_schema::{migrate_with_steps, MIGRATIONS, SQLITE_SUFFIX};
use rusqlite::Connection;
use tempfile::TempDir;

#[derive(Clone, Debug)]
struct CustomEmbedder;

impl Embedder for CustomEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-custom", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

fn assert_generation_id(value: &str) {
    let suffix = value.strip_prefix("pgen1:").expect("versioned generation prefix");
    assert_eq!(suffix.len(), 32);
    assert!(suffix.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)));
}

fn vector_spec() -> ProjectionSpec {
    ProjectionSpec {
        name: "summary".into(),
        roles: [ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(ProjectionFts { tokenizer: None }),
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    }
}

#[test]
fn fresh_generation_is_stable_across_restart() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("fresh{SQLITE_SUFFIX}"));
    let first = Engine::open(&path).unwrap();
    let status = first.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.origin, ProjectionGenerationOriginV1::Fresh);
    assert_eq!(status.readiness, ProjectionReadinessV1::Ready);
    assert_eq!(status.runtime_state, ProjectionRuntimeStateV1::Absent);
    assert_generation_id(status.generation_id.as_str());
    let id = status.generation_id.clone();
    first.engine.close().unwrap();

    let second = Engine::open(&path).unwrap();
    assert_eq!(second.engine.read_projection_generation_status().unwrap().generation_id, id);
}

#[test]
fn caller_embedder_does_not_make_an_empty_database_legacy() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("caller{SQLITE_SUFFIX}"));
    let first = Engine::open_with_embedder_for_test(&path, Arc::new(CustomEmbedder)).unwrap();
    let status = first.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.origin, ProjectionGenerationOriginV1::Fresh);
    let id = status.generation_id;
    first.engine.close().unwrap();

    let second = Engine::open_with_embedder_for_test(&path, Arc::new(CustomEmbedder)).unwrap();
    assert_eq!(second.engine.read_projection_generation_status().unwrap().generation_id, id);
}

#[test]
fn non_noop_configuration_mints_but_exact_replay_reuses_generation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("configuration{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: r#"{"summary":"pending"}"#.into(),
            source_id: SourceId::new("slice40-config-source").unwrap(),
            logical_id: Some("slice40-config-node".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    let before = opened.engine.read_projection_generation_status().unwrap().generation_id;

    let delta = opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    assert!(!delta.unchanged);
    let configured = opened.engine.read_projection_generation_status().unwrap();
    assert_ne!(configured.generation_id, before);
    assert_eq!(configured.origin, ProjectionGenerationOriginV1::Configuration);
    assert_eq!(configured.readiness, ProjectionReadinessV1::Blocked);
    assert!(configured.pending_count > 0);

    let replay = opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    assert!(replay.unchanged);
    assert_eq!(
        opened.engine.read_projection_generation_status().unwrap().generation_id,
        configured.generation_id
    );
}

#[cfg(feature = "operator")]
#[test]
fn each_operator_rebuild_mints_a_distinct_generation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("rebuild{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let initial = opened.engine.read_projection_generation_status().unwrap().generation_id;
    opened.engine.rebuild_vec0().unwrap();
    let first = opened.engine.read_projection_generation_status().unwrap();
    assert_ne!(first.generation_id, initial);
    assert_eq!(first.origin, ProjectionGenerationOriginV1::Rebuild);
    opened.engine.rebuild_vec0().unwrap();
    let second = opened.engine.read_projection_generation_status().unwrap();
    assert_ne!(second.generation_id, first.generation_id);
    assert_eq!(second.origin, ProjectionGenerationOriginV1::Rebuild);
}

#[test]
fn upgraded_nonempty_database_bootstraps_as_legacy_degraded() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("legacy{SQLITE_SUFFIX}"));
    let connection = Connection::open(&path).unwrap();
    migrate_with_steps(&connection, &MIGRATIONS[..31]).unwrap();
    connection
        .execute(
            "INSERT INTO operational_mutations(\
               collection_name,record_key,op_kind,payload_json,schema_id,write_cursor\
             ) VALUES('projection_failures','legacy','append','{}',NULL,1)",
            [],
        )
        .unwrap();
    drop(connection);

    let opened = Engine::open(&path).unwrap();
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.origin, ProjectionGenerationOriginV1::LegacyUnverified);
    assert_eq!(status.readiness, ProjectionReadinessV1::Degraded);
    assert_eq!(status.transition_boundary, 1);
}

#[test]
fn body_edge_without_a_runtime_is_blocked_not_corrupt() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("edge-blocked{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Edge {
            kind: "supports".into(),
            from: "a".into(),
            to: "b".into(),
            source_id: SourceId::new("slice40-edge-source").unwrap(),
            logical_id: Some("slice40-edge".into()),
            body: Some("edge evidence".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();

    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.readiness, ProjectionReadinessV1::Blocked);
    assert_eq!(status.pending_count, 1);
    assert_eq!(status.failed_count, 0);
}

#[test]
fn a_terminal_without_its_physical_vector_is_typed_corruption() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("partial-vector{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".into(),
            body: "partial projection".into(),
            source_id: SourceId::new("slice40-partial-source").unwrap(),
            logical_id: Some("slice40-partial".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();

    assert!(matches!(
        opened.engine.read_projection_generation_status(),
        Err(EngineError::ProjectionGeneration(error))
            if error.reason == ProjectionGenerationErrorReason::ProjectionGenerationCorrupt
                && error.field_path == "/projectionGeneration"
    ));
}
