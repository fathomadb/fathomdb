//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 3 `EXPLAIN QUERY PLAN` pins
//! for Schema 38 indexes and fast paths.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    vector_phase1_sql_for_test, Engine, InitialState, PreparedWrite, ProjectionRole,
    ProjectionSpec, PropertyComparisonOp, PropertyEntityClass, PropertyPredicate, PropertyValue,
    SearchFilter, SourceId, TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

fn explain(conn: &Connection, sql: &str) -> Vec<String> {
    let mut stmt = conn.prepare(&format!("EXPLAIN QUERY PLAN {sql}")).expect("prepare explain");
    stmt.query_map([], |r| r.get::<_, String>(3))
        .expect("query explain")
        .collect::<rusqlite::Result<Vec<_>>>()
        .expect("collect explain")
}

fn assert_uses_index(plan: &[String], index: &str) {
    assert!(
        plan.iter().any(|line| line.contains(index)),
        "expected EXPLAIN QUERY PLAN to use `{index}`, got:\n{}",
        plan.join("\n")
    );
}

#[test]
fn slice38_query_plans_use_declared_indexes_and_skip_cpv_on_unpredicated_search() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("qp{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    engine
        .configure_projections(
            &[
                ProjectionSpec {
                    name: "p_text".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "p_num".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
                ProjectionSpec {
                    name: "p_bool".into(),
                    roles: BTreeSet::from([ProjectionRole::Filterable]),
                    fts: None,
                    vector: None,
                    source: None,
                    entity_class: PropertyEntityClass::Node,
                },
            ],
            &[],
        )
        .expect("configure projections");

    engine
        .write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "planner fixture alpha".into(),
                source_id: SourceId::new("src:qp").unwrap(),
                logical_id: Some("qp-n1".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: Some(BTreeMap::from([
                    ("p_text".into(), PropertyValue::String("v".into())),
                    ("p_num".into(), PropertyValue::Int(10)),
                    ("p_bool".into(), PropertyValue::Bool(true)),
                ])),
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "planner fixture beta".into(),
                source_id: SourceId::new("src:qp").unwrap(),
                logical_id: Some("qp-n2".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "qp-n1".into(),
                to: "qp-n2".into(),
                source_id: SourceId::new("src:qp").unwrap(),
                logical_id: Some("qp-e1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .expect("write");

    // 1. Unpredicated phase-1 vector SQL & search eligibility must emit ZERO canonical_property_values or canonical_edges joins.
    let unpred_vec_sql = vector_phase1_sql_for_test(None);
    assert!(
        !unpred_vec_sql.contains("canonical_property_values")
            && !unpred_vec_sql.contains("canonical_edges"),
        "unpredicated vector phase-1 SQL must not join canonical_property_values or canonical_edges: {unpred_vec_sql}"
    );
    let mut pred_filter = SearchFilter::default();
    pred_filter.property_predicates = vec![PropertyPredicate {
        key: "p_num".into(),
        op: PropertyComparisonOp::Gt,
        value: PropertyValue::Int(5),
    }];
    let pred_vec_sql = vector_phase1_sql_for_test(Some(&pred_filter));
    assert!(
        pred_vec_sql.contains("canonical_property_values"),
        "numeric range vector phase-1 SQL should pre-select via canonical_property_values: {pred_vec_sql}"
    );

    // Confirm unpredicated search_text_only query plan trace contains zero canonical_property_values joins.
    let plan_witness_path = dir.path().join("fts-plan-witness.jsonl");
    unsafe {
        std::env::set_var("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST", &plan_witness_path);
    }
    let _ = engine.search_text_only("planner").expect("search_text_only");
    let _ = engine.search("planner").expect("search");
    unsafe {
        std::env::remove_var("FATHOMDB_FTS_QUERY_PLAN_WITNESS_FOR_TEST");
    }
    if plan_witness_path.exists() {
        let content = std::fs::read_to_string(&plan_witness_path).expect("read fts witness");
        assert!(
            !content.contains("canonical_property_values") && !content.contains("canonical_edges"),
            "unpredicated search/search_text_only must emit zero joins on canonical_property_values or canonical_edges:\n{content}"
        );
    }

    // 2. Endpoint graph_expand plans via Engine seam remain indexed.
    for direction in
        [TraversalDirection::Outgoing, TraversalDirection::Incoming, TraversalDirection::Both]
    {
        let plan = engine.explain_graph_expand_for_test(direction).unwrap();
        assert!(
            !plan.iter().any(|l| l.contains("SCAN canonical_edges") && !l.contains("USING INDEX")),
            "full scan in explain_graph_expand_for_test({direction:?}): {plan:?}"
        );
    }

    engine.close().expect("close");

    // 3. Direct SQLite EXPLAIN QUERY PLAN pins on Schema 38 indexes.
    let conn = Connection::open(&path).expect("open sqlite");

    let text_plan = explain(
        &conn,
        "SELECT write_cursor FROM canonical_property_values WHERE entity_class = 'node' AND attr_name = 'p_text' AND text_val IS NOT NULL AND text_val = 'v'",
    );
    assert_uses_index(&text_plan, "canonical_property_values_text_idx");

    let num_plan = explain(
        &conn,
        "SELECT write_cursor FROM canonical_property_values WHERE entity_class = 'node' AND attr_name = 'p_num' AND num_val IS NOT NULL AND num_val > 5",
    );
    assert_uses_index(&num_plan, "canonical_property_values_num_idx");

    let bool_plan = explain(
        &conn,
        "SELECT write_cursor FROM canonical_property_values WHERE entity_class = 'node' AND attr_name = 'p_bool' AND bool_val IS NOT NULL AND bool_val = 1",
    );
    assert_uses_index(&bool_plan, "canonical_property_values_bool_idx");

    let cpv_delete_one_plan = explain(
        &conn,
        "DELETE FROM canonical_property_values WHERE entity_class IN ('node', 'edge') AND write_cursor = 1",
    );
    assert!(
        cpv_delete_one_plan
            .iter()
            .any(|l| l.contains("SEARCH canonical_property_values USING PRIMARY KEY")),
        "single-cursor CPV delete must use PRIMARY KEY search; got:\n{}",
        cpv_delete_one_plan.join("\n")
    );
    assert!(
        !cpv_delete_one_plan.iter().any(|l| l.contains("SCAN canonical_property_values")),
        "single-cursor CPV delete must never SCAN; got:\n{}",
        cpv_delete_one_plan.join("\n")
    );

    let cpv_delete_many_plan = explain(
        &conn,
        "DELETE FROM canonical_property_values WHERE entity_class IN ('node', 'edge') AND write_cursor IN (SELECT value FROM json_each('[1,2]'))",
    );
    assert!(
        cpv_delete_many_plan
            .iter()
            .any(|l| l.contains("SEARCH canonical_property_values USING PRIMARY KEY")),
        "multi-cursor CPV delete must use PRIMARY KEY search; got:\n{}",
        cpv_delete_many_plan.join("\n")
    );
    assert!(
        !cpv_delete_many_plan.iter().any(|l| l.contains("SCAN canonical_property_values")),
        "multi-cursor CPV delete must never SCAN; got:\n{}",
        cpv_delete_many_plan.join("\n")
    );

    let cursor_plan =
        explain(&conn, "SELECT kind, body, properties FROM canonical_nodes WHERE write_cursor = 1");
    assert_uses_index(&cursor_plan, "canonical_nodes_write_cursor_idx");

    let ts_plan = explain(
        &conn,
        "SELECT write_cursor FROM _fathomdb_commit_timestamps WHERE committed_at_ms >= 100 AND committed_at_ms <= 1800000000000 ORDER BY committed_at_ms, write_cursor",
    );
    assert_uses_index(&ts_plan, "_fathomdb_commit_timestamps_ms_idx");

    let node_sup_plan = explain(
        &conn,
        "SELECT kind, body, write_cursor FROM canonical_nodes WHERE scope = '' AND logical_id = 'qp-n1' AND superseded_at IS NOT NULL AND superseded_at > 10 ORDER BY write_cursor DESC LIMIT 1",
    );
    assert_uses_index(&node_sup_plan, "canonical_nodes_scope_logical_sup_idx");

    // Slice 30 (`AC31-30-02`) — write-time strict endpoint probe must use indexed SEARCH
    // on canonical_nodes_scope_logical_active_idx or canonical_nodes_scope_logical_sup_idx with zero SCAN.
    let endpoint_probe_plan = explain(
        &conn,
        "SELECT 1 FROM canonical_nodes WHERE scope = '' AND logical_id = 'qp-n1' AND superseded_at IS NULL AND state = 'active' LIMIT 1",
    );
    assert!(
        endpoint_probe_plan.iter().any(|l| {
            l.contains("SEARCH canonical_nodes USING INDEX canonical_nodes_scope_logical_active_idx")
                || l.contains("SEARCH canonical_nodes USING INDEX canonical_nodes_scope_logical_sup_idx")
        }),
        "write-time endpoint probe must use indexed SEARCH on canonical_nodes_scope_logical_{{active,sup}}_idx; got:\n{}",
        endpoint_probe_plan.join("\n")
    );
    assert!(
        !endpoint_probe_plan.iter().any(|l| l.contains("SCAN canonical_nodes")),
        "write-time endpoint probe must never SCAN canonical_nodes; got:\n{}",
        endpoint_probe_plan.join("\n")
    );

    // Edge active indexes (empty edge_kinds) and sup_kind indexes (non-empty edge_kinds / suppressed-aware lookup)
    let edge_from_active = explain(
        &conn,
        "SELECT to_id, kind FROM canonical_edges WHERE scope = '' AND from_id = 'qp-n1' AND superseded_at IS NULL",
    );
    assert!(
        edge_from_active.iter().any(|l| {
            l.contains("canonical_edges_scope_from_active_idx")
                || l.contains("canonical_edges_scope_from_sup_kind_idx")
        }),
        "expected from active/sup_kind index, got {edge_from_active:?}"
    );
    let edge_from_active_pinned = explain(
        &conn,
        "SELECT to_id, kind FROM canonical_edges INDEXED BY canonical_edges_scope_from_active_idx WHERE scope = '' AND from_id = 'qp-n1' AND superseded_at IS NULL",
    );
    assert_uses_index(&edge_from_active_pinned, "canonical_edges_scope_from_active_idx");

    let edge_to_active = explain(
        &conn,
        "SELECT from_id, kind FROM canonical_edges WHERE scope = '' AND to_id = 'qp-n2' AND superseded_at IS NULL",
    );
    assert!(
        edge_to_active.iter().any(|l| {
            l.contains("canonical_edges_scope_to_active_idx")
                || l.contains("canonical_edges_scope_to_sup_kind_idx")
        }),
        "expected to active/sup_kind index, got {edge_to_active:?}"
    );
    let edge_to_active_pinned = explain(
        &conn,
        "SELECT from_id, kind FROM canonical_edges INDEXED BY canonical_edges_scope_to_active_idx WHERE scope = '' AND to_id = 'qp-n2' AND superseded_at IS NULL",
    );
    assert_uses_index(&edge_to_active_pinned, "canonical_edges_scope_to_active_idx");

    let edge_from_kind = explain(
        &conn,
        "SELECT to_id FROM canonical_edges WHERE scope = '' AND from_id = 'qp-n1' AND superseded_at IS NULL AND kind = 'supports'",
    );
    assert!(
        edge_from_kind.iter().any(|l| {
            l.contains("canonical_edges_scope_from_sup_kind_idx")
                || l.contains("canonical_edges_scope_from_active_idx")
        }),
        "expected from sup_kind/active index, got {edge_from_kind:?}"
    );
    // Explicitly check sup_kind planner usability when querying specific superseded_at + kind.
    let edge_from_sup_kind = explain(
        &conn,
        "SELECT to_id FROM canonical_edges INDEXED BY canonical_edges_scope_from_sup_kind_idx WHERE scope = '' AND from_id = 'qp-n1' AND superseded_at IS NOT NULL AND superseded_at > 10 AND kind = 'supports'",
    );
    assert_uses_index(&edge_from_sup_kind, "canonical_edges_scope_from_sup_kind_idx");

    let edge_to_sup_kind = explain(
        &conn,
        "SELECT from_id FROM canonical_edges INDEXED BY canonical_edges_scope_to_sup_kind_idx WHERE scope = '' AND to_id = 'qp-n2' AND superseded_at IS NOT NULL AND superseded_at > 10 AND kind = 'supports'",
    );
    assert_uses_index(&edge_to_sup_kind, "canonical_edges_scope_to_sup_kind_idx");
}

#[derive(Clone, Debug)]
struct QpEmbedder;

impl Embedder for QpEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("slice38-qp-embedder", "v1", 8)
    }

    fn embed(&self, text: &str) -> Result<Vector, EmbedderError> {
        let mut vec = vec![0.0_f32; 8];
        for (i, b) in text.bytes().enumerate() {
            vec[i % 8] += f32::from(b) / 255.0;
        }
        if vec.iter().all(|v| *v == 0.0) {
            vec[0] = 1.0;
        }
        Ok(vec)
    }
}

#[test]
fn slice38_vector_preselection_uses_cpv_indexes_and_never_scans_canonical_nodes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("qp-vec{SQLITE_SUFFIX}"));
    let opened =
        Engine::open_with_embedder_for_test(&path, Arc::new(QpEmbedder)).expect("open embedder");
    let engine = opened.engine;

    // Declare 8 filler slots + 3 overflow slots (`z_text`, `z_num`, `z_bool`) so all three
    // predicate types route through pre-selection (`canonical_property_values`) even on `Eq`.
    let mut specs: Vec<ProjectionSpec> = (0..8)
        .map(|i| ProjectionSpec {
            name: format!("a_slot_{i}"),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        })
        .collect();
    for name in ["z_text", "z_num", "z_bool"] {
        specs.push(ProjectionSpec {
            name: name.into(),
            roles: BTreeSet::from([ProjectionRole::Filterable]),
            fts: None,
            vector: None,
            source: None,
            entity_class: PropertyEntityClass::Node,
        });
    }
    engine.configure_projections(&specs, &[]).expect("configure projections");

    engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "vector preselection plan fixture".into(),
            source_id: SourceId::new("src:qp-vec").unwrap(),
            logical_id: Some("qp-vec-1".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: Some(BTreeMap::from([
                ("z_text".into(), PropertyValue::String("alpha".into())),
                ("z_num".into(), PropertyValue::Int(42)),
                ("z_bool".into(), PropertyValue::Bool(true)),
            ])),
        }])
        .expect("write");

    let cases: &[(&str, PropertyPredicate, &str)] = &[
        (
            "string",
            PropertyPredicate {
                key: "z_text".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::String("alpha".into()),
            },
            "canonical_property_values_text_idx",
        ),
        (
            "numeric",
            PropertyPredicate {
                key: "z_num".into(),
                op: PropertyComparisonOp::Gt,
                value: PropertyValue::Int(10),
            },
            "canonical_property_values_num_idx",
        ),
        (
            "boolean",
            PropertyPredicate {
                key: "z_bool".into(),
                op: PropertyComparisonOp::Eq,
                value: PropertyValue::Bool(true),
            },
            "canonical_property_values_bool_idx",
        ),
    ];

    for (label, pred, expected_idx) in cases {
        let mut filter = SearchFilter::default();
        filter.property_predicates = vec![pred.clone()];
        let plan = engine
            .explain_vector_phase1_for_test(Some(&filter))
            .unwrap_or_else(|e| panic!("explain vector phase1 ({label}) failed: {e:?}"));
        assert_uses_index(&plan, expected_idx);
        assert!(
            !plan.iter().any(|line| line.contains("SCAN canonical_nodes")),
            "vector preselection ({label}) must NOT emit SCAN canonical_nodes:\n{}",
            plan.join("\n")
        );
    }

    engine.close().expect("close");
}

#[test]
fn slice38_graph_expand_incident_query_plans_use_active_or_sup_kind_indexes_zero_scan() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("graph-qp{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    engine
        .write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "a".into(),
                source_id: SourceId::new("src:gqp").unwrap(),
                logical_id: Some("gqp-a".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "b".into(),
                source_id: SourceId::new("src:gqp").unwrap(),
                logical_id: Some("gqp-b".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "gqp-a".into(),
                to: "gqp-b".into(),
                source_id: SourceId::new("src:gqp").unwrap(),
                logical_id: Some("gqp-e1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .expect("write");

    let kind_sets: &[&[&str]] = &[&[], &["supports"], &["supports", "contradicts"]];
    for direction in
        [TraversalDirection::Outgoing, TraversalDirection::Incoming, TraversalDirection::Both]
    {
        for kinds in kind_sets {
            let kinds_vec: Vec<String> = kinds.iter().map(|s| (*s).to_string()).collect();
            let plan = engine
                .explain_graph_expand_with_kinds_for_test(direction, &kinds_vec)
                .unwrap_or_else(|e| panic!("explain ({direction:?}, {kinds:?}) failed: {e:?}"));
            let has_from = plan.iter().any(|l| {
                l.contains("SEARCH canonical_edges USING INDEX canonical_edges_from_active_idx")
                    || l.contains(
                        "SEARCH canonical_edges USING INDEX canonical_edges_from_sup_kind_idx",
                    )
            });
            let has_to = plan.iter().any(|l| {
                l.contains("SEARCH canonical_edges USING INDEX canonical_edges_to_active_idx")
                    || l.contains(
                        "SEARCH canonical_edges USING INDEX canonical_edges_to_sup_kind_idx",
                    )
            });
            match direction {
                TraversalDirection::Outgoing => {
                    assert!(has_from && !has_to, "Outgoing {kinds:?} plan:\n{}", plan.join("\n"));
                }
                TraversalDirection::Incoming => {
                    assert!(has_to && !has_from, "Incoming {kinds:?} plan:\n{}", plan.join("\n"));
                }
                TraversalDirection::Both => {
                    assert!(has_from && has_to, "Both {kinds:?} plan:\n{}", plan.join("\n"));
                }
            }
            assert!(
                !plan.iter().any(|l| l.contains("SCAN canonical_edges")),
                "{direction:?} {kinds:?} must never SCAN canonical_edges:\n{}",
                plan.join("\n")
            );
        }
    }

    engine.close().expect("close");
}

#[test]
fn slice38_graph_arm_phase2_union_all_uses_active_or_sup_kind_indexes_zero_scan() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("graph-arm-qp{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    engine
        .write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "a".into(),
                source_id: SourceId::new("src:gaqp").unwrap(),
                logical_id: Some("gaqp-a".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "b".into(),
                source_id: SourceId::new("src:gaqp").unwrap(),
                logical_id: Some("gaqp-b".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "gaqp-a".into(),
                to: "gaqp-b".into(),
                source_id: SourceId::new("src:gaqp").unwrap(),
                logical_id: Some("gaqp-e1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .expect("write");

    let kind_sets: &[&[&str]] = &[&[], &["supports"], &["supports", "contradicts"]];
    for kinds in kind_sets {
        let kinds_vec: Vec<String> = kinds.iter().map(|s| (*s).to_string()).collect();
        let plan = engine
            .explain_graph_arm_phase2_for_test(&kinds_vec)
            .unwrap_or_else(|e| panic!("explain graph arm phase2 ({kinds:?}) failed: {e:?}"));
        let has_from = plan.iter().any(|l| {
            l.contains("SEARCH e USING INDEX canonical_edges_from_active_idx")
                || l.contains("SEARCH e USING INDEX canonical_edges_from_sup_kind_idx")
                || l.contains("SEARCH canonical_edges USING INDEX canonical_edges_from_active_idx")
                || l.contains(
                    "SEARCH canonical_edges USING INDEX canonical_edges_from_sup_kind_idx",
                )
        });
        let has_to = plan.iter().any(|l| {
            l.contains("SEARCH e USING INDEX canonical_edges_to_active_idx")
                || l.contains("SEARCH e USING INDEX canonical_edges_to_sup_kind_idx")
                || l.contains("SEARCH canonical_edges USING INDEX canonical_edges_to_active_idx")
                || l.contains("SEARCH canonical_edges USING INDEX canonical_edges_to_sup_kind_idx")
        });
        assert!(
            has_from && has_to,
            "use_graph_arm Phase 2 UNION ALL ({kinds:?}) must use from+to active/sup_kind indexes; got:\n{}",
            plan.join("\n")
        );
        assert!(
            !plan.iter().any(|l| l.contains("SCAN e") || l.contains("SCAN canonical_edges")),
            "use_graph_arm Phase 2 UNION ALL ({kinds:?}) must never SCAN canonical_edges:\n{}",
            plan.join("\n")
        );
    }

    engine.close().expect("close");
}

#[test]
fn slice40_historical_graph_expand_production_plans_use_dual_indexes_and_reverse_timestamp_seek() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("hist-qp{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).expect("open");
    let engine = opened.engine;

    engine
        .write(&[
            PreparedWrite::Node {
                kind: "note".into(),
                body: "a v1".into(),
                source_id: SourceId::new("src:hqp").unwrap(),
                logical_id: Some("hqp-a".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Node {
                kind: "note".into(),
                body: "b v1".into(),
                source_id: SourceId::new("src:hqp").unwrap(),
                logical_id: Some("hqp-b".into()),
                state: InitialState::Active,
                reason: None,
                valid_from: None,
                valid_until: None,
                properties: None,
            },
            PreparedWrite::Edge {
                kind: "supports".into(),
                from: "hqp-a".into(),
                to: "hqp-b".into(),
                source_id: SourceId::new("src:hqp").unwrap(),
                logical_id: Some("hqp-e1".into()),
                body: None,
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
                properties: None,
            },
        ])
        .expect("write v1");
    engine
        .write(&[PreparedWrite::Node {
            kind: "note".into(),
            body: "a v2".into(),
            source_id: SourceId::new("src:hqp").unwrap(),
            logical_id: Some("hqp-a".into()),
            state: InitialState::Active,
            reason: None,
            valid_from: None,
            valid_until: None,
            properties: None,
        }])
        .expect("write v2");

    // 1. EdgeViewBoundary::AsOf production incident-edge plans across directions & kind sets.
    let kind_sets: &[&[&str]] = &[&[], &["supports"], &["supports", "contradicts"]];
    for check_derived in [false, true] {
        for direction in
            [TraversalDirection::Outgoing, TraversalDirection::Incoming, TraversalDirection::Both]
        {
            for kinds in kind_sets {
                let kinds_vec: Vec<String> = kinds.iter().map(|s| (*s).to_string()).collect();
                let plan = engine
                    .explain_graph_expand_as_of_with_kinds_for_test(
                        direction,
                        &kinds_vec,
                        3,
                        check_derived,
                    )
                    .unwrap_or_else(|e| {
                        panic!("as_of explain ({direction:?}, {kinds:?}, derived={check_derived}) failed: {e:?}")
                    });
                let has_from_active = plan.iter().any(|l| {
                    l.contains("SEARCH canonical_edges USING INDEX canonical_edges_from_active_idx")
                        || l.contains(
                            "SEARCH canonical_edges USING INDEX canonical_edges_from_sup_kind_idx",
                        )
                });
                let has_from_sup = plan.iter().any(|l| {
                    l.contains(
                        "SEARCH canonical_edges USING INDEX canonical_edges_from_sup_kind_idx",
                    )
                });
                let has_to_active = plan.iter().any(|l| {
                    l.contains("SEARCH canonical_edges USING INDEX canonical_edges_to_active_idx")
                        || l.contains(
                            "SEARCH canonical_edges USING INDEX canonical_edges_to_sup_kind_idx",
                        )
                });
                let has_to_sup = plan.iter().any(|l| {
                    l.contains("SEARCH canonical_edges USING INDEX canonical_edges_to_sup_kind_idx")
                });
                match direction {
                    TraversalDirection::Outgoing => {
                        assert!(
                            has_from_active && has_from_sup && !has_to_active && !has_to_sup,
                            "AsOf Outgoing {kinds:?} derived={check_derived} plan:\n{}",
                            plan.join("\n")
                        );
                    }
                    TraversalDirection::Incoming => {
                        assert!(
                            has_to_active && has_to_sup && !has_from_active && !has_from_sup,
                            "AsOf Incoming {kinds:?} derived={check_derived} plan:\n{}",
                            plan.join("\n")
                        );
                    }
                    TraversalDirection::Both => {
                        assert!(
                            has_from_active && has_from_sup && has_to_active && has_to_sup,
                            "AsOf Both {kinds:?} derived={check_derived} plan:\n{}",
                            plan.join("\n")
                        );
                    }
                }
                assert!(
                    !plan.iter().any(|l| l.contains("SCAN canonical_edges")),
                    "AsOf {direction:?} {kinds:?} derived={check_derived} must never SCAN canonical_edges:\n{}",
                    plan.join("\n")
                );
            }
        }
    }

    // 2. Historical two-arm load_node production plan uses logical_active_idx + logical_sup_idx.
    let load_node_plan = engine
        .explain_graph_expand_load_node_as_of_for_test("hqp-a", 3)
        .expect("explain load_node");
    assert!(
        load_node_plan.iter().any(|l| {
            l.contains("SEARCH n USING INDEX canonical_nodes_logical_active_idx")
                || l.contains("SEARCH n USING INDEX canonical_nodes_logical_sup_idx")
        }),
        "historical load_node arm 1 must use active/sup logical index:\n{}",
        load_node_plan.join("\n")
    );
    assert!(
        load_node_plan
            .iter()
            .any(|l| l.contains("SEARCH n USING INDEX canonical_nodes_logical_sup_idx")),
        "historical load_node arm 2 must use canonical_nodes_logical_sup_idx:\n{}",
        load_node_plan.join("\n")
    );
    assert!(
        !load_node_plan.iter().any(|l| l.contains("SCAN n") || l.contains("SCAN canonical_nodes")),
        "historical load_node must never SCAN canonical_nodes:\n{}",
        load_node_plan.join("\n")
    );

    // 3. _fathomdb_commit_timestamps O(log N) reverse-seek production plan.
    let ts_plan = engine
        .explain_commit_timestamp_resolve_for_test(1_800_000_000_000)
        .expect("explain commit timestamp resolve");
    assert_uses_index(&ts_plan, "_fathomdb_commit_timestamps_ms_idx");
    assert!(
        !ts_plan.iter().any(|l| l.contains("SCAN") || l.contains("USE TEMP B-TREE")),
        "commit timestamp resolution must be a single reverse index seek with zero SCAN/TEMP B-TREE:\n{}",
        ts_plan.join("\n")
    );

    engine.close().expect("close");
}
