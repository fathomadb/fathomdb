//! 0.8.31 Slice 40 (`HISTORY-GRAPH`) — Layer 1 historical/bitemporal graph
//! reconstruction, declared-key inline parity, epoch-ms resolution, closure
//! barriers, erasure subordination, pushdown/self-loop bounds, and proptest
//! snapshot BFS oracle.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use fathomdb_engine::{
    ArtifactRevisionId, CanonicalHash, Engine, EngineError, EvidenceArtifactLifecycleV1,
    EvidenceErrorReasonV1, GraphEvidenceArtifactV1, GraphEvidenceResolveRequestV1,
    GraphExpandRequestV1, GraphExpansionErrorReasonV1, GraphReadContextV1, GraphSeedV1, IdSpace,
    InitialState, LifecycleState, PreparedWrite, ProjectionRole, ProjectionSpec,
    PropertyComparisonOp, PropertyEntityClass, PropertyMap, PropertyPredicate, PropertyValue,
    ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SourceDependencyRegistrationV1, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    StructuralDependencyStateV1, TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use tempfile::TempDir;

fn current_ctx(valid_as_of: Option<i64>, eligibility: SearchFilter) -> GraphReadContextV1 {
    GraphReadContextV1::Current {
        schema_version: 1,
        context: ReadContextV1::new(ReadView { valid_as_of, ..ReadView::default() }, eligibility)
            .unwrap(),
    }
}

fn explicit_req(
    logical_ids: &[&str],
    direction: TraversalDirection,
    valid_as_of: Option<i64>,
    max_depth: u32,
) -> GraphExpandRequestV1 {
    GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: logical_ids.iter().map(|id| IdSpace::logical(*id)).collect(),
        },
        direction,
        current_ctx(valid_as_of, SearchFilter::default()),
        max_depth,
        50,
        1_000,
    )
}

fn node_write(
    logical_id: &str,
    kind: &str,
    body: &str,
    source: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source).unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
        properties,
    }
}

#[allow(clippy::too_many_arguments)]
fn edge_write(
    logical_id: Option<&str>,
    kind: &str,
    from: &str,
    to: &str,
    source: &str,
    t_valid: Option<i64>,
    t_invalid: Option<i64>,
    temporal_fallback: Option<bool>,
    properties: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        logical_id: logical_id.map(Into::into),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source).unwrap(),
        body: None,
        t_valid,
        t_invalid,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback,
        properties,
    }
}

fn write_cursor(engine: &Engine, writes: &[PreparedWrite]) -> u64 {
    engine.write(writes).unwrap().cursor
}

fn sha256_hex(input: &str) -> String {
    Sha256::digest(input.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn graph_err_reason_path(err: EngineError) -> (GraphExpansionErrorReasonV1, String) {
    match err {
        EngineError::GraphExpansion(e) => (e.reason, e.field_path),
        other => panic!("expected EngineError::GraphExpansion, got {other:?}"),
    }
}

#[test]
fn bitemporal_four_quadrant_matrix_hydration_and_temporal_fallback_policy() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("bitemporal{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    // Revision 1 (knowledge time c1): root -> target_a with world validity [100, 200),
    // plus an edge with temporal_fallback = true to verify AsOf exclusion policy.
    let p_node_v1 = BTreeMap::from([
        ("version".into(), PropertyValue::Int(1)),
        ("label".into(), PropertyValue::String("alpha-v1".into())),
    ]);
    let p_edge_v1 = BTreeMap::from([("weight".into(), PropertyValue::Int(10))]);
    let c1 = write_cursor(
        &engine,
        &[
            node_write("root", "doc", "root alpha v1", "src:bi", Some(50), Some(500), None),
            node_write(
                "target_a",
                "claim",
                "target alpha v1",
                "src:bi",
                Some(100),
                Some(200),
                Some(p_node_v1.clone()),
            ),
            node_write("fallback_t", "claim", "fallback target", "src:bi", None, None, None),
            edge_write(
                Some("e-root-a"),
                "supports",
                "root",
                "target_a",
                "src:bi",
                Some(100),
                Some(200),
                Some(false),
                Some(p_edge_v1.clone()),
            ),
            edge_write(
                Some("e-fallback"),
                "supports",
                "root",
                "fallback_t",
                "src:bi",
                None,
                None,
                Some(true),
                None,
            ),
        ],
    );

    // Revision 2 (knowledge time c2): supersede target_a (kind/body/properties/validity changed)
    // and supersede e-root-a with world validity [200, 300) and new properties.
    let p_node_v2 = BTreeMap::from([
        ("version".into(), PropertyValue::Int(2)),
        ("label".into(), PropertyValue::String("alpha-v2".into())),
    ]);
    let p_edge_v2 = BTreeMap::from([("weight".into(), PropertyValue::Int(20))]);
    let c2 = write_cursor(
        &engine,
        &[
            node_write(
                "target_a",
                "finding",
                "target alpha v2",
                "src:bi",
                Some(200),
                Some(300),
                Some(p_node_v2.clone()),
            ),
            edge_write(
                Some("e-root-a"),
                "supports",
                "root",
                "target_a",
                "src:bi",
                Some(200),
                Some(300),
                Some(false),
                Some(p_edge_v2.clone()),
            ),
        ],
    );
    assert!(c2 > c1);

    // Quadrant 1: (as_of = c1, valid_as_of = 150) -> sees v1 node + v1 edge, excludes temporal_fallback=true.
    let q1 = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(150), 1)
                .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert_eq!(q1.targets.len(), 1);
    assert_eq!(q1.targets[0].logical_id, "target_a");
    assert_eq!(q1.targets[0].kind, "claim");
    assert_eq!(q1.targets[0].body, "target alpha v1");
    assert!(q1.targets[0].write_cursor <= c1);
    assert_eq!(q1.targets[0].properties, Some(p_node_v1));
    assert_eq!(q1.targets[0].origin.terminal_edge_properties, Some(p_edge_v1));

    // Quadrant 2: (as_of = c1, valid_as_of = 250) -> v1 is out-of-window at 250, v2 doesn't exist yet -> empty.
    let q2 = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(250), 1)
                .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert!(q2.targets.is_empty());

    // Quadrant 3: (as_of = c2, valid_as_of = 150) -> v1 superseded at c2, v2 not yet valid at 150 -> empty.
    let q3 = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(150), 1)
                .with_as_of_write_cursor(c2),
        )
        .unwrap();
    assert!(q3.targets.is_empty());

    // Quadrant 4: (as_of = c2, valid_as_of = 250) -> sees v2 node + v2 edge, still excludes temporal_fallback=true.
    let q4 = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(250), 1)
                .with_as_of_write_cursor(c2),
        )
        .unwrap();
    assert_eq!(q4.targets.len(), 1);
    assert_eq!(q4.targets[0].logical_id, "target_a");
    assert_eq!(q4.targets[0].kind, "finding");
    assert_eq!(q4.targets[0].body, "target alpha v2");
    assert!(q4.targets[0].write_cursor > c1 && q4.targets[0].write_cursor <= c2);
    assert_eq!(q4.targets[0].properties, Some(p_node_v2));
    assert_eq!(q4.targets[0].origin.terminal_edge_properties, Some(p_edge_v2));

    // Reconcile temporal_fallback policy: Current sees fallback_t, whereas AsOf(c2) explicitly excludes temporal_fallback != 0.
    let current = engine
        .graph_expand(&explicit_req(&["root"], TraversalDirection::Outgoing, Some(250), 1))
        .unwrap();
    let current_ids: BTreeSet<_> = current.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(current_ids, BTreeSet::from(["target_a", "fallback_t"]));

    // Query seed at as_of = c1 matches historical body ("root alpha v1").
    let q_seed = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Query { schema_version: 1, text: "alpha".into(), ranked_limit: 5 },
                TraversalDirection::Outgoing,
                current_ctx(Some(150), SearchFilter::default()),
                1,
                10,
                100,
            )
            .with_as_of_write_cursor(c1),
        )
        .unwrap();
    let seed_ids: BTreeSet<_> = q_seed.seeds.iter().map(|s| s.logical_id.as_str()).collect();
    assert_eq!(seed_ids, BTreeSet::from(["root", "target_a"]));

    engine.close().unwrap();
}

#[test]
fn declared_filterable_keys_evaluate_inline_on_superseded_historical_revisions() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("filterable-inline{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "tier".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "score".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "edge_strength".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Edge,
                },
            ],
            &[],
        )
        .unwrap();

    // v1: node has body JSON tier="body_stale" overridden by properties tier="gold", score=95;
    // edge has declared Filterable edge_strength=90 and undeclared tag="v1".
    let c1 = write_cursor(
        &engine,
        &[
            node_write(
                "seed",
                "note",
                r#"{"tier":"gold","score":99}"#,
                "src:filt",
                None,
                None,
                Some(BTreeMap::from([
                    ("tier".into(), PropertyValue::String("gold".into())),
                    ("score".into(), PropertyValue::Int(99)),
                ])),
            ),
            node_write(
                "target",
                "note",
                r#"{"tier":"body_stale","score":1}"#,
                "src:filt",
                None,
                None,
                Some(BTreeMap::from([
                    ("tier".into(), PropertyValue::String("gold".into())),
                    ("score".into(), PropertyValue::Int(95)),
                    ("undeclared_node".into(), PropertyValue::String("match".into())),
                ])),
            ),
            edge_write(
                Some("e1"),
                "supports",
                "seed",
                "target",
                "src:filt",
                None,
                None,
                None,
                Some(BTreeMap::from([
                    ("edge_strength".into(), PropertyValue::Int(90)),
                    ("undeclared_edge".into(), PropertyValue::String("keep".into())),
                ])),
            ),
        ],
    );

    // v2: supersede seed, target, and edge with completely different values so v1's
    // canonical_attributes and canonical_property_values rows are PURGED!
    let _c2 = write_cursor(
        &engine,
        &[
            node_write(
                "seed",
                "note",
                r#"{"tier":"bronze","score":5}"#,
                "src:filt",
                None,
                None,
                Some(BTreeMap::from([
                    ("tier".into(), PropertyValue::String("bronze".into())),
                    ("score".into(), PropertyValue::Int(5)),
                ])),
            ),
            node_write(
                "target",
                "note",
                r#"{"tier":"bronze","score":5}"#,
                "src:filt",
                None,
                None,
                Some(BTreeMap::from([
                    ("tier".into(), PropertyValue::String("bronze".into())),
                    ("score".into(), PropertyValue::Int(5)),
                    ("undeclared_node".into(), PropertyValue::String("nope".into())),
                ])),
            ),
            edge_write(
                Some("e1"),
                "supports",
                "seed",
                "target",
                "src:filt",
                None,
                None,
                None,
                Some(BTreeMap::from([
                    ("edge_strength".into(), PropertyValue::Int(10)),
                    ("undeclared_edge".into(), PropertyValue::String("drop".into())),
                ])),
            ),
        ],
    );

    // Eligibility uses BOTH attributes (tier="gold") and property_predicates (score >= 90),
    // plus graph_expand uses declared Filterable edge_predicates (edge_strength >= 80) AND
    // target_predicates (score == 95) alongside undeclared predicates.
    let mut eligibility = SearchFilter::default();
    eligibility.attributes.push(("tier".into(), "gold".into()));
    eligibility.property_predicates.push(PropertyPredicate {
        key: "score".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Int(90),
    });

    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("seed")] },
        TraversalDirection::Outgoing,
        current_ctx(Some(1_700_000_000), eligibility.clone()),
        1,
        10,
        100,
    )
    .with_edge_predicates(vec![
        PropertyPredicate {
            key: "edge_strength".into(),
            op: PropertyComparisonOp::Gte,
            value: PropertyValue::Int(80),
        },
        PropertyPredicate {
            key: "undeclared_edge".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::String("keep".into()),
        },
    ])
    .with_target_predicates(vec![
        PropertyPredicate {
            key: "score".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::Int(95),
        },
        PropertyPredicate {
            key: "undeclared_node".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::String("match".into()),
        },
    ])
    .with_as_of_write_cursor(c1);

    let res = engine.graph_expand(&req).unwrap();
    assert_eq!(res.targets.len(), 1, "superseded v1 must match via inline evaluation");
    assert_eq!(res.targets[0].logical_id, "target");

    // Verify properties > body precedence on superseded v1: body had tier="body_stale",
    // so filtering eligibility attribute tier="body_stale" must NOT match target.
    let mut stale_body_elig = SearchFilter::default();
    stale_body_elig.attributes.push(("tier".into(), "body_stale".into()));
    let stale_res = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Query {
                    schema_version: 1,
                    text: "body_stale".into(),
                    ranked_limit: 5,
                },
                TraversalDirection::Outgoing,
                current_ctx(Some(1_700_000_000), stale_body_elig),
                1,
                10,
                100,
            )
            .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert!(stale_res.seeds.is_empty());

    // Query seed with eligibility attributes + property_predicates also finds superseded v1!
    let q_res = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Query { schema_version: 1, text: "gold".into(), ranked_limit: 5 },
                TraversalDirection::Outgoing,
                current_ctx(Some(1_700_000_000), eligibility),
                1,
                10,
                100,
            )
            .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert_eq!(q_res.seeds.len(), 1);
    assert_eq!(q_res.seeds[0].logical_id, "seed");
    assert_eq!(q_res.targets.len(), 1);
    assert_eq!(q_res.targets[0].logical_id, "target");

    engine.close().unwrap();
}

#[test]
fn as_of_epoch_ms_cursor_zero_pre_upgrade_and_future_cursor_semantics() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("epoch-cursor{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    let c1 = write_cursor(
        &engine,
        &[
            node_write("n1", "note", "first batch alpha", "src:ep", None, None, None),
            node_write("n2", "note", "second node v1", "src:ep", None, None, None),
            edge_write(Some("e1"), "links", "n1", "n2", "src:ep", None, None, None, None),
        ],
    );

    // Sleep briefly so batch 2 gets a distinct millisecond timestamp (or read stored timestamps).
    std::thread::sleep(std::time::Duration::from_millis(5));
    let c2 = write_cursor(
        &engine,
        &[
            node_write("n2", "note", "second node v2", "src:ep", None, None, None),
            node_write("n3", "note", "third node", "src:ep", None, None, None),
            edge_write(Some("e2"), "links", "n1", "n3", "src:ep", None, None, None, None),
        ],
    );

    let (ts1, ts2): (i64, i64) = {
        let conn = Connection::open(&path).unwrap();
        let t1: i64 = conn
            .query_row(
                "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = ?1",
                [i64::try_from(c1).unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        let t2: i64 = conn
            .query_row(
                "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = ?1",
                [i64::try_from(c2).unwrap()],
                |r| r.get(0),
            )
            .unwrap();
        (t1, t2)
    };
    assert!(ts2 >= ts1);

    // If ts2 == ts1 due to coarse clock, adjust ts1 back by 10ms in SQLite to deterministically test boundary.
    let (ts1, ts2) = if ts1 == ts2 {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "UPDATE _fathomdb_commit_timestamps SET committed_at_ms = ?1 WHERE write_cursor = ?2",
            rusqlite::params![ts1 - 10, i64::try_from(c1).unwrap()],
        )
        .unwrap();
        (ts1 - 10, ts2)
    } else {
        (ts1, ts2)
    };

    // as_of_epoch_ms at ts1 resolves to c1: sees n2 v1 and NOT n3.
    let r_ts1 = engine
        .graph_expand(
            &explicit_req(&["n1"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_epoch_ms(ts1),
        )
        .unwrap();
    assert_eq!(r_ts1.targets.len(), 1);
    assert_eq!(r_ts1.targets[0].logical_id, "n2");
    assert_eq!(r_ts1.targets[0].body, "second node v1");

    // as_of_epoch_ms at ts2 resolves to c2: sees n2 v2 and n3.
    let r_ts2 = engine
        .graph_expand(
            &explicit_req(&["n1"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_epoch_ms(ts2),
        )
        .unwrap();
    let ids_ts2: BTreeSet<_> = r_ts2.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(ids_ts2, BTreeSet::from(["n2", "n3"]));

    // Cursor 0 (direct and via pre-first-commit as_of_epoch_ms):
    // Query seed -> empty; Explicit seed -> GraphSeedUnavailable at /seed/logicalIds/0.
    for req_fn in [
        (|r: GraphExpandRequestV1| r.with_as_of_write_cursor(0))
            as fn(GraphExpandRequestV1) -> GraphExpandRequestV1,
        (|r: GraphExpandRequestV1| r.with_as_of_epoch_ms(0))
            as fn(GraphExpandRequestV1) -> GraphExpandRequestV1,
    ] {
        let q_empty = engine
            .graph_expand(&req_fn(GraphExpandRequestV1::new(
                GraphSeedV1::Query { schema_version: 1, text: "first".into(), ranked_limit: 5 },
                TraversalDirection::Outgoing,
                current_ctx(Some(1_700_000_000), SearchFilter::default()),
                1,
                10,
                100,
            )))
            .unwrap();
        assert!(q_empty.seeds.is_empty());
        assert!(q_empty.targets.is_empty());

        let err = engine
            .graph_expand(&req_fn(explicit_req(
                &["n1"],
                TraversalDirection::Outgoing,
                Some(1_700_000_000),
                1,
            )))
            .unwrap_err();
        assert_eq!(
            graph_err_reason_path(err),
            (GraphExpansionErrorReasonV1::GraphSeedUnavailable, "/seed/logicalIds/0".into())
        );
    }

    // Mutual exclusion refused by Engine::graph_expand at /asOfEpochMs.
    let mut both_req = explicit_req(&["n1"], TraversalDirection::Outgoing, Some(1_700_000_000), 1);
    both_req.as_of_write_cursor = Some(c1);
    both_req.as_of_epoch_ms = Some(ts1);
    let err = engine.graph_expand(&both_req).unwrap_err();
    assert_eq!(
        graph_err_reason_path(err),
        (GraphExpansionErrorReasonV1::GraphContextInvalid, "/asOfEpochMs".into())
    );

    // Future as_of_write_cursor (> current_write_cursor and u64::MAX) binds safely via i64::MAX.
    for future_cursor in [c2 + 10_000, u64::MAX] {
        let future_res = engine
            .graph_expand(
                &explicit_req(&["n1"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                    .with_as_of_write_cursor(future_cursor),
            )
            .unwrap();
        let ids: BTreeSet<_> = future_res.targets.iter().map(|t| t.logical_id.as_str()).collect();
        assert_eq!(ids, BTreeSet::from(["n2", "n3"]));
    }

    // Frozen snapshot + future as_of_write_cursor (> canonical_write_boundary and u64::MAX):
    // must NOT leak post-pin writes (FrozenReadBinding stays authoritative).
    let frozen_at_c2 = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    let frozen_future_before_c3 = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![IdSpace::logical("n1")],
                },
                TraversalDirection::Outgoing,
                GraphReadContextV1::Frozen { schema_version: 1, context: frozen_at_c2.clone() },
                1,
                10,
                100,
            )
            .with_as_of_write_cursor(u64::MAX),
        )
        .unwrap();
    let frozen_c2_ids: BTreeSet<_> =
        frozen_future_before_c3.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(frozen_c2_ids, BTreeSet::from(["n2", "n3"]));

    let c3 = write_cursor(
        &engine,
        &[
            node_write("n4", "note", "post-pin node", "src:ep", None, None, None),
            edge_write(Some("e3"), "links", "n1", "n4", "src:ep", None, None, None, None),
        ],
    );
    // Post-pin writes must NEVER leak through frozen_at_c2 even with as_of_write_cursor = u64::MAX!
    let stale_err = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![IdSpace::logical("n1")],
                },
                TraversalDirection::Outgoing,
                GraphReadContextV1::Frozen { schema_version: 1, context: frozen_at_c2 },
                1,
                10,
                100,
            )
            .with_as_of_write_cursor(u64::MAX),
        )
        .unwrap_err();
    assert!(
        matches!(stale_err, EngineError::FrozenRead(_)),
        "frozen context must reject post-pin writes even when as_of_write_cursor = u64::MAX; got {stale_err:?}"
    );
    let frozen_at_c3 = engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();
    let frozen_future = engine
        .graph_expand(
            &GraphExpandRequestV1::new(
                GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![IdSpace::logical("n1")],
                },
                TraversalDirection::Outgoing,
                GraphReadContextV1::Frozen { schema_version: 1, context: frozen_at_c3 },
                1,
                10,
                100,
            )
            .with_as_of_write_cursor(u64::MAX),
        )
        .unwrap();
    let frozen_ids: BTreeSet<_> =
        frozen_future.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(frozen_ids, BTreeSet::from(["n2", "n3", "n4"]));
    assert!(c3 > c2);

    // Pre-upgrade Schema 37->38 simulation: rows predating _fathomdb_commit_timestamps.
    // Delete c1's timestamp row to simulate a pre-upgrade commit; as_of_epoch_ms before ts2
    // resolves to cursor 0 (empty), while as_of_write_cursor(c1) still reconstructs c1!
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute(
            "DELETE FROM _fathomdb_commit_timestamps WHERE write_cursor = ?1",
            [i64::try_from(c1).unwrap()],
        )
        .unwrap();
    }
    let pre_upgrade_epoch = engine
        .graph_expand(
            &explicit_req(&["n1"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_epoch_ms(ts2 - 1),
        )
        .unwrap_err();
    assert_eq!(
        graph_err_reason_path(pre_upgrade_epoch),
        (GraphExpansionErrorReasonV1::GraphSeedUnavailable, "/seed/logicalIds/0".into())
    );
    let pre_upgrade_cursor = engine
        .graph_expand(
            &explicit_req(&["n1"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert_eq!(pre_upgrade_cursor.targets.len(), 1);
    assert_eq!(pre_upgrade_cursor.targets[0].body, "second node v1");

    engine.close().unwrap();
}

#[test]
fn historical_explanation_reports_registered_after_source_supersession() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("hist-expl{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    let source_body_v1 = "canonical source body v1";
    let digest_v1 = sha256_hex(source_body_v1);
    let c1 = write_cursor(
        &engine,
        &[
            node_write("root", "fact", "root", "control-owner", None, None, None),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "doc".into(),
                body: source_body_v1.into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
                properties: None,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("derived".into()),
                kind: "fact".into(),
                body: "derived from source v1".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest_v1).unwrap(),
                ),
                properties: None,
            }),
            edge_write(
                Some("root-derived"),
                "link",
                "root",
                "derived",
                "control-owner",
                None,
                None,
                None,
                None,
            ),
        ],
    );
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-1", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();

    // Supersede derived first (so derived-r1 remains an active historical revision, superseded_at = c2),
    // then supersede source (source-r1 -> source-r2, both active).
    let _c2 = write_cursor(
        &engine,
        &[node_write(
            "derived",
            "fact",
            "derived v2 independent",
            "control-owner",
            None,
            None,
            None,
        )],
    );
    let source_body_v2 = "canonical source body v2";
    let _c3 = write_cursor(
        &engine,
        &[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some("source".into()),
            kind: "doc".into(),
            body: source_body_v2.into(),
            source_id: SourceId::new("source-owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("source-r2").unwrap(),
                SourceVersionId::new("v2").unwrap(),
            ),
            properties: None,
        })],
    );

    let res = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_write_cursor(c1)
                .with_explanation(true),
        )
        .unwrap();
    assert_eq!(res.targets.len(), 1);
    assert_eq!(res.targets[0].body, "derived from source v1");
    let expl = res.explanation.expect("explanation sidecar");
    assert_eq!(expl.per_target.len(), 1);
    assert_eq!(
        expl.per_target[0].dependency_state,
        StructuralDependencyStateV1::Registered,
        "historical derived target whose canonical source was later superseded must still report Registered at as_of=c1"
    );

    engine.close().unwrap();
}

#[test]
fn current_state_transition_removes_entity_from_historical_reads() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("transitions{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    let c1 = write_cursor(
        &engine,
        &[
            node_write("root", "note", "root", "src:tr", None, None, None),
            node_write("t_unsup", "note", "unsuperseded v1", "src:tr", None, None, None),
            node_write("t_head_del", "note", "superseded-del v1", "src:tr", None, None, None),
            node_write("t_head_pend", "note", "superseded-pend v1", "src:tr", None, None, None),
            edge_write(Some("e1"), "links", "root", "t_unsup", "src:tr", None, None, None, None),
            edge_write(Some("e2"), "links", "root", "t_head_del", "src:tr", None, None, None, None),
            edge_write(
                Some("e3"),
                "links",
                "root",
                "t_head_pend",
                "src:tr",
                None,
                None,
                None,
                None,
            ),
        ],
    );

    // Supersede t_head_del with an active v2, and supersede t_head_pend with a Pending v2 head.
    let _c2 = write_cursor(
        &engine,
        &[
            node_write("t_head_del", "note", "superseded-del v2", "src:tr", None, None, None),
            PreparedWrite::Node {
                logical_id: Some("t_head_pend".into()),
                kind: "note".into(),
                body: "superseded-pend v2".into(),
                source_id: SourceId::new("src:tr").unwrap(),
                state: InitialState::Pending,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
        ],
    );

    // Before transitions, as_of=c1 sees t_unsup and t_head_del (v1), while t_head_pend is
    // already excluded because its current logical head is Pending!
    let before = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_write_cursor(c1),
        )
        .unwrap();
    let before_ids: BTreeSet<_> = before.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(before_ids, BTreeSet::from(["t_unsup", "t_head_del"]));

    // Transition un-superseded node to Deleted and superseded node's active head to Deleted.
    engine.transition("t_unsup", LifecycleState::Deleted, Some("deleted".into())).unwrap();
    engine.transition("t_head_del", LifecycleState::Deleted, Some("deleted".into())).unwrap();

    // All three (Deleted un-superseded, Deleted head, Pending head) must be excluded at c1.
    let after = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert!(
        after.targets.is_empty(),
        "transitioned/pending entities must not replay at pre-transition cursor c1: {:?}",
        after.targets
    );

    // Explicit seed on transitioned logical head is also unavailable at historical c1.
    for seed in ["t_unsup", "t_head_del", "t_head_pend"] {
        let err = engine
            .graph_expand(
                &explicit_req(&[seed], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                    .with_as_of_write_cursor(c1),
            )
            .unwrap_err();
        assert_eq!(
            graph_err_reason_path(err),
            (GraphExpansionErrorReasonV1::GraphSeedUnavailable, "/seed/logicalIds/0".into()),
            "seed={seed}"
        );
    }

    engine.close().unwrap();
}

#[test]
fn erasure_and_soft_deleted_source_block_nodes_and_derived_edges_at_pre_erasure_cursors() {
    // 1. purge subordination
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("purge-sub{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).unwrap().engine;
        let c1 = write_cursor(
            &engine,
            &[
                node_write("root", "note", "root", "src:p", None, None, None),
                node_write("victim", "note", "secret v1", "src:p", None, None, None),
                edge_write(Some("e1"), "links", "root", "victim", "src:p", None, None, None, None),
            ],
        );
        let _c2 = write_cursor(
            &engine,
            &[node_write("victim", "note", "secret v2", "src:p", None, None, None)],
        );
        engine.transition("victim", LifecycleState::Deleted, Some("purge-prep".into())).unwrap();
        engine.purge("victim").unwrap();
        let res = engine
            .graph_expand(
                &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                    .with_as_of_write_cursor(c1),
            )
            .unwrap();
        assert!(res.targets.is_empty());
        engine.close().unwrap();
    }

    // 2. erase_source subordination
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("erase-src-sub{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).unwrap().engine;
        let c1 = write_cursor(
            &engine,
            &[
                node_write("root", "note", "root", "src:keep", None, None, None),
                node_write("victim", "note", "secret", "src:erase", None, None, None),
                edge_write(
                    Some("e1"),
                    "links",
                    "root",
                    "victim",
                    "src:erase",
                    None,
                    None,
                    None,
                    None,
                ),
            ],
        );
        engine.erase_source("src:erase").unwrap();
        let res = engine
            .graph_expand(
                &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                    .with_as_of_write_cursor(c1),
            )
            .unwrap();
        assert!(res.targets.is_empty());
        engine.close().unwrap();
    }

    // 3. Soft-deleted canonical source blocks BOTH derived nodes AND derived edges at pre-deletion c1!
    {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("soft-del-derived{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).unwrap().engine;
        let source_body = "canonical source for derived node and edge";
        let digest = sha256_hex(source_body);
        let c1 = write_cursor(
            &engine,
            &[
                node_write("root", "fact", "root", "control-owner", None, None, None),
                node_write(
                    "plain_dest",
                    "fact",
                    "plain destination",
                    "control-owner",
                    None,
                    None,
                    None,
                ),
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("source".into()),
                    kind: "doc".into(),
                    body: source_body.into(),
                    source_id: SourceId::new("source-owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("source-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                    ),
                    properties: None,
                }),
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("derived_node".into()),
                    kind: "fact".into(),
                    body: "derived node".into(),
                    source_id: SourceId::new("source-owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("derived-node-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                        SourceRevisionId::new("source-r1").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(digest.clone()).unwrap(),
                    ),
                    properties: None,
                }),
                edge_write(
                    Some("plain-to-derived"),
                    "link",
                    "root",
                    "derived_node",
                    "control-owner",
                    None,
                    None,
                    None,
                    None,
                ),
                PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                    kind: "derived_rel".into(),
                    from: "root".into(),
                    to: "plain_dest".into(),
                    source_id: SourceId::new("source-owner").unwrap(),
                    logical_id: Some("derived-edge-1".into()),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    properties: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("derived-edge-r1").unwrap(),
                        SourceVersionId::new("v1").unwrap(),
                        SourceRevisionId::new("source-r1").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(digest).unwrap(),
                    ),
                }),
            ],
        );
        engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("dep-n", "source-r1", "derived-node-r1")
                    .unwrap(),
            )
            .unwrap();
        engine
            .register_source_dependency(
                SourceDependencyRegistrationV1::new("dep-e", "source-r1", "derived-edge-r1")
                    .unwrap(),
            )
            .unwrap();

        // Sanity: both derived_node and plain_dest (via derived_edge) are visible at c1 before deletion.
        let before = engine
            .graph_expand(
                &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                    .with_as_of_write_cursor(c1),
            )
            .unwrap();
        let before_ids: BTreeSet<_> =
            before.targets.iter().map(|t| t.logical_id.as_str()).collect();
        assert_eq!(before_ids, BTreeSet::from(["derived_node", "plain_dest"]));

        // Soft-delete the canonical source!
        engine.transition("source", LifecycleState::Deleted, Some("retracted".into())).unwrap();

        // Neither derived_node nor plain_dest (via derived_edge whose superseded_at > c1!) may appear at c1!
        let after = engine
            .graph_expand(
                &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                    .with_as_of_write_cursor(c1),
            )
            .unwrap();
        assert!(
            after.targets.is_empty(),
            "soft-deleted canonical source must block BOTH derived nodes and derived edges at pre-deletion c1: {:?}",
            after.targets
        );

        engine.close().unwrap();
    }
}

#[cfg(feature = "test-hooks")]
#[test]
fn active_closure_barrier_blocks_derived_nodes_and_derived_edges_in_historical_as_of() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("closure-barrier-hist{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;
    let source_body = "canonical source under closure barrier";
    let digest = sha256_hex(source_body);
    let c1 = write_cursor(
        &engine,
        &[
            node_write("root", "fact", "root", "control-owner", None, None, None),
            node_write("plain_dest", "fact", "plain dest", "control-owner", None, None, None),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("source".into()),
                kind: "doc".into(),
                body: source_body.into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("source-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                ),
                properties: None,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("derived_node".into()),
                kind: "fact".into(),
                body: "derived node".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest.clone()).unwrap(),
                ),
                properties: None,
            }),
            edge_write(
                Some("plain-to-derived"),
                "link",
                "root",
                "derived_node",
                "control-owner",
                None,
                None,
                None,
                None,
            ),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                kind: "derived_rel".into(),
                from: "root".into(),
                to: "plain_dest".into(),
                source_id: SourceId::new("source-owner").unwrap(),
                logical_id: Some("derived-edge-1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("derived-edge-r1").unwrap(),
                    SourceVersionId::new("v1").unwrap(),
                    SourceRevisionId::new("source-r1").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(digest).unwrap(),
                ),
            }),
        ],
    );
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-node", "source-r1", "derived-r1").unwrap(),
        )
        .unwrap();
    engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-edge", "source-r1", "derived-edge-r1")
                .unwrap(),
        )
        .unwrap();

    // Advance write cursor so derived-edge-1 gets superseded_at > c1 when the proving barrier runs,
    // and also supersede derived_node first so derived-r1's state remains 'active' while the barrier is 'proving'!
    let _c2 = write_cursor(
        &engine,
        &[node_write("derived_node", "fact", "derived v2", "control-owner", None, None, None)],
    );

    // Admit non-terminal (`proving`) closure barrier on source-r1 (source-r1 itself stays active!).
    engine.seed_graph_expand_nonterminal_dependency_closure_for_test().unwrap();

    let res = engine
        .graph_expand(
            &explicit_req(&["root"], TraversalDirection::Outgoing, Some(1_700_000_000), 1)
                .with_as_of_write_cursor(c1),
        )
        .unwrap();
    assert!(
        res.targets.is_empty(),
        "active closure barrier must block both derived nodes and derived edges at historical cursor c1: {:?}",
        res.targets
    );

    engine.close().unwrap();
}

#[test]
fn both_direction_self_loop_not_duplicated_and_historical_pushdown_bounds_work_units() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("pushdown-selfloop{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    // Seed nodes: root, dst, and junk.
    let _ = write_cursor(
        &engine,
        &[
            node_write("root", "note", "root", "src:pd", None, None, None),
            node_write("dst", "note", "dst", "src:pd", None, None, None),
            node_write("junk", "note", "junk", "src:pd", None, None, None),
        ],
    );

    // Create 500 pre-as_of superseded edge revisions on root -> junk!
    // Each write supersedes the same logical edge id ("churn-edge") 500 times before target_cursor.
    let mut churn_writes = Vec::with_capacity(500);
    for i in 0..500_u32 {
        churn_writes.push(edge_write(
            Some("churn-edge"),
            "noise",
            "root",
            "junk",
            "src:pd",
            None,
            None,
            None,
            Some(BTreeMap::from([("rev".into(), PropertyValue::Int(i64::from(i)))])),
        ));
    }
    // Commit in batches of 50 so supersession sets superseded_at < target_cursor across 500 rows.
    for chunk in churn_writes.chunks(50) {
        engine.write(chunk).unwrap();
    }
    // Supersede churn-edge one final time with expired t_invalid=10 so it's out-of-window at 1_700_000_000.
    let _ = write_cursor(
        &engine,
        &[edge_write(
            Some("churn-edge"),
            "noise",
            "root",
            "junk",
            "src:pd",
            None,
            Some(10),
            None,
            None,
        )],
    );

    // Now at target_cursor: root has EXACTLY 1 self-loop (root -> root) + 1 valid edge (root -> dst).
    let target_cursor = write_cursor(
        &engine,
        &[
            edge_write(Some("self-loop"), "rel", "root", "root", "src:pd", None, None, None, None),
            edge_write(Some("good-edge"), "rel", "root", "dst", "src:pd", None, None, None, None),
        ],
    );

    // Write post-target_cursor edges that must also be pushed down in SQL and never charged.
    let _ = write_cursor(
        &engine,
        &[
            edge_write(Some("self-loop"), "rel", "root", "root", "src:pd", None, None, None, None),
            edge_write(Some("good-edge"), "rel", "root", "dst", "src:pd", None, None, None, None),
            edge_write(Some("future-1"), "rel", "root", "junk", "src:pd", None, None, None, None),
            edge_write(Some("future-2"), "rel", "root", "junk", "src:pd", None, None, None, None),
        ],
    );

    // Query at target_cursor in Both direction with max_work_units = 2!
    // If ANY of the 500 pre-as_of superseded edges, future edges, or a duplicated self-loop
    // leaked into SQL results, work_units would exceed 2 and fail with GraphExpansionBoundExceeded!
    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("root")] },
        TraversalDirection::Both,
        current_ctx(Some(1_700_000_000), SearchFilter::default()),
        1,
        10,
        2,
    )
    .with_as_of_write_cursor(target_cursor);

    let res = engine.graph_expand(&req).unwrap();
    assert_eq!(
        res.work_units, 2,
        "exactly 2 edges (1 non-duplicated self-loop + 1 valid edge) must be charged"
    );
    assert_eq!(res.targets.len(), 1);
    assert_eq!(res.targets[0].logical_id, "dst");

    engine.close().unwrap();
}

// ===== Proptest Snapshot BFS Oracle =========================================

#[derive(Clone, Debug)]
enum ModelOp {
    WriteNode { node: u8, tag: i64, active_label: u8 },
    WriteEdge { edge_id: u8, from: u8, to: u8, kind: u8, weight: i64 },
    TransitionNode { node: u8, deleted: bool },
}

fn model_op_strategy() -> impl Strategy<Value = ModelOp> {
    prop_oneof![
        3 => (0_u8..5, 0_i64..4, 0_u8..3).prop_map(|(node, tag, active_label)| ModelOp::WriteNode {
            node,
            tag,
            active_label,
        }),
        4 => (0_u8..6, 0_u8..5, 0_u8..5, 0_u8..2, 0_i64..4).prop_map(
            |(edge_id, from, to, kind, weight)| ModelOp::WriteEdge {
                edge_id,
                from,
                to,
                kind,
                weight,
            },
        ),
        1 => (0_u8..5, any::<bool>()).prop_map(|(node, deleted)| ModelOp::TransitionNode {
            node,
            deleted,
        }),
    ]
}

#[derive(Clone)]
struct OracleNodeRev {
    write_cursor: u64,
    superseded_at: Option<u64>,
    state_active: bool,
    tag: i64,
    body: String,
}

#[derive(Clone)]
struct OracleEdgeRev {
    write_cursor: u64,
    superseded_at: Option<u64>,
    from: String,
    to: String,
    kind: String,
    weight: i64,
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn proptest_historical_graph_expand_matches_snapshot_bfs_oracle(
        ops in prop::collection::vec(model_op_strategy(), 4..14),
        direction in prop_oneof![
            Just(TraversalDirection::Outgoing),
            Just(TraversalDirection::Incoming),
            Just(TraversalDirection::Both),
        ],
        max_depth in 1_u32..=2,
        min_edge_weight in 0_i64..=3,
        min_node_tag in 0_i64..=3,
        filter_kind_a in any::<bool>(),
    ) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("prop-oracle{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).unwrap().engine;

        // Declare Filterable keys on BOTH nodes and edges to stress disabled cpv pushdown!
        engine
            .configure_projections(
                &[
                    ProjectionSpec {
                        name: "tag".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    },
                    ProjectionSpec {
                        name: "weight".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Edge,
                    },
                ],
                &[],
            )
            .unwrap();

        let mut node_revs: HashMap<String, Vec<OracleNodeRev>> = HashMap::new();
        let mut node_current_active: HashMap<String, bool> = HashMap::new();
        let mut edge_revs: HashMap<String, Vec<OracleEdgeRev>> = HashMap::new();
        let mut checkpoints: Vec<u64> = Vec::new();

        // Initialize all 5 nodes so edges always satisfy strict write-time endpoint checks.
        let mut init_writes = Vec::new();
        for n in 0..5_u8 {
            let lid = format!("n{n}");
            init_writes.push(node_write(
                &lid,
                "item",
                &format!("init-{n}"),
                "src:prop",
                None,
                None,
                Some(BTreeMap::from([("tag".into(), PropertyValue::Int(0))])),
            ));
        }
        let c0 = engine.write(&init_writes).unwrap().cursor;
        for n in 0..5_u8 {
            let lid = format!("n{n}");
            node_revs.insert(
                lid.clone(),
                vec![OracleNodeRev {
                    write_cursor: c0,
                    superseded_at: None,
                    state_active: true,
                    tag: 0,
                    body: format!("init-{n}"),
                }],
            );
            node_current_active.insert(lid, true);
        }
        checkpoints.push(c0);

        for (step, op) in ops.iter().enumerate() {
            match *op {
                ModelOp::WriteNode { node, tag, active_label } => {
                    let lid = format!("n{node}");
                    let body = format!("n{node}-s{step}-l{active_label}");
                    let c = write_cursor(
                        &engine,
                        &[node_write(
                            &lid,
                            "item",
                            &body,
                            "src:prop",
                            None,
                            None,
                            Some(BTreeMap::from([("tag".into(), PropertyValue::Int(tag))])),
                        )],
                    );
                    let revs = node_revs.entry(lid.clone()).or_default();
                    if let Some(last) = revs.last_mut() {
                        last.superseded_at = Some(c);
                    }
                    revs.push(OracleNodeRev {
                        write_cursor: c,
                        superseded_at: None,
                        state_active: true,
                        tag,
                        body,
                    });
                    node_current_active.insert(lid, true);
                    checkpoints.push(c);
                }
                ModelOp::WriteEdge { edge_id, from, to, kind, weight } => {
                    let from_lid = format!("n{from}");
                    let to_lid = format!("n{to}");
                    // Strict write-time endpoint check requires both endpoints currently active.
                    if !node_current_active.get(&from_lid).copied().unwrap_or(false)
                        || !node_current_active.get(&to_lid).copied().unwrap_or(false)
                    {
                        continue;
                    }
                    let elid = format!("e{edge_id}");
                    let ekind = if kind == 0 { "rel_a" } else { "rel_b" };
                    let c = write_cursor(
                        &engine,
                        &[edge_write(
                            Some(&elid),
                            ekind,
                            &from_lid,
                            &to_lid,
                            "src:prop",
                            None,
                            None,
                            None,
                            Some(BTreeMap::from([("weight".into(), PropertyValue::Int(weight))])),
                        )],
                    );
                    let revs = edge_revs.entry(elid).or_default();
                    if let Some(last) = revs.last_mut() {
                        last.superseded_at = Some(c);
                    }
                    revs.push(OracleEdgeRev {
                        write_cursor: c,
                        superseded_at: None,
                        from: from_lid,
                        to: to_lid,
                        kind: ekind.into(),
                        weight,
                    });
                    checkpoints.push(c);
                }
                ModelOp::TransitionNode { node, deleted } => {
                    let lid = format!("n{node}");
                    let cur = node_current_active.get(&lid).copied().unwrap_or(false);
                    if deleted && cur {
                        engine.transition(&lid, LifecycleState::Deleted, None).unwrap();
                        node_current_active.insert(lid.clone(), false);
                        if let Some(last) = node_revs.get_mut(&lid).and_then(|v| v.last_mut()) {
                            last.state_active = false;
                        }
                    } else if !deleted && !cur {
                        engine.transition(&lid, LifecycleState::Active, None).unwrap();
                        node_current_active.insert(lid.clone(), true);
                        if let Some(last) = node_revs.get_mut(&lid).and_then(|v| v.last_mut()) {
                            last.state_active = true;
                        }
                    }
                }
            }
        }

        // Evaluate EVERY historical checkpoint against the exact in-memory snapshot BFS oracle!
        for &as_of in &checkpoints {
            let seed_id = "n0";
            let node_alive = |lid: &str| -> Option<&OracleNodeRev> {
                if !node_current_active.get(lid).copied().unwrap_or(false) {
                    return None;
                }
                node_revs.get(lid)?.iter().rev().find(|r| {
                    r.state_active
                        && r.write_cursor <= as_of
                        && r.superseded_at.is_none_or(|s| s > as_of)
                })
            };

            let mut req = explicit_req(&[seed_id], direction, Some(1_700_000_000), max_depth)
                .with_edge_predicates(vec![PropertyPredicate {
                    key: "weight".into(),
                    op: PropertyComparisonOp::Gte,
                    value: PropertyValue::Int(min_edge_weight),
                }])
                .with_target_predicates(vec![PropertyPredicate {
                    key: "tag".into(),
                    op: PropertyComparisonOp::Gte,
                    value: PropertyValue::Int(min_node_tag),
                }])
                .with_as_of_write_cursor(as_of);
            if filter_kind_a {
                req = req.with_edge_kinds(vec!["rel_a".into()]);
            }

            if node_alive(seed_id).is_none() {
                let err = engine.graph_expand(&req).unwrap_err();
                prop_assert_eq!(
                    graph_err_reason_path(err),
                    (GraphExpansionErrorReasonV1::GraphSeedUnavailable, "/seed/logicalIds/0".into())
                );
                continue;
            }

            // Compute exact BFS reachable targets & their bodies at `as_of`.
            let mut visited: HashSet<String> = HashSet::from([seed_id.to_string()]);
            let mut frontier: Vec<String> = vec![seed_id.to_string()];
            let mut expected_targets: BTreeMap<String, String> = BTreeMap::new();

            for depth in 0..max_depth {
                let mut next_frontier = Vec::new();
                for cur in &frontier {
                    for revs in edge_revs.values() {
                        let Some(e) = revs.iter().rev().find(|r| {
                            r.write_cursor <= as_of && r.superseded_at.is_none_or(|s| s > as_of)
                        }) else {
                            continue;
                        };
                        if e.weight < min_edge_weight || (filter_kind_a && e.kind != "rel_a") {
                            continue;
                        }
                        let next_opt = match direction {
                            TraversalDirection::Outgoing if e.from == *cur => Some(e.to.as_str()),
                            TraversalDirection::Incoming if e.to == *cur => Some(e.from.as_str()),
                            TraversalDirection::Both => {
                                if e.from == *cur {
                                    Some(e.to.as_str())
                                } else if e.to == *cur {
                                    Some(e.from.as_str())
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };
                        let Some(next) = next_opt else { continue };
                        let Some(nrev) = node_alive(next) else { continue };
                        if depth + 1 < max_depth && visited.insert(next.to_string()) {
                            next_frontier.push(next.to_string());
                        }
                        if next != seed_id && nrev.tag >= min_node_tag {
                            expected_targets.insert(next.to_string(), nrev.body.clone());
                        }
                    }
                }
                frontier = next_frontier;
            }

            let actual = engine.graph_expand(&req).unwrap();
            let actual_map: BTreeMap<String, String> = actual
                .targets
                .into_iter()
                .map(|t| (t.logical_id, t.body))
                .collect();
            prop_assert_eq!(actual_map, expected_targets, "mismatch at as_of={}", as_of);
        }

        engine.close().unwrap();
    }
}

fn assert_evidence_unavailable(err: EngineError) {
    match err {
        EngineError::Evidence(e) => {
            assert_eq!(
                e.reason,
                EvidenceErrorReasonV1::EvidenceUnavailable,
                "expected EvidenceUnavailable, got {e:?}"
            );
        }
        other => panic!("expected EngineError::Evidence(EvidenceUnavailable), got {other:?}"),
    }
}

#[test]
fn ac31_40_06_historical_frozen_evidence_resolves_superseded_self_sourced_and_derived_artifacts() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("hist-evidence{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    let src_v1_body = "canonical source body v1";
    let self_v1_body = "self-sourced target body v1";
    let derived_v1 = |rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new("src-v1").unwrap(),
            SourceRevisionId::new("src-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(src_v1_body)).unwrap(),
        )
    };

    let c1 = write_cursor(
        &engine,
        &[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src-doc".into()),
                kind: "document".into(),
                body: src_v1_body.into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-r1").unwrap(),
                    SourceVersionId::new("src-v1").unwrap(),
                ),
                properties: Some(BTreeMap::from([("rev".into(), PropertyValue::Int(1))])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: "root v1".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_v1("root-r1"),
                properties: None,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("self-target".into()),
                kind: "document".into(),
                body: self_v1_body.into(),
                source_id: SourceId::new("self-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("self-target-r1").unwrap(),
                    SourceVersionId::new("self-v1").unwrap(),
                ),
                properties: Some(BTreeMap::from([(
                    "ver".into(),
                    PropertyValue::String("v1".into()),
                )])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("derived-target".into()),
                kind: "claim".into(),
                body: "derived target v1".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_v1("derived-target-r1"),
                properties: Some(BTreeMap::from([(
                    "ver".into(),
                    PropertyValue::String("v1".into()),
                )])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("stable-target".into()),
                kind: "claim".into(),
                body: "stable target v1".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_v1("stable-target-r1"),
                properties: None,
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-self".into()),
                kind: "cites".into(),
                from: "root".into(),
                to: "self-target".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.9),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_v1("edge-self-r1"),
                properties: Some(BTreeMap::from([("w".into(), PropertyValue::Int(10))])),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-derived".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "derived-target".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.8),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_v1("edge-derived-r1"),
                properties: Some(BTreeMap::from([("w".into(), PropertyValue::Int(20))])),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-stable".into()),
                kind: "mentions".into(),
                from: "root".into(),
                to: "stable-target".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_v1("edge-stable-r1"),
                properties: None,
            }),
        ],
    );
    let dep_target = engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-target-1", "src-r1", "derived-target-r1")
                .unwrap(),
        )
        .unwrap();
    let dep_edge = engine
        .register_source_dependency(
            SourceDependencyRegistrationV1::new("dep-edge-1", "src-r1", "edge-derived-r1").unwrap(),
        )
        .unwrap();

    // Capture commit timestamp for c1 before superseding.
    let conn = Connection::open(&path).unwrap();
    let c1_epoch_ms: i64 = conn
        .query_row(
            "SELECT committed_at_ms FROM _fathomdb_commit_timestamps WHERE write_cursor = ?1",
            [c1 as i64],
            |row| row.get(0),
        )
        .unwrap();
    drop(conn);

    // Ensure distinct timestamp for batch 2 so as_of_epoch_ms(c1_epoch_ms) resolves strictly to c1.
    std::thread::sleep(std::time::Duration::from_millis(5));

    // Supersede self-target, derived-target, edge-self, and edge-derived FIRST (so their v1
    // revisions remain `state = 'active'` with `superseded_at > c1`), then supersede `src-doc`
    // (`src-r1` -> `src-r2`). Leave stable-target and edge-stable un-superseded.
    let src_v2_body = "canonical source body v2";
    let derived_v2 = |rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new("src-b-v1").unwrap(),
            SourceRevisionId::new("src-b-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(src_v2_body)).unwrap(),
        )
    };
    write_cursor(
        &engine,
        &[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src-doc-b".into()),
                kind: "document".into(),
                body: src_v2_body.into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-b-r1").unwrap(),
                    SourceVersionId::new("src-b-v1").unwrap(),
                ),
                properties: Some(BTreeMap::from([("rev".into(), PropertyValue::Int(2))])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("self-target".into()),
                kind: "document".into(),
                body: "self-sourced target body v2".into(),
                source_id: SourceId::new("self-owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("self-target-r2").unwrap(),
                    SourceVersionId::new("self-v2").unwrap(),
                ),
                properties: Some(BTreeMap::from([(
                    "ver".into(),
                    PropertyValue::String("v2".into()),
                )])),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("derived-target".into()),
                kind: "claim".into(),
                body: "derived target v2".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_v2("derived-target-r2"),
                properties: Some(BTreeMap::from([(
                    "ver".into(),
                    PropertyValue::String("v2".into()),
                )])),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-self".into()),
                kind: "cites".into(),
                from: "root".into(),
                to: "self-target".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.5),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_v2("edge-self-r2"),
                properties: Some(BTreeMap::from([("w".into(), PropertyValue::Int(99))])),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-derived".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "derived-target".into(),
                source_id: SourceId::new("owner").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.5),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_v2("edge-derived-r2"),
                properties: Some(BTreeMap::from([("w".into(), PropertyValue::Int(99))])),
            }),
        ],
    );
    write_cursor(
        &engine,
        &[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some("src-doc".into()),
            kind: "document".into(),
            body: src_v2_body.into(),
            source_id: SourceId::new("owner").unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new("src-r2").unwrap(),
                SourceVersionId::new("src-v2").unwrap(),
            ),
            properties: Some(BTreeMap::from([("rev".into(), PropertyValue::Int(2))])),
        })],
    );
    engine.drain(30_000).unwrap();

    let frozen = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();

    for req in [
        GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
            1,
            50,
            10_000,
        )
        .with_evidence(true)
        .with_as_of_write_cursor(c1),
        GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
            1,
            50,
            10_000,
        )
        .with_evidence(true)
        .with_as_of_epoch_ms(c1_epoch_ms),
    ] {
        let expanded = engine.graph_expand(&req).unwrap();
        assert_eq!(expanded.targets.len(), 3);
        let sidecar = expanded.evidence.as_ref().expect("evidence sidecar must be present");
        assert_eq!(sidecar.entries.len(), 3);

        let mut by_rev = HashMap::new();
        for entry in &sidecar.entries {
            assert_eq!(entry.target_evidence_ref.as_str().len(), 704);
            assert!(entry.target_evidence_ref.as_str().starts_with("fdbgev1."));
            assert_eq!(entry.terminal_edge_evidence_ref.as_str().len(), 704);
            assert!(entry.terminal_edge_evidence_ref.as_str().starts_with("fdbgev1."));

            let resolved_target = engine
                .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: entry.target_evidence_ref.clone(),
                    context: frozen.clone(),
                })
                .unwrap();
            let resolved_edge = engine
                .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: entry.terminal_edge_evidence_ref.clone(),
                    context: frozen.clone(),
                })
                .unwrap();
            by_rev.insert(
                resolved_target.artifact_revision_id.as_str().to_string(),
                (resolved_target, resolved_edge),
            );
        }

        // 1. Superseded self-sourced target + superseded derived edge.
        let (self_t, self_e) = &by_rev["self-target-r1"];
        assert_eq!(
            self_t.artifact_lifecycle,
            EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: true }
        );
        assert_eq!(self_t.canonical_source_body, self_v1_body);
        assert_eq!(self_t.evidence_text, self_v1_body);
        assert!(self_t.dependency.is_none());
        match &self_t.artifact {
            GraphEvidenceArtifactV1::Node { logical_id, body, properties, .. } => {
                assert_eq!(logical_id, "self-target");
                assert_eq!(body, self_v1_body);
                assert_eq!(
                    properties.as_ref().and_then(|p| p.get("ver")),
                    Some(&PropertyValue::String("v1".into()))
                );
            }
            other => panic!("expected Node artifact, got {other:?}"),
        }
        assert_eq!(self_e.artifact_revision_id.as_str(), "edge-self-r1");
        assert_eq!(
            self_e.artifact_lifecycle,
            EvidenceArtifactLifecycleV1::Edge { superseded: true, valid_at_effective: true }
        );
        assert_eq!(self_e.canonical_source_body, src_v1_body);

        // 2. Superseded derived target + superseded derived edge with registered dependencies.
        let (der_t, der_e) = &by_rev["derived-target-r1"];
        assert_eq!(
            der_t.artifact_lifecycle,
            EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: true }
        );
        assert_eq!(der_t.canonical_source_body, src_v1_body);
        assert_eq!(der_t.dependency.as_ref(), Some(&dep_target));
        assert_eq!(der_e.artifact_revision_id.as_str(), "edge-derived-r1");
        assert_eq!(
            der_e.artifact_lifecycle,
            EvidenceArtifactLifecycleV1::Edge { superseded: true, valid_at_effective: true }
        );
        assert_eq!(der_e.dependency.as_ref(), Some(&dep_edge));
        match &der_e.artifact {
            GraphEvidenceArtifactV1::Edge { body, properties, .. } => {
                assert_eq!(body.as_deref(), None);
                assert_eq!(
                    properties.as_ref().and_then(|p| p.get("w")),
                    Some(&PropertyValue::Int(20))
                );
            }
            other => panic!("expected Edge artifact, got {other:?}"),
        }

        // 3. Un-superseded target + un-superseded edge (whose source src-r1 WAS superseded!).
        let (stb_t, stb_e) = &by_rev["stable-target-r1"];
        assert_eq!(
            stb_t.artifact_lifecycle,
            EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: false }
        );
        assert_eq!(
            stb_e.artifact_lifecycle,
            EvidenceArtifactLifecycleV1::Edge { superseded: false, valid_at_effective: true }
        );
        assert_eq!(stb_t.source_revision_id.as_str(), "src-r1");
    }

    // Empty cursor-0 result set with Query seed + include_evidence: true succeeds cleanly.
    let empty_req = GraphExpandRequestV1::new(
        GraphSeedV1::Query { schema_version: 1, text: "root".into(), ranked_limit: 10 },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        50,
        10_000,
    )
    .with_evidence(true)
    .with_as_of_write_cursor(0);
    let empty_res = engine.graph_expand(&empty_req).unwrap();
    assert!(empty_res.targets.is_empty());
    assert!(empty_res.evidence.unwrap().entries.is_empty());

    // Future cursor u64::MAX saturates cleanly and resolves current v2 artifacts (superseded=false).
    let max_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("root")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        50,
        10_000,
    )
    .with_edge_kinds(vec!["supports".into()])
    .with_evidence(true)
    .with_as_of_write_cursor(u64::MAX);
    let max_res = engine.graph_expand(&max_req).unwrap();
    assert_eq!(max_res.targets.len(), 1);
    let max_entry = &max_res.evidence.unwrap().entries[0];
    let resolved_max = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: max_entry.target_evidence_ref.clone(),
            context: frozen,
        })
        .unwrap();
    assert_eq!(resolved_max.artifact_revision_id.as_str(), "derived-target-r2");
    assert_eq!(
        resolved_max.artifact_lifecycle,
        EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: false }
    );

    engine.close().unwrap();
}

#[test]
fn ac31_40_06_historical_frozen_evidence_enforces_inline_filterable_eligibility_on_target_and_source(
) {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("hist-evidence-filter{SQLITE_SUFFIX}"));
    let engine = Engine::open(&path).unwrap().engine;

    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "tenant".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "score".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        )
        .unwrap();

    // Body JSON holds mismatching values; properties override with matching values (properties > body).
    let src_good_body = r#"{"tenant":"wrong-body","score":1}"#;
    let src_bad_body = r#"{"tenant":"wrong","score":5}"#;
    let target_body_v1 = r#"{"tenant":"wrong-body","score":1}"#;

    let derived_good = |rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new("src-good-v1").unwrap(),
            SourceRevisionId::new("src-good-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(src_good_body)).unwrap(),
        )
    };
    let derived_bad = |rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new("src-bad-v1").unwrap(),
            SourceRevisionId::new("src-bad-r1").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(src_bad_body)).unwrap(),
        )
    };

    let matching_props = BTreeMap::from([
        ("tenant".into(), PropertyValue::String("acme".into())),
        ("score".into(), PropertyValue::Int(50)),
    ]);
    let mismatch_props = BTreeMap::from([
        ("tenant".into(), PropertyValue::String("other".into())),
        ("score".into(), PropertyValue::Int(5)),
    ]);

    let c1 = write_cursor(
        &engine,
        &[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src-good".into()),
                kind: "document".into(),
                body: src_good_body.into(),
                source_id: SourceId::new("owner-good").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-good-r1").unwrap(),
                    SourceVersionId::new("src-good-v1").unwrap(),
                ),
                properties: Some(matching_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src-bad".into()),
                kind: "document".into(),
                body: src_bad_body.into(),
                source_id: SourceId::new("owner-bad").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-bad-r1").unwrap(),
                    SourceVersionId::new("src-bad-v1").unwrap(),
                ),
                properties: Some(mismatch_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: target_body_v1.into(),
                source_id: SourceId::new("owner-good").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_good("root-r1"),
                properties: Some(matching_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target-good".into()),
                kind: "claim".into(),
                body: target_body_v1.into(),
                source_id: SourceId::new("owner-good").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_good("target-good-r1"),
                properties: Some(matching_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target-bad-src".into()),
                kind: "claim".into(),
                body: target_body_v1.into(),
                source_id: SourceId::new("owner-bad").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_bad("target-bad-src-r1"),
                properties: Some(matching_props.clone()),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-good".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target-good".into(),
                source_id: SourceId::new("owner-good").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_good("edge-good-r1"),
                properties: None,
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("edge-bad-src".into()),
                kind: "denies".into(),
                from: "root".into(),
                to: "target-bad-src".into(),
                source_id: SourceId::new("owner-bad").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_bad("edge-bad-src-r1"),
                properties: None,
            }),
        ],
    );

    // Supersede all sources and targets with mismatching v2 properties (purges active-only
    // canonical_attributes and canonical_property_values for v1 revisions!).
    let src_good_v2 = r#"{"tenant":"other","score":5}"#;
    let derived_good_v2 = |rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new("src-good-v2").unwrap(),
            SourceRevisionId::new("src-good-r2").unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(sha256_hex(src_good_v2)).unwrap(),
        )
    };
    write_cursor(
        &engine,
        &[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src-good".into()),
                kind: "document".into(),
                body: src_good_v2.into(),
                source_id: SourceId::new("owner-good").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-good-r2").unwrap(),
                    SourceVersionId::new("src-good-v2").unwrap(),
                ),
                properties: Some(mismatch_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("src-bad".into()),
                kind: "document".into(),
                body: src_good_v2.into(),
                source_id: SourceId::new("owner-bad").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-bad-r2").unwrap(),
                    SourceVersionId::new("src-bad-v2").unwrap(),
                ),
                properties: Some(matching_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target-good".into()),
                kind: "claim".into(),
                body: src_good_v2.into(),
                source_id: SourceId::new("owner-good").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_good_v2("target-good-r2"),
                properties: Some(mismatch_props.clone()),
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target-bad-src".into()),
                kind: "claim".into(),
                body: src_good_v2.into(),
                source_id: SourceId::new("owner-bad").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::derived(
                    ArtifactRevisionId::new("target-bad-src-r2").unwrap(),
                    SourceVersionId::new("src-bad-v2").unwrap(),
                    SourceRevisionId::new("src-bad-r2").unwrap(),
                    SourceLocator::whole_body(),
                    CanonicalHash::sha256(sha256_hex(src_good_v2)).unwrap(),
                ),
                properties: Some(matching_props),
            }),
        ],
    );
    engine.drain(30_000).unwrap();

    // Freeze context with BOTH eligibility.attributes AND eligibility.property_predicates.
    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("claim".into());
    eligibility.attributes.push(("tenant".into(), "acme".into()));
    eligibility.property_predicates.push(PropertyPredicate {
        key: "score".into(),
        op: PropertyComparisonOp::Gte,
        value: PropertyValue::Int(40),
    });
    let frozen = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();

    // Target + source both match inline (properties > body) at c1 even though v1 was purged from cpv/ca.
    let good_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("root")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        50,
        10_000,
    )
    .with_edge_kinds(vec!["supports".into()])
    .with_evidence(true)
    .with_as_of_write_cursor(c1);
    let expanded = engine.graph_expand(&good_req).unwrap();
    assert_eq!(expanded.targets.len(), 1);
    let entry = &expanded.evidence.unwrap().entries[0];
    let resolved_target = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.target_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(resolved_target.artifact_revision_id.as_str(), "target-good-r1");
    assert_eq!(
        resolved_target.artifact_lifecycle,
        EvidenceArtifactLifecycleV1::Node { state: LifecycleState::Active, superseded: true }
    );
    let resolved_edge = engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: entry.terminal_edge_evidence_ref.clone(),
            context: frozen.clone(),
        })
        .unwrap();
    assert_eq!(resolved_edge.artifact_revision_id.as_str(), "edge-good-r1");

    // Source revision at c1 fails inline eligibility (even though its v2 head matches!) -> fails closed!
    let bad_src_req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("root")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        50,
        10_000,
    )
    .with_edge_kinds(vec!["denies".into()])
    .with_evidence(true)
    .with_as_of_write_cursor(c1);
    assert_evidence_unavailable(engine.graph_expand(&bad_src_req).unwrap_err());

    // Replaying the minted historical token against a different eligibility context fails closed.
    let mut wrong_elig = SearchFilter::default();
    wrong_elig.kind = Some("claim".into());
    wrong_elig.attributes.push(("tenant".into(), "other".into()));
    let wrong_frozen = engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), wrong_elig).unwrap())
        .unwrap();
    assert_evidence_unavailable(
        engine
            .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: entry.target_evidence_ref.clone(),
                context: wrong_frozen,
            })
            .unwrap_err(),
    );

    engine.close().unwrap();
}

#[test]
fn ac31_40_06_historical_evidence_revokes_on_erasure_transition_and_closure_barrier() {
    let build_fixture = |name: &str| {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join(format!("{name}{SQLITE_SUFFIX}"));
        let engine = Engine::open(&path).unwrap().engine;
        let src_v1 = "canonical source v1";
        let derived_v1 = |rev: &str| {
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(rev).unwrap(),
                SourceVersionId::new("src-v1").unwrap(),
                SourceRevisionId::new("src-r1").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(sha256_hex(src_v1)).unwrap(),
            )
        };
        let c1 = write_cursor(
            &engine,
            &[
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("src".into()),
                    kind: "document".into(),
                    body: src_v1.into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("src-r1").unwrap(),
                        SourceVersionId::new("src-v1").unwrap(),
                    ),
                    properties: None,
                }),
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("root".into()),
                    kind: "claim".into(),
                    body: "root".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: derived_v1("root-r1"),
                    properties: None,
                }),
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("target".into()),
                    kind: "claim".into(),
                    body: "target v1".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: derived_v1("target-r1"),
                    properties: None,
                }),
                PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                    logical_id: Some("winner".into()),
                    kind: "supports".into(),
                    from: "root".into(),
                    to: "target".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    provenance: derived_v1("edge-r1"),
                    properties: None,
                }),
            ],
        );
        // Supersede src, target, and edge so historical tokens point at superseded v1 rows.
        let src_v2 = "canonical source v2";
        let derived_v2 = |rev: &str| {
            WriteProvenanceV1::derived(
                ArtifactRevisionId::new(rev).unwrap(),
                SourceVersionId::new("src-v2").unwrap(),
                SourceRevisionId::new("src-r2").unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(sha256_hex(src_v2)).unwrap(),
            )
        };
        write_cursor(
            &engine,
            &[
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("src".into()),
                    kind: "document".into(),
                    body: src_v2.into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("src-r2").unwrap(),
                        SourceVersionId::new("src-v2").unwrap(),
                    ),
                    properties: None,
                }),
                PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("target".into()),
                    kind: "claim".into(),
                    body: "target v2".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: derived_v2("target-r2"),
                    properties: None,
                }),
                PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                    logical_id: Some("winner".into()),
                    kind: "supports".into(),
                    from: "root".into(),
                    to: "target".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    body: None,
                    t_valid: None,
                    t_invalid: None,
                    confidence: None,
                    extractor_model_id: None,
                    temporal_fallback: None,
                    provenance: derived_v2("edge-r2"),
                    properties: None,
                }),
            ],
        );
        engine.drain(30_000).unwrap();
        let read_ctx = ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap();
        let frozen = engine.freeze_read_context(&read_ctx).unwrap();
        let req = GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
            1,
            50,
            10_000,
        )
        .with_evidence(true)
        .with_as_of_write_cursor(c1);
        let expanded = engine.graph_expand(&req).unwrap();
        let entry = expanded.evidence.unwrap().entries.into_iter().next().unwrap();
        (dir, path, engine, c1, read_ctx, frozen, entry)
    };

    // 1. erase_source revokes both target and edge historical tokens (both original & reminted ctx).
    {
        let (_dir, _path, engine, _c1, read_ctx, frozen, entry) = build_fixture("erase-src");
        engine.erase_source("owner").unwrap();
        let reminted = engine.freeze_read_context(&read_ctx).unwrap();
        for ctx in [&frozen, &reminted] {
            for r in [&entry.target_evidence_ref, &entry.terminal_edge_evidence_ref] {
                assert_evidence_unavailable(
                    engine
                        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                            schema_version: 1,
                            evidence_ref: r.clone(),
                            context: ctx.clone(),
                        })
                        .unwrap_err(),
                );
            }
        }
        engine.close().unwrap();
    }

    // 2. purge on target revokes historical tokens (non-resurrection).
    {
        let (_dir, _path, engine, _c1, read_ctx, frozen, entry) = build_fixture("purge-target");
        engine.transition("target", LifecycleState::Deleted, Some("for purge".into())).unwrap();
        engine.purge("target").unwrap();
        let reminted = engine.freeze_read_context(&read_ctx).unwrap();
        for ctx in [&frozen, &reminted] {
            for r in [&entry.target_evidence_ref, &entry.terminal_edge_evidence_ref] {
                assert_evidence_unavailable(
                    engine
                        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                            schema_version: 1,
                            evidence_ref: r.clone(),
                            context: ctx.clone(),
                        })
                        .unwrap_err(),
                );
            }
        }
        engine.close().unwrap();
    }

    // 3. Current logical head of artifact node transitioned to Deleted OR superseded by a Pending
    //    head revokes BOTH historical minting and resolve_graph_evidence (even with reminted ctx!).
    for pending_head in [false, true] {
        let label = if pending_head { "pending" } else { "deleted" };
        let (_dir, _path, engine, c1, read_ctx, _frozen, entry) =
            build_fixture(&format!("trans-target-{label}"));
        if pending_head {
            let src_v2 = "canonical source v2";
            write_cursor(
                &engine,
                &[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("target".into()),
                    kind: "claim".into(),
                    body: "target v3 pending".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Pending,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::derived(
                        ArtifactRevisionId::new("target-r3").unwrap(),
                        SourceVersionId::new("src-v2").unwrap(),
                        SourceRevisionId::new("src-r2").unwrap(),
                        SourceLocator::whole_body(),
                        CanonicalHash::sha256(sha256_hex(src_v2)).unwrap(),
                    ),
                    properties: None,
                })],
            );
            engine.drain(30_000).unwrap();
        } else {
            engine.transition("target", LifecycleState::Deleted, Some("revoked".into())).unwrap();
        }
        let reminted = engine.freeze_read_context(&read_ctx).unwrap();
        // Historical resolve on target token MUST return EvidenceUnavailable (artifact_head_inactive).
        assert_evidence_unavailable(
            engine
                .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                    schema_version: 1,
                    evidence_ref: entry.target_evidence_ref.clone(),
                    context: reminted.clone(),
                })
                .unwrap_err(),
        );
        // Historical graph_expand no longer returns target at c1 (current-head inactive).
        let remint_req = GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: reminted },
            1,
            50,
            10_000,
        )
        .with_evidence(true)
        .with_as_of_write_cursor(c1);
        let expanded = engine.graph_expand(&remint_req).unwrap();
        assert!(expanded.targets.is_empty());
        assert!(expanded.evidence.unwrap().entries.is_empty());
        engine.close().unwrap();
    }

    // 4. Current logical head of canonical source node transitioned to Deleted OR superseded by a
    //    Pending head revokes BOTH target and edge historical tokens AND historical minting!
    for pending_head in [false, true] {
        let label = if pending_head { "pending" } else { "deleted" };
        let (_dir, _path, engine, c1, read_ctx, _frozen, entry) =
            build_fixture(&format!("trans-src-{label}"));
        if pending_head {
            write_cursor(
                &engine,
                &[PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                    logical_id: Some("src".into()),
                    kind: "document".into(),
                    body: "canonical source v3 pending".into(),
                    source_id: SourceId::new("owner").unwrap(),
                    state: InitialState::Pending,
                    reason: None,
                    valid_from: None,
                    valid_until: None,
                    provenance: WriteProvenanceV1::canonical(
                        ArtifactRevisionId::new("src-r3").unwrap(),
                        SourceVersionId::new("src-v3").unwrap(),
                    ),
                    properties: None,
                })],
            );
            engine.drain(30_000).unwrap();
        } else {
            engine
                .transition("src", LifecycleState::Deleted, Some("source revoked".into()))
                .unwrap();
        }
        let reminted = engine.freeze_read_context(&read_ctx).unwrap();
        for r in [&entry.target_evidence_ref, &entry.terminal_edge_evidence_ref] {
            assert_evidence_unavailable(
                engine
                    .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                        schema_version: 1,
                        evidence_ref: r.clone(),
                        context: reminted.clone(),
                    })
                    .unwrap_err(),
            );
        }
        // Historical minting at c1 also fails closed / excludes derived artifacts whose source head is inactive.
        let remint_req = GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: reminted },
            1,
            50,
            10_000,
        )
        .with_evidence(true)
        .with_as_of_write_cursor(c1);
        let err = engine.graph_expand(&remint_req).unwrap_err();
        assert_evidence_unavailable(err);
        engine.close().unwrap();
    }

    // 5. Active closure barrier on src-r1 revokes historical tokens AND historical minting!
    {
        let (_dir, path, engine, c1, read_ctx, _frozen, entry) = build_fixture("barrier-src");
        engine.close().unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.execute_batch(
            "UPDATE _fathomdb_open_state SET value='1' WHERE key='_fathomdb_closure_sequence';
             INSERT INTO _fathomdb_dependency_closures(
               schema_version,closure_operation_id,root_kind,root_value,cause,
               effective_at_epoch_s,admitted_write_boundary,admitted_dependency_generation,
               closure_sequence,retry_fingerprint,phase,affected_count,blocker_code,
               structural_proof_write_boundary,proof_json
             ) VALUES(1,'_fdb:c:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
               'source_revision','src-r1','soft_deleted',0,4,0,1,
               'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
               'proving',1,NULL,NULL,NULL);",
        )
        .unwrap();
        drop(conn);
        let reopened = Engine::open(&path).unwrap().engine;
        let reminted = reopened.freeze_read_context(&read_ctx).unwrap();
        for r in [&entry.target_evidence_ref, &entry.terminal_edge_evidence_ref] {
            assert_evidence_unavailable(
                reopened
                    .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
                        schema_version: 1,
                        evidence_ref: r.clone(),
                        context: reminted.clone(),
                    })
                    .unwrap_err(),
            );
        }
        let remint_req = GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("root")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Frozen { schema_version: 1, context: reminted },
            1,
            50,
            10_000,
        )
        .with_evidence(true)
        .with_as_of_write_cursor(c1);
        assert_evidence_unavailable(reopened.graph_expand(&remint_req).unwrap_err());
        reopened.close().unwrap();
    }
}
