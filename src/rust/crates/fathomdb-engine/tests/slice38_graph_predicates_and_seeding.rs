//! 0.8.31 Slice 30 Layer 2 (`WP-30B`) — `GraphExpandRequestV1` closed typed
//! predicates, SQL pushdown, `UNION ALL`, `t_valid` gating, terminal-skip
//! soundness, closed wire/evidence commitment, erasure non-resurrection, and
//! `proptest` disabled-optimization equivalence (`AC31-30-03`, `AC31-30-04`,
//! `AC31-30-05`, `AC31-30-10`).

use std::collections::{BTreeMap, BTreeSet};

use fathomdb_engine::{
    decode_graph_expand_request_v1, encode_graph_expand_request_v1, ArtifactRevisionId,
    CanonicalHash, Engine, EngineError, EvidenceErrorReasonV1, GraphEvidenceArtifactV1,
    GraphEvidenceResolveRequestV1, GraphExpandRequestV1, GraphExpansionErrorReasonV1,
    GraphReadContextV1, GraphSeedV1, IdSpace, InitialState, LifecycleState, PreparedWrite,
    ProjectionRole, ProjectionSpec, PropertyComparisonOp, PropertyEntityClass, PropertyMap,
    PropertyPredicate, PropertyValue, ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1,
    ReadView, SearchFilter, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
    TraversalDirection, WriteProvenanceV1,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

const INSTANT: i64 = 1_750_000_000;

fn db_path(dir: &TempDir, name: &str) -> std::path::PathBuf {
    dir.path().join(format!("{name}{SQLITE_SUFFIX}"))
}

fn node(id: &str, kind: &str, body: &str, props: Option<PropertyMap>) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.into()),
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new("slice30-src").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
        properties: props,
    }
}

fn edge(
    logical_id: &str,
    kind: &str,
    from: &str,
    to: &str,
    t_valid: Option<i64>,
    t_invalid: Option<i64>,
    props: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("slice30-src").unwrap(),
        body: None,
        t_valid,
        t_invalid,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: props,
    }
}

fn current_ctx(valid_as_of: Option<i64>) -> GraphReadContextV1 {
    let view = ReadView { valid_as_of, ..ReadView::default() };
    GraphReadContextV1::Current {
        schema_version: 1,
        context: ReadContextV1::new(view, SearchFilter::default()).unwrap(),
    }
}

fn explicit_req(
    seeds: &[&str],
    direction: TraversalDirection,
    max_depth: u32,
    result_limit: u32,
    max_work_units: u64,
) -> GraphExpandRequestV1 {
    GraphExpandRequestV1::new(
        GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: seeds.iter().map(|s| IdSpace::logical(*s)).collect(),
        },
        direction,
        current_ctx(Some(INSTANT)),
        max_depth,
        result_limit,
        max_work_units,
    )
}

fn pred(key: &str, op: PropertyComparisonOp, value: PropertyValue) -> PropertyPredicate {
    PropertyPredicate { key: key.into(), op, value }
}

fn graph_err(
    result: Result<fathomdb_engine::GraphExpandResultV1, EngineError>,
) -> (GraphExpansionErrorReasonV1, String) {
    match result.unwrap_err() {
        EngineError::GraphExpansion(e) => (e.reason, e.field_path),
        other => panic!("expected GraphExpansion error, got {other:?}"),
    }
}

fn digest(body: &str) -> String {
    Sha256::digest(body.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn sql_pushdown_filters_superseded_wrong_kind_expired_and_future_t_valid_with_zero_work() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "pushdown")).unwrap();

    let mut writes = vec![
        node("root", "seed", "root", None),
        node("live", "fact", "live target", None),
        node("dead-sup", "fact", "superseded target", None),
        node("wrong-kind", "fact", "wrong kind target", None),
        node("expired", "fact", "expired target", None),
        node("future", "fact", "future target", None),
        // This edge will be superseded by the second write batch.
        edge("e-sup", "supports", "root", "dead-sup", None, None, None),
    ];
    for i in 0..20 {
        writes.push(edge(
            &format!("e-wrong-{i}"),
            "contradicts",
            "root",
            "wrong-kind",
            None,
            None,
            None,
        ));
        writes.push(edge(
            &format!("e-exp-{i}"),
            "supports",
            "root",
            "expired",
            None,
            Some(INSTANT - 1),
            None,
        ));
        writes.push(edge(
            &format!("e-eq-exp-{i}"),
            "supports",
            "root",
            "expired",
            None,
            Some(INSTANT),
            None,
        ));
        writes.push(edge(
            &format!("e-fut-{i}"),
            "supports",
            "root",
            "future",
            Some(INSTANT + 1),
            None,
            None,
        ));
    }
    opened.engine.write(&writes).unwrap();
    // Supersede e-sup so the old row is superseded and the new active row points to `live`
    // with `t_valid == INSTANT` (inclusive lower bound) and `t_invalid == INSTANT + 1` (exclusive upper bound).
    opened
        .engine
        .write(&[edge("e-sup", "supports", "root", "live", Some(INSTANT), Some(INSTANT + 1), None)])
        .unwrap();

    // Even with `max_work_units = 1`, all 81 non-matching/superseded/out-of-window edges
    // must be pushed down in SQL and consume 0 LIMIT slots and 0 work_units.
    let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 1)
        .with_edge_kinds(vec!["supports".into()]);
    let res = opened.engine.graph_expand(&req).unwrap();
    let ids: Vec<&str> = res.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(ids, vec!["live"]);
    assert_eq!(res.work_units, 1);
}

#[test]
fn both_direction_union_all_does_not_duplicate_self_loops() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "both-self-loop")).unwrap();

    opened
        .engine
        .write(&[
            node("root", "seed", "root", None),
            node("out-nbr", "fact", "out", None),
            node("in-nbr", "fact", "in", None),
            edge("e-self", "link", "root", "root", None, None, None),
            edge("e-out", "link", "root", "out-nbr", None, None, None),
            edge("e-in", "link", "in-nbr", "root", None, None, None),
        ])
        .unwrap();

    // Only 3 distinct incident edges exist; self-loop must appear once, not twice.
    let req = explicit_req(&["root"], TraversalDirection::Both, 1, 10, 3);
    let res = opened.engine.graph_expand(&req).unwrap();
    assert_eq!(res.work_units, 3);
    assert_eq!(res.targets.len(), 2);
    assert_eq!(res.targets[0].logical_id, "out-nbr");
    assert_eq!(res.targets[0].origin.terminal_direction, TraversalDirection::Outgoing);
    assert_eq!(res.targets[1].logical_id, "in-nbr");
    assert_eq!(res.targets[1].origin.terminal_direction, TraversalDirection::Incoming);
}

#[test]
fn failing_edge_predicate_does_not_poison_visited_by_seed_for_parallel_edge() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "parallel-edge-visited")).unwrap();

    opened
        .engine
        .write(&[
            node("root", "seed", "root", None),
            node("mid", "fact", "mid", None),
            node("leaf", "fact", "leaf", None),
            // e1 sorts BEFORE e2 (kind "a" < "b"), but fails `score >= 5`.
            edge(
                "e1",
                "a_link",
                "root",
                "mid",
                None,
                None,
                Some(BTreeMap::from([("score".into(), PropertyValue::Int(1))])),
            ),
            edge(
                "e2",
                "b_link",
                "root",
                "mid",
                None,
                None,
                Some(BTreeMap::from([("score".into(), PropertyValue::Int(10))])),
            ),
            edge(
                "e3",
                "a_link",
                "mid",
                "leaf",
                None,
                None,
                Some(BTreeMap::from([("score".into(), PropertyValue::Int(20))])),
            ),
        ])
        .unwrap();

    let req =
        explicit_req(&["root"], TraversalDirection::Outgoing, 2, 10, 100).with_edge_predicates(
            vec![pred("score", PropertyComparisonOp::Gte, PropertyValue::Int(5))],
        );
    let res = opened.engine.graph_expand(&req).unwrap();
    let ids: Vec<&str> = res.targets.iter().map(|t| t.logical_id.as_str()).collect();
    assert_eq!(ids, vec!["mid", "leaf"]);
    assert_eq!(res.targets[0].origin.terminal_edge_kind, "b_link");
    assert_eq!(
        res.targets[0].origin.terminal_edge_properties,
        Some(BTreeMap::from([("score".into(), PropertyValue::Int(10))]))
    );
    assert_eq!(res.targets[1].origin.hop_count, 2);
}

#[test]
fn terminal_depth_early_skip_preserves_exact_origin_cmp_top_n_across_result_limits() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "terminal-skip-soundness")).unwrap();

    opened
        .engine
        .write(&[
            node("s0", "seed", "seed 0", None),
            node("s1", "seed", "seed 1", None),
            node(
                "mid_a",
                "hop",
                "mid a",
                Some(BTreeMap::from([("keep".into(), PropertyValue::Bool(false))])),
            ),
            node(
                "mid_b",
                "hop",
                "mid b",
                Some(BTreeMap::from([("keep".into(), PropertyValue::Bool(false))])),
            ),
            node(
                "leaf_1",
                "fact",
                "leaf 1",
                Some(BTreeMap::from([("keep".into(), PropertyValue::Bool(true))])),
            ),
            node(
                "leaf_2",
                "fact",
                "leaf 2",
                Some(BTreeMap::from([("keep".into(), PropertyValue::Bool(true))])),
            ),
            node(
                "leaf_3",
                "fact",
                "leaf 3",
                Some(BTreeMap::from([("keep".into(), PropertyValue::Bool(true))])),
            ),
            edge("e-s0-mb", "link", "s0", "mid_b", None, None, None),
            edge("e-s0-ma", "link", "s0", "mid_a", None, None, None),
            edge("e-s1-mb", "link", "s1", "mid_b", None, None, None),
            edge("e-mb-l3", "link", "mid_b", "leaf_3", None, None, None),
            edge("e-mb-l1", "link", "mid_b", "leaf_1", None, None, None),
            edge("e-ma-l2", "link", "mid_a", "leaf_2", None, None, None),
            edge("e-ma-l1", "link", "mid_a", "leaf_1", None, None, None),
        ])
        .unwrap();

    let keep = vec![pred("keep", PropertyComparisonOp::Eq, PropertyValue::Bool(true))];
    let full = opened
        .engine
        .graph_expand(
            &explicit_req(&["s0", "s1"], TraversalDirection::Outgoing, 2, 50, 1000)
                .with_target_predicates(keep.clone()),
        )
        .unwrap();
    assert_eq!(full.targets.len(), 3);

    for limit in 1..=3 {
        let limited = opened
            .engine
            .graph_expand(
                &explicit_req(&["s0", "s1"], TraversalDirection::Outgoing, 2, limit, 1000)
                    .with_target_predicates(keep.clone()),
            )
            .unwrap();
        assert_eq!(
            limited.targets,
            full.targets[..limit as usize],
            "result_limit={limit} must match exact prefix of unbounded origin_cmp ordering"
        );
    }
}

#[test]
fn closed_predicate_type_matrix_missing_keys_body_only_and_cross_type() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "predicate-matrix")).unwrap();

    let big_lo: i64 = (1_i64 << 53) + 1; // 9007199254740993
    let big_hi: i64 = (1_i64 << 53) + 2; // 9007199254740994

    opened
        .engine
        .write(&[
            node("root", "seed", "root", None),
            node(
                "n-str",
                "item",
                "string node",
                Some(BTreeMap::from([("k".into(), PropertyValue::String("beta".into()))])),
            ),
            node(
                "n-int-lo",
                "item",
                "int lo",
                Some(BTreeMap::from([("k".into(), PropertyValue::Int(big_lo))])),
            ),
            node(
                "n-int-hi",
                "item",
                "int hi",
                Some(BTreeMap::from([("k".into(), PropertyValue::Int(big_hi))])),
            ),
            node(
                "n-int-min",
                "item",
                "int min",
                Some(BTreeMap::from([("k".into(), PropertyValue::Int(i64::MIN))])),
            ),
            node(
                "n-int-max",
                "item",
                "int max",
                Some(BTreeMap::from([("k".into(), PropertyValue::Int(i64::MAX))])),
            ),
            node(
                "n-flt",
                "item",
                "float node",
                Some(BTreeMap::from([("k".into(), PropertyValue::Float(0.0))])),
            ),
            node(
                "n-bool-t",
                "item",
                "bool true",
                Some(BTreeMap::from([("k".into(), PropertyValue::Bool(true))])),
            ),
            node(
                "n-bool-f",
                "item",
                "bool false",
                Some(BTreeMap::from([("k".into(), PropertyValue::Bool(false))])),
            ),
            node("n-missing", "item", "no props", None),
            // JSON body contains {"k": "beta"}, but inline properties is None -> MUST NOT match!
            node("n-body-json", "item", r#"{"k":"beta"}"#, None),
            edge("e1", "link", "root", "n-str", None, None, None),
            edge("e2", "link", "root", "n-int-lo", None, None, None),
            edge("e3", "link", "root", "n-int-hi", None, None, None),
            edge("e4", "link", "root", "n-int-min", None, None, None),
            edge("e5", "link", "root", "n-int-max", None, None, None),
            edge("e6", "link", "root", "n-flt", None, None, None),
            edge("e7", "link", "root", "n-bool-t", None, None, None),
            edge("e8", "link", "root", "n-bool-f", None, None, None),
            edge("e9", "link", "root", "n-missing", None, None, None),
            edge("e10", "link", "root", "n-body-json", None, None, None),
        ])
        .unwrap();

    let run = |p: PropertyPredicate| -> Vec<String> {
        let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 50, 100)
            .with_target_predicates(vec![p]);
        opened
            .engine
            .graph_expand(&req)
            .unwrap()
            .targets
            .into_iter()
            .map(|t| t.logical_id)
            .collect()
    };

    // Exact i64 > 2^53 precision (no f64 rounding).
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Eq, PropertyValue::Int(big_lo))),
        vec!["n-int-lo"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Gt, PropertyValue::Int(big_lo))),
        vec!["n-int-hi", "n-int-max"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Eq, PropertyValue::Int(i64::MIN))),
        vec!["n-int-min"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Eq, PropertyValue::Int(i64::MAX))),
        vec!["n-int-max"]
    );

    // -0.0 canonicalizes to 0.0 and matches Float(0.0).
    assert_eq!(run(pred("k", PropertyComparisonOp::Eq, PropertyValue::Float(-0.0))), vec!["n-flt"]);

    // String comparisons exclude n-missing, n-body-json, and all non-string types (even for Ne!).
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Eq, PropertyValue::String("beta".into()))),
        vec!["n-str"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Ne, PropertyValue::String("alpha".into()))),
        vec!["n-str"]
    );
    assert!(
        run(pred("k", PropertyComparisonOp::Ne, PropertyValue::String("beta".into()))).is_empty()
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Gt, PropertyValue::String("alpha".into()))),
        vec!["n-str"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Lte, PropertyValue::String("beta".into()))),
        vec!["n-str"]
    );

    // Bool Eq / Ne exclude missing, body-json, and non-bool types.
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Eq, PropertyValue::Bool(true))),
        vec!["n-bool-t"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Ne, PropertyValue::Bool(true))),
        vec!["n-bool-f"]
    );

    // Now declare `k` as Filterable on Node and confirm n-body-json STILL never matches!
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "k".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
                entity_class: PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Eq, PropertyValue::String("beta".into()))),
        vec!["n-str"]
    );
    assert_eq!(
        run(pred("k", PropertyComparisonOp::Ne, PropertyValue::String("alpha".into()))),
        vec!["n-str"]
    );
}

#[test]
fn predicate_validation_and_closed_wire_codec_enforce_all_bounds_and_pointers() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "validation")).unwrap();
    opened.engine.write(&[node("root", "seed", "root", None)]).unwrap();

    // Bool ordering refused for both edge and target predicates.
    for op in [
        PropertyComparisonOp::Lt,
        PropertyComparisonOp::Lte,
        PropertyComparisonOp::Gt,
        PropertyComparisonOp::Gte,
    ] {
        let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10)
            .with_edge_predicates(vec![pred("flag", op, PropertyValue::Bool(true))]);
        assert_eq!(
            graph_err(opened.engine.graph_expand(&req)),
            (
                GraphExpansionErrorReasonV1::GraphEdgePredicatesInvalid,
                "/edgePredicates/0/op".into()
            )
        );

        let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10)
            .with_target_predicates(vec![pred("flag", op, PropertyValue::Bool(false))]);
        assert_eq!(
            graph_err(opened.engine.graph_expand(&req)),
            (
                GraphExpansionErrorReasonV1::GraphTargetPredicatesInvalid,
                "/targetPredicates/0/op".into(),
            )
        );
    }

    // Empty key, NUL, RS in key and value; non-finite float.
    for bad_key in ["", "a\0b", "a\x1eb"] {
        let req =
            explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10).with_edge_predicates(
                vec![pred(bad_key, PropertyComparisonOp::Eq, PropertyValue::Int(1))],
            );
        assert_eq!(
            graph_err(opened.engine.graph_expand(&req)),
            (
                GraphExpansionErrorReasonV1::GraphEdgePredicatesInvalid,
                "/edgePredicates/0/key".into(),
            )
        );
    }
    for bad_val in [
        PropertyValue::String("bad\0val".into()),
        PropertyValue::String("bad\x1eval".into()),
        PropertyValue::Float(f64::NAN),
        PropertyValue::Float(f64::INFINITY),
        PropertyValue::Float(f64::NEG_INFINITY),
    ] {
        let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10)
            .with_target_predicates(vec![pred("k", PropertyComparisonOp::Eq, bad_val)]);
        assert_eq!(
            graph_err(opened.engine.graph_expand(&req)),
            (
                GraphExpansionErrorReasonV1::GraphTargetPredicatesInvalid,
                "/targetPredicates/0/value".into(),
            )
        );
    }

    // > 32 predicates refused.
    let too_many: Vec<PropertyPredicate> = (0..33)
        .map(|i| pred(&format!("k{i}"), PropertyComparisonOp::Eq, PropertyValue::Int(i)))
        .collect();
    let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10)
        .with_edge_predicates(too_many.clone());
    assert_eq!(
        graph_err(opened.engine.graph_expand(&req)),
        (GraphExpansionErrorReasonV1::GraphEdgePredicatesInvalid, "/edgePredicates".into())
    );
    let req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10)
        .with_target_predicates(too_many);
    assert_eq!(
        graph_err(opened.engine.graph_expand(&req)),
        (GraphExpansionErrorReasonV1::GraphTargetPredicatesInvalid, "/targetPredicates".into())
    );

    // Closed wire codec round-trip with exact i64 > 2^53 and explicit [] vs omitted.
    let big = (1_i64 << 53) + 1;
    let req = explicit_req(&["root"], TraversalDirection::Outgoing, 2, 10, 100)
        .with_edge_predicates(vec![
            pred("weight", PropertyComparisonOp::Gte, PropertyValue::Float(0.5)),
            pred("seq", PropertyComparisonOp::Eq, PropertyValue::Int(big)),
        ])
        .with_target_predicates(vec![
            pred("active", PropertyComparisonOp::Eq, PropertyValue::Bool(true)),
            pred("tier", PropertyComparisonOp::Ne, PropertyValue::String("cold".into())),
        ]);
    let encoded = encode_graph_expand_request_v1(&req).unwrap();
    let decoded = decode_graph_expand_request_v1(&encoded).unwrap();
    assert_eq!(decoded, req);

    // Empty predicate vectors are omitted on wire, while explicit `[]` decodes identically.
    let empty_req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 10);
    let empty_bytes = encode_graph_expand_request_v1(&empty_req).unwrap();
    let mut empty_json: Value = serde_json::from_slice(&empty_bytes).unwrap();
    assert!(empty_json.get("edgePredicates").is_none());
    assert!(empty_json.get("targetPredicates").is_none());
    empty_json["edgePredicates"] = json!([]);
    empty_json["targetPredicates"] = json!([]);
    assert_eq!(
        decode_graph_expand_request_v1(&serde_json::to_vec(&empty_json).unwrap()).unwrap(),
        empty_req
    );

    // Unknown field inside predicate object rejected with UnknownField.
    let mut bad_unknown = empty_json.clone();
    bad_unknown["edgePredicates"] = json!([{"key": "k", "op": "eq", "value": 1, "extra": true}]);
    let err =
        decode_graph_expand_request_v1(&serde_json::to_vec(&bad_unknown).unwrap()).unwrap_err();
    assert_eq!(err.reason, GraphExpansionErrorReasonV1::UnknownField);
    assert_eq!(err.field_path, "/edgePredicates/0/extra");

    // Non-scalar predicate value rejected with typed reason.
    let mut bad_val = empty_json.clone();
    bad_val["targetPredicates"] = json!([{"key": "k", "op": "eq", "value": [1, 2]}]);
    let err = decode_graph_expand_request_v1(&serde_json::to_vec(&bad_val).unwrap()).unwrap_err();
    assert_eq!(err.reason, GraphExpansionErrorReasonV1::GraphTargetPredicatesInvalid);
    assert_eq!(err.field_path, "/targetPredicates/0/value");
}

#[test]
fn intermediate_nodes_failing_target_predicates_still_traverse_without_displacing_result_limit() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "intermediate-target-preds")).unwrap();

    opened
        .engine
        .write(&[
            node("root", "seed", "root", None),
            node(
                "mid1",
                "hop",
                "mid1",
                Some(BTreeMap::from([("tier".into(), PropertyValue::String("internal".into()))])),
            ),
            node(
                "mid2",
                "hop",
                "mid2",
                Some(BTreeMap::from([("tier".into(), PropertyValue::String("internal".into()))])),
            ),
            node(
                "leaf",
                "fact",
                "leaf",
                Some(BTreeMap::from([("tier".into(), PropertyValue::String("final".into()))])),
            ),
            edge(
                "e1",
                "next",
                "root",
                "mid1",
                None,
                None,
                Some(BTreeMap::from([("w".into(), PropertyValue::Int(5))])),
            ),
            edge(
                "e2",
                "next",
                "mid1",
                "mid2",
                None,
                None,
                Some(BTreeMap::from([("w".into(), PropertyValue::Int(6))])),
            ),
            edge(
                "e3",
                "next",
                "mid2",
                "leaf",
                None,
                None,
                Some(BTreeMap::from([("w".into(), PropertyValue::Int(7))])),
            ),
        ])
        .unwrap();

    // Target predicates exclude mid1 and mid2, yet traversal MUST reach leaf at depth 3
    // with result_limit = 1!
    let req =
        explicit_req(&["root"], TraversalDirection::Outgoing, 3, 1, 100).with_target_predicates(
            vec![pred("tier", PropertyComparisonOp::Eq, PropertyValue::String("final".into()))],
        );
    let res = opened.engine.graph_expand(&req).unwrap();
    assert_eq!(res.targets.len(), 1);
    assert_eq!(res.targets[0].logical_id, "leaf");
    assert_eq!(res.targets[0].origin.hop_count, 3);
    assert_eq!(
        res.targets[0].properties,
        Some(BTreeMap::from([("tier".into(), PropertyValue::String("final".into()))]))
    );
    assert_eq!(
        res.targets[0].origin.terminal_edge_properties,
        Some(BTreeMap::from([("w".into(), PropertyValue::Int(7))]))
    );

    // Contrast: edge_predicates DO stop traversal when an intermediate edge fails them.
    let blocked_req = explicit_req(&["root"], TraversalDirection::Outgoing, 3, 10, 100)
        .with_edge_predicates(vec![pred("w", PropertyComparisonOp::Gt, PropertyValue::Int(5))]);
    let blocked = opened.engine.graph_expand(&blocked_req).unwrap();
    assert!(blocked.targets.is_empty());
}

#[test]
fn graph_evidence_hydration_and_erasure_non_resurrection_with_predicates() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "evidence-erasure")).unwrap();
    let source_body = "canonical source document";
    let derived_from = |rev: &str, src_ver: &str, src_rev: &str| {
        WriteProvenanceV1::derived(
            ArtifactRevisionId::new(rev).unwrap(),
            SourceVersionId::new(src_ver).unwrap(),
            SourceRevisionId::new(src_rev).unwrap(),
            SourceLocator::whole_body(),
            CanonicalHash::sha256(digest(source_body)).unwrap(),
        )
    };

    let target_props = BTreeMap::from([
        ("tier".into(), PropertyValue::String("gold".into())),
        ("rank".into(), PropertyValue::Int(42)),
    ]);
    let edge_props = BTreeMap::from([
        ("weight".into(), PropertyValue::Float(0.95)),
        ("verified".into(), PropertyValue::Bool(true)),
    ]);

    opened
        .engine
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

    opened
        .engine
        .write(&[
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("doc".into()),
                kind: "document".into(),
                body: source_body.into(),
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
                logical_id: Some("doc-target".into()),
                kind: "document".into(),
                body: source_body.into(),
                source_id: SourceId::new("owner-target").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: WriteProvenanceV1::canonical(
                    ArtifactRevisionId::new("src-target-r1").unwrap(),
                    SourceVersionId::new("src-target-v1").unwrap(),
                ),
                properties: None,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("root".into()),
                kind: "claim".into(),
                body: "root claim".into(),
                source_id: SourceId::new("owner").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_from("root-r1", "src-v1", "src-r1"),
                properties: None,
            }),
            PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
                logical_id: Some("target".into()),
                kind: "claim".into(),
                body: "target claim".into(),
                source_id: SourceId::new("owner-target").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                provenance: derived_from("target-r1", "src-target-v1", "src-target-r1"),
                properties: Some(target_props.clone()),
            }),
            PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
                logical_id: Some("e-win".into()),
                kind: "supports".into(),
                from: "root".into(),
                to: "target".into(),
                source_id: SourceId::new("owner-target").unwrap(),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: Some(0.95),
                extractor_model_id: None,
                temporal_fallback: None,
                provenance: derived_from("edge-r1", "src-target-v1", "src-target-r1"),
                properties: Some(edge_props.clone()),
            }),
        ])
        .unwrap();
    opened.engine.drain(30_000).unwrap();

    let frozen = opened
        .engine
        .freeze_read_context(
            &ReadContextV1::new(
                ReadView { valid_as_of: Some(INSTANT), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        )
        .unwrap();

    let req = GraphExpandRequestV1::new(
        GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical("root")] },
        TraversalDirection::Outgoing,
        GraphReadContextV1::Frozen { schema_version: 1, context: frozen.clone() },
        1,
        10,
        100,
    )
    .with_edge_predicates(vec![
        pred("weight", PropertyComparisonOp::Gte, PropertyValue::Float(0.9)),
        pred("verified", PropertyComparisonOp::Eq, PropertyValue::Bool(true)),
    ])
    .with_target_predicates(vec![
        pred("tier", PropertyComparisonOp::Eq, PropertyValue::String("gold".into())),
        pred("rank", PropertyComparisonOp::Gt, PropertyValue::Int(10)),
    ])
    .with_evidence(true);

    let res = opened.engine.graph_expand(&req).unwrap();
    assert_eq!(res.targets.len(), 1);
    assert_eq!(res.targets[0].properties, Some(target_props.clone()));
    assert_eq!(res.targets[0].origin.terminal_edge_properties, Some(edge_props.clone()));
    let entry = &res.evidence.as_ref().unwrap().entries[0];

    let resolved_target = opened
        .engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            context: frozen.clone(),
            evidence_ref: entry.target_evidence_ref.clone(),
        })
        .unwrap();
    match resolved_target.artifact {
        GraphEvidenceArtifactV1::Node { properties, .. } => {
            assert_eq!(properties, Some(target_props));
        }
        other => panic!("expected Node artifact, got {other:?}"),
    }

    let resolved_edge = opened
        .engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            context: frozen.clone(),
            evidence_ref: entry.terminal_edge_evidence_ref.clone(),
        })
        .unwrap();
    match resolved_edge.artifact {
        GraphEvidenceArtifactV1::Edge { properties, .. } => {
            assert_eq!(properties, Some(edge_props));
        }
        other => panic!("expected Edge artifact, got {other:?}"),
    }

    // Physical erasure of owner-target must wipe target & edge properties and prevent resurrection.
    opened.engine.erase_source("owner-target").unwrap();
    let after_req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 100);
    let after = opened.engine.graph_expand(&after_req).unwrap();
    assert!(after.targets.is_empty());

    let target_err = opened
        .engine
        .resolve_graph_evidence(&GraphEvidenceResolveRequestV1 {
            schema_version: 1,
            context: frozen.clone(),
            evidence_ref: entry.target_evidence_ref.clone(),
        })
        .unwrap_err();
    assert!(matches!(
        target_err,
        EngineError::Evidence(e) if e.reason == EvidenceErrorReasonV1::EvidenceUnavailable
    ));

    // Also verify `purge` on a node with properties wipes target & edge properties from graph_expand.
    opened
        .engine
        .write(&[
            node(
                "purge-target",
                "claim",
                "to be purged",
                Some(BTreeMap::from([("tier".into(), PropertyValue::String("gold".into()))])),
            ),
            edge(
                "e-purge",
                "supports",
                "root",
                "purge-target",
                None,
                None,
                Some(BTreeMap::from([("weight".into(), PropertyValue::Float(0.99))])),
            ),
        ])
        .unwrap();
    let before_purge = opened
        .engine
        .graph_expand(&explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 100))
        .unwrap();
    assert_eq!(before_purge.targets.len(), 1);
    assert!(before_purge.targets[0].properties.is_some());
    assert!(before_purge.targets[0].origin.terminal_edge_properties.is_some());

    opened.engine.transition("purge-target", LifecycleState::Deleted, None).unwrap();
    opened.engine.purge("purge-target").unwrap();
    let after_purge = opened
        .engine
        .graph_expand(&explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, 100))
        .unwrap();
    assert!(after_purge.targets.is_empty());
}

#[test]
fn max_work_units_boundary_exact_vs_exceeded_with_predicates() {
    for declare_filterable in [false, true] {
        let dir = TempDir::new().unwrap();
        let opened =
            Engine::open(db_path(&dir, &format!("work-bound-{declare_filterable}"))).unwrap();
        if declare_filterable {
            opened
                .engine
                .configure_projections(
                    &[ProjectionSpec {
                        name: "w".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Edge,
                    }],
                    &[],
                )
                .unwrap();
        }
        let mut writes = vec![node("root", "seed", "root", None)];
        for i in 0..5 {
            writes.push(node(&format!("t{i}"), "fact", &format!("body {i}"), None));
            writes.push(edge(
                &format!("e{i}"),
                "link",
                "root",
                &format!("t{i}"),
                None,
                None,
                Some(BTreeMap::from([("w".into(), PropertyValue::Int(i))])),
            ));
        }
        opened.engine.write(&writes).unwrap();

        let p = vec![pred("w", PropertyComparisonOp::Gte, PropertyValue::Int(2))];
        // When declared Filterable, SQL pushes `w >= 2` so only 3 edges (`w=2,3,4`) are scanned;
        // when undeclared, all 5 edges are scanned inline.
        let expected_work = if declare_filterable { 3 } else { 5 };

        let ok_req = explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, expected_work)
            .with_edge_predicates(p.clone());
        let ok = opened.engine.graph_expand(&ok_req).unwrap();
        assert_eq!(ok.work_units, expected_work);
        assert_eq!(ok.targets.len(), 3);

        let fail_req =
            explicit_req(&["root"], TraversalDirection::Outgoing, 1, 10, expected_work - 1)
                .with_edge_predicates(p);
        assert_eq!(
            graph_err(opened.engine.graph_expand(&fail_req)),
            (GraphExpansionErrorReasonV1::GraphExpansionBoundExceeded, "/maxWorkUnits".into())
        );
    }
}

// ===== Proptest: AC31-30-05 Disabled-Optimization Equivalence ===============

#[derive(Clone, Debug)]
struct GenNode {
    id: String,
    kind: String,
    body: String,
    props: Option<PropertyMap>,
}

#[derive(Clone, Debug)]
struct GenEdge {
    logical_id: String,
    kind: String,
    from: usize,
    to: usize,
    props: Option<PropertyMap>,
}

fn prop_value_strategy() -> impl Strategy<Value = PropertyValue> {
    prop_oneof![
        prop_oneof![Just("alpha"), Just("beta"), Just("gamma"), Just("")]
            .prop_map(|s| PropertyValue::String(s.into())),
        prop_oneof![
            Just(i64::MIN),
            Just(-((1_i64 << 53) + 1)),
            Just(-10_i64),
            Just(0_i64),
            Just(5_i64),
            Just(10_i64),
            Just((1_i64 << 53) + 1),
            Just(i64::MAX),
        ]
        .prop_map(PropertyValue::Int),
        prop_oneof![Just(-10.5), Just(-0.0), Just(0.0), Just(5.0), Just(5.5), Just(10.0)]
            .prop_map(PropertyValue::Float),
        any::<bool>().prop_map(PropertyValue::Bool),
    ]
}

fn prop_map_strategy(keys: &'static [&'static str]) -> impl Strategy<Value = Option<PropertyMap>> {
    prop_oneof![
        Just(None),
        proptest::collection::vec((proptest::sample::select(keys), prop_value_strategy()), 1..=3)
            .prop_map(|entries| {
                let mut map = PropertyMap::new();
                for (k, v) in entries {
                    map.insert(k.to_string(), v);
                }
                Some(map)
            }),
    ]
}

fn predicate_strategy(
    keys: &'static [&'static str],
) -> impl Strategy<Value = Vec<PropertyPredicate>> {
    let single =
        (proptest::sample::select(keys), prop_value_strategy()).prop_flat_map(|(key, value)| {
            let op_strat = match value {
                PropertyValue::Bool(_) => {
                    prop_oneof![Just(PropertyComparisonOp::Eq), Just(PropertyComparisonOp::Ne)]
                        .boxed()
                }
                _ => prop_oneof![
                    Just(PropertyComparisonOp::Eq),
                    Just(PropertyComparisonOp::Ne),
                    Just(PropertyComparisonOp::Lt),
                    Just(PropertyComparisonOp::Lte),
                    Just(PropertyComparisonOp::Gt),
                    Just(PropertyComparisonOp::Gte),
                ]
                .boxed(),
            };
            op_strat.prop_map(move |op| PropertyPredicate {
                key: key.to_string(),
                op,
                value: value.clone(),
            })
        });
    proptest::collection::vec(single, 0..=3)
}

const NODE_KEYS: &[&str] = &["nk1", "nk2"];
const EDGE_KEYS: &[&str] = &["ek1", "ek2"];

#[derive(Clone, Debug)]
struct GenGraphCase {
    nodes: Vec<GenNode>,
    edges: Vec<GenEdge>,
    direction: TraversalDirection,
    max_depth: u32,
    result_limit: u32,
    edge_preds: Vec<PropertyPredicate>,
    target_preds: Vec<PropertyPredicate>,
}

fn graph_case_strategy() -> impl Strategy<Value = GenGraphCase> {
    let node_count = 6_usize;
    let nodes_strat = proptest::collection::vec(
        (
            prop_oneof![Just("kind_a"), Just("kind_b")],
            prop_oneof![
                Just("plain body".to_string()),
                Just(r#"{"nk1":"alpha","nk2":5}"#.to_string()),
                Just(r#"{"nk1":true,"nk2":10.0}"#.to_string()),
            ],
            prop_map_strategy(NODE_KEYS),
        ),
        node_count..=node_count,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .enumerate()
            .map(|(i, (kind, body, props))| GenNode {
                id: format!("n{i}"),
                kind: kind.to_string(),
                body,
                props,
            })
            .collect::<Vec<_>>()
    });

    let edges_strat = proptest::collection::vec(
        (
            prop_oneof![Just("e_a"), Just("e_b")],
            0..node_count,
            0..node_count,
            prop_map_strategy(EDGE_KEYS),
        ),
        0..=16,
    )
    .prop_map(|rows| {
        rows.into_iter()
            .enumerate()
            .map(|(i, (kind, from, to, props))| GenEdge {
                logical_id: format!("e{i}"),
                kind: kind.to_string(),
                from,
                to,
                props,
            })
            .collect::<Vec<_>>()
    });

    (
        nodes_strat,
        edges_strat,
        prop_oneof![
            Just(TraversalDirection::Outgoing),
            Just(TraversalDirection::Incoming),
            Just(TraversalDirection::Both),
        ],
        1_u32..=3,
        1_u32..=6,
        predicate_strategy(EDGE_KEYS),
        predicate_strategy(NODE_KEYS),
    )
        .prop_map(
            |(nodes, edges, direction, max_depth, result_limit, edge_preds, target_preds)| {
                GenGraphCase {
                    nodes,
                    edges,
                    direction,
                    max_depth,
                    result_limit,
                    edge_preds,
                    target_preds,
                }
            },
        )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn proptest_filterable_cpv_prefilter_matches_undeclared_inline_evaluation(
        case in graph_case_strategy()
    ) {
        let GenGraphCase {
            nodes,
            edges,
            direction,
            max_depth,
            result_limit,
            edge_preds,
            target_preds,
        } = case;
        let dir = TempDir::new().unwrap();
        let undeclared = Engine::open(db_path(&dir, "undeclared")).unwrap().engine;
        let declared = Engine::open(db_path(&dir, "declared")).unwrap().engine;

        declared
            .configure_projections(
                &[
                    ProjectionSpec {
                        name: "nk1".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    },
                    ProjectionSpec {
                        name: "nk2".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    },
                    ProjectionSpec {
                        name: "ek1".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Edge,
                    },
                    ProjectionSpec {
                        name: "ek2".into(),
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

        let mut writes = Vec::with_capacity(nodes.len() + edges.len());
        for n in &nodes {
            writes.push(node(&n.id, &n.kind, &n.body, n.props.clone()));
        }
        for e in &edges {
            writes.push(edge(
                &e.logical_id,
                &e.kind,
                &nodes[e.from].id,
                &nodes[e.to].id,
                None,
                None,
                e.props.clone(),
            ));
        }
        undeclared.write(&writes).unwrap();
        declared.write(&writes).unwrap();

        let req = explicit_req(&["n0"], direction, max_depth, result_limit, 10_000)
            .with_edge_predicates(edge_preds)
            .with_target_predicates(target_preds);

        let res_undeclared = undeclared.graph_expand(&req).unwrap();
        let res_declared = declared.graph_expand(&req).unwrap();

        prop_assert_eq!(res_declared.seeds, res_undeclared.seeds);
        prop_assert_eq!(res_declared.targets, res_undeclared.targets);
    }
}
