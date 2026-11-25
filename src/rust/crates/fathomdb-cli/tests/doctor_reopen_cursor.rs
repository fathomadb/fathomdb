//! 0.8.32 Slice 40 (`WP-40A1`, Bug 1) — CLI operator test verifying
//! `fathomdb doctor data-plane-integrity --json` exits `0` (`0` findings) on an
//! empty DB with `AdminSchema` / `OpStore` terminal-only cursors.

use std::process::Command;

use fathomdb::{Engine, PreparedWrite};
use fathomdb_cli::exit_code;
use serde_json::Value;
use tempfile::TempDir;

#[test]
fn doctor_data_plane_integrity_accepts_admin_schema_and_opstore_terminal_cursors() {
    let dir = TempDir::new().expect("tempdir");
    let db = dir.path().join("admin_and_opstore_only.sqlite");

    {
        let opened = Engine::open(&db).expect("open");
        opened
            .engine
            .write(&[PreparedWrite::AdminSchema {
                name: "checkpoints".to_string(),
                kind: "latest_state".to_string(),
                schema_json: r#"{"type":"object"}"#.to_string(),
                retention_json: "{}".to_string(),
            }])
            .expect("register latest_state collection");
        opened
            .engine
            .write(&[PreparedWrite::AdminSchema {
                name: "audit_log".to_string(),
                kind: "append_only_log".to_string(),
                schema_json: r#"{"type":"object"}"#.to_string(),
                retention_json: "{}".to_string(),
            }])
            .expect("register append_only_log collection");
        opened
            .engine
            .write(&[PreparedWrite::OpStore {
                collection: "checkpoints".to_string(),
                record_key: "cp-1".to_string(),
                schema_id: None,
                body: r#"{"step":1}"#.to_string(),
            }])
            .expect("write latest_state");
        opened
            .engine
            .write(&[PreparedWrite::OpStore {
                collection: "audit_log".to_string(),
                record_key: "ev-1".to_string(),
                schema_id: None,
                body: r#"{"event":"boot"}"#.to_string(),
            }])
            .expect("write append_only_log");
        opened.engine.close().expect("close");
    }

    let output = Command::new(env!("CARGO_BIN_EXE_fathomdb"))
        .args(["doctor", "data-plane-integrity", "--json", db.to_str().unwrap()])
        .output()
        .expect("run fathomdb doctor data-plane-integrity");
    assert_eq!(
        output.status.code(),
        Some(exit_code::OK),
        "expected exit 0, got {:?}; stdout={}, stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_slice(&output.stdout).expect("parse json");
    assert_eq!(value["status"], "clean", "unexpected doctor report: {value:#}");
    assert_eq!(
        value["report"]["findings"].as_array().map(|a| a.len()),
        Some(0),
        "expected 0 findings: {value:#}"
    );
}
