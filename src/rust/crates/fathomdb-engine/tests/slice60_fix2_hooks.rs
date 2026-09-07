//! Slice 60 FIX-2 RED: owned, bounded, cancellation-safe rendezvous controls.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use fathomdb_engine::{
    Engine, GraphExpandRendezvousForTest, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1,
    IdSpace, ReadContextV1, ReadView, SearchFilter, TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn request() -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit {
            schema_version: 1,
            logical_ids: vec![IdSpace::logical("seed")],
        },
        direction: TraversalDirection::Outgoing,
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
        max_depth: 0,
        result_limit: 1,
        max_work_units: 1,
        include_explanation: false,
    }
}

#[test]
fn dropping_a_request_owned_rendezvous_releases_and_disarms_idempotently() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join(format!("fix2-hooks{SQLITE_SUFFIX}"));
    let engine = Arc::new(Engine::open(database).unwrap().engine);
    let rendezvous = GraphExpandRendezvousForTest::before_pin(Duration::from_millis(100));
    let worker_engine = Arc::clone(&engine);
    let worker_rendezvous = rendezvous.clone();
    let worker = thread::spawn(move || {
        worker_engine.graph_expand_with_rendezvous_for_test(&request(), worker_rendezvous)
    });

    rendezvous.wait_until_entered().unwrap();
    drop(rendezvous);
    let _ = worker.join().unwrap();
}

#[test]
fn a_cancelled_or_dropped_rendezvous_cannot_delay_an_unrelated_request() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join(format!("fix2-cancel{SQLITE_SUFFIX}"));
    let engine = Engine::open(database).unwrap().engine;
    let rendezvous = GraphExpandRendezvousForTest::after_pin(Duration::from_millis(100));
    drop(rendezvous);
    let started = std::time::Instant::now();
    let _ = engine.graph_expand(&request());
    assert!(started.elapsed() < Duration::from_millis(100));
}
