use fathomdb_engine::*;
use std::sync::Arc;

fn explicit() -> (tempfile::TempDir, Arc<Engine>) {
    let dir = tempfile::tempdir().unwrap();
    let engine = Engine::open_with_choice_and_config(
        dir.path().join("explicit.sqlite"),
        EmbedderChoice::None,
        EngineConfig {
            physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
            ..Default::default()
        },
    )
    .unwrap()
    .engine;
    (dir, Arc::new(engine))
}

#[test]
fn exclusive_owner_refuses_newcomers_and_drop_releases_only_ephemeral_owner() {
    let (_dir, engine) = explicit();
    let handle = engine.begin_erasure_maintenance(0).unwrap();
    assert!(matches!(
        engine.read_projections(),
        Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy))
    ));
    assert!(matches!(
        engine.begin_erasure_maintenance(0),
        Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy))
    ));
    assert!(matches!(
        engine.set_profiling(true),
        Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy))
    ));
    drop(handle);
    engine.set_profiling(false).unwrap();
    engine.close().unwrap();
}

#[test]
fn explicit_direct_erasure_requires_maintenance_before_mutating() {
    let (_dir, engine) = explicit();
    for result in [engine.erase_source("source").map(|_| ()), engine.purge("missing")] {
        assert!(matches!(
            result,
            Err(EngineError::Maintenance(MaintenanceError::MaintenanceRequired))
        ));
    }
}

#[test]
fn close_revokes_idle_handle_without_waiting_for_it_to_drop() {
    let (_dir, engine) = explicit();
    let mut handle = engine.begin_erasure_maintenance(0).unwrap();
    engine.close().unwrap();
    assert_eq!(handle.close().unwrap(), ErasureServiceState::ClosedOrUnavailable);
    assert!(matches!(engine.begin_erasure_maintenance(0), Err(EngineError::Closing)));
}
