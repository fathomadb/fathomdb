//! 0.8.31 Slice 20 (`PROPERTY-SCHEMA`) — Layer 3 `EXPLAIN QUERY PLAN` pins
//! for Schema 38 indexes and fast paths.

use std::collections::{BTreeMap, BTreeSet};

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

    // 1. Unpredicated phase-1 vector SQL & search eligibility must emit ZERO canonical_property_values joins.
    let unpred_vec_sql = vector_phase1_sql_for_test(None);
    assert!(
        !unpred_vec_sql.contains("canonical_property_values"),
        "unpredicated vector phase-1 SQL must not join canonical_property_values: {unpred_vec_sql}"
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
            !content.contains("canonical_property_values"),
            "unpredicated search/search_text_only must emit zero joins on canonical_property_values:\n{content}"
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
        "SELECT write_cursor FROM canonical_property_values WHERE entity_class = 'node' AND attr_name = 'p_text' AND text_val = 'v'",
    );
    assert_uses_index(&text_plan, "canonical_property_values_text_idx");

    let num_plan = explain(
        &conn,
        "SELECT write_cursor FROM canonical_property_values WHERE entity_class = 'node' AND attr_name = 'p_num' AND num_val > 5",
    );
    assert_uses_index(&num_plan, "canonical_property_values_num_idx");

    let bool_plan = explain(
        &conn,
        "SELECT write_cursor FROM canonical_property_values WHERE entity_class = 'node' AND attr_name = 'p_bool' AND bool_val = 1",
    );
    assert_uses_index(&bool_plan, "canonical_property_values_bool_idx");

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
        "SELECT kind, body, write_cursor FROM canonical_nodes WHERE logical_id = 'qp-n1' AND superseded_at IS NULL ORDER BY write_cursor DESC LIMIT 1",
    );
    assert_uses_index(&node_sup_plan, "canonical_nodes_logical_sup_idx");

    // Edge active indexes (empty edge_kinds) and sup_kind indexes (non-empty edge_kinds / suppressed-aware lookup)
    let edge_from_active = explain(
        &conn,
        "SELECT to_id, kind FROM canonical_edges WHERE from_id = 'qp-n1' AND superseded_at IS NULL",
    );
    assert!(
        edge_from_active.iter().any(|l| {
            l.contains("canonical_edges_from_active_idx")
                || l.contains("canonical_edges_from_sup_kind_idx")
        }),
        "expected from active/sup_kind index, got {edge_from_active:?}"
    );
    let edge_from_active_pinned = explain(
        &conn,
        "SELECT to_id, kind FROM canonical_edges INDEXED BY canonical_edges_from_active_idx WHERE from_id = 'qp-n1' AND superseded_at IS NULL",
    );
    assert_uses_index(&edge_from_active_pinned, "canonical_edges_from_active_idx");

    let edge_to_active = explain(
        &conn,
        "SELECT from_id, kind FROM canonical_edges WHERE to_id = 'qp-n2' AND superseded_at IS NULL",
    );
    assert!(
        edge_to_active.iter().any(|l| {
            l.contains("canonical_edges_to_active_idx")
                || l.contains("canonical_edges_to_sup_kind_idx")
        }),
        "expected to active/sup_kind index, got {edge_to_active:?}"
    );
    let edge_to_active_pinned = explain(
        &conn,
        "SELECT from_id, kind FROM canonical_edges INDEXED BY canonical_edges_to_active_idx WHERE to_id = 'qp-n2' AND superseded_at IS NULL",
    );
    assert_uses_index(&edge_to_active_pinned, "canonical_edges_to_active_idx");

    let edge_from_kind = explain(
        &conn,
        "SELECT to_id FROM canonical_edges WHERE from_id = 'qp-n1' AND superseded_at IS NULL AND kind = 'supports'",
    );
    assert!(
        edge_from_kind.iter().any(|l| {
            l.contains("canonical_edges_from_sup_kind_idx")
                || l.contains("canonical_edges_from_active_idx")
        }),
        "expected from sup_kind/active index, got {edge_from_kind:?}"
    );
    // Explicitly check sup_kind planner usability when querying specific superseded_at + kind.
    let edge_from_sup_kind = explain(
        &conn,
        "SELECT to_id FROM canonical_edges INDEXED BY canonical_edges_from_sup_kind_idx WHERE from_id = 'qp-n1' AND superseded_at IS NULL AND kind = 'supports'",
    );
    assert_uses_index(&edge_from_sup_kind, "canonical_edges_from_sup_kind_idx");

    let edge_to_sup_kind = explain(
        &conn,
        "SELECT from_id FROM canonical_edges INDEXED BY canonical_edges_to_sup_kind_idx WHERE to_id = 'qp-n2' AND superseded_at IS NULL AND kind = 'supports'",
    );
    assert_uses_index(&edge_to_sup_kind, "canonical_edges_to_sup_kind_idx");
}
