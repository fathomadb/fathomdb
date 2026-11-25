//! Slice 20 WP-20B — `migrate_38_to_39` engine operator contract tests.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, OpenOptions};
use std::path::{Path, PathBuf};

use fathomdb_engine::{
    arm_migrate_38_to_39_before_version_stamp_hook_for_test, configure_runtime,
    inspect_data_plane_integrity, migrate_38_to_39, CheckIntegrityOpts, DataPlaneIntegrityCheckV1,
    DataPlaneIntegrityRequestV1, EmbedderChoice, Engine, EngineConfig, MigrateErrorCode,
    RuntimeSqliteMode, Section,
};
use rusqlite::{params, Connection};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const GOLDEN_SCHEMA_38_SHA256: &str =
    "e0a078bde37ec3e973bba8b03c9bfb7c42505b16da1b038ca946fb08332c8036";

fn golden_schema_38_path() -> PathBuf {
    if let Some(path) = std::env::var_os("FATHOMDB_GOLDEN_SCHEMA_38_PATH").filter(|v| !v.is_empty())
    {
        return PathBuf::from(path);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../dev/fixtures/golden-0.8.31-schema38.sqlite")
}

fn sha256_file(path: &Path) -> String {
    let bytes = fs::read(path).expect("read sqlite file for sha256");
    Sha256::digest(&bytes).iter().map(|b| format!("{b:02x}")).collect()
}

fn copy_golden_schema_38(dir: &TempDir, name: &str) -> PathBuf {
    configure_runtime(RuntimeSqliteMode::Performance).expect("configure runtime");
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
    assert_eq!(sha256_file(&dst), GOLDEN_SCHEMA_38_SHA256, "golden schema-38 sha256 mismatch");
    dst
}

#[test]
fn migrate_38_to_39_version_guards_lock_pending_erasure_and_rollback() {
    configure_runtime(RuntimeSqliteMode::Performance).expect("configure runtime");
    let dir = TempDir::new().unwrap();
    let backup_dir = dir.path().join("backups");

    // 1. Version 39 (already current) -> AlreadyCurrent, untouched, no backup.
    let db39 = dir.path().join("already_39.sqlite");
    {
        let opened = Engine::open(&db39).unwrap();
        opened.engine.close().unwrap();
    }
    let before39 = sha256_file(&db39);
    let err = migrate_38_to_39(&db39, &backup_dir).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::AlreadyCurrent);
    assert_eq!(err.seen_version, Some(39));
    assert_eq!(sha256_file(&db39), before39);
    assert!(!backup_dir.exists() || fs::read_dir(&backup_dir).unwrap().count() == 0);

    // 2. Versions 36, 37, and 40 -> UnsupportedVersion (CLI handles 37 chaining).
    for version in [36u32, 37u32, 40u32] {
        let db = copy_golden_schema_38(&dir, &format!("v{version}.sqlite"));
        {
            let conn = Connection::open(&db).unwrap();
            conn.pragma_update(None, "user_version", version).unwrap();
        }
        let before = sha256_file(&db);
        let err = migrate_38_to_39(&db, &backup_dir).unwrap_err();
        assert_eq!(err.code, MigrateErrorCode::UnsupportedVersion);
        assert_eq!(err.seen_version, Some(version));
        assert_eq!(sha256_file(&db), before);
    }

    // 3. Lock held -> LockHeld, untouched, no backup.
    let locked_db = copy_golden_schema_38(&dir, "locked.sqlite");
    let before_locked = sha256_file(&locked_db);
    let mut lock_os = locked_db.as_os_str().to_os_string();
    lock_os.push(".lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(PathBuf::from(lock_os))
        .unwrap();
    lock_file.try_lock().unwrap();
    let err = migrate_38_to_39(&locked_db, &backup_dir).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::LockHeld);
    assert_eq!(sha256_file(&locked_db), before_locked);
    drop(lock_file);

    // 4. Pending erasure -> PendingErasure, untouched.
    let pending_db = copy_golden_schema_38(&dir, "pending.sqlite");
    {
        let conn = Connection::open(&pending_db).unwrap();
        conn.execute(
            "INSERT INTO operational_mutations(collection_name, record_key, op_kind, payload_json, schema_id, write_cursor)
             VALUES ('erasure_pending_redaction', 'r1', 'append', '{}', NULL, 99)",
            [],
        )
        .unwrap();
    }
    let before_pending = sha256_file(&pending_db);
    let err = migrate_38_to_39(&pending_db, &backup_dir).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::PendingErasure);
    assert_eq!(sha256_file(&pending_db), before_pending);

    // 5. Mid-transaction fault injection before version stamp -> full rollback to v38.
    let rollback_db = copy_golden_schema_38(&dir, "rollback.sqlite");
    arm_migrate_38_to_39_before_version_stamp_hook_for_test(&rollback_db, || {
        Err("injected s39 failure".to_string())
    });
    let err = migrate_38_to_39(&rollback_db, &backup_dir).unwrap_err();
    assert_eq!(err.code, MigrateErrorCode::RollbackFailedOrDdlAborted);
    {
        let conn = Connection::open(&rollback_db).unwrap();
        let ver: u32 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(ver, 38, "failed 38->39 transaction must roll back user_version to 38");
        let old_idx: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='canonical_nodes_kind_idx'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old_idx, 1, "rolled-back DDL must keep original Schema 38 indexes");
    }

    // Subsequent retry succeeds cleanly after hook fired once.
    let report = migrate_38_to_39(&rollback_db, &backup_dir).expect("retry after rollback");
    assert_eq!(report.from_version, 38);
    assert_eq!(report.to_version, 39);
}

#[test]
fn migrate_38_to_39_engine_open_refuses_unmigrated_38_and_admits_after_migrate() {
    configure_runtime(RuntimeSqliteMode::Performance).expect("configure runtime");
    let dir = TempDir::new().unwrap();
    let db = copy_golden_schema_38(&dir, "legacy_38.sqlite");

    assert!(Engine::open(&db).is_err(), "Engine::open must refuse un-migrated Schema 38");

    let backup_dir = dir.path().join("backups");
    let report = migrate_38_to_39(&db, &backup_dir).unwrap();
    assert_eq!(report.from_version, 38);
    assert_eq!(report.to_version, 39);

    let opened = Engine::open(&db).expect("Schema 39 open must succeed after migrate_38_to_39");
    let report = opened
        .engine
        .check_integrity(CheckIntegrityOpts { quick: false, full: true, round_trip: false })
        .unwrap();
    assert_eq!(report.physical, Section::Clean);
    assert_eq!(report.logical, Section::Clean);
    assert_eq!(report.semantic, Section::Clean);
    opened.engine.close().unwrap();
}

#[test]
fn migrate_golden_0_8_31_schema38_upgrades_to_8col_vec0_and_passes_integrity() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden_schema_38(&dir, "golden38.sqlite");
    let backup_dir = dir.path().join("backups");

    // Pre-migration: Engine::open refuses Schema 38.
    assert!(Engine::open(&db).is_err());

    let report = migrate_38_to_39(&db, &backup_dir).expect("golden 38 -> 39 upgrade must succeed");
    assert_eq!(report.from_version, 38);
    assert_eq!(report.to_version, 39);
    assert_eq!(report.backup_sha256, GOLDEN_SCHEMA_38_SHA256);
    assert!(
        report.backup_path.file_name().unwrap().to_str().unwrap().contains(".schema38."),
        "expected .schema38. in backup filename: {:?}",
        report.backup_path
    );
    assert_eq!(sha256_file(&report.backup_path), GOLDEN_SCHEMA_38_SHA256);
    assert_eq!(
        fs::read_to_string(&report.backup_sha256_path).unwrap().trim().split_whitespace().next(),
        Some(GOLDEN_SCHEMA_38_SHA256)
    );
    assert!(report.integrity_clean(), "expected clean integrity: {:?}", report.integrity);

    // Raw SQLite inspection via sqlite-vec-registered connection or Engine open:
    {
        let conn = Connection::open(&db).unwrap();
        assert_eq!(conn.query_row("PRAGMA user_version", [], |r| r.get::<_, u32>(0)).unwrap(), 39);
        let nodes: i64 =
            conn.query_row("SELECT COUNT(*) FROM canonical_nodes", [], |r| r.get(0)).unwrap();
        let edges: i64 =
            conn.query_row("SELECT COUNT(*) FROM canonical_edges", [], |r| r.get(0)).unwrap();
        let vec_rows: i64 =
            conn.query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0)).unwrap();
        let op_state: i64 =
            conn.query_row("SELECT COUNT(*) FROM operational_state", [], |r| r.get(0)).unwrap();
        let op_muts: i64 =
            conn.query_row("SELECT COUNT(*) FROM operational_mutations", [], |r| r.get(0)).unwrap();
        let prop_vals: i64 = conn
            .query_row("SELECT COUNT(*) FROM canonical_property_values", [], |r| r.get(0))
            .unwrap();
        assert_eq!(nodes, 6);
        assert_eq!(edges, 3);
        assert_eq!(
            vec_rows, 7,
            "golden fixture has 5 node vectors + 2 edge vectors = 7 vector rows"
        );
        assert_eq!(op_state, 2);
        assert_eq!(op_muts, 3);
        assert_eq!(prop_vals, 14);

        let vec_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='vector_default'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            vec_sql.contains(",scope TEXT,"),
            "vector_default must have scope TEXT built-in column: {vec_sql}"
        );
        assert!(
            !vec_sql.contains("scope TEXT DEFAULT"),
            "vec0 must not emit DEFAULT clause: {vec_sql}"
        );

        let vis_triggers: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='trigger' AND name LIKE '_fathomdb_read_visibility_%'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(vis_triggers, 60);
    }

    // Open with EmbedderChoice::None (no model inference required) & verify full data-plane integrity.
    let opened =
        Engine::open_with_choice_and_config(&db, EmbedderChoice::None, EngineConfig::default())
            .expect("open upgraded golden schema-38 DB");
    let op_integrity = opened
        .engine
        .check_integrity(CheckIntegrityOpts { quick: false, full: true, round_trip: false })
        .unwrap();
    assert_eq!(op_integrity.physical, Section::Clean);
    assert_eq!(op_integrity.logical, Section::Clean);
    assert_eq!(op_integrity.semantic, Section::Clean);
    opened.engine.close().unwrap();

    let dp = inspect_data_plane_integrity(
        &db,
        DataPlaneIntegrityRequestV1 {
            schema_version: 1,
            checks: vec![
                DataPlaneIntegrityCheckV1::DependencyChain,
                DataPlaneIntegrityCheckV1::MutationReadiness,
            ],
            max_work_units: 10_000,
            max_findings: 100,
        },
    )
    .unwrap();
    assert!(dp.findings.is_empty(), "expected zero integrity findings: {dp:?}");

    // Engine::open registered sqlite-vec auto-extension globally; query vector_default directly.
    {
        let conn = Connection::open(&db).unwrap();
        let scopes: Vec<(i64, String)> = conn
            .prepare("SELECT rowid, scope FROM vector_default ORDER BY rowid")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert_eq!(
            scopes,
            vec![
                (3, String::new()),
                (4, String::new()),
                (5, String::new()),
                (7, String::new()),
                (10, String::new()),
                (12, String::new()),
                (16, String::new()),
            ]
        );
    }

    // Schema parity check vs a freshly opened Schema-39 DB (excluding vector_default dynamic attrs).
    let fresh_db = dir.path().join("fresh39.sqlite");
    {
        let fresh_opened = Engine::open(&fresh_db).unwrap();
        fresh_opened.engine.close().unwrap();
    }
    let up_conn = Connection::open(&db).unwrap();
    let fr_conn = Connection::open(&fresh_db).unwrap();
    let read_indexes = |c: &Connection| -> BTreeMap<String, String> {
        c.prepare("SELECT name, tbl_name FROM sqlite_master WHERE type='index' AND sql IS NOT NULL ORDER BY name")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<BTreeMap<_, _>, _>>()
            .unwrap()
    };
    assert_eq!(read_indexes(&up_conn), read_indexes(&fr_conn));
    let read_triggers = |c: &Connection| -> BTreeSet<String> {
        c.prepare("SELECT name FROM sqlite_master WHERE type='trigger' ORDER BY name")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<BTreeSet<_>, _>>()
            .unwrap()
    };
    assert_eq!(read_triggers(&up_conn), read_triggers(&fr_conn));

    // Verify `_fathomdb_projection_registry` upgraded to composite `PRIMARY KEY(entity_class, name)`.
    let mut pk_cols: Vec<(i64, String)> = up_conn
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

#[test]
fn migrate_38_to_39_reshapes_node_and_edge_vectors_preserving_nonzero_scope() {
    let dir = TempDir::new().unwrap();
    let db = copy_golden_schema_38(&dir, "scoped38.sqlite");
    let backup_dir = dir.path().join("backups");

    // Mutate one node cursor and one edge cursor in the Schema-38 fixture to carry non-empty scope
    // BEFORE migrating 38 -> 39, ensuring 7->8-col vec0 reshape joins BOTH canonical_nodes and canonical_edges.
    {
        let conn = Connection::open(&db).unwrap();
        // Temporarily disable coalescing trigger check isn't needed if _fathomdb_read_visibility_state exists;
        // simple UPDATE sets scope on node write_cursor=4 and edge write_cursor=12.
        conn.execute("UPDATE canonical_nodes SET scope = 'agent-alpha' WHERE write_cursor = 4", [])
            .unwrap();
        conn.execute("UPDATE canonical_edges SET scope = 'agent-beta' WHERE write_cursor = 12", [])
            .unwrap();
        conn.execute(
            "UPDATE operational_state SET scope = 'agent-gamma' WHERE collection_name = 'agent_checkpoints' AND record_key = 'step-1'",
            [],
        )
        .unwrap();
    }

    let report = migrate_38_to_39(&db, &backup_dir).expect("38 -> 39 with non-empty scopes");
    assert_eq!(report.to_version, 39);

    // Verify with an Engine::open SQLite-vec enabled probe via sqlite-vec connection:
    let opened =
        Engine::open_with_choice_and_config(&db, EmbedderChoice::None, EngineConfig::default())
            .unwrap();
    opened.engine.close().unwrap();

    // Check raw vec0 scope via Engine-registered sqlite-vec + operational_state + _fathomdb_vector_rows.
    let conn = Connection::open(&db).unwrap();
    let op_scope: String = conn
        .query_row(
            "SELECT scope FROM operational_state WHERE collection_name = 'agent_checkpoints' AND record_key = 'step-1'",
            params![],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(op_scope, "agent-gamma");
    let vec_scopes: BTreeMap<i64, String> = conn
        .prepare("SELECT rowid, scope FROM vector_default ORDER BY rowid")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .collect::<Result<BTreeMap<_, _>, _>>()
        .unwrap();
    assert_eq!(vec_scopes.len(), 7);
    assert_eq!(vec_scopes.get(&4).map(String::as_str), Some("agent-alpha"));
    assert_eq!(vec_scopes.get(&12).map(String::as_str), Some("agent-beta"));
    assert_eq!(vec_scopes.get(&16).map(String::as_str), Some(""));
    let vr_count: i64 =
        conn.query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r.get(0)).unwrap();
    assert_eq!(vr_count, 7);
}
