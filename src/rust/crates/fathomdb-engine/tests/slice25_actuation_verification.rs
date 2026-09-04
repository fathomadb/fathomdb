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

#[test]
fn receipt_formula_corruption_table_fails_closed() {
    let corruptions = [
        "schema_version=2",
        "request_sha256=upper(request_sha256)",
        "operations_count=0",
        "outcome='committed_closure_pending',closure_operation_ids_json='[\"closure-1\"]'",
        "reason_codes_json=' [ ]'",
        "affected_revision_ids_json='[\"source-r1\",\"source-r1\"]'",
        "affected_revision_ids_json='[\"\"]'",
        "pending_projection_write_cursors_json='[\"01\"]'",
        "pending_projection_write_cursors_json='[\"2\",\"1\"]'",
        "closure_operation_ids_json='[\"future\"]'",
        "refused_operation_index=0",
        "resulting_dependency_generation=0",
        "resulting_write_boundary=NULL",
    ];

    for (index, corruption) in corruptions.iter().enumerate() {
        let dir = TempDir::new().unwrap();
        let db_path = path(&dir, &format!("receipt-corruption-{index}"));
        let operation_id = format!("receipt-corruption-{index}");
        let batch = request(&operation_id);
        {
            let opened = Engine::open(&db_path).unwrap();
            opened.engine.actuate(batch.clone()).unwrap();
        }
        let connection = Connection::open(&db_path).unwrap();
        connection.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap();
        connection
            .execute(
                &format!(
                    "UPDATE _fathomdb_actuation_receipts SET {corruption} WHERE operation_id=?1"
                ),
                [&operation_id],
            )
            .unwrap();
        drop(connection);

        let reopened = Engine::open(&db_path).unwrap();
        assert!(
            matches!(reopened.engine.actuate(batch), Err(EngineError::Storage)),
            "receipt corruption case {index} was accepted: {corruption}"
        );
    }
}

#[test]
fn source_reference_corruption_table_fails_closed() {
    let corruptions = [
        "schema_version=2",
        "ref_kind='unknown'",
        "ref_kind='source_id',ref_value='_reserved'",
        "ref_kind='artifact_revision_id',ref_value=''",
    ];

    for (index, corruption) in corruptions.iter().enumerate() {
        let dir = TempDir::new().unwrap();
        let db_path = path(&dir, &format!("ref-corruption-{index}"));
        let operation_id = format!("ref-corruption-{index}");
        let batch = request(&operation_id);
        {
            let opened = Engine::open(&db_path).unwrap();
            opened.engine.actuate(batch.clone()).unwrap();
        }
        let connection = Connection::open(&db_path).unwrap();
        connection.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap();
        connection
            .execute(
                &format!(
                    "UPDATE _fathomdb_actuation_receipt_source_refs SET {corruption} \
                     WHERE operation_id=?1 AND rowid=(\
                       SELECT MIN(rowid) FROM _fathomdb_actuation_receipt_source_refs \
                       WHERE operation_id=?1\
                     )"
                ),
                [&operation_id],
            )
            .unwrap();
        drop(connection);

        let reopened = Engine::open(&db_path).unwrap();
        assert!(
            matches!(reopened.engine.actuate(batch), Err(EngineError::Storage)),
            "source-reference corruption case {index} was accepted: {corruption}"
        );
    }
}

#[test]
fn source_reference_limit_rejects_the_first_over_bound_row() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "ref-limit");
    let operation_id = "ref-limit";
    let batch = request(operation_id);
    {
        let opened = Engine::open(&db_path).unwrap();
        opened.engine.actuate(batch.clone()).unwrap();
    }
    let mut connection = Connection::open(&db_path).unwrap();
    let tx = connection.transaction().unwrap();
    tx.execute(
        "DELETE FROM _fathomdb_actuation_receipt_source_refs WHERE operation_id=?1",
        [operation_id],
    )
    .unwrap();
    for index in 0..1025 {
        tx.execute(
            "INSERT INTO _fathomdb_actuation_receipt_source_refs(\
               operation_id,schema_version,ref_kind,ref_value\
             ) VALUES(?1,1,'artifact_revision_id',?2)",
            [operation_id, &format!("artifact-{index:04}")],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    drop(connection);

    let reopened = Engine::open(&db_path).unwrap();
    assert!(matches!(reopened.engine.actuate(batch), Err(EngineError::Storage)));
}
