//! 0.8.31 Slice 30 Layer 2 (`WP-30B`) — `GraphExpandRequestV1` closed typed
//! predicates, SQL pushdown, `UNION ALL`, `t_valid` gating, terminal-skip
//! soundness, closed wire/evidence commitment, erasure non-resurrection, and
//! `proptest` disabled-optimization equivalence (`AC31-30-03`, `AC31-30-04`,
//! `AC31-30-05`, `AC31-30-10`).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    decode_graph_expand_request_v1, encode_graph_expand_request_v1, ArtifactRevisionId,
    CanonicalHash, Engine, EngineError, EvidenceErrorReasonV1, EvidenceGraphOriginV1,
    EvidenceResolveRequestV1, EvidenceSearchRequestV1, GraphArmOptions, GraphArmSeedSource,
    GraphEvidenceArtifactV1, GraphEvidenceResolveRequestV1, GraphExpandRequestV1,
    GraphExpansionErrorReasonV1, GraphReadContextV1, GraphSeedMode, GraphSeedV1, IdSpace,
    InitialState, LifecycleState, PreparedWrite, ProjectionRole, ProjectionSpec,
    PropertyComparisonOp, PropertyEntityClass, PropertyMap, PropertyPredicate, PropertyValue,
    ProvenancedEdgeV1, ProvenancedNodeV1, ReadContextV1, ReadView, SearchFilter,
    SoftFallbackBranch, SourceId, SourceLocator, SourceRevisionId, SourceVersionId,
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

// ===========================================================================
// Slice 30 Layer 3 (`WP-30C`): `search.rs` Graph Arm `SEED_FTS_N` Fix,
// Phase 2 `UNION ALL`, Opt-In `graph_seed` + Single-Pass CE Reuse,
// `PerHitExplain` & `EvidenceGraphOriginV1` Seed Provenance (`AC31-30-06`,
// `AC31-30-07`, `AC31-30-10`).
// ===========================================================================

fn edge_with_body(
    logical_id: &str,
    kind: &str,
    from: &str,
    to: &str,
    body: Option<&str>,
    source_id: &str,
    props: Option<PropertyMap>,
) -> PreparedWrite {
    PreparedWrite::Edge {
        logical_id: Some(logical_id.into()),
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new(source_id).unwrap(),
        body: body.map(str::to_string),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
        properties: props,
    }
}

/// Deterministic keyword-based embedder: assigns an orthogonal unit vector per
/// semantic cluster so vector search can match a node whose text body does NOT
/// share lexical tokens with the query (proving `FusedTopK` seeds from dense
/// vector hits where `GraphFts` finds zero FTS matches).
#[derive(Clone, Debug)]
struct SemanticClusterEmbedder;

impl Embedder for SemanticClusterEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice30-cluster-embedder", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let lower = text.to_ascii_lowercase();
        let vec = if lower.contains("orbital") || lower.contains("spacecraft") {
            vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]
        } else if lower.contains("glacier") || lower.contains("iceberg") {
            vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0]
        } else if lower.contains("botanical") || lower.contains("fern") {
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0]
        } else {
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 1.0]
        };
        Ok(vec)
    }
}

/// AC31-30-06: Adversarial >= 12-edge `SEED_FTS_N` proof.
/// Top-10 BM25 edges bridge an active hub and Pending/Deleted/filter-excluded
/// endpoints (plus disjoint mixed pairs); ineligible endpoints claim zero slots
/// and zero `seeds_considered` waste, all `SEED_FTS_N = 10` slots go to distinct
/// eligible endpoints, and edges 11..=14 still seed!
#[test]
fn wp30c_seed_fts_n_mixed_edges_never_waste_slots_or_starve_later_eligible_edges() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "seed_fts_n_adversarial")).unwrap();
    let engine = opened.engine;

    let mut writes = vec![node("hub", "person", "central person hub profile", None)];
    // 10 counterparties that will be ineligible (4 Pending, 4 Deleted, 2 kind="company" filter-excluded)
    for i in 0..10 {
        let kind = if i >= 8 { "company" } else { "person" };
        writes.push(node(&format!("inelig-{i}"), kind, &format!("counterparty {i} profile"), None));
    }
    // 10 clean eligible nodes reached ONLY via lower-BM25 edges 10..=14 (5 disjoint pairs + 1 mixed)
    for i in 0..10 {
        writes.push(node(&format!("clean-{i}"), "person", &format!("clean member {i}"), None));
    }
    // Top-10 BM25 edges: high term frequency ("zephyrium zephyrium zephyrium") bridging hub -> inelig-{0..10}
    for i in 0..10 {
        writes.push(edge_with_body(
            &format!("mixed-e-{i}"),
            "knows",
            "hub",
            &format!("inelig-{i}"),
            Some("zephyrium zephyrium zephyrium zephyrium"),
            "src:mixed",
            None,
        ));
    }
    // Lower-BM25 edges 11..=15: lower term frequency ("zephyrium" + long padding) connecting clean pairs
    for i in 0..5 {
        let a = format!("clean-{}", i * 2);
        let b = format!("clean-{}", i * 2 + 1);
        writes.push(edge_with_body(
            &format!("clean-e-{i}"),
            "knows",
            &a,
            &b,
            Some("zephyrium archival padding words one two three four five six seven eight nine"),
            "src:clean",
            None,
        ));
    }
    engine.write(&writes).unwrap();

    // Supersede inelig-0..4 to Pending and transition inelig-4..8 -> Deleted
    let pending_updates: Vec<PreparedWrite> = (0..4)
        .map(|i| PreparedWrite::Node {
            logical_id: Some(format!("inelig-{i}")),
            kind: "person".into(),
            body: format!("counterparty {i} profile pending"),
            source_id: SourceId::new("slice30-src").unwrap(),
            state: InitialState::Pending,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        })
        .collect();
    engine.write(&pending_updates).unwrap();
    for i in 4..8 {
        engine.transition(&format!("inelig-{i}"), LifecycleState::Deleted, None).unwrap();
    }

    let mut filter = SearchFilter::default();
    filter.kind = Some("person".into());
    let opts = GraphArmOptions::default();

    let stats = engine
        ._graph_frontier_stats_with_options_for_test(
            "zephyrium",
            Some(filter.clone()),
            &ReadView::default(),
            &opts,
        )
        .unwrap();

    // Zero ineligible endpoints in seeds_considered: every considered seed resolves!
    assert_eq!(
        stats.seeds_considered, stats.seeds_resolved,
        "ineligible endpoints must NEVER enter candidate_seeds or seeds_considered: {stats:?}"
    );
    // All SEED_FTS_N = 10 slots go to distinct eligible endpoints (hub + 9 clean nodes from edges 11..=15)!
    assert_eq!(
        stats.seeds_resolved, 10,
        "all SEED_FTS_N=10 slots must go to distinct eligible endpoints (no starvation): {stats:?}"
    );

    let res = engine
        .search_reranked_view_with_graph_options(
            "zephyrium",
            Some(filter),
            0,
            true,
            0.3,
            0,
            true,
            &ReadView::default(),
            20,
            None,
            &opts,
        )
        .unwrap();
    let hit_ids: BTreeSet<String> = res.results.iter().map(|h| h.id.value.clone()).collect();
    assert!(hit_ids.contains("hub"), "eligible hub endpoint must still seed: {hit_ids:?}");
    // Edges ranked below top-10 mixed edges MUST seed their clean endpoints!
    for i in 0..8 {
        assert!(
            hit_ids.contains(&format!("clean-{i}")),
            "clean-{i} from lower-BM25 edge must not be starved: {hit_ids:?}"
        );
    }
    for i in 0..10 {
        assert!(
            !hit_ids.contains(&format!("inelig-{i}")),
            "ineligible counterparty inelig-{i} must never appear: {hit_ids:?}"
        );
    }

    engine.close().unwrap();
}

/// AC31-30-07: Opt-in `graph_seed` (`GraphFts`, `FusedTopK`, `Both`), body-less
/// property-graph edges, vector + lexical fallback, `graph_seed_k` cap,
/// `PerHitExplain.{graph_seed_source, graph_hop_depth}`, and `logical_id IS NULL`
/// doc-node exclusion.
#[test]
fn wp30c_opt_in_fused_top_k_seeds_bodyless_edges_from_vector_and_lexical_hits() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        db_path(&dir, "fused_top_k_vec"),
        Arc::new(SemanticClusterEmbedder),
    )
    .unwrap();
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("seed_doc").unwrap();

    engine
        .write(&[
            // Doc node (logical_id = None) matching "orbital" — MUST be skipped by FusedTopK seeding
            PreparedWrite::Node {
                logical_id: None,
                kind: "seed_doc".into(),
                body: "orbital doc without logical id".into(),
                source_id: SourceId::new("src:doc").unwrap(),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            // Entity node matched ONLY by dense vector ("spacecraft" cluster == "orbital" cluster;
            // body does NOT contain "orbital" so FTS returns zero matches for "orbital"!)
            node("seed-craft", "seed_doc", "spacecraft telemetry station", None),
            // Neighbor (non-vector kind) reached ONLY via a BODY-LESS property-graph edge!
            node("nbr-propulsion", "entity", "xenon thruster assembly", None),
            // Body-less edge (body = None, properties = Some(...))
            edge(
                "e-bodyless",
                "uses_subsystem",
                "seed-craft",
                "nbr-propulsion",
                None,
                None,
                Some(BTreeMap::from([
                    ("tier".into(), PropertyValue::Int(1)),
                    ("active_link".into(), PropertyValue::Bool(true)),
                ])),
            ),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();

    // 1. Default `GraphFts` mode: "orbital" has NO matching edge-FTS and NO matching entity-FTS
    //    (only the logical_id=None doc node matched FTS; seed-craft matched only vector).
    let default_res = engine
        .search_reranked_view_with_graph_options(
            "orbital",
            None,
            0,
            true,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
            None,
            &GraphArmOptions::default(),
        )
        .unwrap();
    assert!(
        !default_res.results.iter().any(|h| h.id.value == "nbr-propulsion"),
        "default GraphFts must not seed body-less edge from vector-only hit"
    );

    // 2. Opt-in `FusedTopK` mode: vector hit `seed-craft` seeds the body-less edge!
    let mut fused_opts = GraphArmOptions::default();
    fused_opts.graph_seed = GraphSeedMode::FusedTopK;
    fused_opts.graph_seed_k = 5;
    let fused_res = engine
        .search_reranked_view_with_graph_options(
            "orbital",
            None,
            0,
            true,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
            None,
            &fused_opts,
        )
        .unwrap();
    let explain = fused_res.explanation.as_ref().expect("explanation");
    let nbr_idx = fused_res
        .results
        .iter()
        .position(|h| h.id.value == "nbr-propulsion")
        .expect("nbr-propulsion must be reached via body-less edge under FusedTopK");
    assert_eq!(fused_res.results[nbr_idx].branch, SoftFallbackBranch::GraphArm);
    assert_eq!(explain.per_hit[nbr_idx].graph_seed_source, Some(GraphArmSeedSource::FusedTopK));
    assert_eq!(explain.per_hit[nbr_idx].graph_hop_depth, Some(1));

    // Non-GraphArm hits carry None for graph_seed_source and graph_hop_depth.
    let seed_idx = fused_res
        .results
        .iter()
        .position(|h| h.id.value == "seed-craft")
        .expect("seed-craft present in two-arm hits");
    assert_ne!(fused_res.results[seed_idx].branch, SoftFallbackBranch::GraphArm);
    assert_eq!(explain.per_hit[seed_idx].graph_seed_source, None);
    assert_eq!(explain.per_hit[seed_idx].graph_hop_depth, None);

    // 3. No-reranker graceful fallback with `rerank_depth > 0`
    let rerank_fallback_res = engine
        .search_reranked_view_with_graph_options(
            "orbital",
            None,
            10,
            true,
            0.3,
            10,
            true,
            &ReadView::default(),
            10,
            None,
            &fused_opts,
        )
        .unwrap();
    assert!(
        rerank_fallback_res.results.iter().any(|h| h.id.value == "nbr-propulsion"),
        "rerank_depth > 0 without loaded reranker must gracefully fall back to two-arm FusedTopK"
    );

    engine.close().unwrap();

    // 4. No-embedder lexical fallback: FusedTopK still seeds from lexical FTS hits!
    let opened_no_emb = Engine::open(db_path(&dir, "fused_top_k_no_emb")).unwrap();
    let eng_no_emb = opened_no_emb.engine;
    eng_no_emb
        .write(&[
            node("lex-seed", "note", "crystalline quartz lattice", None),
            node("lex-nbr", "note", "piezoelectric resonator", None),
            edge("e-lex", "enables", "lex-seed", "lex-nbr", None, None, None),
        ])
        .unwrap();
    let lex_res = eng_no_emb
        .search_reranked_view_with_graph_options(
            "crystalline quartz",
            None,
            0,
            true,
            0.3,
            0,
            true,
            &ReadView::default(),
            10,
            None,
            &fused_opts,
        )
        .unwrap();
    let lex_exp = lex_res.explanation.as_ref().expect("explain");
    let idx = lex_res
        .results
        .iter()
        .position(|h| h.id.value == "lex-nbr")
        .expect("no-embedder lexical hit must seed body-less edge under FusedTopK");
    assert_eq!(lex_res.results[idx].branch, SoftFallbackBranch::GraphArm);
    assert_eq!(lex_exp.per_hit[idx].graph_seed_source, Some(GraphArmSeedSource::FusedTopK));
    assert_eq!(lex_exp.per_hit[idx].graph_hop_depth, Some(1));
    eng_no_emb.close().unwrap();
}

/// AC31-30-07 (Single-pass CE score reuse): under `rerank_depth > 0` with an active
/// CE scorer, Pass 1 reranks `two_arm_fused` so `graph_seed_k = 1` picks the CE-promoted
/// hit as seed, and Pass 2 reuses cached CE scores for two-arm hits — scoring each
/// two-arm hit strictly ONCE and invoking CE in Pass 2 ONLY on newly introduced GraphArm hits!
#[cfg(feature = "test-hooks")]
#[test]
fn wp30c_fused_top_k_rerank_single_pass_ce_reuse_zero_double_scoring() {
    use std::sync::Mutex;

    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "ce_single_pass")).unwrap();
    let engine = opened.engine;

    engine
        .write(&[
            // Higher BM25 score for "magnetar" (3 occurrences), but CE will score it low (-5.0)
            node("rrf-first", "note", "magnetar magnetar magnetar catalog entry", None),
            // Lower BM25 score (1 occurrence + padding), but CE will score it high (+5.0)
            node(
                "ce-winner",
                "note",
                "magnetar alpha beta gamma delta epsilon zeta eta theta iota",
                None,
            ),
            node("nbr-of-rrf", "note", "pulsar companion star", None),
            node("nbr-of-ce", "note", "gamma burst remnant", None),
            edge("e-rrf", "linked", "rrf-first", "nbr-of-rrf", None, None, None),
            edge("e-ce", "linked", "ce-winner", "nbr-of-ce", None, None, None),
        ])
        .unwrap();

    let scored_bodies: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let scored_clone = Arc::clone(&scored_bodies);
    let _guard = fathomdb_engine::install_test_ce_scorer_for_test(
        "magnetar",
        Arc::new(move |_query: &str, passages: &[&str]| {
            let mut log = scored_clone.lock().unwrap();
            passages
                .iter()
                .map(|body| {
                    log.push((*body).to_string());
                    if body.contains("alpha beta gamma") {
                        5.0_f32
                    } else if body.contains("gamma burst remnant") {
                        4.0_f32
                    } else {
                        -5.0_f32
                    }
                })
                .collect()
        }),
    );

    let mut opts = GraphArmOptions::default();
    opts.graph_seed = GraphSeedMode::FusedTopK;
    opts.graph_seed_k = 1; // Only top-1 seed is allowed!

    let res = engine
        .search_reranked_view_with_graph_options(
            "magnetar",
            None,
            10,
            true,
            0.9, // strong CE weight so ce-winner beats rrf-first
            10,
            true,
            &ReadView::default(),
            10,
            None,
            &opts,
        )
        .unwrap();

    let ids: Vec<&str> = res.results.iter().map(|h| h.id.value.as_str()).collect();
    assert!(
        ids.contains(&"nbr-of-ce"),
        "CE-promoted hit must be selected as top-1 FusedTopK seed: {ids:?}"
    );
    assert!(
        !ids.contains(&"nbr-of-rrf"),
        "RRF-first hit demoted by CE must NOT be selected when graph_seed_k=1: {ids:?}"
    );

    // Count how many times each passage body was sent to the cross-encoder.
    let log = scored_bodies.lock().unwrap();
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    for body in log.iter() {
        *counts.entry(body.clone()).or_insert(0) += 1;
    }
    for (body, count) in &counts {
        assert_eq!(
            *count, 1,
            "every passage (two-arm OR GraphArm) must be CE-scored at most ONCE; `{body}` was scored {count} times: {counts:?}"
        );
    }
    assert_eq!(
        counts.len(),
        3,
        "expected exactly 3 scored bodies (2 two-arm + 1 newly added GraphArm): {counts:?}"
    );

    engine.close().unwrap();
}

/// AC31-30-06 & AC31-30-07: `graph_edge_kinds` and `graph_edge_predicates` pruning:
/// parallel failing edge does not poison `visited`, >64 failing edges do not starve
/// the 64-neighbor-per-hop cap, declared vs undeclared `Filterable` parity (including
/// JSON `body` with `properties: None`), and request validation regardless of `use_graph_arm`.
#[test]
fn wp30c_graph_edge_kinds_predicates_pruning_parallel_edges_cap_and_validation() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(db_path(&dir, "edge_pred_pruning")).unwrap();
    let engine = opened.engine;

    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "weight".into(),
                roles: BTreeSet::from([ProjectionRole::Filterable]),
                fts: None,
                vector: None,
                source: None,
                entity_class: PropertyEntityClass::Edge,
            }],
            &[],
        )
        .unwrap();

    let mut writes = vec![
        node("root", "note", "obsidian monolith beacon", None),
        node("target-parallel", "note", "emerald vault", None),
        node("target-after-64", "note", "sapphire chamber", None),
        node("target-json-body-only", "note", "amethyst decoy", None),
        node("target-wrong-kind", "note", "ruby decoy", None),
    ];
    // 66 decoy nodes with low-weight edges written BEFORE target-after-64
    for i in 0..66 {
        writes.push(node(&format!("decoy-{i}"), "note", &format!("decoy {i}"), None));
        writes.push(edge(
            &format!("e-decoy-{i}"),
            "supports",
            "root",
            &format!("decoy-{i}"),
            None,
            None,
            Some(BTreeMap::from([
                ("weight".into(), PropertyValue::Int(1)),
                ("tag".into(), PropertyValue::String("reject".into())),
            ])),
        ));
    }
    // Parallel edges root -> target-parallel: first FAILS predicate, second PASSES!
    writes.push(edge(
        "e-par-fail",
        "supports",
        "root",
        "target-parallel",
        None,
        None,
        Some(BTreeMap::from([
            ("weight".into(), PropertyValue::Int(2)),
            ("tag".into(), PropertyValue::String("keep".into())),
        ])),
    ));
    writes.push(edge(
        "e-par-pass",
        "supports",
        "root",
        "target-parallel",
        None,
        None,
        Some(BTreeMap::from([
            ("weight".into(), PropertyValue::Int(90)),
            ("tag".into(), PropertyValue::String("keep".into())),
        ])),
    ));
    // Edge with JSON body and properties: None — MUST NOT match inline edge predicates!
    writes.push(edge_with_body(
        "e-json-body-only",
        "supports",
        "root",
        "target-json-body-only",
        Some(r#"{"weight": 99, "tag": "keep"}"#),
        "slice30-src",
        None,
    ));
    // Edge with wrong kind even though properties match
    writes.push(edge(
        "e-wrong-kind",
        "contradicts",
        "root",
        "target-wrong-kind",
        None,
        None,
        Some(BTreeMap::from([
            ("weight".into(), PropertyValue::Int(99)),
            ("tag".into(), PropertyValue::String("keep".into())),
        ])),
    ));
    // Passing edge written AFTER the 66 failing decoy edges
    writes.push(edge(
        "e-after-64",
        "supports",
        "root",
        "target-after-64",
        None,
        None,
        Some(BTreeMap::from([
            ("weight".into(), PropertyValue::Int(95)),
            ("tag".into(), PropertyValue::String("keep".into())),
        ])),
    ));
    engine.write(&writes).unwrap();

    // Combine declared Filterable ("weight") + undeclared ("tag") predicates + edge kind
    let mut opts = GraphArmOptions::default();
    opts.graph_seed = GraphSeedMode::FusedTopK;
    opts.graph_edge_kinds = vec!["supports".into()];
    opts.graph_edge_predicates = vec![
        pred("weight", PropertyComparisonOp::Gte, PropertyValue::Int(50)),
        pred("tag", PropertyComparisonOp::Eq, PropertyValue::String("keep".into())),
    ];

    let res = engine
        .search_reranked_view_with_graph_options(
            "obsidian monolith",
            None,
            0,
            true,
            0.3,
            0,
            true,
            &ReadView::default(),
            20,
            None,
            &opts,
        )
        .unwrap();
    let ids: BTreeSet<String> = res.results.iter().map(|h| h.id.value.clone()).collect();
    assert!(
        ids.contains("target-parallel"),
        "failing parallel edge #1 must not poison visited for passing edge #2: {ids:?}"
    );
    assert!(
        ids.contains("target-after-64"),
        "66 failing edges must not consume the 64-neighbor-per-hop cap: {ids:?}"
    );
    assert!(
        !ids.contains("target-json-body-only"),
        "JSON body with properties: None must NOT satisfy inline graph_edge_predicates: {ids:?}"
    );
    assert!(!ids.contains("target-wrong-kind"), "wrong edge kind must be filtered out: {ids:?}");

    // Validation at request entry REGARDLESS of `use_graph_arm = false`:
    for bad_k in [0_usize, 101_usize] {
        let mut bad = GraphArmOptions::default();
        bad.graph_seed_k = bad_k;
        let err = engine
            .search_reranked_view_with_graph_options(
                "obsidian",
                None,
                0,
                false, // use_graph_arm = false!
                0.3,
                0,
                false,
                &ReadView::default(),
                10,
                None,
                &bad,
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::InvalidArgument { .. }),
            "graph_seed_k={bad_k} must return InvalidArgument even with use_graph_arm=false, got {err:?}"
        );
    }

    for bad_kind in ["", "bad\0kind", "bad\x1ekind"] {
        let mut bad = GraphArmOptions::default();
        bad.graph_edge_kinds = vec![bad_kind.into()];
        let err = engine
            .search_reranked_view_with_graph_options(
                "obsidian",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView::default(),
                10,
                None,
                &bad,
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::InvalidFilter { .. }),
            "invalid graph_edge_kinds={bad_kind:?} must return InvalidFilter, got {err:?}"
        );
    }

    // > 32 graph_edge_predicates refused
    {
        let mut bad = GraphArmOptions::default();
        bad.graph_edge_predicates = (0..33)
            .map(|i| pred(&format!("k{i}"), PropertyComparisonOp::Eq, PropertyValue::Int(1)))
            .collect();
        let err = engine
            .search_reranked_view_with_graph_options(
                "obsidian",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView::default(),
                10,
                None,
                &bad,
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::InvalidFilter { .. }),
            ">32 graph_edge_predicates must return InvalidFilter, got {err:?}"
        );
    }

    // Bool ordering predicate refused
    {
        let mut bad = GraphArmOptions::default();
        bad.graph_edge_predicates =
            vec![pred("flag", PropertyComparisonOp::Lt, PropertyValue::Bool(true))];
        let err = engine
            .search_reranked_view_with_graph_options(
                "obsidian",
                None,
                0,
                false,
                0.3,
                0,
                false,
                &ReadView::default(),
                10,
                None,
                &bad,
            )
            .unwrap_err();
        assert!(
            matches!(err, EngineError::InvalidFilter { .. }),
            "Bool Lt predicate must return InvalidFilter, got {err:?}"
        );
    }

    engine.close().unwrap();
}

/// AC31-30-07 & AC31-30-10: Deterministic `Both` min-hop merge, `PerHitExplain` +
/// `EvidenceGraphOriginV1.seed_source` round-trip, token tamper rejection,
/// `use_graph_arm = false` byte-identical baseline, and `purge`/`erase_source` non-resurrection.
#[test]
fn wp30c_both_min_hop_merge_evidence_origin_roundtrip_tamper_baseline_and_erasure() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        db_path(&dir, "both_evidence"),
        Arc::new(SemanticClusterEmbedder),
    )
    .unwrap();
    let engine = opened.engine;
    engine.configure_vector_kind_for_test("seed_doc").unwrap();

    let pnode = |id: &str, kind: &str, body: &str, source_name: &str| {
        PreparedWrite::ProvenancedNode(ProvenancedNodeV1 {
            logical_id: Some(id.into()),
            kind: kind.into(),
            body: body.into(),
            source_id: SourceId::new(source_name).unwrap(),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(BTreeMap::from([("owner".into(), PropertyValue::String(id.into()))])),
            provenance: WriteProvenanceV1::canonical(
                ArtifactRevisionId::new(format!("{id}-r1")).unwrap(),
                SourceVersionId::new(format!("{id}-v1")).unwrap(),
            ),
        })
    };
    let pedge = |id: &str,
                 from: &str,
                 to: &str,
                 body: Option<&str>,
                 source_name: &str,
                 src_node_id: &str,
                 src_node_body: &str| {
        PreparedWrite::ProvenancedEdge(ProvenancedEdgeV1 {
            logical_id: Some(id.into()),
            kind: "relates".into(),
            from: from.into(),
            to: to.into(),
            source_id: SourceId::new(source_name).unwrap(),
            body: body.map(str::to_string),
            t_valid: None,
            t_invalid: None,
            confidence: None,
            extractor_model_id: None,
            temporal_fallback: None,
            properties: Some(BTreeMap::from([(
                "edge_id".into(),
                PropertyValue::String(id.into()),
            )])),
            provenance: WriteProvenanceV1::derived(
                ArtifactRevisionId::new(format!("{id}-r1")).unwrap(),
                SourceVersionId::new(format!("{src_node_id}-v1")).unwrap(),
                SourceRevisionId::new(format!("{src_node_id}-r1")).unwrap(),
                SourceLocator::whole_body(),
                CanonicalHash::sha256(digest(src_node_body)).unwrap(),
            ),
        })
    };

    // Graph topology for query "orbital":
    // - `fused-seed`: matches vector ("spacecraft" cluster == "orbital"), NO FTS match for "orbital"
    //   -> seeded ONLY by FusedTopK (depth 0)
    // - `edge-seed-a` <-> `edge-seed-b`: edge body contains "orbital", node bodies do NOT
    //   -> seeded ONLY by EdgeFts (depth 0, emitted as EdgeSeed)
    // - `entity-seed`: body contains "orbital" (FTS match -> EntityFts; note in Both mode it is
    //   also in two_arm_fused! So to test EntityFts independently we also run GraphFts mode below)
    // - `hop1-tie`: reachable at hop 1 from BOTH `fused-seed` (FusedTopK) and `edge-seed-a` (EdgeFts)
    //   -> equal-hop tie MUST be broken by (FusedTopK, EdgeFts, EntityFts) => FusedTopK!
    // - `hop1-edge-beats-hop2-fused`: reachable at hop 1 from `edge-seed-b` (EdgeFts) and hop 2
    //   from `fused-seed` (via `fused-mid`) -> min-hop wins => (EdgeFts, hop 1)!
    // - `fused-mid`: hop 1 from `fused-seed` (FusedTopK), connects to `hop1-edge-beats-hop2-fused`
    // - `erase-me`: hop 1 from `fused-seed` under source `src:erase-target`
    let fused_body = "spacecraft command module";
    let edge_a_body = "harbor lighthouse alpha";
    let edge_b_body = "harbor lighthouse beta";
    let entity_body = "orbital observatory dome";
    let mid_body = "intermediate relay node";
    let erase_body = "ephemeral secret payload";
    engine
        .write(&[
            pnode("fused-seed", "seed_doc", fused_body, "src:fused"),
            pnode("edge-seed-a", "entity", edge_a_body, "src:edge-a"),
            pnode("edge-seed-b", "entity", edge_b_body, "src:edge-b"),
            pnode("entity-seed", "entity", entity_body, "src:entity"),
            pnode("entity-nbr", "entity", "spectrograph mirror array", "src:entity-nbr"),
            pnode("hop1-tie", "entity", "shared junction node", "src:tie"),
            pnode("fused-mid", "entity", mid_body, "src:mid"),
            pnode("hop1-edge-beats-hop2-fused", "entity", "min hop target node", "src:minhop"),
            pnode("erase-me", "entity", erase_body, "src:erase-target"),
            pedge(
                "e-fts",
                "edge-seed-a",
                "edge-seed-b",
                Some("orbital beacon signal"),
                "src:edge-a",
                "edge-seed-a",
                edge_a_body,
            ),
            pedge(
                "e-fused-tie",
                "fused-seed",
                "hop1-tie",
                None,
                "src:fused",
                "fused-seed",
                fused_body,
            ),
            pedge(
                "e-edge-tie",
                "edge-seed-a",
                "hop1-tie",
                None,
                "src:edge-a",
                "edge-seed-a",
                edge_a_body,
            ),
            pedge(
                "e-fused-mid",
                "fused-seed",
                "fused-mid",
                None,
                "src:fused",
                "fused-seed",
                fused_body,
            ),
            pedge(
                "e-mid-minhop",
                "fused-mid",
                "hop1-edge-beats-hop2-fused",
                None,
                "src:mid",
                "fused-mid",
                mid_body,
            ),
            pedge(
                "e-edge-minhop",
                "edge-seed-b",
                "hop1-edge-beats-hop2-fused",
                None,
                "src:edge-b",
                "edge-seed-b",
                edge_b_body,
            ),
            pedge(
                "e-entity-nbr",
                "entity-seed",
                "entity-nbr",
                None,
                "src:entity",
                "entity-seed",
                entity_body,
            ),
            pedge(
                "e-erase",
                "fused-seed",
                "erase-me",
                None,
                "src:erase-target",
                "erase-me",
                erase_body,
            ),
        ])
        .unwrap();
    engine.drain(30_000).unwrap();

    // 1. Byte-identical baseline when `use_graph_arm = false`
    let baseline = engine.search("orbital").unwrap();
    let mut non_default_graph_opts = GraphArmOptions::default();
    non_default_graph_opts.graph_seed = GraphSeedMode::Both;
    non_default_graph_opts.graph_seed_k = 7;
    non_default_graph_opts.graph_edge_kinds = vec!["relates".into()];
    let disabled_arm = engine
        .search_reranked_view_with_graph_options(
            "orbital",
            None,
            0,
            false,
            0.3,
            0,
            false,
            &ReadView::default(),
            10,
            None,
            &non_default_graph_opts,
        )
        .unwrap();
    assert_eq!(
        format!("{:?}", (baseline.projection_cursor, &baseline.soft_fallback, &baseline.results)),
        format!(
            "{:?}",
            (disabled_arm.projection_cursor, &disabled_arm.soft_fallback, &disabled_arm.results)
        ),
        "use_graph_arm=false must remain 100% byte-identical regardless of GraphArmOptions"
    );

    // 2. Evidence search with `GraphSeedMode::Both`
    let ctx = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let req = EvidenceSearchRequestV1::new("orbital".into(), ctx.clone())
        .with_use_graph_arm(true)
        .with_explain(true)
        .with_limit(20)
        .with_graph_seed(GraphSeedMode::Both)
        .with_graph_seed_k(10);
    let ev_res = engine.search_with_evidence(&req).unwrap();
    let exp = ev_res.search_result.explanation.as_ref().expect("explanation");

    let lookup = |lid: &str| {
        let idx =
            ev_res.search_result.results.iter().position(|h| h.id.value == lid).unwrap_or_else(
                || panic!("expected hit `{lid}` in {:?}", ev_res.search_result.results),
            );
        let per_hit = &exp.per_hit[idx];
        let resolved = engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: ev_res.evidence[idx].evidence_ref.clone(),
                context: ctx.clone(),
            })
            .unwrap();
        (per_hit.clone(), resolved, ev_res.evidence[idx].evidence_ref.clone())
    };

    // Depth-0 EdgeFts seed (`edge-seed-a`)
    let (ph_ea, res_ea, ref_ea) = lookup("edge-seed-a");
    assert_eq!(ph_ea.graph_seed_source, Some(GraphArmSeedSource::EdgeFts));
    assert_eq!(ph_ea.graph_hop_depth, Some(0));
    assert!(
        matches!(
            res_ea.projection_origin.graph_origin,
            Some(EvidenceGraphOriginV1::EdgeSeed { seed_source: GraphArmSeedSource::EdgeFts, .. })
        ),
        "expected EdgeSeed(EdgeFts), got {:?}",
        res_ea.projection_origin.graph_origin
    );

    // Equal-hop tie (`hop1-tie`): FusedTopK beats EdgeFts at hop 1!
    let (ph_tie, res_tie, ref_tie) = lookup("hop1-tie");
    assert_eq!(ph_tie.graph_seed_source, Some(GraphArmSeedSource::FusedTopK));
    assert_eq!(ph_tie.graph_hop_depth, Some(1));
    assert!(
        matches!(
            res_tie.projection_origin.graph_origin,
            Some(EvidenceGraphOriginV1::Traversal {
                hop_count: 1,
                seed_source: GraphArmSeedSource::FusedTopK,
                ..
            })
        ),
        "expected Traversal(hop=1, FusedTopK), got {:?}",
        res_tie.projection_origin.graph_origin
    );

    // Min-hop beats source priority (`hop1-edge-beats-hop2-fused`): EdgeFts hop 1 beats FusedTopK hop 2!
    let (ph_min, res_min, _) = lookup("hop1-edge-beats-hop2-fused");
    assert_eq!(ph_min.graph_seed_source, Some(GraphArmSeedSource::EdgeFts));
    assert_eq!(ph_min.graph_hop_depth, Some(1));
    assert!(
        matches!(
            res_min.projection_origin.graph_origin,
            Some(EvidenceGraphOriginV1::Traversal {
                hop_count: 1,
                seed_source: GraphArmSeedSource::EdgeFts,
                ..
            })
        ),
        "expected Traversal(hop=1, EdgeFts), got {:?}",
        res_min.projection_origin.graph_origin
    );

    // 3. EntityFts provenance round-trip under `GraphSeedMode::GraphFts`
    let gfts_req = EvidenceSearchRequestV1::new("orbital".into(), ctx.clone())
        .with_use_graph_arm(true)
        .with_explain(true)
        .with_limit(20)
        .with_graph_seed(GraphSeedMode::GraphFts);
    let gfts_res = engine.search_with_evidence(&gfts_req).unwrap();
    let gfts_exp = gfts_res.search_result.explanation.as_ref().expect("explain");
    let ent_idx = gfts_res
        .search_result
        .results
        .iter()
        .position(|h| h.id.value == "entity-nbr")
        .expect("entity-nbr reached from entity-seed");
    assert_eq!(gfts_exp.per_hit[ent_idx].graph_seed_source, Some(GraphArmSeedSource::EntityFts));
    assert_eq!(gfts_exp.per_hit[ent_idx].graph_hop_depth, Some(1));
    let res_ent = engine
        .resolve_evidence(&EvidenceResolveRequestV1 {
            schema_version: 1,
            evidence_ref: gfts_res.evidence[ent_idx].evidence_ref.clone(),
            context: ctx.clone(),
        })
        .unwrap();
    assert!(
        matches!(
            res_ent.projection_origin.graph_origin,
            Some(EvidenceGraphOriginV1::Traversal {
                hop_count: 1,
                seed_source: GraphArmSeedSource::EntityFts,
                ..
            })
        ),
        "expected Traversal(hop=1, EntityFts), got {:?}",
        res_ent.projection_origin.graph_origin
    );

    // 4. Token tamper rejection on GraphArm evidence tokens (flip payload hex char)
    for token in [ref_ea.as_str(), ref_tie.as_str()] {
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
        let mut payload_bytes = parts[1].as_bytes().to_vec();
        let mid = payload_bytes.len() - 4;
        payload_bytes[mid] = if payload_bytes[mid] == b'0' { b'1' } else { b'0' };
        let tampered =
            format!("{}.{}.{}", parts[0], String::from_utf8(payload_bytes).unwrap(), parts[2]);
        let err = engine
            .resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: fathomdb_engine::EvidenceRefV1::new(tampered).unwrap(),
                context: ctx.clone(),
            })
            .unwrap_err();
        assert!(
            matches!(
                err,
                EngineError::Evidence(ref e) if e.reason == EvidenceErrorReasonV1::EvidenceUnavailable
            ),
            "tampered token must return EvidenceUnavailable, got {err:?}"
        );
    }

    // 5. Erasure non-resurrection (`erase_source` + `purge`)
    let (_, _, erase_ref) = lookup("erase-me");
    engine.erase_source("src:erase-target").unwrap();
    engine.transition("fused-mid", LifecycleState::Deleted, None).unwrap();
    engine.purge("fused-mid").unwrap();

    let post_ctx = engine
        .freeze_read_context(
            &ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        )
        .unwrap();
    let post_req = EvidenceSearchRequestV1::new("orbital".into(), post_ctx.clone())
        .with_use_graph_arm(true)
        .with_explain(true)
        .with_limit(20)
        .with_graph_seed(GraphSeedMode::Both);
    let post_res = engine.search_with_evidence(&post_req).unwrap();
    let post_ids: BTreeSet<String> =
        post_res.search_result.results.iter().map(|h| h.id.value.clone()).collect();
    assert!(!post_ids.contains("erase-me"), "erased source must not resurface: {post_ids:?}");
    assert!(!post_ids.contains("fused-mid"), "purged node must not resurface: {post_ids:?}");
    assert!(
        matches!(
            engine.resolve_evidence(&EvidenceResolveRequestV1 {
                schema_version: 1,
                evidence_ref: erase_ref,
                context: post_ctx,
            }),
            Err(EngineError::Evidence(ref e)) if e.reason == EvidenceErrorReasonV1::EvidenceUnavailable
        ),
        "erased evidence token must fail closed"
    );

    engine.close().unwrap();
}

/// Regression for Slice 30 FIX-1 [P1 #1]: `graph_expand` enforces `GraphContextInvalid`
/// on `include_inactive` / `include_superseded` (`graph_api.rs:532-534`), guards
/// `pushed_target_predicates` defensively in `execution.rs:1067`, and preserves exact
/// `B-tree == inline` parity on `include_out_of_window: true` with declared `Filterable`
/// `target_predicates`.
#[test]
fn wp30b_relaxed_view_declared_target_predicate_matches_inline_parity() {
    let dir = TempDir::new().unwrap();
    let instant = 1_700_000_000_i64;

    let run_store = |name: &str, declare_filterable: bool| -> Vec<String> {
        let opened = Engine::open(db_path(&dir, name)).unwrap();
        let engine = opened.engine;
        if declare_filterable {
            engine
                .configure_projections(
                    &[ProjectionSpec {
                        name: "tier".into(),
                        roles: BTreeSet::from([ProjectionRole::Filterable]),
                        fts: None,
                        vector: None,
                        source: None,
                        entity_class: PropertyEntityClass::Node,
                    }],
                    &[],
                )
                .unwrap();
        }
        let mut gold = PropertyMap::new();
        gold.insert("tier".into(), PropertyValue::String("gold".into()));
        let mut silver = PropertyMap::new();
        silver.insert("tier".into(), PropertyValue::String("silver".into()));

        engine
            .write(&[
                node("seed", "entity", "seed", None),
                PreparedWrite::Node {
                    logical_id: Some("t_oow_gold".into()),
                    kind: "entity".into(),
                    body: "out of window gold".into(),
                    source_id: SourceId::new("slice30-src").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: Some(instant + 1_000),
                    valid_until: None,
                    properties: Some(gold.clone()),
                },
                PreparedWrite::Node {
                    logical_id: Some("t_oow_silver".into()),
                    kind: "entity".into(),
                    body: "out of window silver".into(),
                    source_id: SourceId::new("slice30-src").unwrap(),
                    state: InitialState::Active,
                    reason: None,
                    valid_from: Some(instant + 1_000),
                    valid_until: None,
                    properties: Some(silver),
                },
                node("t_in_window_gold", "entity", "in window gold", Some(gold)),
                edge("e1", "rel", "seed", "t_oow_gold", None, None, None),
                edge("e2", "rel", "seed", "t_oow_silver", None, None, None),
                edge("e3", "rel", "seed", "t_in_window_gold", None, None, None),
            ])
            .unwrap();

        let pred = PropertyPredicate {
            key: "tier".into(),
            op: PropertyComparisonOp::Eq,
            value: PropertyValue::String("gold".into()),
        };

        // `include_inactive` and `include_superseded` are refused at the graph_expand boundary.
        for bad_view in [
            ReadView { include_inactive: true, ..ReadView::default() },
            ReadView { include_superseded: true, ..ReadView::default() },
        ] {
            let bad_req = GraphExpandRequestV1::new(
                GraphSeedV1::Explicit {
                    schema_version: 1,
                    logical_ids: vec![IdSpace::logical("seed")],
                },
                TraversalDirection::Outgoing,
                GraphReadContextV1::Current {
                    schema_version: 1,
                    context: ReadContextV1::new(bad_view, SearchFilter::default()).unwrap(),
                },
                1,
                10,
                100,
            )
            .with_target_predicates(vec![pred.clone()]);
            assert!(matches!(
                engine.graph_expand(&bad_req),
                Err(EngineError::GraphExpansion(ref e))
                    if e.reason == GraphExpansionErrorReasonV1::GraphContextInvalid
                        && e.field_path == "/context"
            ));
        }

        // `include_out_of_window: true` is admitted and must match declared vs undeclared parity.
        let oow_req = GraphExpandRequestV1::new(
            GraphSeedV1::Explicit {
                schema_version: 1,
                logical_ids: vec![IdSpace::logical("seed")],
            },
            TraversalDirection::Outgoing,
            GraphReadContextV1::Current {
                schema_version: 1,
                context: ReadContextV1::new(
                    ReadView {
                        include_out_of_window: true,
                        valid_as_of: Some(instant),
                        ..ReadView::default()
                    },
                    SearchFilter::default(),
                )
                .unwrap(),
            },
            1,
            10,
            100,
        )
        .with_target_predicates(vec![pred]);
        let mut oow_ids: Vec<String> = engine
            .graph_expand(&oow_req)
            .unwrap()
            .targets
            .into_iter()
            .map(|t| t.logical_id)
            .collect();
        oow_ids.sort();

        engine.close().unwrap();
        oow_ids
    };

    let declared_oow = run_store("relaxed_declared", true);
    let undeclared_oow = run_store("relaxed_undeclared", false);

    assert_eq!(declared_oow, undeclared_oow);
    assert_eq!(declared_oow, vec!["t_in_window_gold".to_string(), "t_oow_gold".to_string()]);
}
