//! 0.8.32 Slice 20 (`WP-20D`) — Rust SDK caller-scope isolation & view parity.

use fathomdb_sdk::{
    graph, read, Engine, ErrorKind, FrozenReadContextV1, InitialState, OpenOptions, PreparedWrite,
    ReadContextV1, ReadView, ScopePolicy, SearchExpandOptions, SearchFilter, SourceId,
    TextSearchOptions,
};

fn node(
    logical_id: &str,
    body: &str,
    source: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".to_string(),
        body: body.to_string(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
        properties: None,
    }
}

fn edge(logical_id: &str, from: &str, to: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "links".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical_id.to_string()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
    }
}

#[test]
fn open_rejects_invalid_scope_policies() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("invalid_scope.sqlite");
    let invalid_policies = [
        ScopePolicy::read_only::<Vec<&str>, &str>(Vec::new()),
        ScopePolicy::single(""),
        ScopePolicy::single("*"),
        ScopePolicy::read_write(["agent-a"], "agent-b"),
    ];
    for (idx, policy) in invalid_policies.into_iter().enumerate() {
        let err = Engine::open(&path, OpenOptions::default().with_scope_policy(policy))
            .expect_err("invalid policy must fail open");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument, "case {idx}");
    }
}

#[test]
fn two_callers_are_isolated_and_read_only_rejects_writes_with_scope_violation() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("isolated.sqlite");

    let a = Engine::open(&path, OpenOptions::default().with_scope_policy(ScopePolicy::single("a")))
        .expect("open agent a");
    a.write(&[node("a:only", "scopedneedle alpha private", "src-a", None, None)]).expect("write a");
    a.drain(10_000).expect("drain a");
    a.close().expect("close a");

    let b = Engine::open(&path, OpenOptions::default().with_scope_policy(ScopePolicy::single("b")))
        .expect("open agent b");
    b.write(&[node("b:only", "scopedneedle beta private", "src-b", None, None)]).expect("write b");
    b.drain(10_000).expect("drain b");
    assert!(read::get(&b, "a:only", None).expect("get b").is_none());
    let b_expand =
        graph::search_expand(&b, "scopedneedle", 1, None, SearchExpandOptions::default())
            .expect("expand b");
    assert_eq!(b_expand.all_logical_ids, vec!["b:only"]);
    b.close().expect("close b");

    let a = Engine::open(&path, OpenOptions::default().with_scope_policy(ScopePolicy::single("a")))
        .expect("reopen agent a");
    assert!(read::get(&a, "b:only", None).expect("get a").is_none());
    let a_hits =
        a.search_text_only("scopedneedle", TextSearchOptions::default()).expect("search a");
    assert_eq!(
        a_hits.results.iter().map(|hit| hit.id.value.as_str()).collect::<Vec<_>>(),
        vec!["a:only"]
    );
    a.close().expect("close a");

    let ro = Engine::open(
        &path,
        OpenOptions::default().with_scope_policy(ScopePolicy::read_only(["a"])),
    )
    .expect("open read-only a");
    assert!(read::get(&ro, "a:only", None).expect("get ro").is_some());
    assert!(read::get(&ro, "b:only", None).expect("get ro b").is_none());
    let err = ro
        .write(&[node("a:blocked", "cannot write", "src-ro", None, None)])
        .expect_err("read-only handle must fail writes");
    assert_eq!(err.kind(), ErrorKind::ScopeViolation);
    ro.close().expect("close ro");
}

#[test]
fn search_expand_view_surfaces_out_of_window_scoped_nodes_and_rejects_other_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("expand_view.sqlite");

    let a = Engine::open(&path, OpenOptions::default().with_scope_policy(ScopePolicy::single("a")))
        .expect("open a");
    a.write(&[
        node("a:root", "expandanchor alpha", "src-a", None, None),
        node("a:past", "past leaf", "src-a", Some(1), Some(2)),
        edge("e:a", "a:root", "a:past", "src-a"),
    ])
    .expect("write a");
    a.drain(10_000).expect("drain a");
    a.close().expect("close a");

    let b = Engine::open(&path, OpenOptions::default().with_scope_policy(ScopePolicy::single("b")))
        .expect("open b");
    b.write(&[
        node("b:root", "expandanchor beta", "src-b", None, None),
        node("b:past", "past leaf beta", "src-b", Some(1), Some(2)),
        edge("e:b", "b:root", "b:past", "src-b"),
    ])
    .expect("write b");
    b.drain(10_000).expect("drain b");
    b.close().expect("close b");

    let a = Engine::open(&path, OpenOptions::default().with_scope_policy(ScopePolicy::single("a")))
        .expect("reopen a");

    let strict = graph::search_expand(&a, "expandanchor", 1, None, SearchExpandOptions::default())
        .expect("strict expand");
    assert_eq!(strict.all_logical_ids, vec!["a:root"]);

    let expanded = graph::search_expand(
        &a,
        "expandanchor",
        1,
        None,
        SearchExpandOptions::default()
            .with_view(ReadView { include_out_of_window: true, ..ReadView::default() }),
    )
    .expect("out-of-window expand");
    assert_eq!(
        expanded.search_hits.iter().map(|hit| hit.id.value.as_str()).collect::<Vec<_>>(),
        vec!["a:root"]
    );
    assert_eq!(
        expanded.expanded.iter().map(|(n, _)| n.logical_id.as_str()).collect::<Vec<_>>(),
        vec!["a:past"]
    );
    assert!(!expanded.all_logical_ids.iter().any(|id| id.starts_with("b:")));

    let read_context = ReadContextV1::new(ReadView::default(), SearchFilter::default())
        .expect("valid read context");
    let context: FrozenReadContextV1 = a.freeze_read_context(&read_context).expect("freeze");
    let frozen_view_err = a
        .search_expand_frozen(
            "expandanchor",
            &context,
            1,
            SearchExpandOptions::default().with_view(ReadView::default()),
        )
        .expect_err("search_expand_frozen rejects SearchExpandOptions.view");
    assert_eq!(frozen_view_err.kind(), ErrorKind::InvalidArgument);

    a.close().expect("close a");
}
