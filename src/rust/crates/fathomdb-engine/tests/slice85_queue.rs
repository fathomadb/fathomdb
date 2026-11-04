use fathomdb_engine::*;
use std::sync::Arc;
fn node(id: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("body {id}"),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn fixture() -> (tempfile::TempDir, Arc<Engine>) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("queue.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine.write(&[node("target", "target-source"), node("kept", "kept-source")]).unwrap();
    (dir, engine)
}
fn accepted(result: QueueResult) -> String {
    match result {
        QueueResult::Accepted {
            ticket,
            logical_state: ErasureLogicalState::UnchangedByThisRequest,
        } => ticket,
        other => panic!("not accepted: {other:?}"),
    }
}
const KEY: &str = "d4a78f3242f14db9b5335c44925ad9b1";
#[test]
fn queue_is_non_destructive_exact_retry_and_withdrawal_retires_key() {
    let (_dir, engine) = fixture();
    let selector = PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() };
    assert_eq!(
        engine.queue_physical_erasure(selector.clone(), KEY).unwrap(),
        QueueResult::NeedsMaintenanceAdmission
    );
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = accepted(handle.queue_physical_erasure(selector.clone(), KEY).unwrap());
    handle.close().unwrap();
    assert!(engine.read_get("target", &ReadView::default()).unwrap().is_some());
    assert_eq!(accepted(engine.queue_physical_erasure(selector.clone(), KEY).unwrap()), ticket);
    assert!(matches!(
        engine.queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "other".into() },
            KEY
        ),
        Err(EngineError::Maintenance(MaintenanceError::IdempotencyConflict))
    ));
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::Queued
    );
    assert!(engine.write(&[node("target", "different")]).is_err());
    engine.write(&[node("unrelated", "kept-source")]).unwrap();
    assert_eq!(
        engine.withdraw_queued_erasure(&ticket).unwrap(),
        WithdrawResult::WithdrawnBeforeRemoval
    );
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::WithdrawnBeforeRemoval
    );
    assert!(matches!(
        engine.queue_physical_erasure(selector, KEY),
        Err(EngineError::Maintenance(MaintenanceError::KeyRetired))
    ));
    engine.write(&[node("target", "target-source")]).unwrap();
}

#[test]
fn queue_scope_conflicts_and_survives_reopen_without_worker_or_visibility_change() {
    let (dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let selector = PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() };
    let ticket = accepted(handle.queue_physical_erasure(selector.clone(), KEY).unwrap());
    assert!(matches!(
        handle.queue_physical_erasure(selector, "9a783f3242f14db9b5335c44925ad9b2"),
        Err(EngineError::Maintenance(MaintenanceError::ScopeConflict))
    ));
    handle.close().unwrap();
    engine.close().unwrap();
    let reopened = Engine::open(dir.path().join("queue.sqlite")).unwrap().engine;
    assert_eq!(
        reopened.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::Queued
    );
    assert!(reopened.read_get("target", &ReadView::default()).unwrap().is_some());
    assert!(reopened.write(&[node("target", "target-source")]).is_err());
    assert_eq!(
        reopened
            .physical_erasure_status(&format!("{}:{}", "a".repeat(32), "b".repeat(32)))
            .unwrap(),
        None
    );
}

fn queued_admin_refusal(effect: &str) {
    for effect in [effect] {
        let (_dir, engine) = fixture();
        let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
        let ticket = accepted(
            handle
                .queue_physical_erasure(
                    PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() },
                    KEY,
                )
                .unwrap(),
        );
        handle.close().unwrap();
        let before = engine
            .query_text_col_for_test("SELECT state FROM canonical_nodes WHERE logical_id='target'")
            .unwrap();
        let result = match effect {
            "lifecycle" => engine.transition("target", LifecycleState::Deleted, None),
            "importance" => engine.write_node_importance(1, 0.5),
            _ => engine
                .configure_projections(
                    &[ProjectionSpec {
                        name: "attribute".into(),
                        roles: std::collections::BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                    }],
                    &[],
                )
                .map(|_| ()),
        };
        assert!(
            matches!(result, Err(EngineError::Maintenance(MaintenanceError::QueuedScopeFenced))),
            "{effect} must refuse before state/projection mutation: {result:?}"
        );
        assert_eq!(
            engine
                .query_text_col_for_test(
                    "SELECT state FROM canonical_nodes WHERE logical_id='target'"
                )
                .unwrap(),
            before
        );
        assert_eq!(
            engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
            ErasurePhase::Queued
        );
    }
}

#[test]
fn queued_lifecycle_cannot_change_accepted_scope() {
    queued_admin_refusal("lifecycle");
}
#[test]
fn queued_importance_cannot_mutate_selected_root() {
    queued_admin_refusal("importance");
}
#[test]
fn queued_projection_cannot_rebuild_selected_root() {
    queued_admin_refusal("projection");
}

#[test]
fn indexed_phase_and_key_cannot_diverge_from_queue_authority() {
    for assignment in [
        "phase='complete'",
        "idempotency_key='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'",
        "ticket='aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'",
    ] {
        let (_dir, engine) = fixture();
        let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
        handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() },
                KEY,
            )
            .unwrap();
        handle.close().unwrap();
        let altered =
            engine.execute_for_test(&format!("UPDATE _fathomdb_erasure_requests SET {assignment}"));
        assert!(altered.is_err(), "indexed queue authority must reject divergence: {assignment}");
        assert!(engine.write(&[node("target", "target-source")]).is_err());
    }
}

#[test]
fn acceptance_reserves_terminal_receipt_capacity_for_every_pending_request() {
    let (_dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = accepted(
        handle
            .queue_physical_erasure(
                PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() },
                KEY,
            )
            .unwrap(),
    );
    handle.close().unwrap();
    engine.withdraw_queued_erasure(&ticket).unwrap();
    engine.execute_for_test("WITH RECURSIVE ids(x) AS (VALUES(2) UNION ALL SELECT x+1 FROM ids WHERE x<4095), seed AS (SELECT ticket,payload FROM _fathomdb_erasure_requests LIMIT 1) INSERT INTO _fathomdb_erasure_requests(ticket,idempotency_key,payload) SELECT substr(ticket,1,33)||printf('%032x',x),printf('%032x',x),json_set(payload,'$.ticket',substr(ticket,1,33)||printf('%032x',x),'$.key',printf('%032x',x)) FROM ids,seed").unwrap();
    assert_eq!(
        engine.query_i64_col_for_test("SELECT count(*) FROM _fathomdb_erasure_requests").unwrap(),
        vec![4095]
    );
    let handle = engine.begin_erasure_maintenance(1000).unwrap();
    handle
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "absent-one".into() },
            "dfde83285be14cf5ab859f93bc27f304",
        )
        .unwrap();
    let second = handle.queue_physical_erasure(
        PhysicalErasureSelector::SourceAllAtCut { source_id: "absent-two".into() },
        "0f88329962f741d69cc0bf9c71384377",
    );
    assert!(
        matches!(second, Err(EngineError::Maintenance(MaintenanceError::Capacity))),
        "accepted intents must reserve receipt slots: {second:?}"
    );
}
