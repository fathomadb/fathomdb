use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{Engine, InitialState, PreparedWrite, ReadView, SourceId};
use std::sync::Arc;
use tempfile::TempDir;

struct Provider;
impl Embedder for Provider {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice80", "test", 8)
    }
    fn embed(&self, _: &str) -> Result<Vector, EmbedderError> {
        Ok(vec![1.0; 8])
    }
}
fn node(id: &str, body: &str, state: InitialState) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.into()),
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:slice80").unwrap(),
        state,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}
fn view(superseded: bool, inactive: bool) -> ReadView {
    ReadView { include_superseded: superseded, include_inactive: inactive, ..ReadView::default() }
}
#[test]
fn text_and_dense_history_flags_compose_independently() {
    for dense in [false, true] {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("history.sqlite");
        let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
        opened.engine.configure_vector_kind_for_test("doc").unwrap();
        opened
            .engine
            .write(&[
                node("old", "history retired", InitialState::Active),
                node("inactive", "history pending", InitialState::Pending),
            ])
            .unwrap();
        opened.engine.write(&[node("old", "history current", InitialState::Active)]).unwrap();
        opened.engine.drain(10_000).unwrap();
        opened.engine.set_vector_stage_only_for_test(dense);
        for (superseded, inactive, expected) in
            [(false, false, 1), (true, false, 2), (false, true, 2), (true, true, 3)]
        {
            let result = opened.engine.search_view("history", &view(superseded, inactive)).unwrap();
            assert_eq!(
                result.results.len(),
                expected,
                "dense={dense} view={superseded}/{inactive}: {result:?}"
            );
            assert!(result.soft_fallback.is_none());
        }
        opened.engine.close().unwrap();
    }
}

fn edge(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".into(),
        from: "a".into(),
        to: "b".into(),
        source_id: SourceId::new("test:slice80").unwrap(),
        logical_id: Some(id.into()),
        body: Some(body.into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}
#[test]
fn superseded_edges_are_dense_members_before_and_after_drain() {
    for drain_first in [false, true] {
        for same_identity in [false, true] {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("edges.sqlite");
            let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
            opened
                .engine
                .write(&[
                    node("a", "endpoint a", InitialState::Active),
                    node("b", "endpoint b", InitialState::Active),
                    edge("old", "old historical fact"),
                ])
                .unwrap();
            if drain_first {
                opened.engine.drain(10_000).unwrap();
            }
            opened
                .engine
                .write(&[edge(if same_identity { "old" } else { "new" }, "new historical fact")])
                .unwrap();
            opened.engine.drain(10_000).unwrap();
            let conn = rusqlite::Connection::open(&path).unwrap();
            let count: i64 = conn
                .query_row(
                    "SELECT count(*) FROM _fathomdb_vector_rows WHERE kind='edge_fact'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(count, 2, "drain_first={drain_first}, same_identity={same_identity}");
            opened.engine.set_vector_stage_only_for_test(true);
            let results = opened.engine.search_view("unmatched", &view(true, false)).unwrap();
            assert_eq!(results.results.len(), 2, "{results:?}");
            assert!(results.soft_fallback.is_none());
            assert_eq!(opened.engine.search("unmatched").unwrap().results.len(), 1);
            opened.engine.close().unwrap();
        }
    }
}

#[test]
fn reopen_repairs_legacy_skipped_edge_history_and_rebuild_preserves_it() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("reopen.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened
        .engine
        .write(&[
            node("a", "endpoint a", InitialState::Active),
            node("b", "endpoint b", InitialState::Active),
            edge("fact", "legacy historical fact"),
        ])
        .unwrap();
    opened.engine.write(&[edge("fact", "current fact")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("DELETE FROM vector_default WHERE rowid=3", []).unwrap();
    conn.execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=3", []).unwrap();
    conn.execute("DELETE FROM search_index_edges WHERE write_cursor=3", []).unwrap();
    drop(conn);
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.drain(10_000).unwrap();
    for rebuild in [false, true] {
        if rebuild {
            opened.engine.rebuild_projections().unwrap();
            opened.engine.drain(10_000).unwrap();
        }
        let result = opened.engine.search_text_only_view("legacy", &view(true, false)).unwrap();
        assert_eq!(result.results.len(), 1, "rebuild={rebuild}: {result:?}");
        let conn = rusqlite::Connection::open(&path).unwrap();
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM _fathomdb_vector_rows WHERE kind='edge_fact'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            2
        );
    }
    opened.engine.close().unwrap();
}

#[test]
fn full_rebuild_keeps_expired_edge_lexical_history_for_past_views() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("expired.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    let mut old = edge("fact", "expired historical fact");
    if let PreparedWrite::Edge { t_invalid, .. } = &mut old {
        *t_invalid = Some(2_000);
    }
    opened
        .engine
        .write(&[
            node("a", "endpoint a", InitialState::Active),
            node("b", "endpoint b", InitialState::Active),
            old,
        ])
        .unwrap();
    opened.engine.write(&[edge("fact", "current fact")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    opened.engine.rebuild_projections().unwrap();
    let historical =
        ReadView { valid_as_of: Some(1_000), include_superseded: true, ..ReadView::default() };
    assert_eq!(
        opened.engine.search_text_only_view("expired", &historical).unwrap().results.len(),
        1
    );
    assert!(opened
        .engine
        .search_text_only_view("expired", &view(true, false))
        .unwrap()
        .results
        .is_empty());
    opened.engine.close().unwrap();
}

fn property() -> fathomdb_engine::ProjectionSpec {
    fathomdb_engine::ProjectionSpec {
        name: "summary".into(),
        roles: std::collections::BTreeSet::from([
            fathomdb_engine::ProjectionRole::Searchable,
            fathomdb_engine::ProjectionRole::Filterable,
        ]),
        fts: Some(fathomdb_engine::ProjectionFts { tokenizer: None }),
        vector: None,
        source: None,
    }
}
#[test]
fn historical_property_search_and_filters_use_canonical_values() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("property.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_projections(&[property()], &[]).unwrap();
    opened
        .engine
        .write(&[node("item", r#"{"summary":"café running"}"#, InitialState::Active)])
        .unwrap();
    opened
        .engine
        .write(&[
            node("item", r#"{"summary":"current"}"#, InitialState::Active),
            node("pending", r#"{"summary":"café pending"}"#, InitialState::Pending),
        ])
        .unwrap();
    let history = view(true, true);
    let result = opened.engine.search_projected_text("cafe", "summary", None, &history).unwrap();
    assert_eq!(result.results.len(), 2, "{result:?}");
    assert!(opened
        .engine
        .search_projected_text("cafe", "summary", None, &ReadView::default())
        .unwrap()
        .results
        .is_empty());
    let mut filter = fathomdb_engine::SearchFilter::default();
    filter.attributes = vec![("summary".into(), "café running".into())];
    let result = opened
        .engine
        .search_reranked_view("running", Some(filter.clone()), 0, false, 0.3, 0, false, &history)
        .unwrap();
    assert_eq!(result.results.len(), 1, "{result:?}");
    let result =
        opened.engine.search_projected_text("run", "summary", Some(filter), &history).unwrap();
    assert_eq!(result.results.len(), 1, "{result:?}");
    opened.engine.close().unwrap();
}
#[test]
fn late_attribute_declaration_populates_historical_vectors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("late-attr.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened
        .engine
        .write(&[node("item", r#"{"summary":"retired","nested":"changed"}"#, InitialState::Active)])
        .unwrap();
    opened.engine.write(&[node("item", r#"{"summary":"current"}"#, InitialState::Active)]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let mut declaration = property();
    for (source, wanted) in [(None, "retired"), (Some(vec!["nested".into()]), "changed")] {
        declaration.source = source;
        opened.engine.configure_projections(&[declaration.clone()], &[]).unwrap();
        opened.engine.set_vector_stage_only_for_test(true);
        let mut filter = fathomdb_engine::SearchFilter::default();
        filter.attributes = vec![("summary".into(), wanted.into())];
        let result = opened
            .engine
            .search_reranked_view(
                "unmatched",
                Some(filter),
                0,
                false,
                0.3,
                0,
                false,
                &view(true, false),
            )
            .unwrap();
        assert_eq!(result.results.len(), 1, "source={:?}: {result:?}", declaration.source);
    }
    opened.engine.close().unwrap();
}
