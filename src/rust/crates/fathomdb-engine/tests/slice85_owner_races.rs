use fathomdb_engine::*;
use std::sync::{mpsc, Arc};
use std::time::Duration;

#[test]
fn search_final_telemetry_remains_owned_until_public_operation_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("race.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine.enable_telemetry(dir.path().join("sink.jsonl").to_str().unwrap()).unwrap();
    let (reached_tx, reached_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let search = {
        let engine = Arc::clone(&engine);
        std::thread::spawn(move || {
            arm_explanation_before_telemetry_lock_hook_for_test(Box::new(move || {
                reached_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            }));
            engine.search_explained("needle", None, 0, false, 0.3, 0)
        })
    };
    reached_rx.recv_timeout(Duration::from_secs(5)).expect("actual finalization gap reached");
    let acquire = engine.begin_erasure_maintenance(0);
    let refused =
        matches!(acquire, Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy)));
    drop(acquire);
    release_tx.send(()).unwrap();
    search.join().unwrap().unwrap();
    assert!(refused, "maintenance must drain search final telemetry, not only search_inner");
    drop(engine.begin_erasure_maintenance(1000).unwrap());
}

#[test]
fn releasing_owner_keeps_new_acquisition_excluded_until_projection_thaw() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("release.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let (reached_tx, reached_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let closing = std::thread::spawn(move || {
        arm_physical_erasure_hook_for_test(
            "before_maintenance_thaw",
            Box::new(move || {
                reached_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                Ok(())
            }),
        );
        handle.close()
    });
    reached_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    let second = engine.begin_erasure_maintenance(0);
    let refused =
        matches!(second, Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy)));
    drop(second);
    release_tx.send(()).unwrap();
    closing.join().unwrap().unwrap();
    assert!(refused, "old release must not thaw projection after a new owner acquires");
    drop(engine.begin_erasure_maintenance(1000).unwrap());
}

#[test]
fn already_admitted_projection_scan_is_drained_before_handle_grant() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            dir.path().join("scan.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine.set_projection_scheduler_frozen_for_test(true);
    let mut pause = engine.pause_projection_scan_for_test();
    engine.set_projection_scheduler_frozen_for_test(false);
    pause
        .wait_ready(Duration::from_secs(5))
        .expect("actual dispatcher passed frozen check before fetch");
    let acquire = engine.begin_erasure_maintenance(0);
    let refused =
        matches!(acquire, Err(EngineError::Maintenance(MaintenanceError::MaintenanceBusy)));
    drop(acquire);
    pause.release();
    assert!(refused, "projection scan must be counted even before a job is queued");
    drop(engine.begin_erasure_maintenance(1000).unwrap());
}
