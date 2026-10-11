//! Slice 132 — `admin::configure_runtime` is process-global and must precede
//! the first open, so it has its own test binary (one process, one test).

use fathomdb_sdk::{admin, Engine, ErrorKind, OpenOptions, RuntimeSqliteMode};

#[test]
fn configure_runtime_before_open_then_conflict() {
    let configured = admin::configure_runtime(RuntimeSqliteMode::Diagnostics).expect("configure");
    assert_eq!(configured.sqlite_mode, RuntimeSqliteMode::Diagnostics);
    admin::configure_runtime(RuntimeSqliteMode::Diagnostics).expect("same mode is idempotent");

    let dir = tempfile::tempdir().expect("tempdir");
    let engine = Engine::open(dir.path().join("sdk.sqlite"), OpenOptions::default()).expect("open");
    let error = admin::configure_runtime(RuntimeSqliteMode::Performance).expect_err("conflict");
    assert_eq!(error.kind(), ErrorKind::RuntimeConfiguration);
    engine.close().expect("close");
}
