use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    configure_runtime, Engine, EngineOpenError, OpenedEngine, RuntimeSqliteMode, TestEmbedderChoice,
};
use fathomdb_schema::{SCHEMA_VERSION, SQLITE_SUFFIX};
use rusqlite::Connection;
use std::sync::Arc;
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn configure_test_runtime() {
    configure_runtime(RuntimeSqliteMode::Performance).expect("configure test runtime");
}

fn set_user_version(conn: &Connection, version: u32) {
    conn.pragma_update(None, "user_version", version).unwrap();
}

struct IdentityOnlyEmbedder(EmbedderIdentity);

impl Embedder for IdentityOnlyEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        self.0.clone()
    }

    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        panic!("profile admission must not invoke embedding")
    }
}

fn open_with_identity(
    path: &std::path::Path,
    identity: EmbedderIdentity,
) -> Result<OpenedEngine, EngineOpenError> {
    Engine::open_with_test_choice_for_test(
        path,
        TestEmbedderChoice::Caller(Arc::new(IdentityOnlyEmbedder(identity))),
    )
}

fn supplied_identity() -> EmbedderIdentity {
    EmbedderIdentity::new(
        "fathomdb-bge-small-en-v1.5",
        "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
        384,
    )
}

#[test]
fn ac_047_rejects_05_shaped_database_before_use() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "legacy");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(include_str!("fixtures/v05_shape.sql")).unwrap();
    drop(conn);

    let err = Engine::open(&path).expect_err("0.5-shaped database must not open");

    match err {
        EngineOpenError::IncompatibleSchemaVersion { seen, supported } => {
            assert_eq!(seen, 1);
            assert_eq!(supported, SCHEMA_VERSION);
            assert!(err.to_string().contains("1"));
        }
        other => panic!("expected IncompatibleSchemaVersion, got {other:?}"),
    }
}

#[test]
fn future_schema_version_is_also_rejected() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "future");
    let conn = Connection::open(&path).unwrap();
    set_user_version(&conn, SCHEMA_VERSION + 1);
    drop(conn);

    let err = Engine::open(&path).expect_err("future schema version must not open");

    match err {
        EngineOpenError::IncompatibleSchemaVersion { seen, supported } => {
            assert_eq!(seen, SCHEMA_VERSION + 1);
            assert_eq!(supported, SCHEMA_VERSION);
        }
        other => panic!("expected IncompatibleSchemaVersion, got {other:?}"),
    }
}

#[test]
fn engine_open_emits_migration_step_events() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "events");
    let mut events = Vec::new();

    let opened = Engine::open_with_migration_event_sink(&path, |event| events.push(event.clone()))
        .expect("fresh open should bootstrap");
    opened.engine.close().unwrap();

    let step_ids: Vec<u32> = events.iter().map(|event| event.step_id).collect();
    let expected: Vec<u32> =
        fathomdb_schema::MIGRATIONS.iter().map(|migration| migration.step_id).collect();
    assert_eq!(step_ids, expected);
    assert!(events.iter().all(|event| event.duration_ms.is_some()));
    assert!(events.iter().all(|event| !event.failed));
    assert_eq!(opened.report.migration_steps, events);
}

#[test]
fn ac_048_rejects_stored_embedder_identity_mismatch() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "identity");
    open_with_identity(&path, EmbedderIdentity::new("other-embedder", "rev-b", 384))
        .unwrap()
        .engine
        .close()
        .unwrap();

    let err = open_with_identity(&path, supplied_identity())
        .expect_err("identity mismatch must fail open");

    match err {
        EngineOpenError::EmbedderIdentityMismatch { stored, supplied } => {
            assert_eq!(stored, EmbedderIdentity::new("other-embedder", "rev-b", 384));
            // EU-5b: default identity flipped to bge-small.
            assert_eq!(
                supplied,
                EmbedderIdentity::new(
                    "fathomdb-bge-small-en-v1.5",
                    "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
                    384,
                ),
            );
        }
        other => panic!("expected EmbedderIdentityMismatch, got {other:?}"),
    }
}

#[test]
fn ac_048b_rejects_stored_embedder_dimension_mismatch() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "dimension");
    // EU-5b: store the bge-small identity at a non-default dimension so the
    // open path surfaces `EmbedderDimensionMismatch` (the name+revision both
    // match the engine's default; only the dimension drifts).
    open_with_identity(
        &path,
        EmbedderIdentity::new(
            "fathomdb-bge-small-en-v1.5",
            "5c38ec7c405ec4b44b94cc5a9bb96e735b38267a",
            768,
        ),
    )
    .unwrap()
    .engine
    .close()
    .unwrap();

    let err = open_with_identity(&path, supplied_identity())
        .expect_err("dimension mismatch must fail open");

    match err {
        EngineOpenError::EmbedderDimensionMismatch { stored, supplied } => {
            assert_eq!(stored, 768);
            assert_eq!(supplied, 384);
        }
        other => panic!("expected EmbedderDimensionMismatch, got {other:?}"),
    }
}

#[test]
fn second_live_open_is_locked_and_close_releases_lock() {
    configure_test_runtime();
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "locked");

    let opened = Engine::open(&path).unwrap();
    let err = Engine::open(&path).expect_err("second open must be locked");
    // The lock is always detected. `holder_pid` is read back from the lock file, which works
    // under Unix advisory locks but not on Windows: `File::try_lock` maps to `LockFileEx`,
    // whose exclusive byte-range lock blocks other processes from READING the locked region,
    // so `read_holder_pid` returns None. The lock itself still works (concurrent open is
    // rejected) — only the diagnostic PID is unavailable on Windows. (0.8.9 Slice 20, F-9.)
    #[cfg(unix)]
    assert!(matches!(err, EngineOpenError::DatabaseLocked { holder_pid: Some(_) }));
    #[cfg(not(unix))]
    assert!(matches!(err, EngineOpenError::DatabaseLocked { .. }));

    opened.engine.close().unwrap();
    let reopened = Engine::open(&path).expect("close must release lock");
    reopened.engine.close().unwrap();
}

#[cfg(unix)]
#[test]
fn live_open_uses_one_lock_namespace_across_database_symlink_aliases() {
    let dir = TempDir::new().unwrap();
    let target = db_path(&dir, "symlink-lock-target");
    Engine::open(&target).unwrap().engine.close().unwrap();
    let alias = dir.path().join(format!("symlink-lock-alias{SQLITE_SUFFIX}"));
    std::os::unix::fs::symlink(&target, &alias).unwrap();

    let held_target = Engine::open(&target).unwrap();
    assert!(matches!(Engine::open(&alias), Err(EngineOpenError::DatabaseLocked { .. })));
    held_target.engine.close().unwrap();

    let held_alias = Engine::open(&alias).unwrap();
    assert!(matches!(Engine::open(&target), Err(EngineOpenError::DatabaseLocked { .. })));
    assert!(matches!(Engine::open(&alias), Err(EngineOpenError::DatabaseLocked { .. })));
    held_alias.engine.close().unwrap();
}

#[cfg(unix)]
#[test]
fn dangling_database_symlink_is_not_bootstrapped_as_a_fresh_path() {
    let dir = TempDir::new().unwrap();
    let missing_target = dir.path().join(format!("missing-target{SQLITE_SUFFIX}"));
    let alias = dir.path().join(format!("dangling-alias{SQLITE_SUFFIX}"));
    std::os::unix::fs::symlink(&missing_target, &alias).unwrap();

    assert!(matches!(Engine::open(&alias), Err(EngineOpenError::Io { .. })));
    assert!(!missing_target.exists());
    let mut alias_lock = alias.as_os_str().to_os_string();
    alias_lock.push(".lock");
    assert!(!std::path::PathBuf::from(alias_lock).exists());
}

#[test]
fn open_error_display_is_sanitized() {
    let err = EngineOpenError::MigrationError {
        schema_version_before: 1,
        schema_version_current: 1,
        step_id: 2,
    };

    let display = err.to_string();
    assert!(!display.contains("SELECT"));
    assert!(!display.contains(dir_path_fragment()));
    assert!(!display.contains("line "));
    assert!(!display.contains("column "));
}

fn dir_path_fragment() -> &'static str {
    "/home/"
}
