use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    ActuationBatchV1, ActuationErrorReason, ActuationOperationV1, ActuationOutcomeV1,
    ActuationRefusalReasonV1, ArtifactRevisionId, CanonicalHash, Engine, EngineError,
    GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState,
    LifecycleActuationV1, LifecycleState, PreparedWrite, ProvenancedEdgeV1, ProvenancedNodeV1,
    ReadContextV1, ReadView, SearchFilter, SourceDependencyRegistrationV1, SourceId, SourceLocator,
    SourceRevisionId, SourceVersionId, TraversalDirection, WriteProvenanceV1,
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

fn regular_edge(index: usize) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "supports".into(),
        from: "source".into(),
        to: "anchor".into(),
        source_id: SourceId::new("source-a").unwrap(),
        logical_id: Some(format!("regular-edge-{index:03}")),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn seed_regular_edges(engine: &Engine, count: usize) {
    let edges = (0..count).map(regular_edge).collect::<Vec<_>>();
    for chunk in edges.chunks(128) {
        engine.write(chunk).unwrap();
    }
}

fn seed_source_and_anchor(engine: &Engine) {
    engine
        .actuate(
            ActuationBatchV1::new(
                "seed-source",
                vec![
                    ActuationOperationV1::PutCanonicalNode(canonical(
                        "source-r1",
                        "source",
                        "source body",
                    )),
                    ActuationOperationV1::PutCanonicalNode(canonical(
                        "anchor-r1",
                        "anchor",
                        "anchor body",
                    )),
                ],
            )
            .unwrap(),
        )
        .unwrap();
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
    assert_eq!(receipt.pending_projection_write_cursors.len(), 1);
    assert!(receipt.projection_generation_id.is_some());

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

    let graph = opened
        .engine
        .graph_expand(&GraphExpandRequestV1 {
            schema_version: 1,
            seed: GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("derived-unit")],
            },
            direction: TraversalDirection::Outgoing,
            edge_kinds: vec!["supports".into()],
            target_kinds: vec!["document".into()],
            context: GraphReadContextV1::Current {
                schema_version: 1,
                context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
            },
            max_depth: 1,
            result_limit: 10,
            max_work_units: 100,
            include_explanation: false,
            include_evidence: false,
        })
        .unwrap();
    assert_eq!(graph.targets.len(), 1);
    assert_eq!(graph.targets[0].logical_id, "anchor");
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
        let edge_count: i64 = Connection::open(path(&dir, name))
            .unwrap()
            .query_row("SELECT COUNT(*) FROM canonical_edges WHERE logical_id=?1", [name], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(edge_count, 0);
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
    let connection = Connection::open(path(&dir, "later-delete")).unwrap();
    let effects: (i64, String) = connection
        .query_row(
            "SELECT \
               (SELECT COUNT(*) FROM canonical_edges WHERE logical_id='later-delete-edge'),\
               (SELECT state FROM canonical_nodes WHERE logical_id='anchor' AND superseded_at IS NULL)",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(effects, (0, "active".into()));
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
    let threads = (0..8)
        .map(|_| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            let request = request.clone();
            std::thread::spawn(move || {
                barrier.wait();
                engine.actuate(request).unwrap()
            })
        })
        .collect::<Vec<_>>();
    let receipts = threads.into_iter().map(|thread| thread.join().unwrap()).collect::<Vec<_>>();
    assert!(receipts.iter().all(|receipt| receipt == &expected));

    let engine = Arc::try_unwrap(engine).ok().unwrap();
    engine.close().unwrap();
    let reopened = Engine::open(&db_path).unwrap();
    assert_eq!(reopened.engine.actuate(request.clone()).unwrap(), expected);

    let mut changed = request;
    let ActuationOperationV1::PutDerivedEdge(edge) = &mut changed.operations[2] else {
        panic!("fixture edge moved")
    };
    edge.body = Some("changed".into());
    let error = reopened.engine.actuate(changed).unwrap_err();
    assert!(matches!(
        error,
        EngineError::Actuation(ref error)
            if error.reason == ActuationErrorReason::OperationIdConflict
    ));
}

#[test]
fn eight_unique_callers_commit_complete_graph_units() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "unique-callers");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);
    let engine = Arc::new(opened.engine);
    let barrier = Arc::new(Barrier::new(8));
    let threads = (0..8)
        .map(|index| {
            let engine = Arc::clone(&engine);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                engine.actuate(graph_unit(&format!("unique-{index}"), index % 2 == 0))
            })
        })
        .collect::<Vec<_>>();
    let receipts =
        threads.into_iter().map(|thread| thread.join().unwrap().unwrap()).collect::<Vec<_>>();
    assert!(receipts.iter().all(|receipt| receipt.outcome == ActuationOutcomeV1::Committed));
    let counts: (i64, i64, i64) = Connection::open(db_path)
        .unwrap()
        .query_row(
            "SELECT \
               (SELECT COUNT(*) FROM canonical_nodes WHERE logical_id LIKE 'derived-unique-%'),\
               (SELECT COUNT(*) FROM canonical_edges WHERE logical_id LIKE 'edge-logical-unique-%'),\
               (SELECT COUNT(*) FROM _fathomdb_source_dependencies WHERE dependency_id LIKE 'dependency-unique-%')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(counts, (8, 8, 8));
}

#[test]
fn edge_graph_unit_rolls_back_at_each_injected_operation_boundary() {
    for fault_index in 0..3 {
        let dir = TempDir::new().unwrap();
        let db_path = path(&dir, &format!("fault-{fault_index}"));
        let opened = Engine::open(&db_path).unwrap();
        seed_source_and_anchor(&opened.engine);
        let request = graph_unit(&format!("fault-{fault_index}"), true);
        opened.engine.force_actuation_failure_after_operation_for_test(fault_index);
        assert!(matches!(opened.engine.actuate(request.clone()), Err(EngineError::Storage)));

        let connection = Connection::open(&db_path).unwrap();
        let domain_counts: (i64, i64, i64, i64) = connection
            .query_row(
                "SELECT \
                   (SELECT COUNT(*) FROM canonical_nodes WHERE logical_id LIKE 'derived-fault-%'),\
                   (SELECT COUNT(*) FROM canonical_edges WHERE logical_id LIKE 'edge-logical-fault-%'),\
                   (SELECT COUNT(*) FROM _fathomdb_source_dependencies WHERE dependency_id LIKE 'dependency-fault-%'),\
                   (SELECT COUNT(*) FROM _fathomdb_actuation_receipts WHERE operation_id LIKE 'fault-%')",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap();
        assert_eq!(domain_counts, (0, 0, 0, 0));
        assert_eq!(opened.engine.actuate(request).unwrap().outcome, ActuationOutcomeV1::Committed);
    }
}

#[test]
fn edge_graph_unit_rolls_back_on_forced_terminal_commit_failure() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "commit-failure");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);
    let request = graph_unit("commit-failure", true);
    opened.engine.force_next_commit_failure_for_test();
    assert!(matches!(opened.engine.actuate(request.clone()), Err(EngineError::Storage)));
    let counts: (i64, i64, i64, i64) = Connection::open(&db_path)
        .unwrap()
        .query_row(
            "SELECT \
               (SELECT COUNT(*) FROM canonical_nodes WHERE logical_id='derived-commit-failure'),\
               (SELECT COUNT(*) FROM canonical_edges WHERE logical_id='edge-logical-commit-failure'),\
               (SELECT COUNT(*) FROM _fathomdb_source_dependencies WHERE dependency_id='dependency-commit-failure'),\
               (SELECT COUNT(*) FROM _fathomdb_actuation_receipts WHERE operation_id='commit-failure')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .unwrap();
    assert_eq!(counts, (0, 0, 0, 0));
    assert_eq!(opened.engine.actuate(request).unwrap().outcome, ActuationOutcomeV1::Committed);
}

#[test]
fn edge_projection_failure_redispatches_and_receipt_binds_generation() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        path(&dir, "projection-failure"),
        Arc::new(NoopEmbedder::default()),
    )
    .unwrap();
    seed_source_and_anchor(&opened.engine);
    opened.engine.configure_vector_kind_for_test("supports").unwrap();
    opened.engine.force_next_projection_commit_failure_for_test();
    let receipt = opened.engine.actuate(graph_unit("projection", false)).unwrap();
    assert_eq!(receipt.pending_projection_write_cursors.len(), 1);
    assert!(receipt.projection_generation_id.is_some());
    let cursor = receipt.pending_projection_write_cursors[0];
    opened.engine.drain(10_000).unwrap();
    assert!(opened.engine.has_vector_for_cursor_for_test(cursor).unwrap());
    assert_eq!(opened.engine.projection_failure_count_for_test(cursor).unwrap(), 0);
}

#[test]
fn invalid_edge_provenance_and_revision_collision_refuse_without_partial_effects() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "edge-validation");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);

    let mut invalid = derived_edge("invalid-edge-r1", "invalid-edge", "source", "anchor");
    invalid.provenance = WriteProvenanceV1::derived(
        ArtifactRevisionId::new("invalid-edge-r1").unwrap(),
        SourceVersionId::new("version-source-r1").unwrap(),
        SourceRevisionId::new("source-r1").unwrap(),
        SourceLocator::whole_body(),
        CanonicalHash::sha256("0".repeat(64)).unwrap(),
    );
    let invalid_receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "invalid-provenance",
                vec![ActuationOperationV1::PutDerivedEdge(invalid)],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(invalid_receipt.outcome, ActuationOutcomeV1::Refused);

    let collision_receipt = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "revision-collision",
                vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
                    "source-r1",
                    "collision-edge",
                    "source",
                    "anchor",
                ))],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(collision_receipt.outcome, ActuationOutcomeV1::Refused);
    let count: i64 = Connection::open(db_path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM canonical_edges", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn source_erasure_redacts_edge_bearing_receipt_and_reserves_operation_id() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(path(&dir, "edge-erasure")).unwrap();
    seed_source_and_anchor(&opened.engine);
    let request = graph_unit("edge-erasure", false);
    assert_eq!(
        opened.engine.actuate(request.clone()).unwrap().outcome,
        ActuationOutcomeV1::Committed
    );
    opened.engine.erase_source("source-a").unwrap();
    assert!(matches!(
        opened.engine.actuate(request),
        Err(EngineError::Actuation(ref error))
            if error.reason == ActuationErrorReason::OperationIdErased
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
fn affected_revision_capacity_commits_at_256_and_refuses_257_atomically() {
    let exact_dir = TempDir::new().unwrap();
    let exact = Engine::open(path(&exact_dir, "exact-capacity")).unwrap();
    seed_source_and_anchor(&exact.engine);
    seed_regular_edges(&exact.engine, 255);
    let exact_receipt = exact
        .engine
        .actuate(
            ActuationBatchV1::new(
                "exact-capacity",
                vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
                    "edge-capacity-256",
                    "edge-capacity-256",
                    "source",
                    "anchor",
                ))],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(exact_receipt.outcome, ActuationOutcomeV1::Committed);
    assert_eq!(exact_receipt.affected_revision_ids.len(), 256);

    let over_dir = TempDir::new().unwrap();
    let over_path = path(&over_dir, "over-capacity");
    let over = Engine::open(&over_path).unwrap();
    seed_source_and_anchor(&over.engine);
    seed_regular_edges(&over.engine, 256);
    let mut wrong_role = derived_edge(
        "edge-wrong-role-at-capacity",
        "edge-wrong-role-at-capacity",
        "source",
        "anchor",
    );
    wrong_role.provenance = WriteProvenanceV1::canonical(
        ArtifactRevisionId::new("edge-wrong-role-at-capacity").unwrap(),
        SourceVersionId::new("version-source-r1").unwrap(),
    );
    let role_receipt = over
        .engine
        .actuate(
            ActuationBatchV1::new(
                "wrong-role-at-capacity",
                vec![ActuationOperationV1::PutDerivedEdge(wrong_role)],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(role_receipt.reason_codes, vec![ActuationRefusalReasonV1::ProvenanceRoleMismatch]);
    assert_eq!(
        role_receipt.refused_field_path.as_deref(),
        Some("/operations/0/record/provenance/role")
    );
    let over_receipt = over
        .engine
        .actuate(
            ActuationBatchV1::new(
                "over-capacity",
                vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
                    "edge-capacity-257",
                    "edge-capacity-257",
                    "source",
                    "anchor",
                ))],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(over_receipt.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(over_receipt.reason_codes, vec![ActuationRefusalReasonV1::WriteRefused]);
    assert_eq!(over_receipt.refused_field_path.as_deref(), Some("/operations/0/record"));
    let fact_edges: i64 = Connection::open(over_path)
        .unwrap()
        .query_row("SELECT COUNT(*) FROM canonical_edges WHERE body IS NOT NULL", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(fact_edges, 0);
}

#[test]
fn edge_bearing_receipt_with_unknown_affected_revision_fails_closed() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "corrupt-affected");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);
    let request = graph_unit("corrupt", false);
    opened.engine.actuate(request.clone()).unwrap();
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_actuation_receipts \
             SET affected_revision_ids_json='[\"derived-r-corrupt\",\"edge-corrupt\",\"missing-r\"]' \
             WHERE operation_id='corrupt'",
            [],
        )
        .unwrap();
    assert!(matches!(opened.engine.actuate(request), Err(EngineError::Storage)));
}

#[test]
fn edge_bearing_receipt_rejects_existing_affected_revision_substitution() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "corrupt-affected-existing");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);
    opened
        .engine
        .write(&[PreparedWrite::ProvenancedEdge(derived_edge(
            "edge-old-r1",
            "edge-shared",
            "source",
            "anchor",
        ))])
        .unwrap();
    let request = ActuationBatchV1::new(
        "corrupt-affected-existing",
        vec![ActuationOperationV1::PutDerivedEdge(derived_edge(
            "edge-new-r1",
            "edge-shared",
            "source",
            "anchor",
        ))],
    )
    .unwrap();
    assert_eq!(
        opened.engine.actuate(request.clone()).unwrap().affected_revision_ids,
        vec!["edge-new-r1", "edge-old-r1"]
    );
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "UPDATE _fathomdb_actuation_receipts \
             SET affected_revision_ids_json='[\"edge-new-r1\",\"source-r1\"]' \
             WHERE operation_id='corrupt-affected-existing'",
            [],
        )
        .unwrap();
    assert!(matches!(opened.engine.actuate(request), Err(EngineError::Storage)));
}

#[test]
fn edge_bearing_receipt_with_missing_source_reference_fails_closed() {
    let dir = TempDir::new().unwrap();
    let db_path = path(&dir, "corrupt-source-ref-missing");
    let opened = Engine::open(&db_path).unwrap();
    seed_source_and_anchor(&opened.engine);
    let request = graph_unit("corrupt-source-ref-missing", false);
    opened.engine.actuate(request.clone()).unwrap();
    Connection::open(&db_path)
        .unwrap()
        .execute(
            "DELETE FROM _fathomdb_actuation_receipt_source_refs \
             WHERE operation_id='corrupt-source-ref-missing' \
               AND ref_kind='source_id'",
            [],
        )
        .unwrap();
    assert!(matches!(opened.engine.actuate(request), Err(EngineError::Storage)));
}
