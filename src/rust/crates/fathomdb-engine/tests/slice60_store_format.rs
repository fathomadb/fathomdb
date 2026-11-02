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

#[test]
fn projected_heads_store_unknown_coverage_and_rebuild_it() {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("spans.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write(&[node("héllo"), node("世界")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let conn = Connection::open(&path).unwrap();
    for rebuild in [false, true] {
        if rebuild {
            opened.engine.rebuild_vec0().unwrap();
            opened.engine.drain(10_000).unwrap();
        }
        let rows: Vec<(i64,i64,i64,i64,Option<i64>,i64)> = conn.prepare("SELECT vector_rowid,write_cursor,window_number,byte_start,byte_end,body_length FROM _fathomdb_vector_spans ORDER BY write_cursor").unwrap().query_map([], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).unwrap().collect::<Result<_,_>>().unwrap();
        assert_eq!(rows, [(1, 1, 0, 0, None, 6), (2, 2, 0, 0, None, 6)]);
    }
}

struct CoveredProvider {
    invalid: bool,
}
impl Embedder for CoveredProvider {
    fn identity(&self) -> EmbedderIdentity {
        Provider.identity()
    }
    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        Provider.embed(input)
    }
    fn truncation_offset(&self, input: &str) -> Result<Option<usize>, EmbedderError> {
        Ok(Some(if self.invalid { 1 } else { input.len() }))
    }
}

#[test]
fn projection_carries_provider_coverage() {
    assert_coverage(false);
}

#[test]
fn projection_rejects_non_utf8_offsets() {
    assert_coverage(true);
}

fn assert_coverage(invalid: bool) {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("coverage.sqlite");
        let opened =
            Engine::open_with_embedder_for_test(&path, Arc::new(CoveredProvider { invalid }))
                .unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        opened.engine.set_projection_retry_delays_for_test(&[]);
        opened.engine.write(&[node("écho")]).unwrap();
        opened.engine.drain(10_000).unwrap();
        let conn = Connection::open(&path).unwrap();
        let rows: Vec<Option<i64>> = conn
            .prepare("SELECT byte_end FROM _fathomdb_vector_spans")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        if invalid {
            assert!(rows.is_empty(), "invalid UTF-8 end must not publish a span");
            assert_eq!(
                conn.query_row("SELECT COUNT(*) FROM _fathomdb_vector_rows", [], |r| r
                    .get::<_, u64>(0))
                    .unwrap(),
                0
            );
        } else {
            assert_eq!(rows, [Some(5)]);
        }
    }
}

fn assert_bad_span_rejected(mutation: &str, populated: bool) {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("corrupt-span.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    if populated {
        opened.engine.write(&[node("écho")]).unwrap();
        opened.engine.drain(10_000).unwrap();
    }
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch(mutation).unwrap();
    assert!(opened.engine.read_projection_generation_status().is_err(), "{mutation}");
    drop(conn);
    drop(opened);
    assert!(
        Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).is_err(),
        "reopen: {mutation}"
    );
}

#[test]
fn missing_span_is_not_complete() {
    assert_bad_span_rejected("DELETE FROM _fathomdb_vector_spans", true);
}
#[test]
fn mismatched_span_body_length_is_not_complete() {
    assert_bad_span_rejected("UPDATE _fathomdb_vector_spans SET body_length=99", true);
}
#[test]
fn persisted_non_utf8_span_is_not_complete() {
    assert_bad_span_rejected("UPDATE _fathomdb_vector_spans SET byte_end=1", true);
}
#[test]
fn orphan_span_cannot_hide_in_empty_fast_path() {
    assert_bad_span_rejected("INSERT INTO _fathomdb_vector_spans VALUES(99,99,0,0,NULL,0)", false);
}

fn assert_cleanup_authority_required(alter: bool) {
    fathomdb_engine::configure_runtime(fathomdb_engine::RuntimeSqliteMode::Performance).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cleanup.sqlite");
    drop(Engine::open(&path).unwrap());
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("DROP TRIGGER _fathomdb_vector_spans_mapping_delete;").unwrap();
    if alter {
        conn.execute_batch("CREATE TRIGGER _fathomdb_vector_spans_mapping_delete AFTER DELETE ON _fathomdb_vector_rows BEGIN SELECT 1; END;").unwrap();
    }
    drop(conn);
    assert!(Engine::open(&path).is_err());
}
#[test]
fn missing_span_cleanup_trigger_is_refused() {
    assert_cleanup_authority_required(false);
}
#[test]
fn altered_span_cleanup_trigger_is_refused() {
    assert_cleanup_authority_required(true);
}
