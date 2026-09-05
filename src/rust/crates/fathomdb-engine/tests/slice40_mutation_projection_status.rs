#![cfg(feature = "test-hooks")]

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine, EngineError, InitialState,
    MutationProjectionStatusRequestV1, ProjectionGenerationErrorReason, ProjectionRole,
    ProjectionSpec, ProjectionVector, ProvenancedNodeV1, SourceId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

#[cfg(feature = "operator")]
use fathomdb_engine::PreparedWrite;

#[derive(Debug)]
struct TestEmbedder;

impl Embedder for TestEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice40-receipt", "r1", 8)
    }

    fn embed(&self, _text: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![0.5; 8])
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

fn canonical() -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "slice40 body".into(),
        source_id: SourceId::new("source:slice40").unwrap(),
        logical_id: Some("slice40-node".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("slice40-r1").unwrap(),
            SourceVersionId::new("slice40-v1").unwrap(),
        ),
    }
}

#[test]
fn receipt_has_additive_nullable_generation_and_replays_identically() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("receipt{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let request = ActuationBatchV1::new(
        "slice40-operation",
        vec![ActuationOperationV1::PutCanonicalNode(canonical())],
    )
    .unwrap();

    let first = opened.engine.actuate(request.clone()).unwrap();
    assert!(first.pending_projection_write_cursors.is_empty());
    assert_eq!(first.projection_generation_id, None);
    assert_eq!(opened.engine.actuate(request).unwrap(), first);
}

#[test]
fn mutation_status_rejects_invalid_operation_id_with_exact_path() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("invalid{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let generation = opened.engine.read_projection_generation_status().unwrap().generation_id;
    let request = MutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id: "!".into(),
        write_cursor: 1,
        expected_generation_id: generation,
    };

    assert!(matches!(
        opened.engine.read_mutation_projection_status(request),
        Err(EngineError::ProjectionGeneration(error))
            if error.reason == ProjectionGenerationErrorReason::InvalidOperationId
                && error.field_path == "/operationId"
    ));
}

#[test]
fn pending_receipt_is_bound_to_commit_generation_and_can_be_polled() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("pending{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(TestEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let request = ActuationBatchV1::new(
        "slice40-pending-operation",
        vec![ActuationOperationV1::PutCanonicalNode(canonical())],
    )
    .unwrap();
    let receipt = opened.engine.actuate(request).unwrap();
    let generation = receipt.projection_generation_id.clone().expect("pending generation");
    let cursor = *receipt.pending_projection_write_cursors.first().expect("pending cursor");
    let status = opened
        .engine
        .read_mutation_projection_status(MutationProjectionStatusRequestV1 {
            schema_version: 1,
            operation_id: receipt.operation_id,
            write_cursor: cursor,
            expected_generation_id: generation.clone(),
        })
        .unwrap();
    assert_eq!(status.generation_id, generation);
    assert_eq!(status.write_cursor, cursor);
    assert!(status.pending_count + status.failed_count <= 1);
}

#[test]
fn tracked_cursor_without_a_current_physical_member_is_unavailable_not_ready() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("unavailable{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(TestEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-unavailable-operation",
                vec![ActuationOperationV1::PutCanonicalNode(canonical())],
            )
            .unwrap(),
        )
        .unwrap();
    let generation = receipt.projection_generation_id.expect("pending generation");
    let cursor = receipt.pending_projection_write_cursors[0];
    Connection::open(&path)
        .unwrap()
        .execute("DELETE FROM canonical_nodes WHERE write_cursor=?1", [cursor])
        .unwrap();

    assert!(matches!(
        opened.engine.read_mutation_projection_status(MutationProjectionStatusRequestV1 {
            schema_version: 1,
            operation_id: receipt.operation_id,
            write_cursor: cursor,
            expected_generation_id: generation,
        }),
        Err(EngineError::ProjectionGeneration(error))
            if error.reason
                == ProjectionGenerationErrorReason::ProjectionGenerationUnavailable
                && error.field_path == "/projectionGeneration"
    ));
}

#[test]
fn mutation_status_cache_invalidates_after_worker_publication() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("mutation-cache{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(TestEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-mutation-cache",
                vec![ActuationOperationV1::PutCanonicalNode(canonical())],
            )
            .unwrap(),
        )
        .unwrap();
    let generation = receipt.projection_generation_id.unwrap();
    let cursor = receipt.pending_projection_write_cursors[0];
    let request = MutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id: receipt.operation_id,
        write_cursor: cursor,
        expected_generation_id: generation.clone(),
    };
    assert_eq!(
        opened.engine.read_mutation_projection_status(request.clone()).unwrap().readiness,
        fathomdb_engine::ProjectionReadinessV1::Processing
    );
    opened.engine.publish_projection_success_for_test(cursor, "doc", generation).unwrap();
    assert_eq!(
        opened.engine.read_mutation_projection_status(request).unwrap().readiness,
        fathomdb_engine::ProjectionReadinessV1::Ready
    );
}

#[cfg(feature = "operator")]
#[test]
fn both_status_caches_invalidate_after_same_engine_operational_only_erasure() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("operational-cache{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(TestEmbedder)).unwrap();
    opened.engine.configure_projections(&[vector_spec()], &[]).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "slice40-operational-cache",
                vec![ActuationOperationV1::PutCanonicalNode(canonical())],
            )
            .unwrap(),
        )
        .unwrap();
    let request = MutationProjectionStatusRequestV1 {
        schema_version: 1,
        operation_id: receipt.operation_id,
        write_cursor: receipt.pending_projection_write_cursors[0],
        expected_generation_id: receipt.projection_generation_id.unwrap(),
    };
    opened
        .engine
        .write(&[PreparedWrite::AdminSchema {
            name: "slice40-events".into(),
            kind: "latest_state".into(),
            schema_json: "{}".into(),
            retention_json: "{}".into(),
        }])
        .unwrap();
    opened
        .engine
        .write(
            &(0..8)
                .map(|attempt| PreparedWrite::OpStore {
                    collection: "slice40-events".into(),
                    record_key: format!("subject-{attempt}"),
                    schema_id: None,
                    body: r#"{"state":"present"}"#.into(),
                })
                .collect::<Vec<_>>(),
        )
        .unwrap();

    for attempt in 0..8 {
        let generation_before = opened.engine.read_projection_generation_status().unwrap();
        let mutation_before =
            opened.engine.read_mutation_projection_status(request.clone()).unwrap();
        let key = format!("subject-{attempt}");
        opened.engine.excise_collection_record("slice40-events", &key).unwrap();
        let generation_after = opened.engine.read_projection_generation_status().unwrap();
        let mutation_after =
            opened.engine.read_mutation_projection_status(request.clone()).unwrap();
        if generation_before.effective_at_epoch_s == generation_after.effective_at_epoch_s
            && mutation_before.effective_at_epoch_s == mutation_after.effective_at_epoch_s
        {
            assert!(generation_after.observed_boundary > generation_before.observed_boundary);
            assert!(mutation_after.observed_boundary > mutation_before.observed_boundary);
            return;
        }
    }
    panic!("could not exercise the operational-only mutation within one epoch second");
}
