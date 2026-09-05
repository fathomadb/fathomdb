use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ArtifactRevisionId, Engine, EngineError, InitialState,
    MutationProjectionStatusRequestV1, ProjectionGenerationErrorReason, ProvenancedNodeV1,
    SourceId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

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
