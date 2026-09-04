use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ActuationReceiptV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState,
    LifecycleActuationV1, LifecycleState, ProvenancedNodeV1, SourceDependencyRegistrationV1,
    SourceId, SourceLocator, SourceRevisionId, SourceVersionId, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
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

fn canonical_node(revision: &str, logical: &str, source_id: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "doc".into(),
        body: format!("body-{revision}"),
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

fn derived_node(revision: &str, logical: &str, source_revision: &str) -> ProvenancedNodeV1 {
    let hash = Sha256::digest(b"body-source-r1")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("body-{revision}"),
        source_id: SourceId::new("refusal-source").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new(revision).unwrap(),
            SourceVersionId::new("version-source-r1").unwrap(),
            SourceRevisionId::new(source_revision).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(hash).unwrap(),
        ),
    }
}

fn assert_refusal_replays(
    engine: &Engine,
    request: ActuationBatchV1,
    reason: ActuationRefusalReasonV1,
    path: &str,
) -> ActuationReceiptV1 {
    let receipt = engine.actuate(request.clone()).unwrap();
    assert_eq!(receipt.reason_codes, vec![reason]);
    assert_eq!(receipt.refused_field_path.as_deref(), Some(path));
    assert_eq!(engine.actuate(request).unwrap(), receipt);
    receipt
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
fn emitted_terminal_refusal_classes_are_exactly_replayable() {
    let dir = TempDir::new().unwrap();

    let boundary = Engine::open(path(&dir, "refusal-boundary")).unwrap();
    let boundary_request = ActuationBatchV1::new(
        "wrong-boundary",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "boundary-r1",
            "boundary",
            "boundary-source",
        ))],
    )
    .unwrap()
    .with_expected_write_boundary(7);
    assert_refusal_replays(
        &boundary.engine,
        boundary_request,
        ActuationRefusalReasonV1::ExpectedWriteBoundaryMismatch,
        "/expectedWriteBoundary",
    );

    let role = Engine::open(path(&dir, "refusal-role")).unwrap();
    let role_request = ActuationBatchV1::new(
        "wrong-role",
        vec![ActuationOperationV1::PutDerivedNode(canonical_node(
            "role-r1",
            "role",
            "role-source",
        ))],
    )
    .unwrap();
    assert_refusal_replays(
        &role.engine,
        role_request,
        ActuationRefusalReasonV1::ProvenanceRoleMismatch,
        "/operations/0/record/provenance/role",
    );

    let lifecycle = Engine::open(path(&dir, "refusal-lifecycle")).unwrap();
    let lifecycle_request = ActuationBatchV1::new(
        "missing-lifecycle",
        vec![ActuationOperationV1::TransitionLifecycle(
            LifecycleActuationV1::new(
                "missing",
                ArtifactRevisionId::new("missing-r1").unwrap(),
                LifecycleState::Deleted,
                None,
            )
            .unwrap(),
        )],
    )
    .unwrap();
    assert_refusal_replays(
        &lifecycle.engine,
        lifecycle_request,
        ActuationRefusalReasonV1::LifecycleRefused,
        "/operations/0/expectedCurrentRevisionId",
    );

    let dependency_path = path(&dir, "refusal-dependency");
    let dependency = Engine::open(&dependency_path).unwrap();
    dependency
        .engine
        .actuate(
            ActuationBatchV1::new(
                "dependency-seed",
                vec![
                    ActuationOperationV1::PutCanonicalNode(canonical_node(
                        "source-r1",
                        "source",
                        "refusal-source",
                    )),
                    ActuationOperationV1::PutDerivedNode(derived_node(
                        "derived-r1",
                        "derived",
                        "source-r1",
                    )),
                    ActuationOperationV1::PutDerivedNode(derived_node(
                        "derived-r2",
                        "derived-two",
                        "source-r1",
                    )),
                    ActuationOperationV1::RegisterSourceDependency(
                        SourceDependencyRegistrationV1::new(
                            "dependency-r1",
                            "source-r1",
                            "derived-r1",
                        )
                        .unwrap(),
                    ),
                ],
            )
            .unwrap(),
        )
        .unwrap();
    let conflict = ActuationBatchV1::new(
        "dependency-conflict",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dependency-r2", "source-r1", "derived-r1")
                .unwrap(),
        )],
    )
    .unwrap();
    assert_refusal_replays(
        &dependency.engine,
        conflict,
        ActuationRefusalReasonV1::DependencyRefused,
        "/operations/0/dependency",
    );

    Connection::open(&dependency_path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_open_state SET value=?1 \
             WHERE key='_fathomdb_dependency_generation'",
            [i64::MAX.to_string()],
        )
        .unwrap();
    let exhausted = ActuationBatchV1::new(
        "dependency-exhausted",
        vec![ActuationOperationV1::RegisterSourceDependency(
            SourceDependencyRegistrationV1::new("dependency-r3", "source-r1", "derived-r2")
                .unwrap(),
        )],
    )
    .unwrap();
    assert_refusal_replays(
        &dependency.engine,
        exhausted,
        ActuationRefusalReasonV1::DependencyGenerationExhausted,
        "/operations/0/dependency",
    );
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
fn persisted_refusal_reason_index_and_path_corruption_fails_closed() {
    let corruptions = [
        ("write_refused", "0", "/operations/0/record/provenance"),
        ("write_cursor_exhausted", "0", "/operations/0"),
        ("provenance_role_mismatch", "0", "/operations/0/record/provenance"),
        ("reference_unavailable", "0", "/operations/0/unowned/sourceRevisionId"),
        ("dependency_refused", "0", "/operations/0/dependency/dependencyId"),
        ("dependency_generation_exhausted", "0", "/operations/0/dependency/id"),
        ("lifecycle_refused", "0", "/operations/0/logicalId"),
        ("dependency_closure_required", "0", "/operations/0/dependency"),
        ("expected_write_boundary_mismatch", "0", "/expectedWriteBoundary"),
        ("reference_unavailable", "1", "/operations/1/dependency/derivedRevisionId"),
    ];
    for (case, (reason, refused_index, refused_path)) in corruptions.iter().enumerate() {
        let dir = TempDir::new().unwrap();
        let db_path = path(&dir, &format!("refusal-corruption-{case}"));
        let operation_id = format!("refusal-corruption-{case}");
        let request = ActuationBatchV1::new(
            &operation_id,
            vec![ActuationOperationV1::RegisterSourceDependency(
                fathomdb_engine::SourceDependencyRegistrationV1::new(
                    "missing-dependency",
                    "missing-source",
                    "missing-derived",
                )
                .unwrap(),
            )],
        )
        .unwrap();
        {
            let opened = Engine::open(&db_path).unwrap();
            opened.engine.actuate(request.clone()).unwrap();
        }
        let connection = Connection::open(&db_path).unwrap();
        connection.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap();
        connection
            .execute(
                "UPDATE _fathomdb_actuation_receipts SET reason_codes_json=?1, \
                 refused_operation_index=?2,refused_field_path=?3 WHERE operation_id=?4",
                rusqlite::params![
                    serde_json::to_string(&[reason]).unwrap(),
                    refused_index.parse::<i64>().unwrap(),
                    refused_path,
                    operation_id,
                ],
            )
            .unwrap();
        let reopened = Engine::open(&db_path).unwrap();
        assert!(
            matches!(reopened.engine.actuate(request), Err(EngineError::Storage)),
            "corrupt refusal case {case} was accepted"
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

#[test]
fn source_reference_limit_is_eight_per_operation_not_only_global() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "ref-formula");
    let operation_id = "ref-formula";
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
    for index in 0..9 {
        tx.execute(
            "INSERT INTO _fathomdb_actuation_receipt_source_refs(\
               operation_id,schema_version,ref_kind,ref_value\
             ) VALUES(?1,1,'artifact_revision_id',?2)",
            [operation_id, &format!("artifact-{index}")],
        )
        .unwrap();
    }
    tx.commit().unwrap();
    let reopened = Engine::open(&db_path).unwrap();
    assert!(matches!(reopened.engine.actuate(batch), Err(EngineError::Storage)));
}

#[test]
fn pending_cursor_must_belong_to_a_revision_created_by_this_request() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "pending-created");
    let opened = Engine::open(&db_path).unwrap();
    opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "pending-seed",
                vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
                    "pending-old-r1",
                    "pending-logical",
                    "pending-source",
                ))],
            )
            .unwrap(),
        )
        .unwrap();
    let replacement = ActuationBatchV1::new(
        "pending-replacement",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "pending-new-r1",
            "pending-logical",
            "pending-source",
        ))],
    )
    .unwrap();
    opened.engine.actuate(replacement.clone()).unwrap();
    let connection = Connection::open(&db_path).unwrap();
    let old_cursor: i64 = connection
        .query_row(
            "SELECT write_cursor FROM _fathomdb_artifact_revisions \
             WHERE revision_id='pending-old-r1'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_actuation_receipts \
             SET pending_projection_write_cursors_json=?1 \
             WHERE operation_id='pending-replacement'",
            [serde_json::to_string(&[old_cursor.to_string()]).unwrap()],
        )
        .unwrap();
    assert!(matches!(opened.engine.actuate(replacement), Err(EngineError::Storage)));
}

#[test]
fn source_id_with_embedded_nul_commits_and_replays_exactly() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "nul-source-id");
    let source_id = "source\0id";
    let request = ActuationBatchV1::new(
        "nul-source-id",
        vec![ActuationOperationV1::PutCanonicalNode(canonical_node(
            "nul-r1",
            "nul-logical",
            source_id,
        ))],
    )
    .unwrap();
    let opened = Engine::open(&db_path).unwrap();
    let receipt = opened.engine.actuate(request.clone()).unwrap();
    assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
    let stored: String = Connection::open(&db_path)
        .unwrap()
        .query_row(
            "SELECT source_id FROM canonical_nodes WHERE logical_id='nul-logical'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored.as_bytes(), source_id.as_bytes());
}

#[test]
fn affected_and_pending_collection_bounds_accept_maxima_and_reject_one_over() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "collection-bounds");
    let opened = Engine::open(&db_path).unwrap();
    let seed = (0..128)
        .map(|index| {
            ActuationOperationV1::PutCanonicalNode(canonical_node(
                &format!("old-r{index:03}"),
                &format!("logical-{index:03}"),
                "bounds-source",
            ))
        })
        .collect();
    opened.engine.actuate(ActuationBatchV1::new("bounds-seed", seed).unwrap()).unwrap();
    let replacements = (0..128)
        .map(|index| {
            ActuationOperationV1::PutCanonicalNode(canonical_node(
                &format!("new-r{index:03}"),
                &format!("logical-{index:03}"),
                "bounds-source",
            ))
        })
        .collect();
    let request = ActuationBatchV1::new("bounds-primary", replacements).unwrap();
    let receipt = opened.engine.actuate(request.clone()).unwrap();
    assert_eq!(receipt.affected_revision_ids.len(), 256);
    assert_eq!(opened.engine.actuate(request.clone()).unwrap(), receipt);

    let connection = Connection::open(&db_path).unwrap();
    let pending = connection
        .prepare(
            "SELECT CAST(write_cursor AS TEXT) FROM _fathomdb_artifact_revisions \
             WHERE revision_id LIKE 'new-r%' ORDER BY write_cursor",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(pending.len(), 128);
    let pending_json = serde_json::to_string(&pending).unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_actuation_receipts \
             SET pending_projection_write_cursors_json=?1 WHERE operation_id='bounds-primary'",
            [&pending_json],
        )
        .unwrap();
    assert_eq!(
        opened.engine.actuate(request.clone()).unwrap().pending_projection_write_cursors.len(),
        128
    );

    let mut too_many_pending = pending;
    too_many_pending.push((receipt.resulting_write_boundary.unwrap() + 1).to_string());
    connection.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap();
    connection
        .execute(
            "UPDATE _fathomdb_actuation_receipts \
             SET pending_projection_write_cursors_json=?1 WHERE operation_id='bounds-primary'",
            [serde_json::to_string(&too_many_pending).unwrap()],
        )
        .unwrap();
    assert!(matches!(opened.engine.actuate(request.clone()), Err(EngineError::Storage)));

    let too_many_affected = (0..257).map(|index| format!("extra-r{index:03}")).collect::<Vec<_>>();
    connection
        .execute(
            "UPDATE _fathomdb_actuation_receipts SET \
             pending_projection_write_cursors_json='[]',affected_revision_ids_json=?1 \
             WHERE operation_id='bounds-primary'",
            [serde_json::to_string(&too_many_affected).unwrap()],
        )
        .unwrap();
    assert!(matches!(opened.engine.actuate(request), Err(EngineError::Storage)));
}

#[test]
fn erasure_removes_receipt_evidence_from_database_and_wal_and_uses_reverse_index() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "raw-erasure");
    let body = "slice25-secret-body-5f4a9d8c";
    let revision = "slice25-secret-revision-8c2f6a1d";
    let mut node = canonical_node(revision, "raw-erasure-logical", "raw-erasure-source");
    node.body = body.into();
    let opened = Engine::open(&db_path).unwrap();
    opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "raw-erasure-operation",
                vec![ActuationOperationV1::PutCanonicalNode(node)],
            )
            .unwrap(),
        )
        .unwrap();

    let plan = Connection::open(&db_path)
        .unwrap()
        .prepare(
            "EXPLAIN QUERY PLAN SELECT operation_id \
             FROM _fathomdb_actuation_receipt_source_refs \
             WHERE ref_kind=?1 AND ref_value=?2 AND operation_id>?3 \
             ORDER BY operation_id LIMIT 64",
        )
        .unwrap()
        .query_map(["source_id", "raw-erasure-source", ""], |row| row.get::<_, String>(3))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(plan.iter().any(|detail| {
        detail.contains("_fathomdb_actuation_receipt_refs_reverse")
            && !detail.contains("SCAN _fathomdb_actuation_receipt_source_refs")
    }));

    opened.engine.erase_source("raw-erasure-source").unwrap();
    drop(opened);
    let mut bytes = std::fs::read(&db_path).unwrap();
    let wal_path = format!("{}-wal", db_path.display());
    if let Ok(wal) = std::fs::read(wal_path) {
        bytes.extend_from_slice(&wal);
    }
    for secret in [body.as_bytes(), revision.as_bytes()] {
        assert!(!bytes.windows(secret.len()).any(|window| window == secret));
    }
}
