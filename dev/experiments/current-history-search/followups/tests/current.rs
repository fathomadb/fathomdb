use current_history_followups::current::Certificate;
#[test]
fn certificate_requires_exact_scope_generation_and_half_open_time() {
    let cert = Certificate {
        published_at: 10,
        next_boundary: 20,
        visibility: 3,
        scope: "strict-v1".into(),
    };
    assert!(cert.covers(10, 3, "strict-v1"));
    assert!(cert.covers(19, 3, "strict-v1"));
    assert!(!cert.covers(20, 3, "strict-v1"), "clock boundary must invalidate even with no write");
    assert!(!cert.covers(9, 3, "strict-v1"));
    assert!(!cert.covers(10, 4, "strict-v1"));
    assert!(!cert.covers(10, 3, "archive"));
}

fn timed(
    id: &str,
    body: &str,
    from: Option<i64>,
    until: Option<i64>,
) -> fathomdb_engine::PreparedWrite {
    fathomdb_engine::PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: fathomdb_engine::SourceId::new("source").unwrap(),
        logical_id: Some(id.into()),
        state: fathomdb_engine::InitialState::Active,
        reason: None,
        valid_from: from,
        valid_until: until,
    }
}
#[test]
fn actual_readonly_pool_refreshes_both_outgoing_and_previously_absent_future_rows() {
    use current_history_followups::current::CurrentStore;
    let dir = tempfile::tempdir().unwrap();
    let store = CurrentStore::open(&dir.path().join("current.sqlite")).unwrap();
    store
        .write(&[
            timed("old", "needle old", None, Some(100)),
            timed("future", "needle future", Some(100), None),
        ])
        .unwrap();
    store.bootstrap(50).unwrap();
    assert_eq!(store.search("needle", 50, 10).unwrap().rows[0].body, "needle old");
    let after = store.search("needle", 100, 10).unwrap();
    assert_eq!(
        after.rows.iter().map(|r| r.body.as_str()).collect::<Vec<_>>(),
        vec!["needle future"]
    );
    store.close().unwrap();
}
