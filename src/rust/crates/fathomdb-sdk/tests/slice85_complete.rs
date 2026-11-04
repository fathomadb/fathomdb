use fathomdb_sdk::{
    CleanupResult, Engine, EngineConfig, ErasurePhase, OpenOptions, PhysicalErasurePolicy,
    PhysicalErasureSelector, QueueResult,
};

#[test]
fn sdk_complete_returns_only_its_terminal_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(
        dir.path().join("complete.sqlite"),
        OpenOptions {
            config: EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let accepted = handle
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "test-source".into() },
            "f23e2130b50242e3b8d545d4fa3d8015",
        )
        .unwrap();
    let QueueResult::Accepted { ticket, .. } = accepted else { panic!("heavy admission refused") };
    let completed = handle.complete_physical_erasure(&ticket).unwrap();
    let CleanupResult::Complete { receipt } = completed else { panic!("completion blocked") };
    assert_eq!(receipt.phase, ErasurePhase::Complete);
    handle.close().unwrap();
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::Complete
    );
    engine.close().unwrap();
}
