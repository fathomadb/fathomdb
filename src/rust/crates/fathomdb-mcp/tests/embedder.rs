//! With the default embedder (feature `default-embedder`): the binary warms
//! the embedder at open without writing anything but JSON-RPC to stdout
//! (MR-002.1), and hybrid search runs with a usable dense runtime, so the
//! vector arm is not reported as unavailable (MR-032.1). Needs the pinned
//! model weights in the local cache or network access.

#![cfg(feature = "default-embedder")]

mod common;

use common::{assert_json_rpc_only, node, Fixture, Spawned};
use serde_json::json;

#[test]
fn hybrid_search_with_the_default_embedder_keeps_stdout_pure() {
    let fixture = Fixture::new();
    let writes: Vec<_> = (0..6)
        .map(|i| node(&format!("note:{i}"), "note", &format!("river crossing story {i}")))
        .collect();
    fixture.store("a.sqlite", writes);
    let config = fixture.config_file(json!({
        "stores": {"a": {"path": "a.sqlite", "eager": true}},
        "default_store": "a",
        "use_default_embedder": true,
        "search_mode": "hybrid",
        "call_deadline_ms": 120000
    }));
    let mut server = Spawned::start(&config);
    let started = std::time::Instant::now();
    server.initialize();
    assert!(started.elapsed() < std::time::Duration::from_secs(5), "handshake does not wait");

    let orient = server.call("orient", json!({}));
    assert_ne!(orient["isError"], true, "{orient}");
    assert_eq!(orient["structuredContent"]["embedding_readiness"]["usable_embedder"], true);
    let search = server.call("search", json!({"query": "river"}));
    assert_ne!(search["isError"], true, "{search}");
    let found = &search["structuredContent"];
    assert_eq!(found["mode"], "hybrid");
    assert!(!found["results"].as_array().unwrap().is_empty());
    let degradation = found["degradation"].as_array().unwrap();
    assert!(
        degradation.iter().all(|entry| entry["reason"] != "embedder_unavailable"),
        "{degradation:?}"
    );
    let (status, lines, _) = server.finish();
    assert_eq!(status.code(), Some(0));
    assert_json_rpc_only(&lines);
}
