use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, Engine, EngineError, InitialState,
    LifecycleActuationV1, LifecycleState, PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Barrier};
use tempfile::TempDir;

fn path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn canonical(revision: &str, logical: &str, body: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "document".into(),
        body: body.into(),
        source_id: SourceId::new("source-a").unwrap(),
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

fn derived_node(revision: &str, logical: &str) -> ProvenancedNodeV1 {
    ProvenancedNodeV1 {
        kind: "fact".into(),
        body: format!("derived {revision}"),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: derived_provenance(revision),
    }
}

fn derived_provenance(revision: &str) -> WriteProvenanceV1 {
    let digest =
        Sha256::digest(b"source body").iter().map(|byte| format!("{byte:02x}")).collect::<String>();
    WriteProvenanceV1::derived(
        ArtifactRevisionId::new(revision).unwrap(),
        SourceVersionId::new("version-source-r1").unwrap(),
        SourceRevisionId::new("source-r1").unwrap(),
        SourceLocator::whole_body(),
        CanonicalHash::sha256(digest).unwrap(),
    )
}

fn derived_edge(revision: &str, logical: &str, from: &str, to: &str) -> ProvenancedEdgeV1 {
    ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(logical.into()),
        body: Some(format!("edge {revision} Ω")),
        t_valid: Some(-7),
        t_invalid: None,
        confidence: Some(0.75),
        extractor_model_id: Some("model\0id".into()),
        temporal_fallback: Some(false),
        provenance: derived_provenance(revision),
    }
}

fn plain_node(logical: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "fact".into(),
        body: format!("endpoint {logical}"),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(logical.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn seed_source_and_anchor(engine: &Engine) {
    engine
        .actuate(
            ActuationBatchV1::new(
                "seed-source",
                vec![ActuationOperationV1::PutCanonicalNode(canonical(
                    "source-r1",
                    "source",
                    "source body",
                ))],
            )
            .unwrap(),
        )
        .unwrap();
    engine.write(&[plain_node("anchor")]).unwrap();
}

fn graph_unit(operation_id: &str, edge_first: bool) -> ActuationBatchV1 {
    let edge = ActuationOperationV1::PutDerivedEdge(derived_edge(
        &format!("edge-{operation_id}"),
        &format!("edge-logical-{operation_id}"),
        &format!("derived-{operation_id}"),
        "anchor",
    ));
    let node = ActuationOperationV1::PutDerivedNode(derived_node(
        &format!("derived-r-{operation_id}"),
        &format!("derived-{operation_id}"),
    ));
    let dependency = ActuationOperationV1::RegisterSourceDependency(
        SourceDependencyRegistrationV1::new(
            format!("dependency-{operation_id}"),
            "source-r1",
            format!("derived-r-{operation_id}"),
        )
        .unwrap(),
    );
    let operations =
        if edge_first { vec![edge, node, dependency] } else { vec![node, dependency, edge] };
    ActuationBatchV1::new(operation_id, operations).unwrap()
}

#[test]
fn inherited_operations_and_edge_form_one_atomic_graph_unit() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "graph-unit");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);

    let receipt = opened.engine.actuate(graph_unit("unit", true)).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Committed);
    assert!(receipt.affected_revision_ids.contains(&"derived-r-unit".into()));
    assert!(receipt.affected_revision_ids.contains(&"edge-unit".into()));
    assert_eq!(receipt.resulting_dependency_generation, Some(1));

    let connection = Connection::open(db_path).unwrap();
    let effects: (i64, i64, i64) = (
        connection
            .query_row("SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='derived-unit'", [], |row| row.get(0))
            .unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM _fathomdb_source_dependencies WHERE dependency_id='dependency-unit'", [], |row| row.get(0))
            .unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM canonical_edges WHERE logical_id='edge-logical-unit' AND superseded_at IS NULL", [], |row| row.get(0))
            .unwrap(),
    );
    assert_eq!(effects, (1, 1, 1));
}

#[test]
fn endpoint_validation_uses_complete_final_active_state_and_from_precedence() {
    let cases = [
        ("missing-from", "missing", "anchor", "/operations/0/record/from"),
        ("missing-to", "anchor", "missing", "/operations/0/record/to"),
        ("missing-both", "missing-from", "missing-to", "/operations/0/record/from"),
    ];
    for (name, from, to, expected_path) in cases {
        let dir = TempDir::new().unwrap();
        let opened = Engine::open(path(&dir, name)).unwrap();
        seed_source_and_anchor(&opened.engine);
        let request = ActuationBatchV1::new(
            name,
            vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
                &format!("{name}-r1"),
                name,
                from,
                to,
            ))],
        )
        .unwrap();
        let receipt = opened.engine.actuate(request.clone()).unwrap();
        assert_eq!(receipt.outcome, ActuationOutcomeV1::Refused);
        assert_eq!(receipt.reason_codes, vec![ActuationRefusalReasonV1::ReferenceUnavailable]);
        assert_eq!(receipt.refused_field_path.as_deref(), Some(expected_path));
        assert_eq!(opened.engine.actuate(request).unwrap(), receipt);
    }

    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "later-delete")).unwrap();
    seed_source_and_anchor(&opened.engine);
    let anchor_revision = {
        let connection = Connection::open(path(&dir, "later-delete")).unwrap();
        connection
            .query_row(
                "SELECT ar.revision_id FROM canonical_nodes n JOIN _fathomdb_artifact_revisions ar ON ar.write_cursor=n.write_cursor WHERE n.logical_id='anchor' AND n.superseded_at IS NULL",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap()
    };
    let request = ActuationBatchV1::new(
        "later-delete",
        vec![
            ActuationOperationV1::PutDerivedEdge(derived_edge(
                "later-delete-edge-r1",
                "later-delete-edge",
                "anchor",
                "source",
            )),
            ActuationOperationV1::TransitionLifecycle(
                LifecycleActuationV1::new(
                    "anchor",
                    ArtifactRevisionId::new(anchor_revision).unwrap(),
                    LifecycleState::Deleted,
                    None,
                )
                .unwrap(),
            ),
        ],
    )
    .unwrap();
    let receipt = opened.engine.actuate(request).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(receipt.refused_field_path.as_deref(), Some("/operations/0/record/from"));
}

#[test]
fn edge_digest_replay_conflict_and_concurrent_exact_replay_are_closed() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "replay");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);
    let request = graph_unit("shared-edge", false);
    let expected = opened.engine.actuate(request.clone()).unwrap();

    let engine = Arc::new(opened.engine);
    let barrier = Arc::new(Barrier::new(8));
    let receipts = (0..8)
        .map(|_| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            let request = request.clone();
            std::thread::spawn(move || {
                barrier.wait();
                engine.actuate(request).unwrap()
            })
        })
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert!(receipts.iter().all(|receipt| receipt == &expected));

    let mut changed = request;
    let ActuationOperationV1::PutDerivedEdge(edge) = &mut changed.operations[2] else {
        panic!("fixture edge moved")
    };
    edge.body = Some("changed".into());
    let error = engine.actuate(changed).unwrap_err();
    assert!(matches!(
        error,
        EngineError::Actuation(ref error)
            if error.reason == ActuationErrorReason::OperationIdConflict
    ));
}

#[test]
fn edge_supersession_receipt_carries_g0_g11_and_new_revisions() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "supersession")).unwrap();
    seed_source_and_anchor(&opened.engine);
    opened.engine.write(&[plain_node("other")]).unwrap();
    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedEdge(derived_edge(
                "edge-g0",
                "edge-same",
                "source",
                "anchor",
            )),
            PreparedWrite::ProvenancedEdge(derived_edge(
                "edge-g11",
                "edge-other",
                "anchor",
                "other",
            )),
        ])
        .unwrap();

    let receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "edge-supersession",
                vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
                    "edge-new",
                    "edge-same",
                    "anchor",
                    "other",
                ))],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(receipt.affected_revision_ids, vec!["edge-new", "edge-g0", "edge-g11"]);
}

#[test]
fn prototype_fresh_boundary_is_read_only_and_distinguishes_schema_33_from_34() {
    let dir = TempDir::new().unwrap();
    let old_path = path(&dir, "schema33");
    {
        let opened = Engine::open(&old_path).unwrap();
        opened.engine.close().unwrap();
    }
    let before = std::fs::read(&old_path).unwrap();
    assert!(!fathomdb_engine::classify_fresh_database_candidate_for_test(&old_path, 34).unwrap());
    assert_eq!(std::fs::read(&old_path).unwrap(), before);
    assert!(!old_path.with_extension("sqlite-wal").exists());

    let future_path = path(&dir, "schema34");
    let connection = Connection::open(&future_path).unwrap();
    connection.pragma_update(None, "user_version", 34).unwrap();
    drop(connection);
    assert!(fathomdb_engine::classify_fresh_database_candidate_for_test(&future_path, 34).unwrap());
    assert!(fathomdb_engine::classify_fresh_database_candidate_for_test(
        &path(&dir, "missing"),
        34,
    )
    .unwrap());
}
