use fathomdb_engine::{
    Engine, InitialState, PreparedWrite, ReadContextV1, ReadView, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(id: &str, kind: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        logical_id: Some(id.to_string()),
        kind: kind.to_string(),
        body: body.to_string(),
        source_id: SourceId::new("test:slice35-frontier").unwrap(),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn edge(id: &str, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new("test:slice35-frontier").unwrap(),
        logical_id: Some(id.to_string()),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

#[test]
fn graph_frontier_applies_target_eligibility_before_its_edge_cap() {
    let dir = TempDir::new().unwrap();
    let opened = Engine::open(dir.path().join(format!("frontier-cap{SQLITE_SUFFIX}"))).unwrap();
    let mut writes = vec![node("root", "target", "zephyranchor")];
    for index in 0..64 {
        let id = format!("excluded-{index:02}");
        writes.push(node(&id, "note", "excluded neighbor"));
        writes.push(edge(&format!("edge-{index:02}"), "root", &id));
    }
    writes.push(node("eligible-65", "target", "eligible frontier neighbor"));
    writes.push(edge("edge-65", "root", "eligible-65"));
    opened.engine.write(&writes).unwrap();

    let mut eligibility = SearchFilter::default();
    eligibility.kind = Some("target".to_string());
    let frozen = opened
        .engine
        .freeze_read_context(&ReadContextV1::new(ReadView::default(), eligibility).unwrap())
        .unwrap();
    let result =
        opened.engine.search_frozen("zephyranchor", &frozen, 0, true, 0.3, 0, false, 10).unwrap();

    assert!(
        result.results.iter().any(|hit| hit.id.value == "eligible-65"),
        "the eligible 65th edge must not be hidden by 64 ineligible targets"
    );
}
