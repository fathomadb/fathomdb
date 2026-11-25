//! End-to-end binary execution tests for `fathomdb migrate` (Schema 37/38 -> 39).

use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::Command;

use fathomdb::Engine;
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const GOLDEN_SCHEMA_37_SHA256: &str =
    "59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548";
const GOLDEN_SCHEMA_38_SHA256: &str =
    "e0a078bde37ec3e973bba8b03c9bfb7c42505b16da1b038ca946fb08332c8036";

fn golden_schema_37_path() -> PathBuf {
    if let Some(path) =
        std::env::var_os("FATHOMDB_GOLDEN_SCHEMA_37_PATH").filter(|value| !value.is_empty())
    {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../dev/fixtures/golden-0.8.30-schema37.sqlite")
}

fn golden_schema_38_path() -> PathBuf {
    if let Some(path) =
        std::env::var_os("FATHOMDB_GOLDEN_SCHEMA_38_PATH").filter(|value| !value.is_empty())
    {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../dev/fixtures/golden-0.8.31-schema38.sqlite")
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file for sha256");
    Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn configure_test_runtime() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance)
        .expect("configure test runtime");
}

fn copy_golden(dir: &TempDir, name: &str) -> PathBuf {
    configure_test_runtime();
    let dst = dir.path().join(name);
    let src = golden_schema_37_path();
    fs::copy(&src, &dst).unwrap_or_else(|err| {
        panic!("copy golden schema-37 fixture from {}: {err}", src.display())
    });
    assert_eq!(sha256_file(&dst), GOLDEN_SCHEMA_37_SHA256, "golden fixture checksum mismatch");
    dst
}

fn copy_golden_38(dir: &TempDir, name: &str) -> PathBuf {
    configure_test_runtime();
    let dst = dir.path().join(name);
    let src = golden_schema_38_path();
    fs::copy(&src, &dst).unwrap_or_else(|err| {
        panic!("copy golden schema-38 fixture from {}: {err}", src.display())
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&dst, fs::Permissions::from_mode(0o644)).unwrap();
    }
    assert_eq!(sha256_file(&dst), GOLDEN_SCHEMA_38_SHA256, "golden schema-38 checksum mismatch");
    dst
}

fn dir_file_count(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }
    fs::read_dir(path).unwrap().count()
}

fn run_cli_raw(args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_fathomdb"))
        .args(args)
        .output()
        .expect("spawn fathomdb binary");
    (
        output.status.code().expect("normal exit code"),
        String::from_utf8(output.stdout).expect("utf8 stdout"),
        String::from_utf8(output.stderr).expect("utf8 stderr"),
    )
}

fn run_cli_json(args: &[&str]) -> (i32, Value) {
    let (code, stdout, stderr) = run_cli_raw(args);
    let value: Value = serde_json::from_str(&stdout).unwrap_or_else(|err| {
        panic!("invalid JSON stdout: {err}\nstdout={stdout}\nstderr={stderr}")
    });
    (code, value)
}

#[test]
fn migrate_lock_held_exits_71_with_json_error_and_no_backup() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden(&dir, "locked.sqlite");
    let backup_dir = dir.path().join("backups");

    let mut lock_os = db.as_os_str().to_os_string();
    lock_os.push(".lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(PathBuf::from(lock_os))
        .unwrap();
    lock_file.try_lock().unwrap();

    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        backup_dir.to_str().unwrap(),
        "--json",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, 71, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "error");
    assert_eq!(body["code"], "E_MIGRATE_LOCK_HELD");
    assert!(body["message"].as_str().is_some_and(|m| !m.is_empty()));
    assert_eq!(dir_file_count(&backup_dir), 0);
    assert_eq!(sha256_file(&db), GOLDEN_SCHEMA_37_SHA256);

    drop(lock_file);
}

#[test]
fn migrate_wrong_versions_36_39_40_exit_70_untouched() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let backup_dir = dir.path().join("backups");

    // Version 39 (fresh) -> AlreadyCurrent (exit 70)
    let db39 = dir.path().join("v39.sqlite");
    {
        let opened = Engine::open(&db39).unwrap();
        opened.engine.close().unwrap();
    }
    let before39 = sha256_file(&db39);
    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        backup_dir.to_str().unwrap(),
        "--json",
        db39.to_str().unwrap(),
    ]);
    assert_eq!(code, 70, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "error");
    assert_eq!(body["code"], "E_MIGRATE_ALREADY_CURRENT");
    assert_eq!(sha256_file(&db39), before39);
    assert_eq!(dir_file_count(&backup_dir), 0);

    // Versions 36 and 40 -> UnsupportedVersion (exit 70)
    for version in [36u32, 40u32] {
        let db = copy_golden(&dir, &format!("v{version}.sqlite"));
        {
            let conn = Connection::open(&db).unwrap();
            conn.pragma_update(None, "user_version", version).unwrap();
        }
        let before = sha256_file(&db);
        let (code, body) = run_cli_json(&[
            "migrate",
            "--backup-dir",
            backup_dir.to_str().unwrap(),
            "--json",
            db.to_str().unwrap(),
        ]);
        assert_eq!(code, 70, "version={version}: {body}");
        assert_eq!(body["verb"], "migrate");
        assert_eq!(body["status"], "error");
        assert_eq!(body["code"], "E_MIGRATE_UNSUPPORTED_VERSION");
        assert_eq!(sha256_file(&db), before);
        assert_eq!(dir_file_count(&backup_dir), 0);
    }
}

#[test]
fn migrate_pending_erasure_exits_70_without_backup_or_mutation() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden(&dir, "pending_erasure.sqlite");
    let backup_dir = dir.path().join("backups");

    {
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "INSERT INTO operational_mutations(
                collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
             ) VALUES ('erasure_pending_redaction', 'r1', 'append', '{}', NULL, 99)",
            [],
        )
        .unwrap();
    }
    let before = sha256_file(&db);

    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        backup_dir.to_str().unwrap(),
        "--json",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, 70, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "error");
    assert_eq!(body["code"], "E_MIGRATE_PENDING_ERASURE");
    assert_eq!(sha256_file(&db), before);
    assert_eq!(dir_file_count(&backup_dir), 0);
}

#[test]
fn migrate_dangling_edges_fail_closed_70_and_superseded_only_admitted_0() {
    let dir = TempDir::new().unwrap();

    // 1. Dangling edge endpoint -> exit 70, E_MIGRATE_DANGLING_EDGES, no backup, DB untouched
    let dangling_db = copy_golden(&dir, "dangling.sqlite");
    let dangling_backup = dir.path().join("backups_dangling");
    {
        let conn = Connection::open(&dangling_db).unwrap();
        conn.execute(
            "INSERT INTO canonical_edges(write_cursor, kind, from_id, to_id, source_id)
             VALUES (50, 'relates', 'ver:1', 'missing:node', 'src-keep-1')",
            [],
        )
        .unwrap();
    }
    let before_dangling = sha256_file(&dangling_db);

    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        dangling_backup.to_str().unwrap(),
        "--json",
        dangling_db.to_str().unwrap(),
    ]);
    assert_eq!(code, 70, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "error");
    assert_eq!(body["code"], "E_MIGRATE_DANGLING_EDGES");
    assert_eq!(body["preflight"]["dangling_edge_endpoints"], 1);
    assert_eq!(body["preflight"]["superseded_edge_endpoints"], 0);
    assert_eq!(dir_file_count(&dangling_backup), 0);
    assert_eq!(sha256_file(&dangling_db), before_dangling);

    // 2. Superseded-only target endpoint (dangling == 0, superseded == 1) -> exit 0
    let sup_db = copy_golden(&dir, "superseded.sqlite");
    let sup_backup = dir.path().join("backups_superseded");
    {
        let conn = Connection::open(&sup_db).unwrap();
        conn.execute(
            "DELETE FROM canonical_attributes
             WHERE write_cursor IN (
                 SELECT write_cursor FROM canonical_nodes
                 WHERE logical_id = 'req:1' AND superseded_at IS NULL
             )",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE canonical_nodes SET superseded_at = 7 WHERE logical_id = 'req:1' AND superseded_at IS NULL",
            [],
        )
        .unwrap();
    }
    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        sup_backup.to_str().unwrap(),
        "--json",
        sup_db.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["from_version"], 37);
    assert_eq!(body["to_version"], 39);
    assert_eq!(body["preflight"]["dangling_edge_endpoints"], 0);
    assert_eq!(body["preflight"]["superseded_edge_endpoints"], 1);
    assert_eq!(body["integrity"]["status"], "clean");
    assert_eq!(body["integrity"]["full"], true);

    let (doc_code, doc_body) =
        run_cli_json(&["doctor", "check-integrity", "--full", "--json", sup_db.to_str().unwrap()]);
    assert_eq!(doc_code, 0, "{doc_body}");
}

#[test]
fn migrate_golden_schema_37_db_json_and_human_and_full_integrity_check() {
    let dir = TempDir::new().unwrap();

    // JSON mode on golden copy (37 -> 38 -> 39 preserving original schema-37 backup)
    let db = copy_golden(&dir, "golden_json.sqlite");
    let backup_dir = dir.path().join("backups_json");
    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        backup_dir.to_str().unwrap(),
        "--json",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["from_version"], 37);
    assert_eq!(body["to_version"], 39);
    assert_eq!(body["backup_sha256"], GOLDEN_SCHEMA_37_SHA256);
    assert_eq!(body["preflight"]["dangling_edge_endpoints"], 0);
    assert_eq!(body["preflight"]["superseded_edge_endpoints"], 0);
    assert_eq!(body["integrity"]["status"], "clean");
    assert_eq!(body["integrity"]["full"], true);

    let backup_path = PathBuf::from(body["backup_path"].as_str().expect("backup_path string"));
    assert!(backup_path.is_file());
    assert_eq!(sha256_file(&backup_path), GOLDEN_SCHEMA_37_SHA256);

    // Follow-up doctor check-integrity --full exits 0
    let (doc_code, doc_body) =
        run_cli_json(&["doctor", "check-integrity", "--full", "--json", db.to_str().unwrap()]);
    assert_eq!(doc_code, 0, "{doc_body}");
    assert_eq!(doc_body["verb"], "check-integrity");
    assert_eq!(doc_body["physical"]["status"], "clean");
    assert_eq!(doc_body["logical"]["status"], "clean");
    assert_eq!(doc_body["semantic"]["status"], "clean");

    // Verify `_fathomdb_projection_registry` upgraded cleanly to PRIMARY KEY(entity_class, name).
    {
        let conn = Connection::open(&db).unwrap();
        let mut pk_cols: Vec<(i64, String)> = conn
            .prepare("PRAGMA table_info(_fathomdb_projection_registry)")
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, i64>(5)?, r.get::<_, String>(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .into_iter()
            .filter(|(pk, _)| *pk > 0)
            .collect();
        pk_cols.sort_by_key(|(pk, _)| *pk);
        assert_eq!(pk_cols, vec![(1, "entity_class".to_string()), (2, "name".to_string())]);
    }

    // Human text mode on a second golden copy
    let db_human = copy_golden(&dir, "golden_human.sqlite");
    let backup_human = dir.path().join("backups_human");
    let (human_code, stdout, stderr) = run_cli_raw(&[
        "migrate",
        "--backup-dir",
        backup_human.to_str().unwrap(),
        db_human.to_str().unwrap(),
    ]);
    assert_eq!(human_code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("37") && stdout.contains("39"), "stdout={stdout}");
    assert!(stdout.contains(GOLDEN_SCHEMA_37_SHA256), "stdout={stdout}");
}

#[test]
fn migrate_golden_schema_38_db_json_and_human_and_full_integrity_check() {
    let dir = TempDir::new().unwrap();

    // JSON mode on golden Schema-38 copy (38 -> 39)
    let db = copy_golden_38(&dir, "golden38_json.sqlite");
    let backup_dir = dir.path().join("backups_38_json");
    let (code, body) = run_cli_json(&[
        "migrate",
        "--backup-dir",
        backup_dir.to_str().unwrap(),
        "--json",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{body}");
    assert_eq!(body["verb"], "migrate");
    assert_eq!(body["status"], "ok");
    assert_eq!(body["from_version"], 38);
    assert_eq!(body["to_version"], 39);
    assert_eq!(body["backup_sha256"], GOLDEN_SCHEMA_38_SHA256);
    assert_eq!(body["preflight"]["dangling_edge_endpoints"], 0);
    assert_eq!(body["preflight"]["superseded_edge_endpoints"], 0);
    assert_eq!(body["integrity"]["status"], "clean");
    assert_eq!(body["integrity"]["full"], true);

    let backup_path = PathBuf::from(body["backup_path"].as_str().expect("backup_path string"));
    assert!(backup_path.is_file());
    assert_eq!(sha256_file(&backup_path), GOLDEN_SCHEMA_38_SHA256);

    // Follow-up doctor check-integrity --full exits 0
    let (doc_code, doc_body) =
        run_cli_json(&["doctor", "check-integrity", "--full", "--json", db.to_str().unwrap()]);
    assert_eq!(doc_code, 0, "{doc_body}");
    assert_eq!(doc_body["verb"], "check-integrity");
    assert_eq!(doc_body["physical"]["status"], "clean");
    assert_eq!(doc_body["logical"]["status"], "clean");
    assert_eq!(doc_body["semantic"]["status"], "clean");

    // Verify `_fathomdb_projection_registry` upgraded cleanly to PRIMARY KEY(entity_class, name).
    {
        let conn = Connection::open(&db).unwrap();
        let mut pk_cols: Vec<(i64, String)> = conn
            .prepare("PRAGMA table_info(_fathomdb_projection_registry)")
            .unwrap()
            .query_map([], |r| Ok((r.get::<_, i64>(5)?, r.get::<_, String>(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
            .into_iter()
            .filter(|(pk, _)| *pk > 0)
            .collect();
        pk_cols.sort_by_key(|(pk, _)| *pk);
        assert_eq!(pk_cols, vec![(1, "entity_class".to_string()), (2, "name".to_string())]);
    }

    // Human text mode on a second golden Schema-38 copy
    let db_human = copy_golden_38(&dir, "golden38_human.sqlite");
    let backup_human = dir.path().join("backups_38_human");
    let (human_code, stdout, stderr) = run_cli_raw(&[
        "migrate",
        "--backup-dir",
        backup_human.to_str().unwrap(),
        db_human.to_str().unwrap(),
    ]);
    assert_eq!(human_code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("38") && stdout.contains("39"), "stdout={stdout}");
    assert!(stdout.contains(GOLDEN_SCHEMA_38_SHA256), "stdout={stdout}");
}
