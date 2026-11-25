use fathomdb_sdk::{
    graph, EndpointValidationMode, Engine, ErrorKind, InitialState, NeighborsOptions, OpenOptions,
    PreparedWrite, ScopePolicy, SourceId, WriteOptions,
};

fn source(name: &str) -> SourceId {
    SourceId::new(name).expect("valid source id")
}

fn node(logical_id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "entity".into(),
        body: body.into(),
        source_id: source("sdk-bulk"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn edge(from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "links".into(),
        from: from.into(),
        to: to.into(),
        source_id: source("sdk-bulk"),
        logical_id: None,
        body: Some("edge body".into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
    }
}

#[test]
fn sdk_write_with_options_enforces_strict_default_bulk_opt_in_guards_and_scope() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db = dir.path().join("sdk-bulk.sqlite");
    let engine = Engine::open(&db, OpenOptions::default()).expect("open");

    // Default Engine::write and strict WriteOptions reject missing endpoints.
    let err = engine.write(&[edge("n:a", "n:b")]).expect_err("strict write must reject");
    assert_eq!(err.kind(), ErrorKind::WriteValidation);
    let err = engine
        .write_with_options(&[edge("n:a", "n:b")], &WriteOptions::default())
        .expect_err("strict write_with_options must reject");
    assert_eq!(err.kind(), ErrorKind::WriteValidation);

    // SDK boundary guard still rejects embedded NUL bytes on write_with_options (AC-068a).
    let nul_err = engine
        .write_with_options(&[edge("n:\0bad", "n:b")], &WriteOptions::bulk_non_strict())
        .expect_err("guard must reject embedded NUL");
    assert_eq!(nul_err.kind(), ErrorKind::WriteValidation);
    assert!(nul_err.to_string().contains("NUL"), "expected guard NUL rejection, got: {nul_err}");

    // BulkNonStrict admits out-of-order edges with truthful dangling counts.
    let receipt = engine
        .write_with_options(
            &[edge("n:a", "n:b")],
            &WriteOptions::default()
                .with_endpoint_validation(EndpointValidationMode::BulkNonStrict),
        )
        .expect("bulk write succeeds");
    assert_eq!(receipt.dangling_edge_endpoints, 2);

    // Writing one endpoint and a second edge leaves 1 dangling on the new edge.
    let r2 = engine
        .write_with_options(
            &[node("n:a", "alpha"), edge("n:a", "n:c")],
            &WriteOptions::bulk_non_strict(),
        )
        .expect("partial bulk write succeeds");
    assert_eq!(r2.dangling_edge_endpoints, 1);

    // Writing the missing nodes resolves traversal without rewriting edges.
    let r3 = engine
        .write_with_options(
            &[node("n:b", "beta"), node("n:c", "gamma")],
            &WriteOptions::bulk_non_strict(),
        )
        .expect("node-only bulk write succeeds");
    assert_eq!(r3.dangling_edge_endpoints, 0);

    let neighbors = graph::neighbors(&engine, "n:a", 1, NeighborsOptions::default())
        .expect("neighbors resolve");
    let mut ids: Vec<&str> = neighbors.iter().map(|n| n.logical_id.as_str()).collect();
    ids.sort_unstable();
    assert_eq!(ids, vec!["n:b", "n:c"]);
    engine.close().expect("close");

    // Cross-scope active endpoint is refused atomically with ScopeViolation even under BulkNonStrict.
    let scoped_db = dir.path().join("sdk-bulk-scoped.sqlite");
    let scope_a = Engine::open(
        &scoped_db,
        OpenOptions::default().with_scope_policy(ScopePolicy::single("scope-a")),
    )
    .expect("open scope-a");
    scope_a.write(&[node("shared:a", "in scope a")]).expect("seed scope-a");
    scope_a.close().expect("close scope-a");

    let scope_b = Engine::open(
        &scoped_db,
        OpenOptions::default().with_scope_policy(ScopePolicy::single("scope-b")),
    )
    .expect("open scope-b");
    let cross_err = scope_b
        .write_with_options(
            &[node("local:b", "in scope b"), edge("local:b", "shared:a")],
            &WriteOptions::bulk_non_strict(),
        )
        .expect_err("cross-scope endpoint must be refused");
    assert_eq!(cross_err.kind(), ErrorKind::ScopeViolation);
    scope_b.close().expect("close scope-b");
}
