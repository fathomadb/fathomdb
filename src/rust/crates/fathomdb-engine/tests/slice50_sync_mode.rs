//! Slice 50 WP-50A (`AC32-SYNC-01`, `AC32-SYNC-02`):
//! Declared WAL `SyncMode::{Normal, Full}` configuration and PRAGMA synchronous
//! enforcement on both primary writer and projection/runtime connections.

use fathomdb_engine::{EmbedderChoice, Engine, EngineConfig, SyncMode};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

#[cfg(feature = "test-hooks")]
use std::path::Path;
#[cfg(feature = "test-hooks")]
use std::sync::Mutex;

#[cfg(feature = "test-hooks")]
static ENV_LOCK: Mutex<()> = Mutex::new(());

#[cfg(feature = "test-hooks")]
struct EnvGuard(Option<String>);

#[cfg(feature = "test-hooks")]
impl EnvGuard {
    fn set(path: &Path) -> Self {
        let prev = std::env::var("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST").ok();
        unsafe {
            std::env::set_var("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", path);
        }
        Self(prev)
    }
}

#[cfg(feature = "test-hooks")]
impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.0 {
            Some(prev) => unsafe {
                std::env::set_var("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST", prev);
            },
            None => unsafe {
                std::env::remove_var("FATHOMDB_WRITER_PRAGMA_WITNESS_FOR_TEST");
            },
        }
    }
}

#[cfg(feature = "test-hooks")]
fn read_witness(path: &Path) -> Vec<serde_json::Value> {
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("valid witness jsonl"))
        .collect()
}

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

#[test]
fn sync_mode_defaults_strings_and_builder_round_trip() {
    #[cfg(feature = "test-hooks")]
    let _lock = ENV_LOCK.lock().expect("env lock");

    assert_eq!(SyncMode::default(), SyncMode::Normal);
    assert_eq!(SyncMode::Normal.as_pragma(), "NORMAL");
    assert_eq!(SyncMode::Full.as_pragma(), "FULL");
    assert_eq!(SyncMode::Normal.as_str(), "normal");
    assert_eq!(SyncMode::Full.as_str(), "full");

    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "roundtrip");

    let omitted =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, EngineConfig::default())
            .expect("open default");
    assert_eq!(omitted.engine.config().sync_mode, None);
    omitted.engine.close().unwrap();

    let full_cfg = EngineConfig::default().with_sync_mode(SyncMode::Full);
    assert_eq!(full_cfg.sync_mode, Some(SyncMode::Full));
    let full = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, full_cfg.clone())
        .expect("reopen full");
    assert_eq!(full.engine.config(), &full_cfg);
    assert_eq!(full.engine.config().sync_mode, Some(SyncMode::Full));
    full.engine.close().unwrap();

    let normal_cfg = EngineConfig::default().with_sync_mode(SyncMode::Normal);
    let normal =
        Engine::open_with_choice_and_config(&path, EmbedderChoice::None, normal_cfg.clone())
            .expect("reopen normal");
    assert_eq!(normal.engine.config(), &normal_cfg);
    assert_eq!(normal.engine.config().sync_mode, Some(SyncMode::Normal));
    normal.engine.close().unwrap();
}

#[cfg(feature = "test-hooks")]
#[test]
fn pragma_synchronous_applies_to_writer_and_runtime_connections_across_fresh_open_and_reopen() {
    let _lock = ENV_LOCK.lock().expect("env lock");
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "sync-witness");

    let cases: &[(&str, Option<SyncMode>, i64)] = &[
        ("fresh-omitted", None, 1),
        ("reopen-full", Some(SyncMode::Full), 2),
        ("reopen-normal", Some(SyncMode::Normal), 1),
        ("reopen-full-again", Some(SyncMode::Full), 2),
    ];

    for (label, requested_mode, expected_pragma) in cases {
        let witness_path = dir.path().join(format!("{label}.jsonl"));
        let _guard = EnvGuard::set(&witness_path);

        let mut cfg =
            EngineConfig { scheduler_runtime_threads: Some(2), ..EngineConfig::default() };
        if let Some(mode) = *requested_mode {
            cfg = cfg.with_sync_mode(mode);
        }

        let opened = Engine::open_with_choice_and_config(&path, EmbedderChoice::None, cfg)
            .unwrap_or_else(|e| panic!("{label}: open failed: {e}"));

        assert_eq!(
            opened.engine.runtime_synchronous_pragma_for_test().unwrap(),
            *expected_pragma,
            "{label}: runtime probe connection must observe PRAGMA synchronous={expected_pragma}"
        );
        opened.engine.close().unwrap();

        let rows = read_witness(&witness_path);
        assert!(
            rows.iter().any(|row| {
                row["role"] == "writer"
                    && row["journal_mode"] == "wal"
                    && row["synchronous"] == *expected_pragma
            }),
            "{label}: expected writer witness with synchronous={expected_pragma}, got {rows:?}"
        );
        assert!(
            rows.iter().any(|row| {
                row["role"] == "projection_dispatcher"
                    && row["journal_mode"] == "wal"
                    && row["synchronous"] == *expected_pragma
            }),
            "{label}: expected projection_dispatcher witness with synchronous={expected_pragma}, got {rows:?}"
        );
        assert!(
            rows.iter().any(|row| {
                row["role"] == "projection_worker"
                    && row["journal_mode"] == "wal"
                    && row["synchronous"] == *expected_pragma
            }),
            "{label}: expected projection_worker witness with synchronous={expected_pragma}, got {rows:?}"
        );
        assert!(
            rows.iter().all(|row| row["synchronous"] == *expected_pragma),
            "{label}: every observed connection must use synchronous={expected_pragma}, got {rows:?}"
        );
    }
}
