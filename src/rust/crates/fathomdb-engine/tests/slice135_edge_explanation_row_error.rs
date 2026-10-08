//! A failed edge explanation count must not masquerade as an exact zero.
#![cfg(feature = "test-hooks")]

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    EmbedderChoice, Engine, EngineError, InitialState, PreparedWrite, ProjectionRole,
    ProjectionSpec, SearchFilter, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use std::collections::BTreeSet;
use std::sync::Arc;
use tempfile::TempDir;

fn open(path: &std::path::Path) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_choice(path, EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())))
        .expect("open")
}

fn node(id: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        source_id: SourceId::new("slice135:edge-explanation").expect("source ID"),
        logical_id: Some(id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn explained(engine: &Engine) -> Result<fathomdb_engine::SearchResult, EngineError> {
    let mut filter = SearchFilter::default();
    filter.attributes = vec![("priority".into(), "high".into())];
    engine.search_explained("slice135edgeexplanation", Some(filter), 0, false, 0.3, 0)
}

#[test]
fn edge_count_decode_error_must_not_report_zero_dropped_edge_hits() {
    let dir = TempDir::new().expect("temporary database");
    let path = dir.path().join(format!("edge-explanation{SQLITE_SUFFIX}"));
    let opened = open(&path);
    let mut roles = BTreeSet::new();
    roles.insert(ProjectionRole::Filterable);
    opened
        .engine
        .configure_projections(
            &[ProjectionSpec {
                name: "priority".into(),
                roles,
                fts: None,
                vector: None,
                source: None,
            }],
            &[],
        )
        .expect("configure priority filter");
    opened
        .engine
        .write(&[
            node("left", r#"{"title":"left"}"#),
            node("right", r#"{"title":"right"}"#),
            node("matching", r#"{"title":"slice135edgeexplanation node","priority":"high"}"#),
            PreparedWrite::Edge {
                kind: "link".into(),
                from: "left".into(),
                to: "right".into(),
                source_id: SourceId::new("slice135:edge-explanation").expect("source ID"),
                logical_id: Some("edge".into()),
                body: Some("slice135edgeexplanation edge".into()),
                t_valid: None,
                t_invalid: None,
                confidence: None,
                extractor_model_id: None,
                temporal_fallback: None,
            },
        ])
        .expect("write corpus");
    opened.engine.drain(10_000).expect("drain");
    let control = explained(&opened.engine).expect("valid explained search");
    assert_eq!(
        control.explanation.expect("explanation").trace.dropped_edge_hits,
        1,
        "the valid edge must contribute to the explanation"
    );
    unsafe { std::env::set_var("FATHOMDB_EDGE_EXPLANATION_BAD_CURSOR_FOR_TEST", "1") };
    let result = explained(&opened.engine);
    unsafe { std::env::remove_var("FATHOMDB_EDGE_EXPLANATION_BAD_CURSOR_FOR_TEST") };
    assert!(
        matches!(result, Err(EngineError::Storage)),
        "a failed edge-count decode must not return an exact zero explanation"
    );
    opened.engine.close().expect("close after probe");
}
