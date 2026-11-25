//! Slice 50 (0.8.31) — `migrate_37_to_38` engine operator contract tests.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use fathomdb_engine::{
    arm_migrate_37_to_38_before_version_stamp_hook_for_test, configure_runtime, migrate_37_to_38,
    migrate_38_to_39, Engine, EngineError, EngineOpenError, GraphExpandRequestV1,
    GraphExpansionErrorReasonV1, GraphReadContextV1, GraphSeedV1, IdSpace, MigrateErrorCode,
    ReadContextV1, ReadView, RuntimeSqliteMode, SearchFilter, TraversalDirection,
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const GOLDEN_SCHEMA_37_SHA256: &str =
    "59d56eabb9a6e817168c5c540b4bb2ec6eb7f741fb24a72c7617df4aa5004548";

fn golden_schema_37_path() -> PathBuf {
    if let Some(path) =
        std::env::var_os("FATHOMDB_GOLDEN_SCHEMA_37_PATH").filter(|value| !value.is_empty())
    {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../dev/fixtures/golden-0.8.30-schema37.sqlite")
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read file for sha256");
    Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn configure_test_runtime() {
    configure_runtime(RuntimeSqliteMode::Performance).expect("configure test runtime");
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

fn dir_file_count(path: &Path) -> usize {
    if !path.exists() {
        return 0;
    }
    fs::read_dir(path).unwrap().count()
}

#[test]
fn engine_open_still_refuses_schema_37() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden(&dir, "refuse37.sqlite");
    let err = Engine::open(&db).unwrap_err();
    assert!(
        matches!(err, EngineOpenError::IncompatibleSchemaVersion { seen: 37, supported: 39 }),
        "expected IncompatibleSchemaVersion 37/39, got {err:?}"
    );
    assert_eq!(sha256_file(&db), GOLDEN_SCHEMA_37_SHA256, "open refusal must not mutate db");
}

#[test]
fn lock_contention_fails_with_lock_held() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden(&dir, "locked.sqlite");
    let backup_dir = dir.path().join("backups");

    // Hold exclusive flock on `<db>.lock`
    let mut lock_os = db.as_os_str().to_os_string();
    lock_os.push(".lock");
    let lock_path = PathBuf::from(lock_os);
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock_path)
        .unwrap();
    lock_file.try_lock().unwrap();

    let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::LockHeld);
    assert_eq!(dir_file_count(&backup_dir), 0);

    drop(lock_file);
}

#[test]
fn wrong_user_version_36_38_39_fails_closed_without_backup_or_mutation() {
    let dir = TempDir::new().unwrap();
    let backup_dir = dir.path().join("backups");

    // 1. True Schema 38 fixture (golden 37 upgraded via migrate_37_to_38) -> AlreadyCurrent
    let db38 = copy_golden(&dir, "v38.sqlite");
    let setup_backups = dir.path().join("setup_backups");
    let r38 = migrate_37_to_38(&db38, &setup_backups).unwrap();
    assert_eq!(r38.to_version, 38);
    let err38 = migrate_37_to_38(&db38, &backup_dir).unwrap_err();
    assert_eq!(err38.code, MigrateErrorCode::AlreadyCurrent);
    assert_eq!(err38.seen_version, Some(38));
    assert_eq!(dir_file_count(&backup_dir), 0);

    // 2. Schema 36 & 39 -> UnsupportedVersion
    for version in [36u32, 39u32] {
        let db = copy_golden(&dir, &format!("v{version}.sqlite"));
        {
            let conn = Connection::open(&db).unwrap();
            conn.pragma_update(None, "user_version", version).unwrap();
        }
        let before = sha256_file(&db);
        let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
        assert_eq!(err.code, MigrateErrorCode::UnsupportedVersion);
        assert_eq!(err.seen_version, Some(version));
        assert_eq!(sha256_file(&db), before);
        assert_eq!(dir_file_count(&backup_dir), 0);
    }
}

#[test]
fn unfinished_erasure_gates_all_four_fail_closed_before_backup() {
    let dir = TempDir::new().unwrap();
    let backup_dir = dir.path().join("backups");

    // Gate 1: pending dependency closure (`at_rest_pending` physical closure)
    {
        let db = copy_golden(&dir, "debt_closure.sqlite");
        let conn = Connection::open(&db).unwrap();
        let hex64 = "a".repeat(64);
        let closure_id = format!("_fdb:c:{hex64}");
        let proof_json = serde_json::json!({
            "schema_version": 1,
            "proof_write_boundary": 11,
            "current_active_dependent_nodes": 0,
            "current_derived_edges": 0,
            "view_eligible_dependents": 0,
            "ownerless_projection_rows": 0,
            "post_admission_registrations": 0,
            "remaining_dependency_rows": 0,
            "remaining_canonical_rows": 0,
            "remaining_projection_rows": 0,
            "remaining_receipt_reference_rows": 0
        })
        .to_string();
        conn.execute(
            "INSERT OR REPLACE INTO _fathomdb_open_state(key, value)
             VALUES ('_fathomdb_closure_sequence', '1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO _fathomdb_dependency_closures(
                schema_version, closure_operation_id, root_kind, root_value, cause,
                effective_at_epoch_s, admitted_write_boundary, admitted_dependency_generation,
                closure_sequence, retry_fingerprint, phase, affected_count, blocker_code,
                structural_proof_write_boundary, proof_json
             ) VALUES (
                1, ?1, 'source_bucket', 'src-erase-1', 'source_erased',
                1700000000, 11, 0, 1, ?2, 'at_rest_pending', 1, NULL, 11, ?3
             )",
            rusqlite::params![closure_id, hex64, proof_json],
        )
        .unwrap();
        drop(conn);
        let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
        assert_eq!(err.code, MigrateErrorCode::PendingErasure);
        assert_eq!(dir_file_count(&backup_dir), 0);
    }

    // Gate 2: active/queued _fathomdb_erasure_requests (all 4 phases)
    for phase in ["queued", "executing", "sanitizing", "finalizing"] {
        let db = copy_golden(&dir, &format!("debt_req_{phase}.sqlite"));
        let conn = Connection::open(&db).unwrap();
        let store_id: String = conn
            .query_row(
                "SELECT value FROM _fathomdb_open_state WHERE key = '_fathomdb_erasure_store'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let key = "1".repeat(32);
        let ticket = format!("{store_id}:{key}");
        let is_queued = phase == "queued";
        let is_finalizing = phase == "finalizing";
        let payload = serde_json::json!({
            "version": 1,
            "metadata_budget": 1024,
            "ticket": ticket,
            "key": key,
            "selector": if is_finalizing {
                serde_json::Value::Null
            } else {
                serde_json::json!({ "kind": "source_all_at_cut", "source_id": "src-keep-1" })
            },
            "cut": if is_finalizing { 0 } else { 11 },
            "scope": { "cursors": [], "sources": [], "identities": [] },
            "originals": [],
            "status": {
                "phase": phase,
                "logical_state": if is_queued { "unchanged_by_this_request" } else { "removed" },
                "blocker": null,
                "service_state": "normal_with_queued_restrictions",
                "target_count": 0
            }
        })
        .to_string();
        conn.execute(
            "INSERT INTO _fathomdb_erasure_requests(ticket, idempotency_key, payload)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![ticket, key, payload],
        )
        .unwrap();
        if !is_queued {
            // Active phases require matching physical root & FTS sanitation records
            let fts_json = if is_finalizing {
                r#"{"version":1,"phase":"finalizing","family":4,"started":false}"#
            } else {
                r#"{"version":1,"phase":"cleaning","family":0,"started":true}"#
            };
            conn.execute(
                "INSERT OR REPLACE INTO _fathomdb_open_state(key, value)
                 VALUES ('_fathomdb_fts_sanitation', ?1)",
                [fts_json],
            )
            .unwrap();
            let root_json = serde_json::json!({
                "ticket": ticket,
                "version": 2,
                "retry": if is_finalizing { String::new() } else { "0".repeat(64) },
                "cursors": [],
                "vectors": [],
                "boundary": 11,
                "generation": 0,
                "closures": [],
                "safe_boundary": 11,
                "denied_sources": if is_finalizing { Vec::<String>::new() } else { vec!["a".repeat(64)] },
                "denied_identities": [],
                "redacted": is_finalizing,
                "blocker": null
            })
            .to_string();
            conn.execute(
                "INSERT OR REPLACE INTO _fathomdb_open_state(key, value)
                 VALUES ('_fathomdb_physical_erasure', ?1)",
                [root_json],
            )
            .unwrap();
        }
        drop(conn);
        let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
        assert_eq!(err.code, MigrateErrorCode::PendingErasure, "phase={phase}, err={err:?}");
        assert_eq!(dir_file_count(&backup_dir), 0);
    }

    // Gate 3: physical erasure root/FTS debt in _fathomdb_open_state
    {
        let db = copy_golden(&dir, "debt_physical.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO _fathomdb_open_state(key, value)
             VALUES ('_fathomdb_fts_sanitation', '{\"version\":1,\"phase\":\"cleaning\",\"family\":0,\"started\":true}')",
            [],
        )
        .unwrap();
        drop(conn);
        let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
        assert_eq!(err.code, MigrateErrorCode::PendingErasure);
        assert_eq!(dir_file_count(&backup_dir), 0);
    }

    // Gate 4: ERASURE_PENDING_REDACTION_COLLECTION in operational_mutations
    {
        let db = copy_golden(&dir, "debt_redaction.sqlite");
        let conn = Connection::open(&db).unwrap();
        conn.execute(
            "INSERT INTO operational_mutations(
                collection_name, record_key, op_kind, payload_json, schema_id, write_cursor
             ) VALUES ('erasure_pending_redaction', 'r1', 'append', '{}', NULL, 99)",
            [],
        )
        .unwrap();
        drop(conn);
        let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
        assert_eq!(err.code, MigrateErrorCode::PendingErasure);
        assert_eq!(dir_file_count(&backup_dir), 0);
    }
}

#[test]
fn dangling_edge_endpoints_fail_closed_and_superseded_endpoints_are_admitted() {
    let dir = TempDir::new().unwrap();

    // 1. Genuinely dangling endpoint -> fail closed BEFORE backup/DDL
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
    let err = migrate_37_to_38(&dangling_db, &dangling_backup).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::DanglingEdges);
    let preflight = err.preflight.expect("preflight counts must be present");
    assert_eq!(preflight.dangling_edge_endpoints, 1);
    assert_eq!(preflight.superseded_edge_endpoints, 0);
    assert_eq!(dir_file_count(&dangling_backup), 0, "must fail before creating backup");

    // 2. Superseded-only target endpoint (dangling == 0, superseded == 1) -> admitted!
    let sup_db = copy_golden(&dir, "superseded_ok.sqlite");
    let sup_backup = dir.path().join("backups_sup");
    {
        let conn = Connection::open(&sup_db).unwrap();
        // Mark the currently active `req:1` revision (cursor 6) as superseded so active edge
        // `ver:1 -> req:1` (cursor 10) points to a logical_id that exists in canonical_nodes
        // ONLY via superseded revisions (without advancing max write_cursor past projection head).
        conn.execute(
            "UPDATE canonical_nodes SET superseded_at = 7 WHERE logical_id = 'req:1' AND superseded_at IS NULL",
            [],
        )
        .unwrap();
    }
    let report = migrate_37_to_38(&sup_db, &sup_backup).expect("superseded-only endpoint admitted");
    assert_eq!(report.preflight.dangling_edge_endpoints, 0);
    assert_eq!(report.preflight.superseded_edge_endpoints, 1);
    assert!(report.integrity_clean());
    let report39 = migrate_38_to_39(&sup_db, &sup_backup)
        .expect("chain 38 -> 39 before Schema 39 Engine::open");
    assert!(report39.integrity_clean());
    let opened = Engine::open(&sup_db).expect("upgraded db opens cleanly");
    opened.engine.close().unwrap();
}

#[test]
fn mid_ddl_rollback_leaves_schema_37_byte_identical_and_backup_intact() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden(&dir, "rollback.sqlite");
    let backup_dir = dir.path().join("backups");

    arm_migrate_37_to_38_before_version_stamp_hook_for_test(&db, || {
        Err("injected mid-DDL failure before user_version = 38".to_string())
    });

    let err = migrate_37_to_38(&db, &backup_dir).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::RollbackFailedOrDdlAborted);

    // Source DB must be byte-identical to pre-migrate Schema 37
    assert_eq!(
        sha256_file(&db),
        GOLDEN_SCHEMA_37_SHA256,
        "rolled-back source DB must stay byte-identical to Schema 37"
    );

    // Verify zero partial Schema-38 objects exist
    let conn = Connection::open(&db).unwrap();
    let version: u32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
    assert_eq!(version, 37);
    let schema38_objects: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE name IN (
                 'canonical_property_values',
                 '_fathomdb_commit_timestamps',
                 'canonical_nodes_logical_sup_idx',
                 'canonical_edges_from_active_idx',
                 'canonical_edges_to_active_idx',
                 'canonical_edges_from_sup_kind_idx',
                 'canonical_edges_to_sup_kind_idx'
             )",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(schema38_objects, 0);
    drop(conn);

    // Verified backup must remain intact on disk
    let backup_path = err.backup_path.expect("backup path preserved on DDL rollback");
    assert!(backup_path.is_file());
    assert_eq!(sha256_file(&backup_path), GOLDEN_SCHEMA_37_SHA256);
    assert_eq!(err.backup_sha256.as_deref(), Some(GOLDEN_SCHEMA_37_SHA256));
}

#[test]
fn fresh_vs_migrated_sqlite_master_structural_parity() {
    let dir = TempDir::new().unwrap();
    let fresh_db = dir.path().join("fresh38.sqlite");
    {
        let engine = Engine::open(&fresh_db).unwrap().engine;
        engine.close().unwrap();
    }
    let migrated_db = copy_golden(&dir, "migrated38.sqlite");
    let backup_dir = dir.path().join("backups");
    let report = migrate_37_to_38(&migrated_db, &backup_dir).unwrap();
    assert!(report.integrity_clean());
    let report39 = migrate_38_to_39(&migrated_db, &backup_dir).unwrap();
    assert!(report39.integrity_clean());

    let normalize = |sql: &str| -> String { sql.split_whitespace().collect::<Vec<_>>().join(" ") };

    let collect_objects = |path: &Path| -> BTreeMap<(String, String), String> {
        let conn = Connection::open(path).unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT type, name, sql FROM sqlite_master
                 WHERE name NOT LIKE 'sqlite_%'
                   AND name NOT LIKE 'vector_default%'
                   AND name NOT LIKE 'search_index%'
                   AND name NOT LIKE 'property_search_index%'
                   AND sql IS NOT NULL
                 ORDER BY type, name",
            )
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
                normalize(&row.get::<_, String>(2)?),
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect()
    };

    let fresh_objects = collect_objects(&fresh_db);
    let migrated_objects = collect_objects(&migrated_db);

    for (key, fresh_sql) in &fresh_objects {
        let migrated_sql = migrated_objects
            .get(key)
            .unwrap_or_else(|| panic!("migrated DB missing schema object {key:?}"));
        assert_eq!(
            migrated_sql, fresh_sql,
            "DDL mismatch for {key:?}:\n fresh:    {fresh_sql}\n migrated: {migrated_sql}"
        );
    }

    // Explicitly assert key Schema 38 objects exist
    for expected in [
        ("table", "canonical_property_values"),
        ("index", "canonical_property_values_text_idx"),
        ("index", "canonical_property_values_num_idx"),
        ("index", "canonical_property_values_bool_idx"),
        ("table", "_fathomdb_commit_timestamps"),
        ("index", "_fathomdb_commit_timestamps_ms_idx"),
        ("index", "canonical_nodes_scope_logical_sup_idx"),
        ("index", "canonical_edges_scope_from_active_idx"),
        ("index", "canonical_edges_scope_to_active_idx"),
        ("index", "canonical_edges_scope_from_sup_kind_idx"),
        ("index", "canonical_edges_scope_to_sup_kind_idx"),
        ("trigger", "_fathomdb_read_visibility_cpv_ai"),
        ("trigger", "_fathomdb_read_visibility_cpv_au"),
        ("trigger", "_fathomdb_read_visibility_cpv_ad"),
    ] {
        assert!(
            migrated_objects.contains_key(&(expected.0.to_string(), expected.1.to_string())),
            "missing expected Schema 38 object {expected:?}"
        );
    }
    assert!(
        migrated_objects
            .get(&("table".to_string(), "canonical_property_values".to_string()))
            .is_some_and(|sql| sql.contains("WITHOUT ROWID")),
        "canonical_property_values must be a WITHOUT ROWID table"
    );
    assert!(
        !migrated_objects.contains_key(&(
            "index".to_string(),
            "canonical_property_values_cursor_idx".to_string()
        )),
        "redundant canonical_property_values_cursor_idx must not exist"
    );
}

#[test]
fn golden_schema_37_upgrade_end_to_end() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden(&dir, "golden_upgrade.sqlite");
    let backup_dir = dir.path().join("backups");

    let report = migrate_37_to_38(&db, &backup_dir).expect("golden upgrade must succeed");
    assert_eq!(report.from_version, 37);
    assert_eq!(report.to_version, 38);
    assert_eq!(report.backup_sha256, GOLDEN_SCHEMA_37_SHA256);
    assert_eq!(sha256_file(&report.backup_path), GOLDEN_SCHEMA_37_SHA256);
    assert!(report.backup_sha256_path.is_file());
    assert!(
        report.integrity_clean(),
        "post-upgrade integrity must be clean: {:?}",
        report.integrity
    );

    let report39 =
        migrate_38_to_39(&db, &backup_dir).expect("chain 38 -> 39 before Schema 39 Engine::open");
    assert_eq!(report39.from_version, 38);
    assert_eq!(report39.to_version, 39);
    assert!(report39.integrity_clean());

    // Open migrated DB with Engine::open (Schema 39 head)
    let opened = Engine::open(&db).expect("Engine::open must succeed on migrated Schema 39 DB");
    assert_eq!(opened.report.schema_version_before, 39);
    assert_eq!(opened.report.schema_version_after, 39);

    // Existing live nodes present with default empty properties
    let req1 =
        opened.engine.read_get("req:1", &ReadView::default()).unwrap().expect("req:1 exists");
    assert_eq!(req1.properties, None);
    let ver1 =
        opened.engine.read_get("ver:1", &ReadView::default()).unwrap().expect("ver:1 exists");
    assert_eq!(ver1.properties, None);

    // Pre-upgrade erased source `src-erase-1` must NOT be resurrected
    let all_notes = opened.engine.read_list("note", &[], 100, &ReadView::default()).unwrap();
    assert_eq!(all_notes.len(), 2, "only req:1 and ver:1 remain active (erased row absent)");
    {
        let conn = Connection::open(&db).unwrap();
        let erased_rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM canonical_nodes WHERE source_id = 'src-erase-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(erased_rows, 0, "pre-upgrade erased source must not resurrect");
    }

    // Pre-upgrade `as_of_epoch_ms` strictly prior to any Schema-38 commit resolves cleanly to cursor 0 / empty
    let live_ctx = GraphReadContextV1::Current {
        schema_version: 1,
        context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
    };
    let query_empty = opened
        .engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Query { schema_version: 1, text: "req".into(), ranked_limit: 5 },
                TraversalDirection::Outgoing,
                live_ctx.clone(),
                1,
                50,
                10_000,
            )
            .with_as_of_epoch_ms(1_700_000_000_000),
        )
        .unwrap();
    assert!(
        query_empty.targets.is_empty(),
        "as_of_epoch_ms before first Schema-38 commit must resolve to cursor 0 / empty"
    );
    let explicit_empty = opened
        .engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![IdSpace::logical("ver:1")],
                },
                TraversalDirection::Outgoing,
                live_ctx,
                1,
                50,
                10_000,
            )
            .with_as_of_epoch_ms(1_700_000_000_000),
        )
        .unwrap_err();
    assert!(
        matches!(
            explicit_empty,
            EngineError::GraphExpansion(ref ge)
                if ge.reason == GraphExpansionErrorReasonV1::GraphSeedUnavailable
        ),
        "explicit seed at pre-upgrade as_of_epoch_ms must see empty cursor-0 snapshot, got {explicit_empty:?}"
    );

    opened.engine.close().unwrap();
}
