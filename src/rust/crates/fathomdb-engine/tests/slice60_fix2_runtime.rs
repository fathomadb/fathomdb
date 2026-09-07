//! Slice 60 FIX-2 RED: production-plan identity and real high-bound fixtures.

use fathomdb_engine::{
    Engine, EngineError, GraphExpandRequestV1, GraphExpansionErrorReasonV1, GraphReadContextV1,
    GraphSeedV1, IdSpace, InitialState, PreparedWrite, ReadContextV1, ReadView, SearchFilter,
    SourceId, TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use rusqlite::Connection;
use tempfile::TempDir;

fn request(work: u64) -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Both,
        edge_kinds: Vec::new(),
        target_kinds: Vec::new(),
        context: GraphReadContextV1::Current {
            schema_version: 1,
            context: ReadContextV1::new(
                ReadView { valid_as_of: Some(1_700_000_000), ..ReadView::default() },
                SearchFilter::default(),
            )
            .unwrap(),
        },
        max_depth: 1,
        result_limit: 1,
        max_work_units: work,
        include_explanation: false,
    }
}

fn node(id: impl Into<String>) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.into()),
        kind: "fact".into(),
        body: "fixture".into(),
        source_id: SourceId::new("test:slice60-fix2").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn edge(id: String, to: String) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".into(),
        from: "root".into(),
        to,
        source_id: SourceId::new("test:slice60-fix2").unwrap(),
        logical_id: Some(id),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn open_populated(directory: &TempDir, name: &str, rows: usize) -> (std::path::PathBuf, Engine) {
    let path = directory.path().join(format!("{name}{SQLITE_SUFFIX}"));
    let opened = Engine::open(&path).unwrap();
    let mut writes = Vec::with_capacity(1 + rows * 2);
    writes.push(node("root"));
    for index in 0..rows {
        let target = format!("target-{index:05}");
        writes.push(node(target.clone()));
        writes.push(edge(format!("edge-{index:05}"), target));
    }
    opened.engine.write(&writes).unwrap();
    (path, opened.engine)
}

#[test]
fn both_explain_is_the_exact_production_or_statement_and_binds() {
    let directory = TempDir::new().unwrap();
    let (path, engine) = open_populated(&directory, "both-plan", 1);
    let observed = engine.explain_graph_expand_for_test(TraversalDirection::Both).unwrap();
    let connection = Connection::open(path).unwrap();
    let sql = "EXPLAIN QUERY PLAN SELECT write_cursor,kind,from_id,to_id,logical_id,superseded_at,t_invalid FROM canonical_edges WHERE (from_id=?1 OR to_id=?1) LIMIT ?2";
    let mut statement = connection.prepare(sql).unwrap();
    let expected = statement
        .query_map(rusqlite::params!["root", 10_i64], |row| row.get::<_, String>(3))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(observed, expected);
}

#[test]
fn real_sqlite_exactly_w_and_w_plus_one_rows_have_typed_all_or_nothing_outcomes() {
    let directory = TempDir::new().unwrap();
    let (_, success_engine) = open_populated(&directory, "w-success", 10_000);
    let success = success_engine.graph_expand(&request(10_000)).unwrap();
    assert!(success.complete);
    assert_eq!(success.work_units, 10_000);

    let (_, failure_engine) = open_populated(&directory, "w-failure", 10_001);
    let error = failure_engine.graph_expand(&request(10_000)).unwrap_err();
    assert!(matches!(
        error,
        EngineError::GraphExpansion(ref graph)
            if graph.reason == GraphExpansionErrorReasonV1::GraphExpansionBoundExceeded
                && graph.field_path == "/maxWorkUnits"
    ));
}

#[test]
#[cfg(target_os = "linux")]
fn measurement_is_observed_from_the_real_high_bound_execution_not_a_placeholder() {
    let directory = TempDir::new().unwrap();
    let (_, engine) = open_populated(&directory, "rss", 10_000);
    let _ = engine.graph_expand(&request(10_000)).unwrap();
    let measurement = engine.measure_graph_expand_for_test();
    assert!(measurement.peak_rss_delta_bytes > 0);
    assert!(measurement.peak_rss_delta_bytes < 128 * 1024 * 1024);
}
