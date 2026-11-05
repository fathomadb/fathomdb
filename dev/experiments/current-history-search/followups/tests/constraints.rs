use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::*;
use std::sync::Arc;
struct Fixed;
impl Embedder for Fixed {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("poc", "v1", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.; 8])
    }
}
fn node(id: &str, body: &str, state: InitialState) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("source").unwrap(),
        logical_id: Some(id.into()),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
#[test]
fn unchanged_head_index_g11_and_independent_existing_flags_are_witnessed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("constraints.sqlite");
    let engine = Engine::open_with_embedder_for_test(&path, Arc::new(Fixed)).unwrap().engine;
    engine
        .write(&[
            node("claim", "needle old", InitialState::Active),
            node("inactive", "needle inactive", InitialState::Pending),
        ])
        .unwrap();
    engine.write(&[node("claim", "needle current", InitialState::Active)]).unwrap();
    let raw = rusqlite::Connection::open(&path).unwrap();
    let ddl:String=raw.query_row("SELECT sql FROM sqlite_schema WHERE type='index' AND tbl_name='canonical_nodes' AND sql LIKE '%UNIQUE%' AND sql LIKE '%logical_id%'",[],|r|r.get(0)).unwrap();
    assert!(ddl.contains("superseded_at IS NULL"));
    let duplicate=raw.execute("INSERT INTO canonical_nodes(write_cursor,logical_id,kind,body,state) VALUES(99999,'claim','other','illegal','active')",[]).unwrap_err();
    assert!(
        matches!(duplicate,rusqlite::Error::SqliteFailure(error,_) if error.extended_code==2067),
        "{duplicate:?}"
    );
    for (superseded, inactive, expected) in
        [(false, false, 1), (true, false, 2), (false, true, 2), (true, true, 3)]
    {
        let view = ReadView {
            include_superseded: superseded,
            include_inactive: inactive,
            ..Default::default()
        };
        assert_eq!(engine.search_text_only_view("needle", &view).unwrap().results.len(), expected);
    }
    let edge = |id: &str, source: &str| PreparedWrite::Edge {
        kind: "rel".into(),
        from: "a".into(),
        to: "b".into(),
        source_id: SourceId::new(source).unwrap(),
        logical_id: Some(id.into()),
        body: Some(id.into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    };
    engine.write(&[edge("independent-a", "source-a")]).unwrap();
    engine.write(&[edge("independent-b", "source-b")]).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM canonical_edges", [], |r| r.get::<_, i64>(0)).unwrap(),
        2
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM canonical_edges WHERE superseded_at IS NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1,
        "G11 contradicts simultaneous independent source heads: successor operation required"
    );
    assert!(conn.query_row("SELECT superseded_at IS NOT NULL FROM canonical_edges WHERE logical_id='independent-a'",[],|r|r.get::<_,bool>(0)).unwrap());
    engine.close().unwrap();
}
