//! Slice 60 FIX-5 RED: isolated subprocess RSS witnesses live graph expansion.

#![cfg(feature = "test-hooks")]

use fathomdb_engine::{
    Engine, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1, IdSpace, ReadContextV1,
    ReadView, SearchFilter, TraversalDirection,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Fixture {
    proportional_ceiling_bytes: u64,
    arms: Vec<Arm>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Arm {
    work_units: u64,
    unrelated_nodes: u64,
}

fn request() -> GraphExpandRequestV1 {
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
            context: ReadContextV1::new(ReadView::default(), SearchFilter::default()).unwrap(),
        },
        max_depth: 1,
        result_limit: 1,
        max_work_units: 10_000,
        include_explanation: false,
    }
}

#[test]
fn isolated_process_rss_captures_live_peak_for_small_exact_work_and_unrelated_controls() {
    let fixture: Fixture =
        serde_json::from_str(include_str!("../../../../../dev/fixtures/slice60-fix5-rss-v1.json"))
            .unwrap();
    let work_units = fixture.arms.iter().map(|arm| arm.work_units).collect::<Vec<_>>();
    let unrelated_nodes = fixture.arms.iter().map(|arm| arm.unrelated_nodes).collect::<Vec<_>>();
    let samples = Engine::graph_expand_isolated_process_rss_samples_for_test(
        &request(),
        &work_units,
        &unrelated_nodes,
    )
    .unwrap();

    assert_eq!(samples.len(), 3);
    assert!(samples.iter().all(|sample| sample.process_id != std::process::id()));
    assert_eq!(samples[0].work_units, 1);
    assert_eq!(samples[1].work_units, 10_000);
    assert_eq!(samples[2].work_units, 10_000);
    assert!(
        samples[1].peak_rss_delta_bytes
            <= samples[0].peak_rss_delta_bytes + fixture.proportional_ceiling_bytes
    );
    assert!(
        samples[2].peak_rss_delta_bytes
            <= samples[1].peak_rss_delta_bytes + fixture.proportional_ceiling_bytes
    );
}
