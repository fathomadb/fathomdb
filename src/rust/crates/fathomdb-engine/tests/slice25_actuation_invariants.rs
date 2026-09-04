use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ArtifactRevisionId, Engine, EngineError, InitialState, ProvenancedNodeV1, SourceId,
    SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn canonical(revision: &str, logical: &str, source_id: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: format!("body for {revision}"),
        source_id: SourceId::new(source_id).unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new(format!("version-{revision}")).unwrap(),
        ),
    }
}

#[test]
fn engine_revalidates_public_request_fields_before_digest_or_storage() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "request-validation")).unwrap();
    let mut request = ActuationBatchV1::new(
        "valid-operation",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source", "source-a"))],
    )
    .unwrap();
    request.schema_version = 2;

    assert!(matches!(
        opened.engine.actuate(request),
        Err(EngineError::Actuation(error))
            if error.reason == ActuationErrorReason::UnsupportedSchemaVersion
                && error.field_path == "/schemaVersion"
    ));
}

#[test]
fn refused_batch_rolls_back_every_domain_row_but_persists_terminal_receipt() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "rollback");
    let opened = Engine::open(&db_path).unwrap();
    let request = ActuationBatchV1::new(
        "rollback-operation",
        vec![
            ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source", "source-a")),
            ActuationOperationV1::PutCanonicalNode(canonical(
                "source-r1",
                "different-logical",
                "source-b",
            )),
        ],
    )
    .unwrap();

    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Refused);

    let connection = Connection::open(&db_path).unwrap();
    let node_count: i64 =
        connection.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
    let receipt_count: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_actuation_receipts", [], |row| row.get(0))
        .unwrap();
    assert_eq!(node_count, 0);
    assert_eq!(receipt_count, 1);
}

#[test]
fn source_erasure_redacts_receipt_and_permanently_reserves_operation_id() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "erasure");
    let opened = Engine::open(&db_path).unwrap();
    let request = ActuationBatchV1::new(
        "erased-operation",
        vec![ActuationOperationV1::PutCanonicalNode(canonical("source-r1", "source", "source-a"))],
    )
    .unwrap();
    opened.engine.actuate(request.clone()).unwrap();

    opened.engine.erase_source("source-a").unwrap();
    assert!(matches!(
        opened.engine.actuate(request),
        Err(EngineError::Actuation(error))
            if error.reason == ActuationErrorReason::OperationIdErased
                && error.field_path == "/operationId"
    ));

    let connection = Connection::open(&db_path).unwrap();
    let row: (String, Option<String>, String, String) = connection
        .query_row(
            "SELECT outcome,request_sha256,affected_revision_ids_json,reason_codes_json \
             FROM _fathomdb_actuation_receipts WHERE operation_id='erased-operation'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(row, ("erased".into(), None, "[]".into(), "[]".into()));
    let ref_count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM _fathomdb_actuation_receipt_source_refs \
             WHERE operation_id='erased-operation'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(ref_count, 0);
}
