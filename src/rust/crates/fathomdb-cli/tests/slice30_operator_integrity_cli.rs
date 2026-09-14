//! 0.8.26 Slice 30 RED process contract for immutable operator inspection.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use fathomdb::{
    admin, inspect_data_plane_integrity, DataPlaneIntegrityRequestV1, Engine, RuntimeSqliteMode,
};
use fathomdb_cli::exit_code;
use rusqlite::Connection;
use serde_json::Value;
use tempfile::TempDir;

const SCHEMA: &str = "fathomdb.doctor.data-plane-integrity.v1";

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut value = OsString::from(path.as_os_str());
    value.push(suffix);
    PathBuf::from(value)
}

fn file_set(path: &Path) -> BTreeMap<String, Option<Vec<u8>>> {
    ["", ".lock", "-wal", "-shm", "-journal"]
        .into_iter()
        .map(|suffix| {
            let candidate = sidecar(path, suffix);
            (suffix.to_string(), fs::read(candidate).ok())
        })
        .collect()
}

fn fresh_database(name: &str) -> (TempDir, PathBuf) {
    let directory = TempDir::new().expect("temporary database directory");
    let path = directory.path().join(name);
    let opened = Engine::open(&path).expect("create current database");
    opened.engine.close().expect("close current database");
    drop(opened);
    (directory, path)
}

fn run(path: &Path, extra: &[&str]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_fathomdb"));
    command.args(["doctor", "data-plane-integrity", "--json"]);
    command.args(extra);
    command.arg(path);
    command.output().expect("run data-plane-integrity")
}

fn payload(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "expected one JSON object: {error}; stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn assert_error(output: &Output, exit: i32, reason: &str, field_path: &str) {
    assert_eq!(output.status.code(), Some(exit), "{output:#?}");
    let value = payload(output);
    assert_eq!(value["schemaVersion"], SCHEMA);
    assert_eq!(value["status"], "error");
    assert_eq!(value["verb"], "data-plane-integrity");
    assert_eq!(value["code"], "FDB_DATA_PLANE_INTEGRITY");
    assert_eq!(value["reason"], reason);
    assert_eq!(value["fieldPath"], field_path);
    assert!(value.get("detail").is_none(), "must not disclose raw SQLite diagnostics: {value}");
}

#[test]
fn clean_inspection_reuses_compatible_runtime_and_changes_no_product_file() {
    let (_directory, path) = fresh_database("clean.sqlite");
    admin::configure_runtime(RuntimeSqliteMode::Performance).expect("compatible runtime reuse");
    let before = file_set(&path);

    let direct = inspect_data_plane_integrity(
        &path,
        DataPlaneIntegrityRequestV1::all(10_000, 100).expect("valid request"),
    )
    .expect("compatible in-process runtime inspection");
    assert!(direct.findings.is_empty());
    assert!(file_set(&path) == before, "direct inspection changed product files");

    let output = run(&path, &[]);

    assert_eq!(output.status.code(), Some(exit_code::OK), "{output:#?}");
    let value = payload(&output);
    assert_eq!(value["schemaVersion"], SCHEMA);
    assert_eq!(value["status"], "clean");
    assert!(file_set(&path) == before, "clean inspection changed product files");
}

#[test]
fn existing_database_without_product_lock_refuses_without_creation() {
    let (_directory, path) = fresh_database("missing-lock.sqlite");
    fs::remove_file(sidecar(&path, ".lock")).expect("remove lock fixture");
    let before = file_set(&path);

    let output = run(&path, &[]);

    assert_error(&output, exit_code::UNRECOVERABLE, "inspection_lock_missing", "/dbPath");
    assert!(file_set(&path) == before, "missing lock was recreated");
}

#[test]
fn persistent_shm_without_wal_is_allowed_and_unchanged() {
    let (_directory, path) = fresh_database("persistent-shm.sqlite");
    fs::write(sidecar(&path, "-shm"), b"closed-reader-pool-shm").expect("write shm fixture");
    let before = file_set(&path);

    let output = run(&path, &[]);

    assert_eq!(output.status.code(), Some(exit_code::OK), "{output:#?}");
    assert_eq!(payload(&output)["status"], "clean");
    assert!(file_set(&path) == before, "persistent shm fixture changed");
}

#[test]
fn uri_reserved_path_bytes_are_encoded_and_unchanged() {
    let (_directory, path) = fresh_database("reserved space#%3F.sqlite");
    let before = file_set(&path);

    let output = run(&path, &[]);

    assert_eq!(output.status.code(), Some(exit_code::OK), "{output:#?}");
    assert_eq!(payload(&output)["status"], "clean");
    assert!(file_set(&path) == before, "URI-reserved path fixture changed");
}

#[test]
fn held_product_lock_refuses_with_private_v1_error() {
    let directory = TempDir::new().expect("temporary database directory");
    let path = directory.path().join("held.sqlite");
    let opened = Engine::open(&path).expect("hold product lock");
    let before = file_set(&path);

    let output = run(&path, &[]);

    assert_error(&output, exit_code::LOCK_HELD, "inspection_not_quiescent", "/dbPath");
    assert!(file_set(&path) == before, "held-lock refusal changed product files");
    opened.engine.close().expect("close held engine");
}

#[test]
fn nonempty_wal_and_journal_refuse_without_alteration() {
    for suffix in ["-wal", "-journal"] {
        let (_directory, path) = fresh_database(&format!("pending{}.sqlite", &suffix[1..]));
        fs::write(sidecar(&path, suffix), b"pending-state").expect("write sidecar fixture");
        let before = file_set(&path);

        let output = run(&path, &[]);

        assert_error(&output, exit_code::LOCK_HELD, "inspection_not_quiescent", "/dbPath");
        assert!(file_set(&path) == before, "sidecar {suffix} changed");
    }
}

#[test]
fn lower_and_higher_schema_refuse_without_migration() {
    for delta in [-1_i64, 1] {
        let (_directory, path) = fresh_database(&format!("schema-{delta}.sqlite"));
        let connection = Connection::open(&path).expect("open schema fixture");
        let current: i64 =
            connection.query_row("PRAGMA user_version", [], |row| row.get(0)).unwrap();
        connection
            .pragma_update(None, "user_version", current + delta)
            .expect("stamp mismatched schema");
        drop(connection);
        let before = file_set(&path);

        let output = run(&path, &[]);

        assert_error(
            &output,
            exit_code::UNRECOVERABLE,
            "database_schema_mismatch",
            "/databaseSchemaVersion",
        );
        assert!(file_set(&path) == before, "schema delta {delta} was changed");
    }
}

#[test]
fn missing_database_and_lock_are_not_created() {
    let directory = TempDir::new().expect("temporary database directory");
    let path = directory.path().join("missing.sqlite");
    let before = file_set(&path);

    let output = run(&path, &[]);

    assert_error(&output, exit_code::UNRECOVERABLE, "inspection_unavailable", "/dbPath");
    assert!(file_set(&path) == before, "missing-path refusal created product files");
}

#[test]
fn corrupt_database_is_a_private_v1_error() {
    let (_directory, path) = fresh_database("corrupt.sqlite");
    fs::write(&path, b"not a sqlite database").expect("corrupt database fixture");
    let before = file_set(&path);

    let output = run(&path, &[]);

    assert_error(&output, exit_code::UNRECOVERABLE, "integrity_corrupt", "");
    assert!(file_set(&path) == before, "corruption refusal changed product files");
}

#[test]
fn semantic_request_errors_use_the_v1_envelope() {
    let (_directory, path) = fresh_database("invalid-request.sqlite");
    let output = run(&path, &["--max-work", "0"]);

    assert_error(&output, exit_code::UNRECOVERABLE, "integrity_limit_invalid", "/maxWorkUnits");
}

#[test]
fn findings_use_exit_65_without_raw_content() {
    let (_directory, path) = fresh_database("findings.sqlite");
    let connection = Connection::open(&path).expect("open findings fixture");
    connection
        .execute(
            "INSERT INTO search_index(rowid,body,kind,write_cursor) VALUES(900,?1,'doc',900)",
            ["raw-secret-body"],
        )
        .expect("insert orphan projection");
    drop(connection);
    let before = file_set(&path);

    let output = run(&path, &["--check", "active_searchable_orphans"]);

    assert_eq!(output.status.code(), Some(exit_code::DOCTOR_FOUND_ISSUES), "{output:#?}");
    let value = payload(&output);
    assert_eq!(value["status"], "findings");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("raw-secret-body"));
    assert!(file_set(&path) == before, "findings inspection changed product files");
}
