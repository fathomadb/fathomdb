//! Slice 60 FIX-2 RED: a global rendezvous must not leak into another request.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use fathomdb_engine::{
    arm_graph_expand_before_pin_hook_for_test, Engine, GraphExpandRequestV1, GraphReadContextV1,
    GraphSeedV1, IdSpace, ReadContextV1, ReadView, SearchFilter, TraversalDirection,
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
    }
}

#[test]
fn an_unrelated_request_cannot_consume_a_request_scoped_rendezvous() {
    let directory = TempDir::new().unwrap();
    let database = directory.path().join(format!("fix2-global{SQLITE_SUFFIX}"));
    let engine = Engine::open(database).unwrap().engine;
    let fired = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&fired);
    arm_graph_expand_before_pin_hook_for_test(Box::new(move || {
        observed.fetch_add(1, Ordering::SeqCst);
    }));

    let _ = engine.graph_expand(&request("unrelated"));
    assert_eq!(fired.load(Ordering::SeqCst), 0);
}
