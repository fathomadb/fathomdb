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
    eprintln!(
        "slice60 fix5 live RSS samples: {:?}",
        samples.iter().map(|sample| sample.peak_rss_delta_bytes).collect::<Vec<_>>()
    );
    for sample in &samples {
        assert!(sample.retained_edge_batch_rows <= sample.work_units);
        assert!(sample.frontier_states <= sample.work_units.saturating_add(1));
        assert!(sample.visited_states <= sample.work_units.saturating_add(1));
        assert!(sample.candidate_targets <= 1);
    }
    assert_eq!(
        (
            samples[1].retained_edge_batch_rows,
            samples[1].frontier_states,
            samples[1].visited_states,
            samples[1].candidate_targets,
        ),
        (
            samples[2].retained_edge_batch_rows,
            samples[2].frontier_states,
            samples[2].visited_states,
            samples[2].candidate_targets,
        )
    );
}
