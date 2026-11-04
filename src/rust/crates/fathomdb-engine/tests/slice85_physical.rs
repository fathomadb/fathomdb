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

#[test]
fn late_registered_second_original_cannot_be_substituted_and_both_are_redacted() {
    let (dir, engine) = fixture();
    let first = dir.path().join("first.jsonl");
    let second = dir.path().join("second.jsonl");
    let wrong = dir.path().join("wrong.jsonl");
    engine.enable_telemetry(first.to_str().unwrap()).unwrap();
    engine.search_explained("uniquephysicalmarker85", None, 0, false, 0.3, 0).unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    handle.close().unwrap();
    engine.enable_telemetry(second.to_str().unwrap()).unwrap();
    engine.search_explained("uniquephysicalmarker85", None, 0, false, 0.3, 0).unwrap();
    assert!(std::fs::read_to_string(&first).unwrap().contains("l:target"));
    assert!(std::fs::read_to_string(&second).unwrap().contains("l:target"));
    std::fs::write(&wrong, "unregistered original impersonation").unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let blocked = handle.complete_physical_erasure(&ticket).unwrap();
    assert!(matches!(
        blocked,
        CleanupResult::Incomplete {
            status: ErasureStatus {
                phase: ErasurePhase::Queued,
                logical_state: ErasureLogicalState::UnchangedByThisRequest,
                blocker: Some(ErasureBlocker::OriginalSinkUnavailable),
                ..
            }
        }
    ));
    assert!(matches!(
        handle.reattach_original_erasure_sink(&ticket, wrong.to_str().unwrap()),
        Err(EngineError::Maintenance(MaintenanceError::OriginalSinkMismatch))
    ));
    handle.reattach_original_erasure_sink(&ticket, first.to_str().unwrap()).unwrap();
    handle.reattach_original_erasure_sink(&ticket, second.to_str().unwrap()).unwrap();
    assert!(matches!(
        handle.complete_physical_erasure(&ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
    assert!(!std::fs::read_to_string(&first).unwrap().contains("l:target"));
    assert!(!std::fs::read_to_string(&second).unwrap().contains("l:target"));
    assert_eq!(std::fs::read_to_string(&wrong).unwrap(), "unregistered original impersonation");
}

#[test]
fn pinned_external_reader_keeps_redacted_ticket_fenced_until_actual_truncate() {
    let (_dir, engine) = fixture();
    let reader = rusqlite::Connection::open(engine.path()).unwrap();
    reader.execute_batch("BEGIN;SELECT count(*) FROM canonical_nodes;").unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    let blocked = handle.complete_physical_erasure(&ticket).unwrap();
    assert!(matches!(
        blocked,
        CleanupResult::Incomplete {
            status: ErasureStatus {
                phase: ErasurePhase::Finalizing,
                logical_state: ErasureLogicalState::Removed,
                blocker: Some(ErasureBlocker::WalCheckpoint),
                ..
            }
        }
    ));
    assert_eq!(handle.close().unwrap(), ErasureServiceState::ReadsAllowedWritesFenced);
    assert!(engine.write(&[node("new", "unrelated", "later")]).is_err());
    assert!(engine.read_get("survivor", &ReadView::default()).unwrap().is_some());
    reader.execute_batch("ROLLBACK").unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    assert!(matches!(
        handle.complete_physical_erasure(&ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
}

#[cfg(feature = "operator")]
#[test]
fn offline_completion_publishes_same_activated_receipt_without_activating_queued_intent() {
    let (dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let queued = ticket(&handle);
    handle.close().unwrap();
    engine.close().unwrap();
    recover_complete_erasures(dir.path().join("physical.sqlite")).unwrap();
    let engine = Arc::new(Engine::open(dir.path().join("physical.sqlite")).unwrap().engine);
    assert_eq!(
        engine.physical_erasure_status(&queued).unwrap().unwrap().phase,
        ErasurePhase::Queued
    );
    assert!(engine.read_get("target", &ReadView::default()).unwrap().is_some());
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    arm_physical_erasure_hook_for_test(
        "after_delete_commit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(handle.complete_physical_erasure(&queued).is_err());
    handle.close().unwrap();
    engine.close().unwrap();
    recover_complete_erasures(dir.path().join("physical.sqlite")).unwrap();
    let engine = Engine::open(dir.path().join("physical.sqlite")).unwrap().engine;
    assert_eq!(
        engine.physical_erasure_status(&queued).unwrap().unwrap().phase,
        ErasurePhase::Complete
    );
    assert!(engine.read_get("target", &ReadView::default()).unwrap().is_none());
}

#[test]
fn unrelated_growth_leaves_ticket_queued_when_resource_envelope_changes() {
    let (_dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    handle.close().unwrap();
    engine.write(&[node("large", "other-source", &"x".repeat((2 << 20) + 1))]).unwrap();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let blocked = handle.complete_physical_erasure(&ticket).unwrap();
    assert!(matches!(
        blocked,
        CleanupResult::Incomplete {
            status: ErasureStatus {
                phase: ErasurePhase::Queued,
                logical_state: ErasureLogicalState::UnchangedByThisRequest,
                blocker: Some(ErasureBlocker::ResourceEnvelopeChanged),
                ..
            }
        }
    ));
    handle.close().unwrap();
    assert!(engine.read_get("target", &ReadView::default()).unwrap().is_some());
    assert_eq!(
        engine.withdraw_queued_erasure(&ticket).unwrap(),
        WithdrawResult::WithdrawnBeforeRemoval
    );
}

#[test]
fn absent_logical_root_never_completes_over_retained_touching_edges() {
    let (_dir, engine) = fixture();
    engine
        .write(&[PreparedWrite::Edge {
            kind: "rel".into(),
            from: "already-absent".into(),
            to: "survivor".into(),
            source_id: SourceId::new("edge-source").unwrap(),
            logical_id: Some("dangling-edge".into()),
            body: Some("danglingphysicalmarker85".into()),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
        }])
        .unwrap();
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM canonical_edges WHERE from_id='already-absent'"
            )
            .unwrap(),
        vec![1]
    );
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = match handle
        .queue_physical_erasure(
            PhysicalErasureSelector::DeletedLogicalAllAtCut { logical_id: "already-absent".into() },
            "d89e3922b4ae4a1bb51f99ce5bf24bb4",
        )
        .unwrap()
    {
        QueueResult::Accepted { ticket, .. } => ticket,
        _ => panic!("heavy scope refused"),
    };
    assert!(matches!(
        handle.complete_physical_erasure(&ticket).unwrap(),
        CleanupResult::Complete { .. }
    ));
    handle.close().unwrap();
    assert_eq!(
        engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM canonical_edges WHERE from_id='already-absent'"
            )
            .unwrap(),
        vec![0],
        "Complete must cover the positively admitted touching-edge scope"
    );
    assert!(engine.read_get("survivor", &ReadView::default()).unwrap().is_some());
}

#[cfg(feature = "operator")]
#[test]
fn offline_refuses_orphaned_explicit_root_before_any_cleanup_mutation() {
    for edit in ["json_remove(value,'$.ticket')","json_set(value,'$.ticket','aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb')"]{
        let(dir,engine)=fixture();let mut handle=engine.begin_erasure_maintenance(1000).unwrap();let ticket=ticket(&handle);arm_physical_erasure_hook_for_test("after_delete_commit",Box::new(||Err(EngineError::Storage)));assert!(handle.complete_physical_erasure(&ticket).is_err());handle.close().unwrap();engine.execute_for_test(&format!("UPDATE _fathomdb_open_state SET value={edit} WHERE key='_fathomdb_physical_erasure'")).unwrap();engine.close().unwrap();
        let path=dir.path().join("physical.sqlite");let before=std::fs::read(&path).unwrap();let result=recover_complete_erasures(&path);assert!(result.is_err(),"offline must reject root/ticket mismatch before sanitation: {result:?}");assert_eq!(std::fs::read(&path).unwrap(),before);
    }
}
#[cfg(feature = "operator")]
#[test]
fn offline_refuses_unknown_format36_policy_before_cleanup() {
    let (dir, engine) = fixture();
    engine.execute_for_test("UPDATE _fathomdb_open_state SET value='unknown' WHERE key='_fathomdb_physical_erasure_policy'").unwrap();
    engine.close().unwrap();
    assert!(recover_complete_erasures(dir.path().join("physical.sqlite")).is_err());
}

#[test]
fn sanitation_failure_reports_sanitizing_instead_of_native_execution() {
    let (_dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    arm_physical_erasure_hook_for_test(
        "after_sanitation_unit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(handle.complete_physical_erasure(&ticket).is_err());
    handle.close().unwrap();
    assert_eq!(
        engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
        ErasurePhase::Sanitizing
    );
}

#[test]
fn partial_identity_bearing_finalization_authority_refuses_open() {
    let (dir, engine) = fixture();
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let ticket = ticket(&handle);
    arm_physical_erasure_hook_for_test(
        "after_delete_commit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(handle.complete_physical_erasure(&ticket).is_err());
    handle.close().unwrap();
    engine.execute_for_test("UPDATE _fathomdb_erasure_requests SET payload=json_set(payload,'$.status.phase','finalizing')").unwrap();
    engine.close().unwrap();
    assert!(
        Engine::open(dir.path().join("physical.sqlite")).is_err(),
        "Finalizing requires identity-free queue and redacted root authority"
    );
}
