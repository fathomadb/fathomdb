//! TC-5's benchmark seam is a direct reader-worker request, not `Engine::search`.

use std::sync::Arc;

use fathomdb_embedder_api::{Embedder, EmbedderError, EmbedderIdentity, Vector};
use fathomdb_engine::{
    tc5_benchmark::{RouteObserver, VectorStageError, VectorStageRequest, VectorStageScope},
    Engine, PreparedWrite, SourceId,
};
use tempfile::tempdir;

#[derive(Clone)]
struct FixedEmbedder;

impl Embedder for FixedEmbedder {
    fn identity(&self) -> EmbedderIdentity {
        EmbedderIdentity::new("tc5-test", "1", 384)
    }
    fn embed(&self, input: &str) -> Result<Vector, EmbedderError> {
        Ok(match input {
            "a" => vector(10.0, 10.0),
            "b" => vector(0.9, 0.01),
            "c" => vector(1.0, -0.01),
            _ => unit(0),
        })
    }
}

#[test]
fn direct_vector_stage_refuses_manifest_population_drift() {
    let dir = tempdir().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        dir.path().join("tc5-drift.db"),
        Arc::new(FixedEmbedder),
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine.write(&[node("a"), node("b")]).unwrap();
    engine.drain(5_000).unwrap();
    let error = engine
        .tc5_vector_stage(VectorStageRequest {
            query_vector: unit(0),
            candidate_k: 2,
            top_k: 1,
            scope: VectorStageScope::kind("doc"),
            expected_vector_rows: 3,
            route_observer: RouteObserver::new(),
        })
        .unwrap_err();
    assert_eq!(error, VectorStageError::SelectionDrift { expected: 3, observed: 2 });
}

fn unit(index: usize) -> Vector {
    let mut vector = vec![0.0; 384];
    vector[index] = 1.0;
    vector
}

fn vector(first: f32, second: f32) -> Vector {
    let mut vector = vec![0.0; 384];
    vector[0] = first;
    vector[1] = second;
    vector
}

fn node(body: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".into(),
        body: body.into(),
        logical_id: Some(body.into()),
        source_id: SourceId::new("tc5").unwrap(),
        valid_from: None,
        valid_until: None,
        state: Default::default(),
        reason: None,
    }
}

#[test]
fn direct_vector_stage_uses_one_scope_and_distinct_truth_route() {
    let dir = tempdir().unwrap();
    let opened =
        Engine::open_with_embedder_for_test(dir.path().join("tc5.db"), Arc::new(FixedEmbedder))
            .unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    engine.write(&[node("a"), node("b"), node("c")]).unwrap();
    engine.drain(5_000).unwrap();
    let request = VectorStageRequest {
        query_vector: {
            let mut vector = unit(0);
            vector[1] = 0.01;
            vector
        },
        candidate_k: 2,
        top_k: 1,
        scope: VectorStageScope::kind("doc"),
        expected_vector_rows: 3,
        route_observer: RouteObserver::new(),
    };
    let result = engine.tc5_vector_stage(request).unwrap();
    assert_eq!(result.candidate_count, 2);
    assert_eq!(result.rerank.len(), 1);
    assert_eq!(result.ground_truth.len(), 1);
    assert_eq!(result.routes.vector_stage, 1);
    assert_eq!(result.routes.search, 0);
    assert_eq!(result.routes.fts, 0);
    assert_eq!(result.routes.fusion, 0);
    assert_eq!(result.routes.graph, 0);
    assert_eq!(result.routes.cross_encoder, 0);
    assert_eq!(result.routes.observation_source(), "request_scoped_trap");
    assert_ne!(result.rerank, result.ground_truth);
    assert_eq!(result.candidate_execution, "cpu/sqlite-vec");
    assert_eq!(result.rerank_execution, "cpu/sqlite-vec");
}

#[test]
fn direct_vector_stage_excludes_the_manifest_pinned_query_source_everywhere() {
    let dir = tempdir().unwrap();
    let opened = Engine::open_with_embedder_for_test(
        dir.path().join("tc5-exclusion.db"),
        Arc::new(FixedEmbedder),
    )
    .unwrap();
    let engine = &opened.engine;
    engine.configure_vector_kind_for_test("doc").unwrap();
    let receipt = engine.write(&[node("a"), node("b"), node("c")]).unwrap();
    engine.drain(5_000).unwrap();
    let excluded_row = receipt.row_cursors[0];

    let result = engine
        .tc5_vector_stage(VectorStageRequest {
            query_vector: vector(10.0, 10.0),
            candidate_k: 2,
            top_k: 1,
            scope: VectorStageScope::kind_excluding_logical_id("doc", "a"),
            expected_vector_rows: 2,
            route_observer: RouteObserver::new(),
        })
        .unwrap();

    assert!(!result.candidates.contains(&excluded_row));
    assert!(!result.rerank.contains(&excluded_row));
    assert!(!result.ground_truth.contains(&excluded_row));
    assert_ne!(result.selection_digest, VectorStageScope::kind("doc").selection_digest());
}
