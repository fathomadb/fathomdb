use fathomdb_engine::*;
use std::sync::Arc;
use std::time::{Duration, Instant};
const KEY: &str = "47efde44f14f443a9f5eea807c96504f";
fn node(id: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("crashbody85 {id}"),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn selector() -> PhysicalErasureSelector {
    PhysicalErasureSelector::SourceAllAtCut { source_id: "target-source".into() }
}
fn accepted(result: QueueResult) -> String {
    match result {
        QueueResult::Accepted { ticket, .. } => ticket,
        _ => panic!("heavy admission refused"),
    }
}
#[test]
#[ignore]
fn crash_child() {
    let path = std::env::var("S85_CRASH_PATH").unwrap();
    let phase = std::env::var("S85_CRASH_PHASE").unwrap();
    let phase: &'static str = Box::leak(phase.into_boxed_str());
    let engine = Arc::new(
        Engine::open_with_choice_and_config(
            path,
            EmbedderChoice::None,
            EngineConfig {
                physical_erasure_policy: Some(PhysicalErasurePolicy::ExplicitMaintenance),
                ..Default::default()
            },
        )
        .unwrap()
        .engine,
    );
    engine.write(&[node("target", "target-source"), node("survivor", "kept-source")]).unwrap();
    let handle = engine.begin_erasure_maintenance(1000).unwrap();
    if phase.contains("queue_commit") {
        arm_physical_erasure_hook_for_test(phase, Box::new(|| std::process::exit(73)));
    }
    let ticket = accepted(handle.queue_physical_erasure(selector(), KEY).unwrap());
    if !phase.contains("queue_commit") {
        arm_physical_erasure_hook_for_test(phase, Box::new(|| std::process::exit(73)));
    }
    handle.complete_physical_erasure(&ticket).unwrap();
    std::process::exit(99);
}
#[test]
fn actual_process_exits_preserve_atomic_intent_removal_redaction_and_receipt() {
    for phase in [
        "before_queue_commit",
        "after_queue_commit",
        "before_delete_commit",
        "after_delete_commit",
        "before_redacted_commit",
        "after_redacted_commit",
        "before_completion_commit",
        "after_completion_commit",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crash.sqlite");
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "crash_child", "--ignored", "--nocapture"])
            .env("S85_CRASH_PATH", &path)
            .env("S85_CRASH_PHASE", phase)
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let status = loop {
            if let Some(status) = child.try_wait().unwrap() {
                break status;
            }
            if Instant::now() >= deadline {
                child.kill().unwrap();
                let status = child.wait().unwrap();
                panic!("child deadlocked at {phase}: {status}")
            }
            std::thread::sleep(Duration::from_millis(10));
        };
        assert_eq!(status.code(), Some(73), "real fault boundary not reached: {phase}");
        let engine = Arc::new(Engine::open(&path).unwrap().engine);
        let tickets = engine
            .query_text_col_for_test(&format!(
                "SELECT ticket FROM _fathomdb_erasure_requests WHERE idempotency_key='{KEY}'"
            ))
            .unwrap();
        let before_removal =
            matches!(phase, "before_queue_commit" | "after_queue_commit" | "before_delete_commit");
        assert_eq!(
            engine.read_get("target", &ReadView::default()).unwrap().is_some(),
            before_removal,
            "logical visibility at {phase}"
        );
        assert!(engine.read_get("survivor", &ReadView::default()).unwrap().is_some());
        if phase == "before_queue_commit" {
            assert!(tickets.is_empty());
        } else {
            let status = engine.physical_erasure_status(&tickets[0]).unwrap().unwrap();
            assert_eq!(
                status.logical_state,
                if before_removal {
                    ErasureLogicalState::UnchangedByThisRequest
                } else {
                    ErasureLogicalState::Removed
                }
            );
            if phase == "after_completion_commit" {
                assert_eq!(status.phase, ErasurePhase::Complete);
            }
        }
        let mut handle = engine.begin_erasure_maintenance(1000).unwrap();
        let ticket = if tickets.is_empty() {
            accepted(handle.queue_physical_erasure(selector(), KEY).unwrap())
        } else {
            tickets[0].clone()
        };
        assert!(
            matches!(
                handle.complete_physical_erasure(&ticket).unwrap(),
                CleanupResult::Complete { .. }
            ),
            "retry at {phase}"
        );
        handle.close().unwrap();
        assert_eq!(
            engine.physical_erasure_status(&ticket).unwrap().unwrap().phase,
            ErasurePhase::Complete
        );
        assert!(engine.read_get("target", &ReadView::default()).unwrap().is_none());
    }
}
