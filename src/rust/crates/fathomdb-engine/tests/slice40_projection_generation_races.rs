#![cfg(feature = "test-hooks")]

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine,
    MutationProjectionStatusRequestV1, ProjectionGenerationErrorReason, ProjectionRole,
    ProjectionSpec, ProjectionVector, ProvenancedNodeV1, SourceId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[derive(Debug)]
struct RaceEmbedder;

impl Embedder for RaceEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-race", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.25; 8])
    }
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

#[test]
fn stale_worker_result_cannot_publish_into_a_new_generation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("stale-publication{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(RaceEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);

    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-stale-publication",
                vec![ActuationOperationV1::PutCanonicalNode(ProvenancedNodeV1 {
                    kind: "doc".into(),
                    body: "generation-bound evidence".into(),
                    source_id: SourceId::new("source:slice40-race").unwrap(),
                    logical_id: Some("slice40-race-node".into()),
                    state: fathomdb_engine::InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("slice40-race-r1").unwrap(),
                        SourceVersionId::new("slice40-race-v1").unwrap(),
                    ),
                })],
            )
            .unwrap(),
        )
        .unwrap();
    let cursor = receipt.pending_projection_write_cursors[0];
    let old_generation = receipt.projection_generation_id.unwrap();

    let new_generation = opened.engine.transition_projection_generation_for_test().unwrap();
    assert_ne!(new_generation, old_generation);
    opened
        .engine
        .publish_projection_success_for_test(cursor, "doc", old_generation.clone())
        .unwrap();
    assert!(!opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());

    let error = opened
        .engine
        .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
            schema_version: 1,
            operation_id: receipt.operation_id,
            write_cursor: cursor,
            expected_generation_id: old_generation,
        })
        .unwrap_err();
    assert!(matches!(
        error,
        fathomdb_engine::EngineError::ProjectionGeneration(error)
            if error.reason
                == ProjectionGenerationErrorReason::ProjectionGenerationUnavailable
    ));

    opened.engine.set_projection_scheduler_frozen_for_test(false);
    opened.engine.drain(5_000).unwrap();
    let status = opened.engine.read_projection_generation_status().unwrap();
    assert_eq!(status.generation_id, new_generation);
    assert_eq!(status.readiness.as_str(), "ready");
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
}
