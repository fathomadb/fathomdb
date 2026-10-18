//! Hints off is byte-identical to the output before hints existed
//! (hints-sidecar § 4.1, § 8 test 11). `fixtures/hints_off_baseline.jsonl`
//! is a regression snapshot of the tool-call responses of the determinism
//! session, recorded before any hint code landed; `initialize` and
//! `tools/list` are left out because their instructions and schemas change
//! by design.

mod common;

use common::{edge, node, Fixture, Spawned};
use serde_json::{json, Value};

const PINNED: i64 = 1_900_000_000;
const BASELINE: &str = include_str!("fixtures/hints_off_baseline.jsonl");

fn fixture() -> Fixture {
    let fixture = Fixture::new();
    let mut writes = Vec::new();
    for i in 0..15 {
        writes.push(node(&format!("note:{i:02}"), "note", &format!("river crossing story {i}")));
    }
    let long = "river crossing told at length, with the water high and the ford gone. ".repeat(30);
    writes.push(node("note:long", "note", &long));
    writes.push(edge("link", "note:00", "note:01"));
    writes.push(edge("link", "note:00", "note:02"));
    fixture.store("a.sqlite", writes);
    fixture
}

/// The determinism session's tool calls, each with `extra` merged into its
/// arguments; returns each tool-call response line as written.
fn session(fixture: &Fixture, config: Value, extra: &Value) -> Vec<String> {
    let config = fixture.config_file(config);
    let mut server = Spawned::start(&config);
    server.initialize();
    let mut call = |tool: &str, mut args: Value| {
        for (key, value) in extra.as_object().unwrap() {
            args[key] = value.clone();
        }
        server.call_raw(tool, args)
    };
    let mut lines = vec![call("use_store", json!({"store": "a"}))];
    lines.push(call("list_stores", json!({})));
    let first = call("search", json!({"query": "river", "limit": 4, "as_of": PINNED}));
    let first_value: Value = serde_json::from_str(&first).unwrap();
    let continuation = first_value["result"]["structuredContent"]["continuation"].clone();
    lines.push(first);
    lines.push(call("search", json!({"continuation": continuation})));
    lines.push(call(
        "search",
        json!({"query": "story", "mode": "hybrid", "explain": true, "as_of": PINNED, "limit": 6}),
    ));
    lines.push(call(
        "search",
        json!({"query": "river", "limit": 15, "max_tokens": 600, "as_of": PINNED}),
    ));
    lines.push(call("search", json!({"query": "zebra", "as_of": PINNED})));
    lines.push(call(
        "fetch",
        json!({"refs": ["l:note:03", "l:missing", "h:x", "l:note:01"], "as_of": PINNED}),
    ));
    lines.push(call("fetch", json!({"refs": ["l:note:long"], "max_tokens": 256, "as_of": PINNED})));
    lines.push(call("expand", json!({"refs": ["l:note:00"], "as_of": PINNED})));
    let (status, _, _) = server.finish();
    assert_eq!(status.code(), Some(0));
    lines
}

fn assert_baseline(lines: &[String], how: &str) {
    let expected: Vec<&str> = BASELINE.lines().collect();
    assert_eq!(lines.len(), expected.len(), "{how}");
    for (line, expected) in lines.iter().zip(expected) {
        assert_eq!(line, expected, "{how}");
    }
}

#[test]
fn hints_off_output_is_byte_identical_to_baseline() {
    let fixture = fixture();
    let stores = json!({"a": {"path": "a.sqlite"}});
    let by_config = session(&fixture, json!({"stores": stores, "hints": false}), &json!({}));
    assert_baseline(&by_config, "config hints: false");
    let by_call = session(&fixture, json!({"stores": stores}), &json!({"hints": false}));
    assert_baseline(&by_call, "per-call hints: false");
}
