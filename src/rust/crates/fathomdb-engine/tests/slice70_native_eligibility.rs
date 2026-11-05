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

fn assert_dense_survives(cause: &str) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("native.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let mut writes = (0..200)
        .map(|i| {
            node(
                &format!("excluded{i}"),
                (cause == "future").then_some(i64::MAX),
                (cause == "expired").then_some(0),
            )
        })
        .collect::<Vec<_>>();
    writes.push(node("eligible", None, None));
    opened.engine.write(&writes).unwrap();
    opened.engine.drain(10_000).unwrap();
    if cause == "superseded" {
        let replacements = (0..200)
            .map(|i| node(&format!("excluded{i}"), Some(i64::MAX), None))
            .collect::<Vec<_>>();
        opened.engine.write(&replacements).unwrap();
        opened.engine.drain(10_000).unwrap();
    }
    if cause == "inactive" {
        for i in 0..200 {
            opened
                .engine
                .transition(
                    &format!("l:excluded{i}"),
                    fathomdb_engine::LifecycleState::Deleted,
                    Some("test".into()),
                )
                .unwrap();
        }
    }
    opened.engine.set_vector_stage_only_for_test(true);
    let result = opened.engine.search("query with no lexical match").unwrap();
    assert_eq!(result.results.len(), 1, "{cause}: {result:?}");
    assert_eq!(result.results[0].id.value, "eligible");
    assert_eq!(result.results[0].branch, fathomdb_engine::SoftFallbackBranch::Vector);
    assert!(result.soft_fallback.is_none(), "{cause}: {result:?}");
}

#[test]
fn expired_rows_do_not_disable_or_starve_dense_knn() {
    assert_dense_survives("expired");
}
#[test]
fn future_rows_do_not_disable_or_starve_dense_knn() {
    assert_dense_survives("future");
}
#[test]
fn superseded_rows_do_not_disable_or_starve_dense_knn() {
    assert_dense_survives("superseded");
}
#[test]
fn inactive_rows_do_not_disable_or_starve_dense_knn() {
    assert_dense_survives("inactive");
}

#[test]
fn pre_extension_format35_shape_is_refused_without_rewrite() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("old-shape.sqlite");
    Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap().engine.close().unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE vector_default; CREATE VIRTUAL TABLE vector_default USING vec0(embedding float[8],embedding_bin bit[8],source_type TEXT partition key,kind TEXT,created_at INTEGER,status TEXT,chunk_size=64);").unwrap();
    let sql: String = conn
        .query_row("SELECT sql FROM sqlite_master WHERE name='vector_default'", [], |r| r.get(0))
        .unwrap();
    assert!(Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).is_err());
    assert_eq!(
        conn.query_row("SELECT sql FROM sqlite_master WHERE name='vector_default'", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        sql
    );
}

#[test]
fn filterable_attribute_capacity_is_ten_and_overflow_is_atomic() {
    use fathomdb_engine::{ProjectionRole, ProjectionSpec};
    use std::collections::BTreeSet;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("capacity.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    let specs = (0..11)
        .map(|i| ProjectionSpec {
            name: format!("attr{i}"),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: None,
            vector: None,
            source: None,
        })
        .collect::<Vec<_>>();
    opened.engine.configure_projections(&specs[..10], &[]).unwrap();
    let conn = Connection::open(&path).unwrap();
    let before: String = conn
        .query_row("SELECT sql FROM sqlite_master WHERE name='vector_default'", [], |r| r.get(0))
        .unwrap();
    assert!(opened.engine.configure_projections(&specs, &[]).is_err());
    assert_eq!(
        conn.query_row("SELECT count(*) FROM _fathomdb_projection_registry", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert_eq!(
        conn.query_row("SELECT sql FROM sqlite_master WHERE name='vector_default'", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        before
    );
}

#[test]
fn atomic_actuation_updates_native_lifecycle() {
    use fathomdb_engine::{
        ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ArtifactRevisionId,
        LifecycleActuationV1, LifecycleState, ProvenancedNodeV1, SourceVersionId,
        WriteProvenanceV1,
    };
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("actuation.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    let revision = ArtifactRevisionId::new("source-revision").unwrap();
    let write = ProvenancedNodeV1 {
        logical_id: Some("item".into()),
        kind: "doc".into(),
        body: "body".into(),
        source_id: SourceId::new("test:slice70").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            revision.clone(),
            SourceVersionId::new("version1").unwrap(),
        ),
    };
    opened
        .engine
        .actuate(
            ActuationBatchV1::new("seed", vec![ActuationOperationV1::PutCanonicalNode(write)])
                .unwrap(),
        )
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    let transition = LifecycleActuationV1::new(
        "item",
        revision,
        LifecycleState::Deleted,
        Some("retired".into()),
    )
    .unwrap();
    let result = opened
        .engine
        .actuate(
            ActuationBatchV1::new(
                "delete",
                vec![ActuationOperationV1::TransitionLifecycle(transition)],
            )
            .unwrap(),
        )
        .unwrap();
    assert_eq!(result.outcome, ActuationOutcomeV1::Committed);
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT lifecycle FROM vector_default", [], |r| r.get::<_, i64>(0)).unwrap(),
        1
    );
}

#[test]
fn purge_refuses_an_unmapped_head_without_deleting_canonical_state() {
    use fathomdb_engine::LifecycleState;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("orphan-head.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write(&[node("item", None, None)]).unwrap();
    opened.engine.drain(10_000).unwrap();
    opened.engine.transition("l:item", LifecycleState::Deleted, Some("retired".into())).unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=1", []).unwrap();
    assert!(
        opened.engine.purge("l:item").is_err(),
        "unmapped vector ownership must not report successful erasure"
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM canonical_nodes WHERE write_cursor=1", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM vector_default WHERE rowid=1", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM search_index WHERE write_cursor=1", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
