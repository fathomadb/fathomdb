use fathomdb_sdk::{
    Engine, EngineConfig, ErasureLogicalState, ErasurePhase, OpenOptions, PhysicalErasurePolicy,
    PhysicalErasureSelector, QueueResult, WithdrawResult,
};

#[test]
fn sdk_queue_receipt_and_withdrawal_preserve_unchanged_logical_state() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(
        dir.path().join("queue.sqlite"),
        OpenOptions {
            config: EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let selector = PhysicalErasureSelector::SourceAllAtCut { source_id: "test-source".into() };
    let opaque = "0123456789abcdef0123456789abcdef";
    assert_eq!(
        engine.queue_physical_erasure(selector.clone(), opaque).unwrap(),
        QueueResult::NeedsMaintenanceAdmission
    );
    let mut handle = engine.begin_erasure_maintenance(100).unwrap();
    let queued = handle.queue_physical_erasure(selector.clone(), opaque).unwrap();
    let QueueResult::Accepted { ticket, logical_state } = queued else {
        panic!("heavy admission refused")
    };
    assert_eq!(logical_state, ErasureLogicalState::UnchangedByThisRequest);
    handle.close().unwrap();
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::Queued
    );
    assert_eq!(
        engine.queue_physical_erasure(selector, opaque).unwrap(),
        QueueResult::Accepted { ticket: ticket.clone(), logical_state }
    );
    assert_eq!(
        engine.withdraw_queued_erasure(&ticket).unwrap(),
        WithdrawResult::WithdrawnBeforeRemoval
    );
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::WithdrawnBeforeRemoval
    );
    engine.close().unwrap();
}
