use fathomdb_engine::{
    recover_complete_erasures, Engine, EngineError, InitialState, PreparedWrite, SourceId,
};
use rusqlite::Connection;
use std::{path::Path, process::Command};

fn node(id: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: format!("privateuniqueterm {id}"),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn seed(path: &Path) -> Engine {
    let engine = Engine::open(path).unwrap().engine;
    engine.write(&[node("subject", "target"), node("survivor", "survivor")]).unwrap();
    engine
}

#[test]
fn reopen_defers_historical_edge_reconciliation_until_physical_debt_finishes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("boot.sqlite");
    let engine = seed(&path);
    for body in ["old history", "new history"] {
        engine
            .write(&[PreparedWrite::Edge {
                kind: "rel".into(),
                from: "survivor".into(),
                to: "survivor".into(),
                source_id: SourceId::new("survivor").unwrap(),
                logical_id: Some("edge".into()),
                body: Some(body.into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            }])
            .unwrap();
    }
    engine.execute_for_test("DELETE FROM search_index_edges WHERE write_cursor IN (SELECT write_cursor FROM canonical_edges WHERE superseded_at IS NOT NULL)").unwrap();
    let pin = Connection::open(&path).unwrap();
    pin.execute_batch("BEGIN; SELECT * FROM canonical_nodes;").unwrap();
    assert!(matches!(engine.erase_source("target"), Err(EngineError::ErasureIncomplete { .. })));
    pin.execute_batch("ROLLBACK").unwrap();
    engine.close().unwrap();
    let before: i64 =
        pin.query_row("SELECT count(*) FROM search_index_edges", [], |r| r.get(0)).unwrap();
    drop(pin);
    let reopened = Engine::open(&path).unwrap().engine;
    let after =
        reopened.query_i64_col_for_test("SELECT count(*) FROM search_index_edges").unwrap()[0];
    assert_eq!(
        after, before,
        "reopen must not restore projections while sanitation awaits finalization"
    );
    reopened.erase_source("target").unwrap();
    reopened.close().unwrap();
}

#[test]
#[ignore = "subprocess invoked by corruption preflight witness"]
fn corrupt_child() {
    let path = std::path::PathBuf::from(std::env::var_os("SLICE82_DB").unwrap());
    let engine = seed(&path);
    engine.execute_for_test("INSERT INTO _fathomdb_open_state(key,value) VALUES('_fathomdb_fts_sanitation','{\"version\":1,\"phase\":\"invalid\",\"family\":0,\"started\":false}')").unwrap();
    std::process::exit(73);
}

#[test]
fn malformed_sanitation_refuses_recovery_before_any_writable_connection() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("malformed.sqlite");
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "corrupt_child", "--ignored"])
        .env("SLICE82_DB", &path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    let wal = std::path::PathBuf::from(format!("{}-wal", path.display()));
    let before_main = std::fs::read(&path).unwrap();
    let before_wal = std::fs::read(&wal).unwrap();
    assert!(!before_wal.is_empty());
    assert!(recover_complete_erasures(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before_main, "refusal must preserve main bytes");
    assert_eq!(std::fs::read(wal).unwrap(), before_wal, "refusal must preserve WAL bytes");
}

#[test]
#[ignore = "subprocess invoked by physical crash witnesses"]
fn erasure_crash_child() {
    let path = std::path::PathBuf::from(std::env::var_os("SLICE82_DB").unwrap());
    let phase = match std::env::var("SLICE82_PHASE").unwrap().as_str() {
        "before_delete_commit" => "before_delete_commit",
        "after_delete_commit" => "after_delete_commit",
        "before_sanitation_commit" => "before_sanitation_commit",
        "after_sanitation_unit" => "after_sanitation_unit",
        "before_completion_commit" => "before_completion_commit",
        "after_sink_redaction" => "after_sink_redaction",
        _ => panic!("unknown physical crash boundary"),
    };
    let engine = Engine::open(&path).unwrap().engine;
    if phase == "after_sink_redaction" {
        engine.enable_telemetry(path.with_extension("jsonl").to_str().unwrap()).unwrap();
    }
    fathomdb_engine::arm_physical_erasure_hook_for_test(phase, Box::new(|| std::process::exit(73)));
    engine.erase_source("target").unwrap();
    panic!("crash rendezvous was not reached");
}

#[test]
fn crash_admission_units_and_completion_replay_preserve_scope_and_debt() {
    for phase in [
        "before_delete_commit",
        "after_delete_commit",
        "before_sanitation_commit",
        "after_sanitation_unit",
        "before_completion_commit",
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("crash.sqlite");
        let engine = seed(&path);
        engine.close().unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "erasure_crash_child", "--ignored"])
            .env("SLICE82_DB", &path)
            .env("SLICE82_PHASE", phase)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73), "{phase}");
        let engine = Engine::open(&path).unwrap().engine;
        let absent = phase != "before_delete_commit";
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT count(*) FROM canonical_nodes WHERE source_id='target'"
                )
                .unwrap(),
            vec![i64::from(!absent)]
        );
        let owed = engine
            .query_i64_col_for_test(
                "SELECT count(*) FROM _fathomdb_open_state WHERE key='_fathomdb_physical_erasure'",
            )
            .unwrap()[0];
        assert_eq!(owed, i64::from(absent), "admission and deletion commit together: {phase}");
        if phase == "before_sanitation_commit" {
            assert_eq!(engine.query_i64_col_for_test("SELECT json_extract(value,'$.family') FROM _fathomdb_open_state WHERE key='_fathomdb_fts_sanitation'").unwrap(), vec![0], "rolled-back merge cannot advance progress");
        }
        if absent {
            assert!(matches!(
                engine.write(&[node("forbidden", "target")]),
                Err(EngineError::ErasureIncomplete { .. })
            ));
        }
        engine.erase_source("target").unwrap();
        assert_eq!(
            engine
                .query_i64_col_for_test(
                    "SELECT count(*) FROM canonical_nodes WHERE source_id='survivor'"
                )
                .unwrap(),
            vec![1]
        );
        assert!(engine.pending_erasures().unwrap().is_empty());
        engine.close().unwrap();
        let report = recover_complete_erasures(&path).unwrap();
        assert!(report.closure_ids.is_empty(), "completed replay is a no-op");
    }
}

#[test]
fn external_sink_cleanup_before_process_exit_is_idempotently_replayed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sink.sqlite");
    let sink = path.with_extension("jsonl");
    let engine = seed(&path);
    engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    std::fs::write(&sink, "{\"type\":\"event\",\"result_stable_ids\":[\"l:subject\"]}\n").unwrap();
    engine.close().unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "erasure_crash_child", "--ignored"])
        .env("SLICE82_DB", &path)
        .env("SLICE82_PHASE", "after_sink_redaction")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(73));
    assert!(!std::fs::read_to_string(&sink).unwrap().contains("l:subject"));
    assert!(
        recover_complete_erasures(&path).is_err(),
        "external cleanup alone cannot retire durable sink debt"
    );
    let engine = Engine::open(&path).unwrap().engine;
    engine.enable_telemetry(sink.to_str().unwrap()).unwrap();
    engine.erase_source("target").unwrap();
    assert!(engine.pending_erasures().unwrap().is_empty());
    engine.close().unwrap();
}

#[test]
fn root_debt_is_discoverable_and_completable_offline_after_commit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("offline.sqlite");
    let engine = seed(&path);
    fathomdb_engine::arm_physical_erasure_hook_for_test(
        "after_delete_commit",
        Box::new(|| Err(EngineError::Storage)),
    );
    assert!(engine.erase_source("target").is_err());
    engine.close().unwrap();
    let report = recover_complete_erasures(&path).unwrap();
    assert!(report.closure_ids.contains(&"root-erasure".to_string()));
    let reopened = Engine::open(&path).unwrap().engine;
    assert!(reopened.pending_erasures().unwrap().is_empty());
    reopened.write(&[node("new", "other")]).unwrap();
    reopened.close().unwrap();
}
