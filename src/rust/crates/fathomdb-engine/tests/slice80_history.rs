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
        let drop = if declaration.source.is_some() { vec!["summary".to_string()] } else { vec![] };
        opened.engine.configure_projections(&[declaration.clone()], &drop).unwrap();
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

#[test]
fn graph_history_uses_newest_view_eligible_revision() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("graph.sqlite");
    let opened = Engine::open(&path).unwrap();
    let mut relation = edge("relation", "unused");
    if let PreparedWrite::Edge { body, .. } = &mut relation {
        *body = None;
    }
    opened
        .engine
        .write(&[
            node("a", "anchorquery", InitialState::Active),
            node("b", "old neighbor", InitialState::Active),
            relation,
        ])
        .unwrap();
    opened.engine.write(&[node("b", "pending neighbor", InitialState::Pending)]).unwrap();
    for (v, expected) in [
        (view(true, false), "old neighbor"),
        (view(false, true), "pending neighbor"),
        (view(true, true), "pending neighbor"),
    ] {
        let result = opened
            .engine
            .search_reranked_view("anchorquery", None, 0, true, 0.3, 0, false, &v)
            .unwrap();
        assert!(
            result
                .results
                .iter()
                .any(|h| h.body == expected
                    && h.branch == fathomdb_engine::SoftFallbackBranch::GraphArm),
            "{v:?}: {result:?}"
        );
    }
    assert_eq!(
        opened
            .engine
            .search_reranked_view("anchorquery", None, 0, true, 0.3, 0, false, &ReadView::default())
            .unwrap()
            .results
            .len(),
        1
    );
    opened.engine.close().unwrap();
}
#[test]
fn incomplete_history_disables_entire_dense_arm_without_lexical_match() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("fallback.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened.engine.write(&[node("ready", "ready vector", InitialState::Active)]).unwrap();
    opened.engine.drain(10_000).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    opened.engine.write(&[node("old", "pending historical vector", InitialState::Active)]).unwrap();
    opened.engine.write(&[node("old", "pending current vector", InitialState::Active)]).unwrap();
    opened.engine.set_vector_stage_only_for_test(true);
    let result = opened.engine.search_view("lexicallyunmatched", &view(true, false));
    opened.engine.set_projection_scheduler_frozen_for_test(false);
    let result = result.unwrap();
    assert_eq!(
        result.soft_fallback.as_ref().map(|f| f.branch),
        Some(fathomdb_engine::SoftFallbackBranch::Vector),
        "{result:?}"
    );
    assert!(result.results.is_empty(), "partial dense history must not be served");
    opened.engine.drain(10_000).unwrap();
    opened.engine.close().unwrap();
}
#[test]
fn absent_runtime_history_reports_vector_fallback_but_direct_text_does_not() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("absent.sqlite");
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[node("item", "history body", InitialState::Active)]).unwrap();
    let result = opened.engine.search_view("unmatched", &view(true, false)).unwrap();
    assert_eq!(
        result.soft_fallback.as_ref().map(|f| f.branch),
        Some(fathomdb_engine::SoftFallbackBranch::Vector)
    );
    assert!(opened
        .engine
        .search_text_only_view("history", &view(true, false))
        .unwrap()
        .soft_fallback
        .is_none());
    opened.engine.close().unwrap();
}
#[test]
fn reopen_refuses_partial_historical_vector_tuple() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("partial.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened
        .engine
        .write(&[
            node("a", "endpoint a", InitialState::Active),
            node("b", "endpoint b", InitialState::Active),
            edge("fact", "old fact"),
        ])
        .unwrap();
    opened.engine.write(&[edge("fact", "new fact")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    opened.engine.close().unwrap();
    drop(opened);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("DELETE FROM vector_default WHERE rowid=3", []).unwrap();
    drop(conn);
    let reopened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider));
    assert!(reopened.is_err(), "partial historical projections must fail open");
}

#[test]
fn historical_entity_seed_provenance_matches_selected_revision() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("seed-source.sqlite");
    let opened = Engine::open(&path).unwrap();
    opened.engine.write(&[node("entity", "oldmatch", InitialState::Active)]).unwrap();
    let mut replacement = node("entity", "newbody", InitialState::Active);
    if let PreparedWrite::Node { source_id, .. } = &mut replacement {
        *source_id = SourceId::new("new-source").unwrap();
    }
    opened.engine.write(&[replacement]).unwrap();
    let result = opened
        .engine
        .search_reranked_view("oldmatch", None, 0, true, 0.3, 0, false, &view(true, false))
        .unwrap();
    let selected = result
        .results
        .iter()
        .find(|hit| hit.body == "newbody")
        .expect("graph resolves newest version");
    assert_eq!(selected.source_id.as_deref(), Some("new-source"));
    opened.engine.close().unwrap();
}

#[test]
fn pending_history_cannot_mask_a_later_corrupt_tuple() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("pending-corrupt.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_vector_kind_for_test("doc").unwrap();
    opened
        .engine
        .write(&[
            node("pending", "first", InitialState::Active),
            node("corrupt", "second", InitialState::Active),
        ])
        .unwrap();
    opened.engine.drain(10_000).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(true);
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute("DELETE FROM vector_default WHERE rowid IN (1,2)", []).unwrap();
    conn.execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=1", []).unwrap();
    conn.execute("DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=1", []).unwrap();
    let result = opened.engine.search_view("unmatched", &view(true, false));
    // Restore physical consistency before unfreezing; the assertion concerns
    // the observed read, not worker cleanup after the deliberately torn tuple.
    conn.execute("DELETE FROM _fathomdb_vector_rows WHERE write_cursor=2", []).unwrap();
    conn.execute("DELETE FROM _fathomdb_projection_terminal WHERE write_cursor=2", []).unwrap();
    opened.engine.set_projection_scheduler_frozen_for_test(false);
    assert!(result.is_err(), "pending rows must not hide later corruption: {result:?}");
    opened.engine.drain(10_000).unwrap();
    opened.engine.close().unwrap();
}

#[test]
fn frozen_expansion_uses_historical_attributes_and_keeps_historical_edge_hits() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("expanded.sqlite");
    let opened = Engine::open_with_embedder_for_test(&path, Arc::new(Provider)).unwrap();
    opened.engine.configure_projections(&[property()], &[]).unwrap();
    let mut relation = edge("relation", "unused");
    if let PreparedWrite::Edge { body, .. } = &mut relation {
        *body = None;
    }
    opened
        .engine
        .write(&[
            node("a", r#"{"summary":"same","text":"anchorquery"}"#, InitialState::Active),
            node("b", r#"{"summary":"same","text":"old neighbor"}"#, InitialState::Active),
            relation,
        ])
        .unwrap();
    opened
        .engine
        .write(&[node(
            "b",
            r#"{"summary":"different","text":"new neighbor"}"#,
            InitialState::Active,
        )])
        .unwrap();
    let mut filter = fathomdb_engine::SearchFilter::default();
    filter.attributes = vec![("summary".into(), "same".into())];
    let context = fathomdb_engine::ReadContextV1::new(view(true, false), filter).unwrap();
    let frozen = opened.engine.freeze_read_context(&context).unwrap();
    let result = opened.engine.search_expand_frozen("anchorquery", &frozen, 1, 10).unwrap();
    assert!(
        result.expanded.iter().any(|(node, _)| node.body.contains("old neighbor")),
        "{result:?}"
    );
    opened.engine.write(&[edge("fact", "oldedgequery")]).unwrap();
    opened.engine.write(&[edge("fact", "newedgequery")]).unwrap();
    opened.engine.drain(10_000).unwrap();
    let frozen = opened
        .engine
        .freeze_read_context(
            &fathomdb_engine::ReadContextV1::new(view(true, false), Default::default()).unwrap(),
        )
        .unwrap();
    let result = opened.engine.search_expand_frozen("oldedgequery", &frozen, 1, 10).unwrap();
    assert!(result.search_hits.iter().any(|hit| hit.body == "oldedgequery"), "{result:?}");
    opened.engine.close().unwrap();
}

#[test]
fn relaxed_property_views_preserve_bm25_when_no_rows_are_added() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("property-rank.sqlite");
    let opened = Engine::open(&path).unwrap();
    let mut notes = property();
    notes.name = "notes".into();
    let mut scope = property();
    scope.name = "scope".into();
    scope.fts = None;
    opened.engine.configure_projections(&[property(), notes, scope], &[]).unwrap();
    let mut future = node(
        "future",
        r#"{"summary":"needle needle","notes":"needle long unrelated notes corpus terms","scope":"other"}"#,
        InitialState::Active,
    );
    if let PreparedWrite::Node { valid_from, .. } = &mut future {
        *valid_from = Some(i64::MAX);
    }
    opened.engine.write(&[
        node("a",r#"{"summary":"needle","notes":"unrelated corpus","scope":"keep"}"#,InitialState::Active),
        node("b",r#"{"summary":"needle needle long text","notes":"needle needle needle","scope":"keep"}"#,InitialState::Active),future,
    ]).unwrap();
    let mut filter = fathomdb_engine::SearchFilter::default();
    filter.attributes = vec![("scope".into(), "keep".into())];
    for selected_filter in [None, Some(filter)] {
        let strict = opened
            .engine
            .search_projected_text(
                "needle",
                "summary",
                selected_filter.clone(),
                &ReadView::default(),
            )
            .unwrap();
        let signature = |result: fathomdb_engine::SearchResult| {
            result.results.into_iter().map(|hit| (hit.body, hit.score)).collect::<Vec<_>>()
        };
        let expected = signature(strict);
        for v in [view(true, false), view(false, true), view(true, true)] {
            let result = opened
                .engine
                .search_projected_text("needle", "summary", selected_filter.clone(), &v)
                .unwrap();
            assert_eq!(signature(result), expected, "{v:?}");
        }
    }
    opened.engine.close().unwrap();
}
