use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, EngineOpenError};
use rusqlite::Connection;
use tempfile::TempDir;

struct Provider;
impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice60", "test", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

#[test]
fn format35_fresh_and_rebuilt_vec0_use_64_row_chunks() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("store.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(conn.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0)).unwrap(), 35);
    for rebuild in [false, true] {
        if rebuild {
            opened.engine.rebuild_vec0().unwrap();
        }
        let sql: String = conn
            .query_row("SELECT sql FROM sqlite_master WHERE name='vector_default'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!(sql.contains("chunk_size=64"), "{sql}");
    }
}

#[test]
fn format34_is_refused_without_mutating_database() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old.sqlite");
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("CREATE TABLE marker(value); INSERT INTO marker VALUES('preserve'); PRAGMA user_version=34;").unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        Engine::open(&path),
        Err(EngineOpenError::IncompatibleSchemaVersion { seen: 34, supported: 35 })
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn current_format_refuses_old_vec0_shape() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad-shape.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    drop(opened);
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE vector_default; CREATE VIRTUAL TABLE vector_default USING vec0(embedding float[8],embedding_bin bit[8],source_type TEXT partition key,kind TEXT,created_at INTEGER,status TEXT);").unwrap();
    drop(conn);
    assert!(Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).is_err());
}

fn node(body: &str) -> fathomdb_engine::PreparedWrite {
    fathomdb_engine::PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: fathomdb_engine::SourceId::new("test:slice60").unwrap(),
        logical_id: None,
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

#[test]
fn body_fts_rowid_tracks_cursor_gaps_and_rebuild() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fts.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write_vector_for_test("doc", "cursor gap").unwrap();
    opened.engine.write(&[node("alpha"), node("beta")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let conn = Connection::open(&path).unwrap();
    for rebuild in [false, true] {
        if rebuild {
            opened.engine.rebuild_projections().unwrap();
            opened.engine.drain(10_000).unwrap();
        }
        let rows: Vec<(i64, i64)> = conn
            .prepare("SELECT rowid,write_cursor FROM search_index ORDER BY write_cursor")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|(rowid, cursor)| rowid == cursor), "{rows:?}");
    }
}
