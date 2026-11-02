use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, SourceId};
use rusqlite::Connection;
use tempfile::TempDir;

struct Provider;
impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice70", "test", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}

fn node(id: &str, from: Option<i64>, until: Option<i64>) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.into()),
        kind: "doc".into(),
        body: "stored body".into(),
        source_id: SourceId::new("test:slice70").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: from,
        valid_until: until,
    }
}

#[test]
fn projected_metadata_survives_rebuild_and_reopen() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("metadata.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened
        .engine
        .write(&[node("bounded", Some(100), Some(200)), node("unbounded", None, None)])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let conn = Connection::open(&path).unwrap();
    for rebuild in [false, true] {
        if rebuild {
            opened.engine.rebuild_vec0().unwrap();
            opened.engine.drain(10_000).unwrap();
        }
        let rows: Vec<(i64, i64, i64)> = conn
            .prepare("SELECT lifecycle,valid_from,valid_until FROM vector_default ORDER BY rowid")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert_eq!(rows, [(0, 100, 199), (0, i64::MIN, i64::MAX)]);
    }
    opened.engine.close().unwrap();
    drop(opened);
    Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap().engine.close().unwrap();
}

#[test]
fn lifecycle_metadata_tracks_delete_reactivation_and_supersession() {
    use fathomdb_engine::LifecycleState;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("lifecycle.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write(&[node("item", None, None)]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let conn = Connection::open(&path).unwrap();
    let state = || {
        conn.query_row("SELECT lifecycle FROM vector_default WHERE rowid=1", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap()
    };
    opened.engine.transition("l:item", LifecycleState::Deleted, Some("test".into())).unwrap();
    assert_eq!(state(), 1);
    opened.engine.transition("l:item", LifecycleState::Active, None).unwrap();
    assert_eq!(state(), 0);
    opened.engine.write(&[node("item", None, None)]).unwrap();
    opened.engine.drain(10_000).unwrap();
    assert_eq!(state(), 2);
}
