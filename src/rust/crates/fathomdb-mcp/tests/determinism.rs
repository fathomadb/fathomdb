//! Determinism (design §9 order 9): every read tool, called in two process
//! lifetimes against an unchanged database with the validity instant
//! pinned, writes byte-identical responses (MR-043.1); unpinned, the
//! outputs differ only in `validity_instant` (MR-043.2).

mod common;

use common::{edge, node, Fixture, Spawned};
use serde_json::{json, Value};

fn fixture() -> Fixture {
    let fixture = Fixture::new();
    let mut writes = Vec::new();
    for i in 0..15 {
        writes.push(node(&format!("note:{i:02}"), "note", &format!("river crossing story {i}")));
    }
    writes.push(edge("link", "note:00", "note:01"));
    writes.push(edge("link", "note:00", "note:02"));
    fixture.store("a.sqlite", writes);
    fixture
}

const PINNED: i64 = 1_900_000_000;

/// One scripted session; returns each response line as written.
fn session(fixture: &Fixture) -> Vec<String> {
    let config = fixture.config_file(json!({"stores": {"a": {"path": "a.sqlite"}}}));
    let mut server = Spawned::start(&config);
    server.initialize();
    let mut lines = vec![server.request_raw("tools/list", json!({}))];
    lines.push(server.call_raw("use_store", json!({"store": "a"})));
    lines.push(server.call_raw("list_stores", json!({})));
    let first = server.call_raw("search", json!({"query": "river", "limit": 4, "as_of": PINNED}));
    let first_value: Value = serde_json::from_str(&first).unwrap();
    let continuation = first_value["result"]["structuredContent"]["continuation"].clone();
    lines.push(first);
    lines.push(server.call_raw("search", json!({"continuation": continuation})));
    lines.push(server.call_raw(
        "search",
        json!({"query": "story", "mode": "hybrid", "explain": true, "as_of": PINNED, "limit": 6}),
    ));
    lines.push(server.call_raw(
        "search",
        json!({"query": "river", "limit": 15, "max_tokens": 600, "as_of": PINNED}),
    ));
    lines.push(server.call_raw(
        "fetch",
        json!({"refs": ["l:note:03", "l:missing", "h:x", "l:note:01"], "as_of": PINNED}),
    ));
    lines.push(server.call_raw("expand", json!({"refs": ["l:note:00"], "as_of": PINNED})));
    let (status, _, _) = server.finish();
    assert_eq!(status.code(), Some(0));
    lines
}

#[test]
fn pinned_sessions_in_two_process_lifetimes_are_byte_identical() {
    let fixture = fixture();
    let one = session(&fixture);
    let two = session(&fixture);
    assert_eq!(one.len(), two.len());
    for (left, right) in one.iter().zip(&two) {
        assert_eq!(left, right, "MR-043.1");
    }
    // The pinned search really paged, so continuation handles were compared too.
    assert!(one[3].contains("\"continuation\":\"c1-"), "{}", one[3]);
}

fn without_instant(line: &str) -> String {
    let value: Value = serde_json::from_str(line).unwrap();
    let instant = value["result"]["structuredContent"]["validity_instant"].clone();
    let mut text = line.to_string();
    if let Some(instant) = instant.as_i64() {
        text = text.replace(&instant.to_string(), "<instant>");
    }
    text
}

#[test]
fn unpinned_sessions_differ_only_in_the_validity_instant() {
    let fixture = fixture();
    let run = || {
        let config = fixture.config_file(json!({"stores": {"a": {"path": "a.sqlite"}}}));
        let mut server = Spawned::start(&config);
        server.initialize();
        let lines = vec![
            server.call_raw("orient", json!({"store": "a"})),
            server.call_raw("search", json!({"store": "a", "query": "river"})),
            server.call_raw("fetch", json!({"store": "a", "refs": ["l:note:03"]})),
            server.call_raw("expand", json!({"store": "a", "refs": ["l:note:00"]})),
        ];
        server.finish();
        lines
    };
    let one = run();
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let two = run();
    for (left, right) in one.iter().zip(&two) {
        assert_eq!(without_instant(left), without_instant(right), "MR-043.2");
        let left: Value = serde_json::from_str(left).unwrap();
        assert!(left["result"]["structuredContent"]["validity_instant"].is_i64());
    }
    // No identifier is derived from the instant.
    let ids = |lines: &[String]| -> Value {
        let value: Value = serde_json::from_str(&lines[1]).unwrap();
        value["result"]["structuredContent"]["response_id"].clone()
    };
    assert_eq!(ids(&one), ids(&two));
}
