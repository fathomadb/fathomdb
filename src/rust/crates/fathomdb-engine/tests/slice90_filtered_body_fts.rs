use fathomdb_engine::{
    EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, ProjectionRole,
    ProjectionSpec, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(id: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body,
        logical_id: Some(id.into()),
        source_id: SourceId::new("slice90").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn edge(id: &str, body: String) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".into(),
        from: "n0".into(),
        to: "n1".into(),
        source_id: SourceId::new("slice90").unwrap(),
        logical_id: Some(id.into()),
        body: Some(body),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn filterable(name: &str) -> ProjectionSpec {
    ProjectionSpec {
        name: name.into(),
        roles: [ProjectionRole::Filterable].into(),
        fts: None,
        vector: None,
        source: None,
    }
}

#[test]
fn selective_filter_precedes_node_cap_and_node_scoped_attributes_exclude_edges() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("s90{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_projections(&[filterable("group"), filterable("level")], &[]).unwrap();
    let mut writes: Vec<_> = (0..120)
        .map(|i| {
            node(
                &format!("n{i}"),
                format!(r#"{{"text":"s90needle","group":"other","level":1,"i":{i}}}"#),
            )
        })
        .collect();
    let target =
        format!(r#"{{"text":"s90needle {}","group":"wanted","level":2}}"#, "padding ".repeat(100));
    writes.push(node("target", target.clone()));
    writes.push(edge("edge", target));
    engine.write(&writes).unwrap();
    let unfiltered = engine.search_text_only_with_limit("s90needle", 100).unwrap();
    assert!(
        !unfiltered.results.iter().any(|hit| hit.id.value == "target"),
        "fixture target is below the unfiltered node cap"
    );
    let mut filter = SearchFilter::default();
    filter.source_type = Some("article".into());
    filter.kind = Some("doc".into());
    filter.attributes = vec![("group".into(), "wanted".into()), ("level".into(), "2".into())];
    let result = engine
        .search_text_only_filtered("s90needle", Some(filter.clone()), &ReadView::default(), 1)
        .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.value, "target");
    assert!(result.soft_fallback.is_none());
    filter.source_type = None;
    filter.kind = None;
    let result = engine
        .search_text_only_filtered("s90needle", Some(filter.clone()), &ReadView::default(), 100)
        .unwrap();
    assert_eq!(result.results.iter().map(|h| h.id.value.as_str()).collect::<Vec<_>>(), ["target"]);
    filter.attributes[1].1 = "3".into();
    assert!(engine
        .search_text_only_filtered("s90needle", Some(filter), &ReadView::default(), 10)
        .unwrap()
        .results
        .is_empty());
    engine.close().unwrap();
}

#[test]
fn filtered_node_edge_prefix_and_empty_filter_preserve_order_and_scores() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("prefix{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let engine = &opened.engine;
    let mut writes: Vec<_> = (0..60)
        .map(|i| node(&format!("n{i}"), format!("s90prefix matching document {i:02}")))
        .collect();
    writes.push(edge("duplicate", "s90prefix matching document 10".into()));
    engine.write(&writes).unwrap();
    let view = ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() };
    for filter in [None, Some(SearchFilter::default()), {
        let mut f = SearchFilter::default();
        f.kind = Some("doc".into());
        Some(f)
    }] {
        let small =
            engine.search_text_only_filtered("s90prefix", filter.clone(), &view, 10).unwrap();
        let large = engine.search_text_only_filtered("s90prefix", filter, &view, 50).unwrap();
        assert_eq!(small.results.len(), 10);
        assert_eq!(large.results.len(), 50);
        assert_eq!(
            small.results.iter().map(|h| (&h.id, h.score)).collect::<Vec<_>>(),
            large.results.iter().take(10).map(|h| (&h.id, h.score)).collect::<Vec<_>>()
        );
    }
    let mut edge_filter = SearchFilter::default();
    edge_filter.source_type = Some("edge_fact".into());
    edge_filter.kind = Some("link".into());
    let result = engine
        .search_text_only_filtered("s90prefix", Some(edge_filter.clone()), &view, 10)
        .unwrap();
    assert_eq!(result.results.len(), 1);
    assert_eq!(result.results[0].id.value, "duplicate");
    edge_filter.kind = Some("doc".into());
    assert!(engine
        .search_text_only_filtered("s90prefix", Some(edge_filter), &view, 10)
        .unwrap()
        .results
        .is_empty());
    let legacy = engine.search_text_only_view_with_limit("s90prefix", &view, 50).unwrap();
    let empty = engine
        .search_text_only_filtered("s90prefix", Some(SearchFilter::default()), &view, 50)
        .unwrap();
    assert_eq!(legacy.results, empty.results);
    engine.close().unwrap();
}

#[test]
fn attribute_validation_and_view_axes_compose_with_direct_text_filter() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_choice(
        dir.path().join(format!("view{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_projections(&[filterable("group")], &[]).unwrap();
    let old = node("version", r#"{"text":"s90history old","group":"wanted"}"#.into());
    let mut inactive =
        node("inactive", r#"{"text":"s90history inactive","group":"wanted"}"#.into());
    if let PreparedWrite::Node { state, reason, .. } = &mut inactive {
        *state = InitialState::Pending;
        *reason = Some("test".into());
    }
    let mut future = node("future", r#"{"text":"s90history future","group":"wanted"}"#.into());
    if let PreparedWrite::Node { valid_from, .. } = &mut future {
        *valid_from = Some(200);
    }
    engine.write(&[old, inactive, future]).unwrap();
    engine
        .write(&[node("version", r#"{"text":"s90history current","group":"other"}"#.into())])
        .unwrap();
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("group".into(), "wanted".into())];
    let base = ReadView { valid_as_of: Some(100), ..ReadView::default() };
    assert!(engine
        .search_text_only_filtered("s90history", Some(filter.clone()), &base, 10)
        .unwrap()
        .results
        .is_empty());
    for view in [
        ReadView { include_superseded: true, ..base },
        ReadView { include_inactive: true, ..base },
        ReadView { include_out_of_window: true, ..base },
    ] {
        assert_eq!(
            engine
                .search_text_only_filtered("s90history", Some(filter.clone()), &view, 10)
                .unwrap()
                .results
                .len(),
            1
        );
    }
    let all = ReadView {
        include_superseded: true,
        include_inactive: true,
        include_out_of_window: true,
        ..base
    };
    assert_eq!(
        engine
            .search_text_only_filtered("s90history", Some(filter.clone()), &all, 10)
            .unwrap()
            .results
            .len(),
        3
    );
    filter.attributes[0].0 = "undeclared".into();
    assert!(matches!(
        engine.search_text_only_filtered("nohit", Some(filter), &base, 10),
        Err(EngineError::InvalidFilter { .. })
    ));
    engine.close().unwrap();
}

#[test]
fn metadata_keeps_projected_sentinel_time_and_missing_row_semantics() {
    use fathomdb_embedder::NoopEmbedder;
    use fathomdb_engine::TestEmbedderChoice;
    use std::sync::Arc;
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("metadata{SQLITE_SUFFIX}"));
    let opened = Engine::open_with_test_choice_for_test(
        &path,
        TestEmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine
        .write(&[
            node("n0", "s90metadata first".into()),
            node("n1", "s90metadata second".into()),
            edge("edge", "s90metadata edge".into()),
        ])
        .unwrap();
    engine.drain(10_000).unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    let timestamp: i64 = conn
        .query_row("SELECT created_at FROM vector_default WHERE rowid = 1", [], |r| r.get(0))
        .unwrap();
    let mut f = SearchFilter::default();
    f.created_after = Some(timestamp);
    f.status = Some(String::new());
    f.kind = Some("doc".into());
    let hits = engine
        .search_text_only_filtered("s90metadata first", Some(f.clone()), &ReadView::default(), 10)
        .unwrap();
    assert_eq!(hits.results.len(), 1, "inclusive projection timestamp and empty sentinel");
    f.created_after = Some(timestamp + 1);
    assert!(engine
        .search_text_only_filtered("s90metadata first", Some(f.clone()), &ReadView::default(), 10)
        .unwrap()
        .results
        .is_empty());
    f.created_after = None;
    f.kind = None;
    f.source_type = Some("edge_fact".into());
    assert_eq!(
        engine
            .search_text_only_filtered("s90metadata", Some(f.clone()), &ReadView::default(), 10)
            .unwrap()
            .results
            .len(),
        1
    );
    f.status = Some("open".into());
    assert!(engine
        .search_text_only_filtered("s90metadata", Some(f), &ReadView::default(), 10)
        .unwrap()
        .results
        .is_empty());
    engine.close().unwrap();
    drop(conn);
    let no_provider = Engine::open_with_choice(
        dir.path().join(format!("missing{SQLITE_SUFFIX}")),
        EmbedderChoice::None,
    )
    .unwrap();
    no_provider
        .engine
        .write(&[
            node("n0", "s90metadata first".into()),
            node("n1", "s90metadata second".into()),
            edge("edge", "s90metadata edge".into()),
        ])
        .unwrap();
    for mut f in [SearchFilter::default(), SearchFilter::default()] {
        if f.status.is_none() {
            f.status = Some(String::new());
        }
        f.created_after = Some(0);
        assert!(no_provider
            .engine
            .search_text_only_filtered("s90metadata", Some(f), &ReadView::default(), 10)
            .unwrap()
            .results
            .is_empty());
    }
    no_provider.engine.close().unwrap();
}
