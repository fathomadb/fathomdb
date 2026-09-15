use std::collections::BTreeMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use fathomdb_engine::{Engine, EngineOpenError};
use fathomdb_schema::{MIGRATIONS, SCHEMA_VERSION};
use rusqlite::Connection;
use tempfile::TempDir;

const WAL_CHILD_MODE: &str = "FATHOMDB_SLICE40_WAL_CHILD_MODE";
const WAL_CHILD_PATH: &str = "FATHOMDB_SLICE40_WAL_CHILD_PATH";
const OPEN_CHILD_MODE: &str = "FATHOMDB_SLICE40_OPEN_CHILD_MODE";
const OPEN_CHILD_PATH: &str = "FATHOMDB_SLICE40_OPEN_CHILD_PATH";
const OPEN_CHILD_SECOND_PATH: &str = "FATHOMDB_SLICE40_OPEN_CHILD_SECOND_PATH";

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

fn run_open_child(mode: &str, path: &Path, second_path: Option<&Path>) {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .arg("--exact")
        .arg("slice40_fresh_process_open_child")
        .arg("--nocapture")
        .env(OPEN_CHILD_MODE, mode)
        .env(OPEN_CHILD_PATH, path);
    if let Some(second_path) = second_path {
        command.env(OPEN_CHILD_SECOND_PATH, second_path);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "open child failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
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
fn slice40_fresh_process_open_child() {
    let Ok(mode) = std::env::var(OPEN_CHILD_MODE) else {
        return;
    };
    let path = PathBuf::from(std::env::var_os(OPEN_CHILD_PATH).unwrap());
    match mode.as_str() {
        "open-current" => {
            let opened = Engine::open(path).unwrap();
            assert_eq!(opened.report.schema_version_before, 34);
            assert_eq!(opened.report.schema_version_after, 34);
            assert!(opened.report.migration_steps.is_empty());
            opened.engine.close().unwrap();
        }
        "refuse-then-fresh" => {
            assert_incompatible(Engine::open(path), 33);
            let fresh_path = PathBuf::from(std::env::var_os(OPEN_CHILD_SECOND_PATH).unwrap());
            let opened = Engine::open(fresh_path).unwrap();
            assert_eq!(opened.report.schema_version_before, 0);
            assert_eq!(opened.report.schema_version_after, 34);
            opened.engine.close().unwrap();
        }
        other => panic!("unknown open child mode: {other}"),
    }
}

#[test]
fn fresh_process_reopens_clean_current_database() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "clean-current");
    Engine::open(&db).unwrap().engine.close().unwrap();

    run_open_child("open-current", &db, None);
}

#[test]
fn fresh_process_reopens_current_wal_with_and_without_shm() {
    for remove_shm in [false, true] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, if remove_shm { "wal-no-shm" } else { "wal-shm" });
        Engine::open(&db).unwrap().engine.close().unwrap();
        let connection = Connection::open(&db).unwrap();
        connection.pragma_update(None, "journal_mode", "DELETE").unwrap();
        connection.pragma_update(None, "user_version", 33).unwrap();
        connection.close().unwrap();
        run_wal_child(&db, 34);
        if remove_shm {
            fs::remove_file(sidecar(&db, "-shm")).unwrap();
        }

        run_open_child("open-current", &db, None);
    }
}

#[test]
fn schema_33_refusal_does_not_poison_fresh_open_in_same_process() {
    let dir = TempDir::new().unwrap();
    let old = path(&dir, "old");
    let fresh = path(&dir, "fresh-after-refusal");
    create_versioned_sqlite(&old, 33);

    run_open_child("refuse-then-fresh", &old, Some(&fresh));
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
fn schema_33_without_a_lock_sidecar_creates_only_the_persistent_lock_namespace() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "old-no-lock");
    create_versioned_sqlite(&db, 33);
    let before = directory_bytes(&dir);

    assert_incompatible(Engine::open(&db), 33);
    let mut after = directory_bytes(&dir);
    assert_eq!(after.remove("old-no-lock.sqlite.lock"), Some(Vec::new()));
    assert_eq!(after, before);
}

#[test]
fn future_and_nonempty_zero_version_databases_are_refused() {
    for (name, version) in [("future", 35), ("foreign", 0)] {
        let dir = TempDir::new().unwrap();
        let db = path(&dir, name);
        create_versioned_sqlite(&db, version);
        fs::write(sidecar(&db, ".lock"), b"historical-lock-bytes").unwrap();
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
        for include_lock in [false, true] {
            let dir = TempDir::new().unwrap();
            let name = format!(
                "old-{}-{}",
                if remove_shm { "no-shm" } else { "shm" },
                if include_lock { "lock" } else { "no-lock" }
            );
            let db = path(&dir, &name);
            create_versioned_sqlite(&db, 32);
            if include_lock {
                fs::write(sidecar(&db, ".lock"), b"historical-lock-bytes").unwrap();
            }
            run_wal_child(&db, 33);
            if remove_shm {
                fs::remove_file(sidecar(&db, "-shm")).unwrap();
            }
            let mut before = directory_bytes(&dir);

            assert_incompatible(Engine::open(&db), 33);
            let mut after = directory_bytes(&dir);
            let lock_name = format!("{name}.sqlite.lock");
            if include_lock {
                assert_eq!(after.get(&lock_name), Some(&b"historical-lock-bytes".to_vec()));
            } else {
                assert_eq!(after.remove(&lock_name), Some(Vec::new()));
                before.remove(&lock_name);
            }
            assert_eq!(after, before);
        }
    }
}

#[test]
fn locked_older_opener_cannot_be_migrated_by_the_current_opener() {
    let dir = TempDir::new().unwrap();
    let db = path(&dir, "older-winner");
    let lock_path = sidecar(&db, ".lock");
    let mut older_lock =
        OpenOptions::new().read(true).write(true).create_new(true).open(&lock_path).unwrap();
    older_lock.lock().unwrap();
    older_lock.write_all(b"4242").unwrap();
    create_versioned_sqlite(&db, 33);

    match Engine::open(&db).expect_err("the current opener must respect the older lock") {
        EngineOpenError::DatabaseLocked { holder_pid } => {
            assert_eq!(holder_pid, Some(4242));
        }
        other => panic!("expected DatabaseLocked, got {other:?}"),
    }

    drop(older_lock);
    let before = directory_bytes(&dir);
    assert_incompatible(Engine::open(&db), 33);
    assert_eq!(directory_bytes(&dir), before);
}
