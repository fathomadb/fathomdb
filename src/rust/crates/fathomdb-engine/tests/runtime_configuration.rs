use std::process::Command;

use fathomdb_engine::{configure_runtime, Engine, RuntimeConfigurationError, RuntimeSqliteMode};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

const SCENARIO_ENV: &str = "FATHOMDB_RUNTIME_CONFIGURATION_SCENARIO";

fn run_scenario(name: &str) {
    let output = Command::new(std::env::current_exe().expect("test executable"))
        .args(["--exact", "runtime_configuration_child", "--nocapture", "--test-threads=1"])
        .env(SCENARIO_ENV, name)
        .output()
        .expect("spawn scenario");
    assert!(
        output.status.success(),
        "scenario {name} failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn path(name: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
    (dir, path)
}

#[test]
fn default_open_selects_performance_mode() {
    run_scenario("default-performance");
}

#[test]
fn diagnostics_preserves_memory_controls() {
    run_scenario("diagnostics-memory");
}

#[test]
fn repeated_and_conflicting_configuration_is_deterministic() {
    run_scenario("repeat-conflict");
}

#[test]
fn prior_sqlite_initialization_is_rejected_without_shutdown() {
    run_scenario("too-late");
}

#[test]
fn concurrent_configuration_and_open_have_one_winner() {
    run_scenario("concurrent");
}

#[test]
fn configured_mode_survives_multiple_engine_lifetimes() {
    run_scenario("multiple-engines");
}

#[test]
fn runtime_configuration_child() {
    let Ok(scenario) = std::env::var(SCENARIO_ENV) else {
        return;
    };
    match scenario.as_str() {
        "default-performance" => {
            let (_dir, db) = path("default");
            let opened = Engine::open(&db).expect("default open");
            assert_eq!(opened.runtime_configuration().sqlite_mode, RuntimeSqliteMode::Performance);
            assert_eq!(unsafe { rusqlite::ffi::sqlite3_memory_used() }, 0);
            opened.engine.close().expect("close");
        }
        "diagnostics-memory" => {
            let configured = configure_runtime(RuntimeSqliteMode::Diagnostics).expect("configure");
            assert_eq!(configured.sqlite_mode, RuntimeSqliteMode::Diagnostics);
            let connection = rusqlite::Connection::open_in_memory().expect("connection");
            connection
                .execute_batch("CREATE TABLE t(value BLOB); INSERT INTO t VALUES(zeroblob(65536));")
                .expect("allocate");
            assert!(unsafe { rusqlite::ffi::sqlite3_memory_used() } > 0);
            let prior = unsafe { rusqlite::ffi::sqlite3_hard_heap_limit64(1024) };
            assert!(prior >= 1024);
            let denied = unsafe { rusqlite::ffi::sqlite3_malloc64(1_048_576) };
            assert!(denied.is_null(), "hard heap limit must reject an oversized allocation");
            unsafe {
                rusqlite::ffi::sqlite3_hard_heap_limit64(prior);
            }
        }
        "repeat-conflict" => {
            let first = configure_runtime(RuntimeSqliteMode::Performance).expect("first");
            let second = configure_runtime(RuntimeSqliteMode::Performance).expect("repeat");
            assert_eq!(first, second);
            assert!(matches!(
                configure_runtime(RuntimeSqliteMode::Diagnostics),
                Err(RuntimeConfigurationError::Conflict {
                    requested: RuntimeSqliteMode::Diagnostics,
                    effective: RuntimeSqliteMode::Performance
                })
            ));
        }
        "too-late" => {
            let connection = rusqlite::Connection::open_in_memory().expect("host SQLite");
            assert!(matches!(
                configure_runtime(RuntimeSqliteMode::Performance),
                Err(RuntimeConfigurationError::TooLate)
            ));
            drop(connection);
        }
        "concurrent" => {
            let (_dir, db) = path("concurrent");
            let configure =
                std::thread::spawn(|| configure_runtime(RuntimeSqliteMode::Diagnostics));
            let open = std::thread::spawn(move || Engine::open(&db));
            let configured = configure.join().expect("configuration thread");
            let opened = open.join().expect("open thread");
            match (configured, opened) {
                (Ok(config), Ok(opened)) => {
                    assert_eq!(config.sqlite_mode, RuntimeSqliteMode::Diagnostics);
                    assert_eq!(
                        opened.runtime_configuration().sqlite_mode,
                        RuntimeSqliteMode::Diagnostics
                    );
                    opened.engine.close().expect("close");
                }
                (Err(RuntimeConfigurationError::Conflict { effective, requested }), Ok(opened)) => {
                    assert_eq!(effective, RuntimeSqliteMode::Performance);
                    assert_eq!(requested, RuntimeSqliteMode::Diagnostics);
                    assert_eq!(
                        opened.runtime_configuration().sqlite_mode,
                        RuntimeSqliteMode::Performance
                    );
                    opened.engine.close().expect("close");
                }
                (configured, opened) => panic!("unexpected outcomes: {configured:?} / {opened:?}"),
            }
        }
        "multiple-engines" => {
            configure_runtime(RuntimeSqliteMode::Diagnostics).expect("configure");
            for index in 0..2 {
                let (_dir, db) = path(&format!("engine-{index}"));
                let opened = Engine::open(&db).expect("open");
                assert_eq!(
                    opened.runtime_configuration().sqlite_mode,
                    RuntimeSqliteMode::Diagnostics
                );
                opened.engine.close().expect("close");
            }
        }
        other => panic!("unknown scenario {other}"),
    }
}
