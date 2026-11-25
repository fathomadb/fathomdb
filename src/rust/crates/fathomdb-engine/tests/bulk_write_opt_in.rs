use std::path::Path;
use std::sync::Arc;

use fathomdb_engine::lifecycle::{Event, EventCategory, Phase, Subscriber};
use fathomdb_engine::{
    ActuationBatchV1, ActuationOperationV1, ActuationOutcomeV1, ArtifactRevisionId, CanonicalHash,
    EmbedderChoice, EndpointValidationMode, Engine, EngineConfig, EngineError,
    GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, InitialState, LifecycleState,
    PreparedWrite, PropertyMap, PropertyValue, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1,
    ReadView, ScopePolicy, SearchFilter, SourceId, SourceLocator, SourceRevisionId,
    SourceVersionId, TraversalDirection, WriteOptions, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn open_scoped(path: &Path, policy: ScopePolicy) -> Engine {
    Engine::open_with_choice_and_config(
        path,
        EmbedderChoice::None,
        EngineConfig::default().with_scope_policy(policy),
    )
    .expect("open scoped engine")
    .engine
}

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:bulk-write").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn pending_node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("test:bulk-write").unwrap(),
        logical_id: Some(id.into()),
        state: InitialState::Pending,
        reason: Some("quarantined".into()),
        valid_from: None,
        valid_until: None,
        properties: None,
    }
}

fn edge(logical_id: Option<&str>, kind: &str, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("test:bulk-write").unwrap(),
        logical_id: logical_id.map(str::to_owned),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn write_options_constructors_and_defaults_are_strict() {
    assert_eq!(EndpointValidationMode::default(), EndpointValidationMode::Strict);
    assert_eq!(WriteOptions::default(), WriteOptions::strict());
    assert_eq!(WriteOptions::default().endpoint_validation, EndpointValidationMode::Strict);
    assert_eq!(
        WriteOptions::bulk_non_strict().endpoint_validation,
        EndpointValidationMode::BulkNonStrict
    );
    assert_eq!(
        WriteOptions::strict().with_endpoint_validation(EndpointValidationMode::BulkNonStrict),
        WriteOptions::bulk_non_strict()
    );
    assert_eq!(
        WriteOptions::bulk_non_strict().with_endpoint_validation(EndpointValidationMode::Strict),
        WriteOptions::strict()
    );
}

#[test]
fn default_write_remains_strict_for_missing_inactive_superseded_and_cross_scope_endpoints() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "strict-default");
    {
        let alpha = open_scoped(&path, ScopePolicy::single("alpha"));
        let r = alpha
            .write(&[
                node("n-active", "active node"),
                pending_node("n-pending", "pending node"),
                node("n-deleted", "deleted node"),
                node("n-purged", "purged node"),
                node("n-superseded", "v1"),
            ])
            .unwrap();
        assert_eq!(r.dangling_edge_endpoints, 0);

        alpha.transition("n-deleted", LifecycleState::Deleted, Some("test".into())).unwrap();
        alpha.transition("n-purged", LifecycleState::Deleted, Some("test".into())).unwrap();
        alpha.purge("n-purged").unwrap();
        // Supersede n-superseded with a pending version so its active row is superseded.
        alpha.write(&[pending_node("n-superseded", "v2 pending")]).unwrap();

        let count_before =
            alpha.query_i64_col_for_test("SELECT COUNT(*) FROM canonical_edges").unwrap();
        assert_eq!(count_before, vec![0]);

        // Missing endpoint rolls back whole batch atomically (including the companion node).
        let err = alpha
            .write(&[node("companion-1", "must roll back"), edge(None, "rel", "n-active", "ghost")])
            .unwrap_err();
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        assert_eq!(
            alpha
                .query_i64_col_for_test(
                    "SELECT COUNT(*) FROM canonical_nodes WHERE logical_id = 'companion-1'"
                )
                .unwrap(),
            vec![0]
        );

        // Inactive (pending / deleted / purged) and superseded endpoints all fail strictly.
        for bad_target in ["n-pending", "n-deleted", "n-purged", "n-superseded"] {
            let err = alpha.write(&[edge(None, "rel", "n-active", bad_target)]).unwrap_err();
            assert!(
                matches!(err, EngineError::WriteValidation),
                "target {bad_target}: expected WriteValidation, got {err:?}"
            );
        }
        alpha.close().unwrap();
    }

    // Cross-scope endpoint under strict default also rejects with WriteValidation.
    {
        let beta = open_scoped(&path, ScopePolicy::single("beta"));
        beta.write(&[node("beta-only", "in beta")]).unwrap();
        let err = beta.write(&[edge(None, "rel", "beta-only", "n-active")]).unwrap_err();
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");
        beta.close().unwrap();
    }
}

#[test]
fn actuate_remains_strict_only_and_refuses_dangling_derived_edges() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "actuate-strict");
    let opened = Engine::open(&path).unwrap();
    let engine = &opened.engine;

    let src_body = "canonical source body";
    let src_node = ProvenancedNodeV1 {
        kind: "doc".into(),
        body: src_body.into(),
        source_id: SourceId::new("actuate-src").unwrap(),
        logical_id: Some("src-1".into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        provenance: WriteProvenanceV1::canonical(
            ArtifactRevisionId::new("src-r1").unwrap(),
            SourceVersionId::new("src-v1").unwrap(),
        ),
        properties: None,
    };
    engine.write(&[PreparedWrite::ProvenancedNode(src_node)]).unwrap();

    let dangling_edge = ProvenancedEdgeV1 {
        kind: "supports".into(),
        from: "src-1".into(),
        to: "missing-target".into(),
        source_id: SourceId::new("actuate-src").unwrap(),
        logical_id: Some("e-actuate".into()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        provenance: WriteProvenanceV1::derived(
            ArtifactRevisionId::new("edge-r1").unwrap(),
            SourceVersionId::new("src-v1").unwrap(),
            SourceRevisionId::new("src-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(src_body.as_bytes())).unwrap(),
        ),
        properties: None,
    };
    let batch = ActuationBatchV1::new(
        "op-dangling-1",
        vec![ActuationOperationV1::PutDerivedEdge(dangling_edge)],
    )
    .unwrap();
    let receipt = engine.actuate(batch).unwrap();
    assert_eq!(receipt.outcome, ActuationOutcomeV1::Refused);
    assert_eq!(
        engine.query_i64_col_for_test("SELECT COUNT(*) FROM canonical_edges").unwrap(),
        vec![0]
    );
}

#[test]
fn bulk_non_strict_admits_out_of_order_edges_with_truthful_dangling_counts_and_resolves_later() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "bulk-counts-and-resolve");
    let opened = Engine::open(&path).unwrap();
    let engine = &opened.engine;

    // 1. Node-only batch under BulkNonStrict -> 0 dangling.
    let r_nodes = engine
        .write_with_options(&[node("n1", "first node alpha")], &WriteOptions::bulk_non_strict())
        .unwrap();
    assert_eq!(r_nodes.dangling_edge_endpoints, 0);

    // 2. Edges with 0, 1, and 2 missing endpoints in separate batches.
    let r_both_missing = engine
        .write_with_options(
            &[edge(Some("e-2missing"), "links", "ghost-a", "ghost-b")],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();
    assert_eq!(r_both_missing.dangling_edge_endpoints, 2);

    let r_one_missing = engine
        .write_with_options(
            &[edge(Some("e-1missing"), "links", "n1", "n2")],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();
    assert_eq!(r_one_missing.dangling_edge_endpoints, 1);

    // 3. Same-batch node + edge resolves (0 dangling).
    let r_same_batch = engine
        .write_with_options(
            &[
                edge(Some("e-same-batch"), "links", "n1", "n3"),
                node("n3", "third node in same batch"),
            ],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();
    assert_eq!(r_same_batch.dangling_edge_endpoints, 0);

    // 4. Same-batch edge supersession counts ONLY surviving active edges.
    let r_superseded_to_clean = engine
        .write_with_options(
            &[
                edge(Some("e-supersede-1"), "links", "ghost-x", "ghost-y"),
                edge(Some("e-supersede-1"), "links", "n1", "n3"),
            ],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();
    assert_eq!(r_superseded_to_clean.dangling_edge_endpoints, 0);

    let r_superseded_to_dangling = engine
        .write_with_options(
            &[
                edge(Some("e-supersede-2"), "links", "n1", "n3"),
                edge(Some("e-supersede-2"), "links", "ghost-z", "n1"),
            ],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();
    assert_eq!(r_superseded_to_dangling.dangling_edge_endpoints, 1);

    // 5. Subsequent batch writing missing node `n2` resolves `e-1missing` during graph_expand
    //    and search without rewriting the edge!
    let r_resolve = engine
        .write_with_options(
            &[node("n2", "second node beta arriving later")],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();
    assert_eq!(r_resolve.dangling_edge_endpoints, 0);

    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("n1")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        2,
        20,
        1_000,
    );
    let expanded = engine.graph_expand(&req).unwrap();
    let targets: Vec<&str> = expanded.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert!(
        targets.contains(&"n2"),
        "expected n2 reachable via previously-dangling edge: {targets:?}"
    );
    assert!(targets.contains(&"n3"), "expected n3 reachable: {targets:?}");

    let search = engine.search_text_only("beta arriving later").unwrap();
    assert_eq!(search.results.len(), 1);
    assert!(search.results[0].body.contains("second node beta arriving later"));
}

#[test]
fn bulk_non_strict_atomically_refuses_cross_scope_endpoints_with_scope_violation() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "bulk-cross-scope");

    {
        let alpha = open_scoped(&path, ScopePolicy::single("alpha"));
        alpha
            .write(&[
                node("alpha-secret", "belongs only to alpha"),
                node("shared-id", "alpha copy of shared-id"),
            ])
            .unwrap();
        alpha.close().unwrap();
    }

    {
        let beta = open_scoped(&path, ScopePolicy::single("beta"));
        beta.write(&[
            node("beta-node", "belongs to beta"),
            node("shared-id", "beta copy of shared-id"),
        ])
        .unwrap();

        let cursor_before = beta
            .query_i64_col_for_test("SELECT COALESCE(MAX(write_cursor), 0) FROM canonical_nodes")
            .unwrap()[0];
        let edge_count_before =
            beta.query_i64_col_for_test("SELECT COUNT(*) FROM canonical_edges").unwrap()[0];

        // Cross-scope active endpoint must abort atomically with ScopeViolation!
        let err = beta
            .write_with_options(
                &[
                    node("beta-rolled-back", "must not persist"),
                    edge(Some("e-cross"), "links", "beta-node", "alpha-secret"),
                ],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::ScopeViolation { .. }),
            "expected ScopeViolation, got {err:?}"
        );

        assert_eq!(
            beta.query_i64_col_for_test(
                "SELECT COUNT(*) FROM canonical_nodes WHERE logical_id = 'beta-rolled-back'"
            )
            .unwrap(),
            vec![0],
            "companion node in failed cross-scope batch must be rolled back"
        );
        assert_eq!(
            beta.query_i64_col_for_test("SELECT COUNT(*) FROM canonical_edges").unwrap()[0],
            edge_count_before
        );
        assert_eq!(
            beta.query_i64_col_for_test(
                "SELECT COALESCE(MAX(write_cursor), 0) FROM canonical_nodes"
            )
            .unwrap()[0],
            cursor_before
        );

        // Edge to `shared-id` (active in BOTH beta and alpha) resolves in beta with 0 dangling
        // and does NOT falsely trigger ScopeViolation.
        let r_shared = beta
            .write_with_options(
                &[edge(Some("e-shared"), "links", "beta-node", "shared-id")],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap();
        assert_eq!(r_shared.dangling_edge_endpoints, 0);

        // Edge to genuinely missing id (not active in ANY scope) IS admitted as dangling = 1.
        let r_dangling = beta
            .write_with_options(
                &[edge(Some("e-missing-ok"), "links", "beta-node", "nowhere-yet")],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap();
        assert_eq!(r_dangling.dangling_edge_endpoints, 1);

        beta.close().unwrap();
    }
}

#[test]
fn bulk_non_strict_counts_inactive_and_superseded_same_scope_nodes_as_dangling() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "bulk-inactive-superseded");
    let opened = Engine::open(&path).unwrap();
    let engine = &opened.engine;

    engine
        .write(&[
            node("n-active", "active"),
            pending_node("n-pending", "pending"),
            node("n-deleted", "deleted"),
            node("n-purged", "purged"),
            node("n-sup", "v1 active"),
        ])
        .unwrap();
    engine.transition("n-deleted", LifecycleState::Deleted, Some("test".into())).unwrap();
    engine.transition("n-purged", LifecycleState::Deleted, Some("test".into())).unwrap();
    engine.purge("n-purged").unwrap();
    engine.write(&[pending_node("n-sup", "v2 pending")]).unwrap();

    for target in ["n-pending", "n-deleted", "n-purged", "n-sup"] {
        let receipt = engine
            .write_with_options(
                &[edge(None, "rel", "n-active", target)],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap();
        assert_eq!(
            receipt.dangling_edge_endpoints, 1,
            "target {target} is not active+non-superseded and must count as dangling"
        );
    }
}

#[test]
fn bulk_non_strict_keeps_batch_invariants_and_read_only_scope_strict() {
    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "bulk-invariants");
    {
        let engine = open_scoped(&path, ScopePolicy::single("alpha"));
        let seed = engine.write(&[node("seed", "seed body")]).unwrap();
        let seed_cursor = seed.cursor;

        // Invalid validity window (valid_from >= valid_until) rolls back atomically.
        let bad_window = PreparedWrite::Node {
            kind: "doc".into(),
            body: "bad window".into(),
            source_id: SourceId::new("test:bulk-write").unwrap(),
            logical_id: Some("bad-win".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: Some(200),
            valid_until: Some(100),
            properties: None,
        };
        let err = engine
            .write_with_options(
                &[edge(None, "rel", "ghost-1", "ghost-2"), bad_window],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");

        // Invalid property value (non-finite float) rolls back atomically.
        let mut bad_props = PropertyMap::new();
        bad_props.insert("score".to_string(), PropertyValue::Float(f64::NAN));
        let bad_prop_node = PreparedWrite::Node {
            kind: "doc".into(),
            body: "bad prop".into(),
            source_id: SourceId::new("test:bulk-write").unwrap(),
            logical_id: Some("bad-prop".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(bad_props),
        };
        let err = engine
            .write_with_options(
                &[edge(None, "rel", "ghost-1", "ghost-2"), bad_prop_node],
                &WriteOptions::bulk_non_strict(),
            )
            .unwrap_err();
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");

        // Empty batch rejected without advancing cursor.
        let err = engine.write_with_options(&[], &WriteOptions::bulk_non_strict()).unwrap_err();
        assert!(matches!(err, EngineError::WriteValidation), "got {err:?}");

        assert_eq!(
            engine.query_i64_col_for_test("SELECT COUNT(*) FROM canonical_edges").unwrap(),
            vec![0]
        );
        // Verify next_cursor did not advance on rolled-back batches.
        let next = engine.write(&[node("after", "after")]).unwrap();
        assert_eq!(next.cursor, seed_cursor + 1);

        engine.close().unwrap();
    }

    // Read-only ScopePolicy refuses write_with_options with ScopeViolation.
    {
        let ro = open_scoped(&path, ScopePolicy::read_only(["alpha"]));
        let err = ro
            .write_with_options(&[node("ro-fail", "nope")], &WriteOptions::bulk_non_strict())
            .unwrap_err();
        assert!(
            matches!(err, EngineError::ScopeViolation { .. }),
            "expected ScopeViolation, got {err:?}"
        );
        ro.close().unwrap();
    }
}

#[test]
fn write_with_options_preserves_lifecycle_instrumentation_envelope() {
    struct Capture(std::sync::Mutex<Vec<(Phase, EventCategory, Option<&'static str>)>>);
    impl Subscriber for Capture {
        fn on_event(&self, event: &Event) {
            self.0.lock().unwrap().push((event.phase, event.category, event.code));
        }
    }

    let dir = TempDir::new().unwrap();
    let path = db_path(&dir, "bulk-events");
    let opened = Engine::open(&path).unwrap();
    let engine = &opened.engine;

    let capture = Arc::new(Capture(std::sync::Mutex::new(Vec::new())));
    let _sub = engine.subscribe(Arc::clone(&capture) as Arc<dyn Subscriber>);

    engine
        .write_with_options(
            &[edge(None, "links", "missing-1", "missing-2")],
            &WriteOptions::bulk_non_strict(),
        )
        .unwrap();

    let err = engine
        .write_with_options(
            &[edge(None, "links", "missing-1", "missing-2")],
            &WriteOptions::strict(),
        )
        .unwrap_err();
    assert!(matches!(err, EngineError::WriteValidation));

    let captured = capture.0.lock().unwrap().clone();
    assert!(captured.contains(&(Phase::Started, EventCategory::Writer, None)));
    assert!(captured.contains(&(Phase::Finished, EventCategory::Writer, None)));
    assert!(captured.iter().any(|(p, c, code)| {
        *p == Phase::Failed && *c == EventCategory::Writer && code.is_some()
    }));
    assert!(captured.iter().any(|(p, c, code)| {
        *p == Phase::Failed && *c == EventCategory::Error && code.is_some()
    }));
}
