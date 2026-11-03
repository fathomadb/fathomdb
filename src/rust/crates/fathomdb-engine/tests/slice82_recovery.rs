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
