//! Slice 55 CLI contract for the separate operator integrity verb.

use std::process::Command;

use fathomdb::{Engine, IntegrityReport};
use fathomdb_cli::exit_code;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn slice55_existing_operator_contracts_are_unchanged() {
    let _legacy: Option<IntegrityReport> = None;
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("legacy.sqlite");
    Engine::open(&db).unwrap().engine.close().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fathomdb"))
        .args(["doctor", "check-integrity", "--json", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(output.status.success());
}

#[test]
fn slice55_data_plane_integrity_cli() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("integrity.sqlite");
    Engine::open(&db).unwrap().engine.close().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fathomdb"))
        .args(["doctor", "data-plane-integrity", "--json", db.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(exit_code::OK));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schemaVersion"], "fathomdb.doctor.data-plane-integrity.v1");
    assert_eq!(value["status"], "clean");
}

#[test]
fn slice55_data_plane_integrity_cli_indexes_invalid_checks() {
    let dir = TempDir::new().unwrap();
    let db = dir.path().join("invalid-check.sqlite");
    Engine::open(&db).unwrap().engine.close().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_fathomdb"))
        .args([
            "doctor",
            "data-plane-integrity",
            "--json",
            "--check",
            "dependency_chain",
            "--check",
            "not_a_check",
            db.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(exit_code::UNRECOVERABLE));
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["reason"], "integrity_check_invalid");
    assert_eq!(value["fieldPath"], "/checks/1");
}
