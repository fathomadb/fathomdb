use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ArtifactRevisionId, Engine,
    EngineError, InitialState, ProvenancedNodeV1, SourceId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn request(operation_id: &str) -> ActuationBatchV1 {
    let node = ProvenancedNodeV1 {
        kind: "doc".into(),
        body: "source body".into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some("source".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("source-r1").unwrap(),
            SourceVersionId::new("source-v1").unwrap(),
        ),
    };
    ActuationBatchV1::new(operation_id, vec![ActuationOperationV1::PutCanonicalNode(node)]).unwrap()
}

#[test]
fn precommit_failure_rolls_back_domain_and_receipt_then_retry_commits() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "commit-failure");
    let opened = Engine::open(&db_path).unwrap();
    let batch = request("retryable-operation");

    opened.engine.force_next_commit_failure_for_test();
    assert!(matches!(opened.engine.actuate(batch.clone()), Err(EngineError::Storage)));

    let connection = Connection::open(&db_path).unwrap();
    let nodes: i64 =
        connection.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
    let receipts: i64 = connection
        .query_row("SELECT COUNT(*) FROM _fathomdb_actuation_receipts", [], |row| row.get(0))
        .unwrap();
    assert_eq!((nodes, receipts), (0, 0));
    drop(connection);

    let receipt = opened.engine.actuate(batch).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
}

#[test]
fn terminal_receipt_replays_after_restart_without_domain_work() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "restart-replay");
    let batch = request("restart-operation");
    let first = {
        let opened = Engine::open(&db_path).unwrap();
        opened.engine.actuate(batch.clone()).unwrap()
    };

    let reopened = Engine::open(&db_path).unwrap();
    assert_eq!(reopened.engine.actuate(batch).unwrap(), first);
    let connection = Connection::open(&db_path).unwrap();
    let nodes: i64 =
        connection.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get(0)).unwrap();
    assert_eq!(nodes, 1);
}

#[test]
fn malformed_persisted_receipt_fails_closed_on_keyed_replay() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "corrupt-receipt");
    let batch = request("corrupt-operation");
    {
        let opened = Engine::open(&db_path).unwrap();
        opened.engine.actuate(batch.clone()).unwrap();
    }
    let connection = Connection::open(&db_path).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_actuation_receipts \
             SET affected_revision_ids_json='[\"source-r1\",\"source-r1\"]' \
             WHERE operation_id=?1",
            ["corrupt-operation"],
        )
        .unwrap();
    drop(connection);

    let reopened = Engine::open(&db_path).unwrap();
    assert!(matches!(reopened.engine.actuate(batch), Err(EngineError::Storage)));
}
