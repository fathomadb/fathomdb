use fathomdb_engine::*;
use std::sync::Arc;
#[test]
fn later_original_inventory_growth_stays_bounded_and_reopenable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("budget.sqlite");
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            &path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    let mut writes = Vec::new();
    for group in 0..6 {
        for index in 0..16384 {
            writes.push(PreparedWrite::Node {
                kind: "doc".into(),
                body: "small".into(),
                source_id: SourceId::new(format!("réserve-{group}")).unwrap(),
                logical_id: Some(format!("n{group}-{index}")),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
            });
        }
    }
    engine.write(&writes).unwrap();
    drop(writes);
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let mut tickets = Vec::new();
    for group in 0..6 {
        tickets.push(
            match handle
                .queue_physical_erasure(
                    PhysicalErasureSelector::SourceAllAtCut {
                        source_id: format!("réserve-{group}"),
                    },
                    &format!("{group:032x}"),
                )
                .unwrap()
            {
                QueueResult::Accepted { ticket, .. } => ticket,
                _ => panic!("heavy admission refused"),
            },
        );
    }
    handle.close().unwrap();
    for sink in 0..4096 {
        engine
            .enable_telemetry(dir.path().join(format!("sink-{sink}.jsonl")).to_str().unwrap())
            .unwrap();
    }
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    let original = dir.path().join("sink-0.jsonl");
    let mut blocked = None;
    for ticket in &tickets {
        match handle.reattach_original_erasure_sink(ticket, original.to_str().unwrap()) {
            Ok(()) => {}
            Err(EngineError::Maintenance(MaintenanceError::Capacity)) => {
                blocked = Some(ticket.clone());
                break;
            }
            Err(error) => panic!("unexpected reattachment error: {error}"),
        }
    }
    handle.close().unwrap();
    let bytes = engine
        .query_i64_col_for_test(
            "SELECT SUM(length(CAST(payload AS BLOB))) FROM _fathomdb_erasure_requests",
        )
        .unwrap()[0];
    assert!(
        bytes <= 8 << 20,
        "legitimate later originals must not create unopenable overbudget metadata: {bytes}"
    );
    let blocked = blocked.expect("nearbudget expansion must refuse before changing authority");
    let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
    assert!(matches!(
        handle.complete_physical_erasure(&blocked).unwrap(),
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
    engine.close().unwrap();
    let reopened = Engine::open(&path).unwrap().engine;
    assert_eq!(
        reopened.physical_erasure_status(&blocked).unwrap().unwrap().phase,
        ErasurePhase::Queued
    );
}
