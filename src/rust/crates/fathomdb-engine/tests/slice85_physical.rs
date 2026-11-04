use fathomdb_engine::*;
use std::sync::Arc;
fn node(id: &str, source: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
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
            dir.path().join("physical.sqlite"),
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine
        .write(&[
            node("target", "target-source", "uniquephysicalmarker85"),
            node("survivor", "kept-source", "retainedsurvivor85"),
        ])
        .unwrap();
    (dir, engine)
}
fn ticket(handle: &ErasureMaintenance) -> String {
    match handle
        .queue_physical_erasure(
            PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() },
            "ddcfe624b1464d388b6ec5bfe6483c45",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        _ => panic!("heavy admission refused"),
    }
}
#[test]
fn actual_complete_scrubs_queue_identity_main_wal_and_preserves_survivor() {
    let (_dir, engine) = fixture();
    let path = engine.path().to_owned();
    let original = std::fs::read(format!("{}-wal", path.display())).unwrap();
    assert!(original.windows(22).any(|w| w == b"uniquephysicalmarker85"));
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    let result = handle.complete_physical_erasure(&ticket).unwrap();
    assert!(matches!(result, CleanupResult::Complete { .. }), "{result:?}");
    handle.close().unwrap();
    assert!(engine.read_get("target", &ReadView::default()).unwrap().is_none());
    assert!(engine.read_get("survivor", &ReadView::default()).unwrap().is_some());
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::Complete
    );
    assert_eq!(engine.query_i64_col_for_test("SELECT count(*) FROM _fathomdb_erasure_requests WHERE json_extract(payload,'$.selector') IS NOT NULL OR json_array_length(payload,'$.scope.cursors')!=0 OR json_array_length(payload,'$.scope.sources')!=0 OR json_array_length(payload,'$.scope.identities')!=0").unwrap(),vec![0]);
    for suffix in ["", "-wal"] {
        let bytes = std::fs::read(format!("{}{suffix}", path.display())).unwrap_or_default();
        assert!(
            !bytes.windows(22).any(|w| w == b"uniquephysicalmarker85"),
            "target bytes in {suffix}"
        );
    }
}
#[test]
fn committed_failure_keeps_global_fence_through_handle_release_and_explicit_reopen() {
    let (dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    arm_physical_erasure_hook_for_test(
        "after_delete_commit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(matches!(handle.complete_physical_erasure(&ticket), Err(EngineError::Storage)));
    assert_eq!(handle.close().unwrap(), ErasureServiceState::ReadsAllowedWritesFenced);
    assert!(engine.write(&[node("new", "kept-source", "later")]).is_err());
    engine.close().unwrap();
    let engine = Arc::new(Engine::open(dir.path().join("physical.sqlite")).unwrap().engine);
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().logical_state,
        ErasureLogicalState::Removed
    );
    assert!(engine.write(&[node("new", "kept-source", "later")]).is_err());
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    assert!(matches!(
        handle.complete_physical_erasure(&ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
    engine.write(&[node("new", "kept-source", "later")]).unwrap();
}

#[test]
fn postdelete_sink_registration_refuses_before_creating_file() {
    let (dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    arm_physical_erasure_hook_for_test(
        "after_delete_commit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(handle.complete_physical_erasure(&ticket).is_err());
    handle.close().unwrap();
    let replacement = dir.path().join("must-not-create.jsonl");
    let outcome = engine.enable_telemetry(replacement.to_str().unwrap());
    assert!(outcome.is_err(), "post-delete sink registration is a forbidden mutation");
    assert!(!replacement.exists(), "refusal must precede file effects");
}

#[test]
fn postdelete_feedback_refuses_and_allowed_reads_do_not_capture() {
    let (dir, engine) = fixture();
    let sink = dir.path().join("original.jsonl");
    engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    engine.search_explained("uniquephysicalmarker85", None, 0, false, 0.3, 0).unwrap();
    let query_id = engine.last_telemetry_query_id().unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    arm_physical_erasure_hook_for_test(
        "after_delete_commit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(handle.complete_physical_erasure(&ticket).is_err());
    handle.close().unwrap();
    let before = std::fs::read(&sink).unwrap();
    let feedback = engine.record_feedback(&query_id, &[1], &[], "human");
    assert!(feedback.is_err(), "post-delete feedback must not append a durable copy");
    engine.search_explained("retainedsurvivor85", None, 0, false, 0.3, 0).unwrap();
    assert_eq!(std::fs::read(&sink).unwrap(), before);
}
