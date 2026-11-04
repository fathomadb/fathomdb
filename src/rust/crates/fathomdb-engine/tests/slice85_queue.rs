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
