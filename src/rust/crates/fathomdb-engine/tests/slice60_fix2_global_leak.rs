//! Slice 60 FIX-2 RED: a request-scoped rendezvous must not leak into another request.

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use fathomdb_engine::{
    Engine, GraphExpandRendezvousForTest, GraphExpandRequestV1, GraphReadContextV1, GraphSeedV1,
    IdSpace, ReadContextV1, ReadView, SearchFilter, TraversalDirection,
};
use fathomdb_schema::SQLITE_SUFFIX;
use tempfile::TempDir;

fn request(id: &str) -> GraphExpandRequestV1 {
    GraphExpandRequestV1 {
        schema_version: 1,
        seed: GraphSeedV1::Explicit { schema_version: 1, logical_ids: vec![IdSpace::logical(id)] },
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
        include_evidence: false,
    }
}

#[test]
fn an_unrelated_request_cannot_consume_a_request_scoped_rendezvous() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join(format!("fix2-global{SQLITE_SUFFIX}"));
    let engine = Arc::new(Engine::open(database).unwrap().engine);
    let rendezvous = GraphExpandRendezvousForTest::before_pin(Duration::from_millis(100));

    let _ = engine.graph_expand(&request("unrelated"));
    assert!(rendezvous.wait_until_entered().is_err());

    let worker = {
        let engine = Arc::clone(&engine);
        let rendezvous = rendezvous.clone();
        thread::spawn(move || {
            engine.graph_expand_with_rendezvous_for_test(&request("intended"), rendezvous)
        })
    };
    rendezvous.wait_until_entered().unwrap();
    rendezvous.release();
    let _ = worker.join().unwrap();
}
