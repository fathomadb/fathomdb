use std::collections::BTreeSet;

use fathomdb_engine::{
    Engine, EngineError, InitialState, PreparedWrite, ProjectionFts, ProjectionRole,
    ProjectionSpec, ReadView, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn node(
    logical_id: &str,
    body: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body: body.to_string(),
        source_id: SourceId::new("slice80-view-paths").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
    }
}

fn edge(from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: "link".to_string(),
        from: from.to_string(),
        to: to.to_string(),
        source_id: SourceId::new("slice80-view-paths").expect("source id"),
        logical_id: Some(format!("{from}-{to}")),
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

fn at(instant: i64) -> ReadView {
    ReadView { valid_as_of: Some(instant), ..ReadView::default() }
}

fn relaxed() -> ReadView {
    ReadView { include_out_of_window: true, ..ReadView::default() }
}

fn assert_existence_relaxation_refused(
    mut call: impl FnMut(&ReadView) -> Result<fathomdb_engine::SearchResult, EngineError>,
) {
    for view in [
        ReadView { include_superseded: true, ..ReadView::default() },
        ReadView { include_inactive: true, ..ReadView::default() },
    ] {
        assert!(
            matches!(call(&view), Err(EngineError::InvalidArgument { .. })),
            "search must refuse existence-axis relaxation"
        );
    }
}

#[test]
fn graph_arm_applies_the_search_validity_view_to_reached_neighbors() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("graph-view{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).expect("open");
    opened
        .engine
        .write(&[
            node("seed", "unique anchor phrase", None, None),
            node("neighbor", "retired graph neighbor", Some(1_000), Some(2_000)),
            edge("seed", "neighbor"),
        ])
        .expect("seed graph");

    let search = |view: &ReadView| {
        opened.engine.search_reranked_view("unique anchor", None, 0, true, 0.3, 0, false, view)
    };
    let contains_neighbor = |view: &ReadView| {
        search(view)
            .expect("graph search")
            .results
            .iter()
            .any(|hit| hit.body == "retired graph neighbor")
    };

    assert!(
        !contains_neighbor(&ReadView::default()),
        "default view must hide the expired neighbor"
    );
    assert!(contains_neighbor(&relaxed()), "relaxed validity must prove the graph arm reached it");
    assert!(contains_neighbor(&at(1_500)), "an instant inside the neighbor window must include it");
    assert_existence_relaxation_refused(search);

    opened.engine.close().expect("close");
}

#[test]
fn projected_text_applies_the_search_validity_view() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join(format!("projected-view{SQLITE_SUFFIX}"));
    let opened = Engine::open(path).expect("open");
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "summary".to_string(),
                roles: BTreeSet::from([ProjectionRole::Searchable]),
                fts: Some(ProjectionFts { tokenizer: None }),
                vector: None,
                source: None,
            }],
            &[],
        )
        .expect("declare projected text");
    opened
        .engine
        .write(&[
            node("current", r#"{"summary":"shared projected current"}"#, None, None),
            node("expired", r#"{"summary":"shared projected expired"}"#, Some(1_000), Some(2_000)),
        ])
        .expect("seed projected rows");

    let search =
        |view: &ReadView| opened.engine.search_projected_text("shared", "summary", None, view);
    let bodies = |view: &ReadView| {
        search(view)
            .expect("projected search")
            .results
            .iter()
            .map(|hit| hit.body.clone())
            .collect::<Vec<_>>()
    };

    let default = bodies(&ReadView::default());
    assert!(default.iter().any(|body| body.contains("projected current")));
    assert!(
        default.iter().all(|body| !body.contains("projected expired")),
        "default view must hide the expired projected hit"
    );
    assert!(bodies(&relaxed()).iter().any(|body| body.contains("projected expired")));
    assert!(bodies(&at(1_500)).iter().any(|body| body.contains("projected expired")));
    assert_existence_relaxation_refused(search);

    opened.engine.close().expect("close");
}
