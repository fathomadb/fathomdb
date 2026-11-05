use fathomdb_sdk::{Engine, EngineConfig, OpenOptions, PhysicalErasurePolicy};

#[test]
fn sdk_persisted_policy_and_exclusive_handle_match_core() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sdk.sqlite");
    let engine = Engine::open(
        &path,
        OpenOptions {
            config: EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(engine.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    let mut handle = engine.begin_erasure_maintenance(0).unwrap();
    assert!(engine.erase_source("source").is_err());
    handle.close().unwrap();
    engine.close().unwrap();
    let reopened = Engine::open(&path, OpenOptions::default()).unwrap();
    assert_eq!(reopened.physical_erasure_policy(), PhysicalErasurePolicy::ExplicitMaintenance);
    reopened.close().unwrap();
}
