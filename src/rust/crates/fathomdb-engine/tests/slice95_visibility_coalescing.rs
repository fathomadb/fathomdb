use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use fathomdb_engine::{
    configure_runtime, Engine, EngineError, EngineOpenError, InitialState, PreparedWrite,
    ReadContextV1, ReadView, RuntimeSqliteMode, SearchFilter, SourceId,
};
use rusqlite::Connection;
use tempfile::TempDir;

fn node() -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: "visibility sentinel".into(),
        source_id: SourceId::new("test:s95-visibility").unwrap(),
        logical_id: None,
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn bytes(dir: &Path) -> BTreeMap<std::ffi::OsString, Vec<u8>> {
    fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), fs::read(entry.path()).unwrap())
        })
        .collect()
}

#[test]
fn format37_has_constrained_clean_state_and_all57_gated_triggers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("format37.sqlite");
    let opened = Engine::open(&path).unwrap();
    assert_eq!(opened.report.schema_version_after, 37);
    let connection = Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .query_row(
                "SELECT coalescing FROM _fathomdb_read_visibility_state WHERE singleton=1",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap(),
        0,
    );
    for value in ["NULL", "2", "0.5", "'corrupt'"] {
        assert!(
            connection
                .execute(
                    &format!("UPDATE _fathomdb_read_visibility_state SET coalescing={value}"),
                    [],
                )
                .is_err(),
            "accepted {value}"
        );
    }
    let sql = connection
        .prepare(
            "SELECT sql FROM sqlite_master WHERE type='trigger'
         AND name LIKE '_fathomdb_read_visibility_%'",
        )
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert_eq!(sql.len(), 57);
    assert!(sql.iter().all(|sql| sql.contains("coalescing")));
    assert_eq!(connection.query_row(
        "SELECT sql FROM sqlite_master WHERE name='_fathomdb_vector_spans_mapping_delete'",
        [], |row| row.get::<_, String>(0),
    ).unwrap(), "CREATE TRIGGER _fathomdb_vector_spans_mapping_delete\n              AFTER DELETE ON _fathomdb_vector_rows\n              BEGIN DELETE FROM _fathomdb_vector_spans WHERE vector_rowid=OLD.rowid; END");
    opened.engine.close().unwrap();
}

#[test]
fn committed_armed_state_refuses_canonical_fallback_admin_and_frozen_bindings() {
    for fallback in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("armed.sqlite");
        let opened = Engine::open(&path).unwrap();
        let connection = Connection::open(&path).unwrap();
        if fallback {
            connection
                .execute_batch(
                    "CREATE TABLE observed(value INTEGER);
                 CREATE TRIGGER custom_node AFTER INSERT ON canonical_nodes
                 BEGIN INSERT INTO observed VALUES(1); END;",
                )
                .unwrap();
        }
        connection.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=1", []).unwrap();
        assert!(matches!(opened.engine.write(&[node()]), Err(EngineError::Storage)));
        assert!(matches!(
            opened.engine.write(&[PreparedWrite::AdminSchema {
                name: "s95_armed".into(),
                kind: "operational".into(),
                schema_json: "{}".into(),
                retention_json: "{}".into(),
            }]),
            Err(EngineError::Storage)
        ));
        let context = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        assert!(opened.engine.freeze_read_context(&context).is_err());
        assert_eq!(
            connection
                .query_row("SELECT COUNT(*) FROM canonical_nodes", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            0
        );
        connection.execute("UPDATE _fathomdb_read_visibility_state SET coalescing=0", []).unwrap();
        assert_eq!(opened.engine.write(&[node()]).unwrap().cursor, 1);
        opened.engine.close().unwrap();
    }
}

#[test]
fn external_normal_writes_still_advance_generation_and_nonce() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("external.sqlite");
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[node()]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let connection = Connection::open(&path).unwrap();
    let before: (i64, String) = connection
        .query_row(
            "SELECT generation,state_nonce FROM _fathomdb_read_visibility_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    connection.execute("UPDATE canonical_nodes SET importance=0.5", []).unwrap();
    let after: (i64, String) = connection
        .query_row(
            "SELECT generation,state_nonce FROM _fathomdb_read_visibility_state",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(after.0, before.0 + 1);
    assert_ne!(after.1, before.1);
    opened.engine.close().unwrap();
}

#[test]
fn format36_refusal_preserves_database_wal_shm_and_lock_bytes() {
    configure_runtime(RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("prior.sqlite");
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;
         CREATE TABLE marker(value TEXT); INSERT INTO marker VALUES('preserve');
         PRAGMA user_version=36;",
        )
        .unwrap();
    fs::write(dir.path().join("prior.sqlite.lock"), b"preserved-lock").unwrap();
    let before = bytes(dir.path());
    assert!(matches!(
        Engine::open(&path),
        Err(EngineOpenError::IncompatibleSchemaVersion { seen: 36, supported: 37 })
    ));
    assert_eq!(bytes(dir.path()), before);
    assert_eq!(
        connection
            .query_row("SELECT value FROM marker", [], |row| row.get::<_, String>(0))
            .unwrap(),
        "preserve"
    );
}
