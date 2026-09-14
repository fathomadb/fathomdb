//! Slice 60 FIX-4 RED: graph expansion reports isolated current-RSS samples.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    Engine, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, ReadContextV1,
    ReadView, SearchFilter, TraversalDirection,
};

fn request() -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("root")],
        },
        direction: TraversalDirection::Both,
        edge_kinds: vec![],
        target_kinds: vec![],
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
        max_work_units: 10_000,
        include_explanation: false,
        include_evidence: false,
    }
}

#[test]
fn isolated_current_rss_compares_small_exact_work_and_unrelated_database_size() {
    let samples = Engine::graph_expand_current_rss_samples_for_test(
        &request(),
        &[1, 10_000, 10_000],
        &[0, 0, 100_000],
    )
    .unwrap();
    assert_eq!(samples.len(), 3);
    assert!(
        samples[1].current_rss_delta_bytes <= samples[0].current_rss_delta_bytes + 4 * 1024 * 1024
    );
    assert!(
        samples[2].current_rss_delta_bytes <= samples[1].current_rss_delta_bytes + 4 * 1024 * 1024
    );
    assert_eq!(samples[1].work_units, 10_000);
}
