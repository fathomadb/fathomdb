use fathomdb_sdk::{
    Engine, EngineConfig, EngineError, Error, MaintenanceError, OpenOptions, PhysicalErasurePolicy,
    PhysicalErasureSelector, QueueResult,
};

#[test]
fn sdk_original_sink_attachment_refuses_substitution_and_accepts_recorded_original() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open(
        dir.path().join("sink.sqlite"),
        OpenOptions {
            config: EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let original = dir.path().join("original.jsonl");
    engine.enable_telemetry(original.to_str().unwrap()).unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let accepted = handle
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "source".into() },
            "f23e2130b50242e3b8d545d4fa3d8015",
        )
        .unwrap();
    let QueueResult::Accepted { ticket, .. } = accepted else { panic!("heavy admission refused") };
    let wrong = dir.path().join("replacement.jsonl");
    std::fs::write(&wrong, "").unwrap();
    let refusal =
        handle.reattach_original_erasure_sink(&ticket, wrong.to_str().unwrap()).unwrap_err();
    assert!(matches!(
        refusal,
        Error::Engine(EngineError::Maintenance(MaintenanceError::OriginalSinkMismatch))
    ));
    handle.reattach_original_erasure_sink(&ticket, original.to_str().unwrap()).unwrap();
    handle.close().unwrap();
    engine.close().unwrap();
}
