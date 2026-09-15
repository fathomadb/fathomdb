use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use fathomdb_engine::{Engine, EngineOpenError};
use fathomdb_schema::{MIGRATIONS, SCHEMA_VERSION};
use rusqlite::Connection;
use tempfile::TempDir;

const WAL_CHILD_MODE: &str = "FATHOMDB_SLICE40_WAL_CHILD_MODE";
const WAL_CHILD_PATH: &str = "FATHOMDB_SLICE40_WAL_CHILD_PATH";

fn path(dir: &TempDir, name: &str) -> PathBuf {
    dir.path().join(format!("{name}.sqlite"))
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(suffix);
    PathBuf::from(value)
}

fn directory_bytes(dir: &TempDir) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name().to_string_lossy().into_owned(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

fn create_versioned_sqlite(path: &Path, version: u32) {
    let connection = Connection::open(path).unwrap();
    connection.execute("CREATE TABLE marker(value TEXT NOT NULL)", []).unwrap();
    connection.execute("INSERT INTO marker VALUES('preserve-me')", []).unwrap();
    connection.pragma_update(None, "user_version", version).unwrap();
    connection.close().unwrap();
}

fn assert_incompatible(result: Result<fathomdb_engine::OpenedEngine, EngineOpenError>, seen: u32) {
    match result.expect_err("non-current database must be refused") {
        EngineOpenError::IncompatibleSchemaVersion { seen: actual, supported } => {
            assert_eq!(actual, seen);
            assert_eq!(supported, 34);
        }
        other => panic!("expected IncompatibleSchemaVersion, got {other:?}"),
    }
}

fn run_wal_child(path: &Path, version: u32) {
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("slice40_wal_fixture_child")
        .arg("--nocapture")
        .env(WAL_CHILD_MODE, version.to_string())
        .env(WAL_CHILD_PATH, path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "WAL child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(sidecar(path, "-wal").is_file(), "child must leave WAL");
    assert!(sidecar(path, "-shm").is_file(), "child must leave SHM");
}

#[test]
fn slice40_wal_fixture_child() {
    let Ok(version) = std::env::var(WAL_CHILD_MODE) else {
        return;
    };
    let path = PathBuf::from(std::env::var_os(WAL_CHILD_PATH).unwrap());
    let connection = Connection::open(path).unwrap();
    connection.pragma_update(None, "journal_mode", "WAL").unwrap();
    connection.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
    connection.pragma_update(None, "user_version", version.parse::<u32>().unwrap()).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE; CREATE TABLE IF NOT EXISTS slice40_wal_marker(value TEXT); INSERT INTO slice40_wal_marker VALUES('committed'); COMMIT;").unwrap();
    std::mem::forget(connection);
}

#[test]
fn schema_34_bootstraps_fresh_and_reopens_without_migration() {
    assert_eq!(SCHEMA_VERSION, 34);
    assert_eq!(MIGRATIONS.last().map(|migration| migration.step_id), Some(34));
    assert_eq!(MIGRATIONS.len(), 34);

    let dir = TempDir::new().unwrap();
    let db = path(&dir, "fresh");
    let fresh = Engine::open(&db).unwrap();
    assert_eq!(fresh.report.schema_version_before, 0);
    assert_eq!(fresh.report.schema_version_after, 34);
    assert_eq!(fresh.report.migration_steps.last().map(|step| step.step_id), Some(34));
    fresh.engine.close().unwrap();

    let reopened = Engine::open(&db).unwrap();
    assert_eq!(reopened.report.schema_version_before, 34);
    assert_eq!(reopened.report.schema_version_after, 34);
    assert!(reopened.report.migration_steps.is_empty());
    reopened.engine.close().unwrap();
}

#[test]
fn schema_33_public_opens_refuse_without_any_durable_change() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "old");
    create_versioned_sqlite(&db, 33);
    fs::write(sidecar(&db, ".lock"), b"historical-lock-bytes").unwrap();
    let before = directory_bytes(&dir);

    assert_incompatible(Engine::open(&db), 33);
    assert_eq!(directory_bytes(&dir), before);

    let mut events = Vec::new();
    assert_incompatible(
        Engine::open_with_migration_event_sink(&db, |event| events.push(event.clone())),
        33,
    );
    assert!(events.is_empty());
    assert_eq!(directory_bytes(&dir), before);
}

#[test]
fn future_and_nonempty_zero_version_databases_are_refused() {
    for (name, version) in [("future", 35), ("foreign", 0)] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, name);
        create_versioned_sqlite(&db, version);
        let before = directory_bytes(&dir);
        assert_incompatible(Engine::open(&db), version);
        assert_eq!(directory_bytes(&dir), before);
    }
}

#[test]
fn current_wal_state_is_admitted_with_and_without_shm() {
    for remove_shm in [false, true] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, if remove_shm { "current-no-shm" } else { "current-shm" });
        let opened = Engine::open(&db).unwrap();
        opened.engine.close().unwrap();
        let connection = Connection::open(&db).unwrap();
        connection.pragma_update(None, "journal_mode", "DELETE").unwrap();
        connection.pragma_update(None, "user_version", 33).unwrap();
        connection.close().unwrap();
        run_wal_child(&db, 34);
        if remove_shm {
            fs::remove_file(sidecar(&db, "-shm")).unwrap();
        }

        let recovered = Engine::open(&db).unwrap();
        assert_eq!(recovered.report.schema_version_before, 34);
        assert!(recovered.report.migration_steps.is_empty());
        recovered.engine.close().unwrap();
    }
}

#[test]
fn refused_schema_33_wal_state_restores_existing_or_missing_shm() {
    for remove_shm in [false, true] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, if remove_shm { "old-no-shm" } else { "old-shm" });
        create_versioned_sqlite(&db, 32);
        fs::write(sidecar(&db, ".lock"), b"historical-lock-bytes").unwrap();
        run_wal_child(&db, 33);
        if remove_shm {
            fs::remove_file(sidecar(&db, "-shm")).unwrap();
        }
        let before = directory_bytes(&dir);

        assert_incompatible(Engine::open(&db), 33);
        assert_eq!(directory_bytes(&dir), before);
    }
}
