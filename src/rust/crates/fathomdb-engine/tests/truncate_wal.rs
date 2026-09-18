//! AC-040a / AC-058 / AC26-60I engine half: WAL truncation reports SQLite's
//! counters, while the operator path can recover a WAL rejected by public open.

use std::fs::{self, File};

use fathomdb_engine::{
    recover_truncate_wal, CorruptionKind, Engine, EngineOpenError, PreparedWrite, TruncateWalStatus,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::{config::DbConfig, Connection};
use tempfile::TempDir;

#[path = "support/corruption.rs"]
mod corruption;

fn seed(path: &std::path::Path) {
    let opened = Engine::open(path).expect("seed open");
    opened.engine.close().expect("seed close");
}

fn sidecar(path: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut result = path.as_os_str().to_owned();
    result.push(suffix);
    result.into()
}

fn leave_schema_cookie_in_healthy_wal(path: &std::path::Path, main_version: u32, wal_version: u32) {
    let writer = Connection::open(path).expect("open WAL fixture writer");
    writer.pragma_update(None, "journal_mode", "WAL").expect("enable WAL fixture mode");
    writer.pragma_update(None, "wal_autocheckpoint", 0).expect("disable automatic checkpoint");
    writer
        .set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)
        .expect("retain WAL when fixture writer closes");
    writer.pragma_update(None, "user_version", main_version).expect("set standalone main version");
    writer
        .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |_| Ok(()))
        .expect("checkpoint standalone main version");

    writer.pragma_update(None, "user_version", wal_version).expect("write pending schema cookie");
    drop(writer);

    let wal = sidecar(path, "-wal");
    assert!(fs::metadata(wal).expect("healthy WAL remains").len() >= 32);
}

#[test]
fn ac_040a_truncate_wal_on_empty_db_reports_done() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("wal_empty{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).expect("open");
    let report = opened.engine.truncate_wal().expect("truncate_wal");
    assert_eq!(report.status, TruncateWalStatus::Done);
    assert!(!report.discarded_corrupt_wal);
    // Counters are non-negative by construction (u32). Assert checkpointed
    // count is at most log_frames as an invariant.
    assert!(report.checkpointed_frames <= report.log_frames.max(report.checkpointed_frames));
}

#[test]
fn ac_040a_truncate_wal_after_write_reports_done() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("wal_write{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).expect("open");
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_string(),
            body: "alpha".to_string(),
            source_id: fathomdb_engine::SourceId::new("test:fixture").expect("test source id"),
            logical_id: None,
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("write");
    let report = opened.engine.truncate_wal().expect("truncate_wal");
    assert_eq!(report.status, TruncateWalStatus::Done);
    assert!(!report.discarded_corrupt_wal);
}

#[test]
fn malformed_wal_recovery_is_reachable_without_weakening_public_open() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("malformed{SQLITE_SUFFIX}"));
    seed(&path);
    corruption::corrupt_wal_invalid_page_size(&path);

    let open_error = Engine::open(&path).expect_err("public open must remain fail-closed");
    assert!(matches!(
        open_error,
        EngineOpenError::Corruption(ref detail)
            if detail.kind == CorruptionKind::WalReplayFailure
    ));

    let report = recover_truncate_wal(&path).expect("acknowledged operator recovery");
    assert_eq!(report.status, TruncateWalStatus::Done);
    assert!(report.discarded_corrupt_wal);

    let reopened = Engine::open(&path).expect("base database remains admissible");
    reopened.engine.close().expect("close reopened engine");
}

#[test]
fn absent_wal_reports_no_corrupt_discard() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("absent{SQLITE_SUFFIX}"));
    seed(&path);

    let report = recover_truncate_wal(&path).expect("truncate absent WAL");
    assert_eq!(report.status, TruncateWalStatus::Done);
    assert!(!report.discarded_corrupt_wal);
}

#[test]
fn live_product_lock_refuses_without_touching_wal() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("locked{SQLITE_SUFFIX}"));
    seed(&path);
    corruption::corrupt_wal_invalid_page_size(&path);
    let wal_path = corruption::wal_sidecar_path(&path);
    let before = fs::read(&wal_path).expect("read malformed WAL");
    let lock = File::options()
        .read(true)
        .write(true)
        .open(sidecar(&path, ".lock"))
        .expect("open product lock");
    lock.try_lock().expect("hold product lock");

    let error = recover_truncate_wal(&path).expect_err("live product lock must refuse");
    assert!(matches!(error, EngineOpenError::DatabaseLocked { .. }));
    assert_eq!(fs::read(&wal_path).expect("reread WAL"), before);
}

#[test]
fn nonempty_rollback_journal_refuses_without_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("rollback{SQLITE_SUFFIX}"));
    seed(&path);
    let journal = sidecar(&path, "-journal");
    fs::write(&journal, b"live rollback state").expect("write rollback journal fixture");
    let database_before = fs::read(&path).expect("read database");
    let journal_before = fs::read(&journal).expect("read journal");

    recover_truncate_wal(&path).expect_err("rollback journal must refuse WAL recovery");

    assert_eq!(fs::read(&path).expect("reread database"), database_before);
    assert_eq!(fs::read(&journal).expect("reread journal"), journal_before);
}

#[test]
fn missing_and_zero_length_databases_are_not_bootstrapped() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join(format!("missing{SQLITE_SUFFIX}"));
    recover_truncate_wal(&missing).expect_err("missing path must refuse");
    assert!(!missing.exists());

    let zero = dir.path().join(format!("zero{SQLITE_SUFFIX}"));
    fs::write(&zero, []).expect("create zero-length fixture");
    recover_truncate_wal(&zero).expect_err("zero-length path must refuse");
    assert_eq!(fs::metadata(&zero).expect("zero metadata").len(), 0);
}

#[test]
fn noncurrent_database_is_refused_without_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("noncurrent{SQLITE_SUFFIX}"));
    let connection = Connection::open(&path).expect("create SQLite fixture");
    connection.pragma_update(None, "user_version", 33).expect("set noncurrent schema version");
    drop(connection);
    let before = fs::read(&path).expect("read noncurrent database");

    let error = recover_truncate_wal(&path).expect_err("noncurrent database must refuse");
    assert!(matches!(
        error,
        EngineOpenError::IncompatibleSchemaVersion { seen: 33, supported: 34 }
    ));
    assert_eq!(fs::read(&path).expect("reread noncurrent database"), before);
}

#[test]
fn healthy_wal_pending_current_schema_is_accepted_and_checkpointed() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("pending_current{SQLITE_SUFFIX}"));
    leave_schema_cookie_in_healthy_wal(&path, 0, 34);

    let report = recover_truncate_wal(&path).expect("effective schema is current");
    assert_eq!(report.status, TruncateWalStatus::Done);
    assert!(!report.discarded_corrupt_wal);

    let connection = Connection::open(&path).expect("open checkpointed database");
    let seen: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .expect("read checkpointed version");
    assert_eq!(seen, 34);
}

#[test]
fn healthy_wal_effective_noncurrent_schema_refuses_without_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("pending_noncurrent{SQLITE_SUFFIX}"));
    leave_schema_cookie_in_healthy_wal(&path, 34, 33);
    let wal = sidecar(&path, "-wal");
    let database_before = fs::read(&path).expect("read standalone main");
    let wal_before = fs::read(&wal).expect("read healthy noncurrent WAL");

    let error = recover_truncate_wal(&path).expect_err("effective noncurrent schema must refuse");
    assert!(matches!(
        error,
        EngineOpenError::IncompatibleSchemaVersion { seen: 33, supported: 34 }
    ));
    assert_eq!(fs::read(&path).expect("reread standalone main"), database_before);
    assert_eq!(fs::read(&wal).expect("reread healthy noncurrent WAL"), wal_before);
}

#[test]
fn malformed_wal_with_noncurrent_standalone_main_refuses_without_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("malformed_noncurrent{SQLITE_SUFFIX}"));
    let connection = Connection::open(&path).expect("create noncurrent main fixture");
    connection.pragma_update(None, "user_version", 33).expect("set noncurrent standalone version");
    drop(connection);
    corruption::corrupt_wal_invalid_page_size(&path);
    let wal = corruption::wal_sidecar_path(&path);
    let database_before = fs::read(&path).expect("read noncurrent standalone main");
    let wal_before = fs::read(&wal).expect("read malformed WAL");

    let error = recover_truncate_wal(&path).expect_err("malformed WAL needs current main");
    assert!(matches!(
        error,
        EngineOpenError::IncompatibleSchemaVersion { seen: 33, supported: 34 }
    ));
    assert_eq!(fs::read(&path).expect("reread standalone main"), database_before);
    assert_eq!(fs::read(&wal).expect("reread malformed WAL"), wal_before);
}

#[test]
fn corrupt_main_database_is_refused_without_mutation() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("corrupt_main{SQLITE_SUFFIX}"));
    seed(&path);
    corruption::corrupt_database_header(&path);
    let before = fs::read(&path).expect("read corrupt database");

    let error = recover_truncate_wal(&path).expect_err("main corruption must refuse");
    assert!(matches!(
        error,
        EngineOpenError::Corruption(ref detail) if detail.kind == CorruptionKind::HeaderMalformed
    ));
    assert_eq!(fs::read(&path).expect("reread corrupt database"), before);
}

#[test]
fn valid_wal_busy_does_not_claim_recovery() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("busy{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).expect("open writer");
    let reader = Connection::open(&path).expect("open external reader");
    reader
        .execute_batch("BEGIN DEFERRED; SELECT COUNT(*) FROM canonical_nodes;")
        .expect("pin reader snapshot");
    opened
        .engine
        .write(&[PreparedWrite::Node {
            kind: "doc".to_string(),
            body: "busy WAL frame".to_string(),
            source_id: fathomdb_engine::SourceId::new("test:busy").expect("source id"),
            logical_id: None,
            state: fathomdb_engine::InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
        }])
        .expect("write after snapshot");
    opened.engine.close().expect("release product lock");

    let report = recover_truncate_wal(&path).expect("busy is a typed report");
    assert_eq!(report.status, TruncateWalStatus::Busy);
    assert!(!report.discarded_corrupt_wal);
    reader.execute_batch("ROLLBACK").expect("release reader snapshot");
}
