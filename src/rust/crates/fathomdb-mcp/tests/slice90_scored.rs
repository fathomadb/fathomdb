#![cfg(feature = "default-embedder")]
mod common;

use common::{Fixture, Harness};
use fathomdb_sdk::{
    Engine, OpenOptions, ProjectionRole, ProjectionSpec, ProjectionVector, PropertyEntityClass,
    SearchOptions,
};
use serde_json::json;
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread")]
async fn s90_real_mcp_vector_hit_keeps_scored_extent_and_final_budget() {
    let fixture = Fixture::new();
    let body = "\x1e\x11é\n\x1e\x12🦀 zed";
    assert_eq!(body.len(), 15);
    let engine = Arc::new(
        Engine::open(
            fixture.path("vector.sqlite"),
            OpenOptions { use_default_embedder: true, ..OpenOptions::default() },
        )
        .unwrap(),
    );
    engine
        .configure_projections(
            &[ProjectionSpec {
                name: "body".into(),
                roles: [ProjectionRole::Searchable].into_iter().collect(),
                fts: None,
                vector: Some(ProjectionVector {
                    embedder: None,
                    dense_readiness: None,
                    kinds: None,
                }),
                source: None,
                entity_class: PropertyEntityClass::Node,
            }],
            &[],
        )
        .unwrap();
    engine.write(&[common::node("covered", "doc", body)]).unwrap();
    engine.drain(30_000).unwrap();
    let mut options = SearchOptions::default();
    options.explain = true;
    let control = engine.search("zed", options).unwrap();
    assert_eq!(control.results.len(), 1);
    assert!(control.explanation.unwrap().per_hit[0].vector_rank.is_some());
    for format in ["structured", "text"] {
        let harness = Harness::start(fixture.config(json!({"result_format":format}))).await;
        harness.adapter.register_engine("a", engine.clone()).unwrap();
        let result = harness
            .call(
                "search",
                json!({"store":"a","query":"zed","mode":"hybrid","hints":false,"max_tokens":4000}),
            )
            .await;
        assert_ne!(result.is_error, Some(true));
        let text = common::text_of(&result);
        if let Some(payload) = &result.structured_content {
            assert_eq!(payload["results"][0]["scored_bytes"], 15, "MCP drops exact scored extent");
            let mut without_budget = payload.clone();
            without_budget.as_object_mut().unwrap().remove("budget");
            let expected =
                (serde_json::to_string(&without_budget).unwrap().len() + text.len()).div_ceil(4);
            assert_eq!(payload["budget"]["used_tokens"], expected);
            assert!((2 * text.len()).div_ceil(4) <= 4000);
        } else {
            assert!(text.lines().any(|line| line.contains(" scored_bytes=15 | ")), "{text}");
            let first = text.lines().next().unwrap();
            assert!(first.contains(&format!("tokens={}/4000", text.len().div_ceil(4))), "{text}");
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
}
