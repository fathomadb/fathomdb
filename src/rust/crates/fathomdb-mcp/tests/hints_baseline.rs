//! Hints-off historical semantics remain pinned, with the exact reviewed
//! format/budget adaptation; config/per-call hints-off bytes match per format. `fixtures/hints_off_baseline.jsonl`
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
    let continuation = first_value["result"]
        .get("structuredContent")
        .map(|payload| payload["continuation"].clone())
        .unwrap_or_else(|| {
            let text = first_value["result"]["content"][0]["text"].as_str().unwrap();
            text.lines()
                .next()
                .unwrap()
                .split_whitespace()
                .find_map(|word| word.strip_prefix("continuation="))
                .map_or(Value::Null, |handle| json!(handle))
        });
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

// This is a reference calculation over authored payloads, never a production
// renderer/shaper or a generated replacement fixture.
fn reference_json(mut payload: Value) -> (Value, String) {
    if payload.get("budget").is_none() {
        let text = serde_json::to_string(&payload).unwrap();
        return (payload, text);
    }
    payload["budget"]["used_tokens"] = json!(0);
    for _ in 0..4 {
        let text = serde_json::to_string(&payload).unwrap();
        let mut without = payload.clone();
        without.as_object_mut().unwrap().remove("budget");
        let used = (serde_json::to_string(&without).unwrap().len() + text.len()).div_ceil(4) as u64;
        if payload["budget"]["used_tokens"] == used {
            return (payload, text);
        }
        payload["budget"]["used_tokens"] = json!(used);
    }
    panic!("authored reference count must converge within four renders");
}

fn reference_handle(prefix: char, epoch: u64, parts: &[&str]) -> String {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    let suffix =
        hash.finalize().iter().take(8).map(|byte| format!("{byte:02x}")).collect::<String>();
    format!("{prefix}{epoch}-{suffix}")
}

fn historical_search_prefix(expected: &Value, kept: usize) -> Value {
    let mut payload = expected.clone();
    let results = expected["results"].as_array().unwrap()[..kept].to_vec();
    let refs =
        results.iter().map(|hit| hit["ref"].as_str().unwrap()).collect::<Vec<_>>().join("\n");
    // Exact authored request of the historical 600-token call, including field
    // order used by the existing versioned handle contract.
    let canonical = r#"{"query":"river","mode":"text","kind":null,"source_type":null,"status":null,"created_after":null,"attributes":[],"as_of":1900000000,"limit":15,"explain":false}"#;
    let store = expected["store"].as_str().unwrap();
    let epoch = expected["epoch"].as_u64().unwrap();
    payload["results"] = json!(results);
    payload["budget"]["cut"] = json!(true);
    payload["completeness"] = json!("cut_by_budget");
    payload["response_id"] = json!(reference_handle('r', epoch, &[store, canonical, "0", &refs]));
    payload["continuation"] = if kept == 0 {
        Value::Null
    } else {
        json!(reference_handle('c', epoch, &[store, canonical, &kept.to_string()]))
    };
    reference_json(payload).0
}

// The retained structured cutting policy is deterministic: measure both final
// JSON copies, shorten the longest body by ceil(excess_tokens * 4 / 2)
// characters (at least one), and repeat. This authored case has one ASCII
// body, so character/UTF-8 boundaries coincide. No production shaper is called.
fn historical_fetch_prefix(original: &Value) -> Result<Value, String> {
    let full = "river crossing told at length, with the water high and the ford gone. ".repeat(30);
    let historical =
        original["items"][0]["record"]["body"].as_str().ok_or("historical body type")?;
    if !full.starts_with(historical) {
        return Err("historical body is not the authored fixture prefix".into());
    }
    let max = original["budget"]["max_tokens"].as_u64().ok_or("historical fetch max")?;
    let mut kept = full.len();
    loop {
        let mut candidate = original.clone();
        candidate["items"][0]["record"]["body"] = json!(&full[..kept]);
        candidate["items"][0]["record"]["body_cut"] = json!(kept < full.len());
        candidate["budget"]["cut"] = json!(kept < full.len());
        let (candidate, text) = reference_json(candidate);
        let whole = (2 * text.len()).div_ceil(4) as u64;
        if whole <= max {
            if kept > historical.len() {
                return Err(
                    "fetch exceeds historical semantic coverage; additional oracle review required"
                        .into(),
                );
            }
            return Ok(candidate);
        }
        if kept == 0 {
            return Err("mandatory fetch metadata does not fit".into());
        }
        kept = kept.saturating_sub(((whole - max) as usize * 4).div_ceil(2).max(1));
    }
}

fn current_wire(expected: &Value, payload: Value) -> Value {
    let mut current = expected.clone();
    let (payload, text) = reference_json(payload);
    current["result"]["structuredContent"] = payload;
    current["result"]["content"][0]["text"] = json!(text);
    current
}

fn compare_baseline(current: &Value, expected: &Value, index: usize) -> Result<(), String> {
    let payload = &current["result"]["structuredContent"];
    if !payload.is_object() {
        return Err("structured payload missing or wrong type".into());
    }
    let text = serde_json::to_string(payload).unwrap();
    if current["result"]["content"][0]["text"] != text {
        return Err("text is not exact final payload JSON".into());
    }
    let original = &expected["result"]["structuredContent"];
    let mut desired = original.clone();
    if let Some(budget) = payload.get("budget") {
        let max = original["budget"]["max_tokens"].as_u64().ok_or("unexpected budget")?;
        if budget["max_tokens"] != max {
            return Err("max_tokens changed".into());
        }
        let mut without = payload.clone();
        without.as_object_mut().unwrap().remove("budget");
        let used = (serde_json::to_string(&without).unwrap().len() + text.len()).div_ceil(4) as u64;
        if budget["used_tokens"] != used {
            return Err("false used_tokens".into());
        }
        if (2 * text.len()).div_ceil(4) as u64 > max {
            return Err("both final JSON copies exceed budget".into());
        }
    }
    if index == 5 {
        let max = original["budget"]["max_tokens"].as_u64().unwrap();
        let covered = original["results"].as_array().unwrap().len();
        let fitting = (0..=covered)
            .rev()
            .map(|n| historical_search_prefix(original, n))
            .find(|candidate| {
                (2 * serde_json::to_string(candidate).unwrap().len()).div_ceil(4) as u64 <= max
            })
            .ok_or("mandatory search metadata does not fit")?;
        if payload["results"] != fitting["results"] {
            return Err("search is not the independently sized leading prefix".into());
        }
        for key in ["results", "completeness", "response_id", "continuation"] {
            desired[key] = fitting[key].clone();
        }
        desired["budget"]["cut"] = fitting["budget"]["cut"].clone();
    }
    if index == 8 {
        let fitting = historical_fetch_prefix(original)?;
        if payload["items"][0]["record"]["body"] != fitting["items"][0]["record"]["body"] {
            return Err(
                "fetch body differs from the independently derived retained cutting policy".into(),
            );
        }
        desired["items"][0]["record"]["body"] = fitting["items"][0]["record"]["body"].clone();
        desired["items"][0]["record"]["body_cut"] =
            fitting["items"][0]["record"]["body_cut"].clone();
        desired["budget"]["cut"] = fitting["budget"]["cut"].clone();
    }
    if desired.get("budget").is_some() {
        desired["budget"]["used_tokens"] = payload["budget"]["used_tokens"].clone();
    }
    let mut allowed = expected.clone();
    allowed["result"]["structuredContent"] = desired;
    allowed["result"]["content"][0]["text"] = json!(text);
    if &allowed != current {
        return Err("unapproved field/value/type/order or envelope change".into());
    }
    Ok(())
}

fn assert_baseline(lines: &[String], how: &str) {
    let expected = BASELINE
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), expected.len(), "{how}");
    for (index, (line, expected)) in lines.iter().zip(expected).enumerate() {
        let current = serde_json::from_str(line).unwrap();
        compare_baseline(&current, &expected, index)
            .unwrap_or_else(|error| panic!("{how} line {index}: {error}\n{line}"));
    }
}

#[test]
fn hints_off_preserves_historical_semantics_and_each_current_format_is_byte_identical() {
    let fixture = fixture();
    let stores = json!({"a":{"path":"a.sqlite"}});
    for format in ["structured", "text"] {
        let by_config = session(
            &fixture,
            json!({"stores":stores,"hints":false,"result_format":format}),
            &json!({}),
        );
        let by_call = session(
            &fixture,
            json!({"stores":stores,"result_format":format}),
            &json!({"hints":false}),
        );
        assert_eq!(by_config, by_call, "config/per-call hints off in {format}");
        if format == "structured" {
            assert_baseline(&by_config, "immutable historical semantics");
        }
    }
}

#[test]
fn historical_comparator_rejects_unapproved_semantic_and_accounting_changes() {
    let rows = BASELINE
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    let valid = current_wire(&rows[2], rows[2]["result"]["structuredContent"].clone());
    assert!(compare_baseline(&valid, &rows[2], 2).is_ok());
    for change in ["score", "source", "ref_order", "unknown", "type", "state", "verdict", "handle"]
    {
        let mut payload = rows[2]["result"]["structuredContent"].clone();
        match change {
            "score" => payload["results"][0]["score"] = json!(9),
            "source" => payload["results"][0]["source_id"] = json!("wrong"),
            "ref_order" => payload["results"].as_array_mut().unwrap().swap(0, 1),
            "unknown" => payload["unknown"] = json!(true),
            "type" => payload["epoch"] = json!("1"),
            "state" => {
                payload.as_object_mut().unwrap().remove("state");
            }
            "verdict" => {
                payload.as_object_mut().unwrap().remove("verdict");
            }
            "handle" => payload["continuation"] = json!("c1-0000000000000000"),
            _ => unreachable!(),
        }
        assert!(
            compare_baseline(&current_wire(&rows[2], payload), &rows[2], 2).is_err(),
            "{change}"
        );
    }
    let mut bad = valid.clone();
    bad["result"]["structuredContent"]["budget"]["used_tokens"] = json!(1);
    bad["result"]["content"][0]["text"] =
        json!(serde_json::to_string(&bad["result"]["structuredContent"]).unwrap());
    assert!(compare_baseline(&bad, &rows[2], 2).is_err(), "false count");
    let mut payload = rows[7]["result"]["structuredContent"].clone();
    payload["items"][1]["error"]["code"] = json!("wrong");
    assert!(
        compare_baseline(&current_wire(&rows[7], payload), &rows[7], 7).is_err(),
        "wrong per-item error"
    );
    let original = &rows[5]["result"]["structuredContent"];
    let max = original["budget"]["max_tokens"].as_u64().unwrap();
    let prefix = (0..=original["results"].as_array().unwrap().len())
        .rev()
        .map(|n| historical_search_prefix(original, n))
        .find(|candidate| {
            (2 * serde_json::to_string(candidate).unwrap().len()).div_ceil(4) as u64 <= max
        })
        .unwrap();
    assert!(compare_baseline(&current_wire(&rows[5], prefix.clone()), &rows[5], 5).is_ok());
    let mut middle = prefix.clone();
    middle["results"][1] = original["results"][2].clone();
    assert!(
        compare_baseline(&current_wire(&rows[5], middle), &rows[5], 5).is_err(),
        "middle-hit removal"
    );
    let mut wrong_handle = prefix;
    wrong_handle["response_id"] = json!("r1-0000000000000000");
    assert!(
        compare_baseline(&current_wire(&rows[5], wrong_handle), &rows[5], 5).is_err(),
        "false derived handle"
    );
    let original = &rows[8]["result"]["structuredContent"];
    let fitting = historical_fetch_prefix(original).unwrap();
    assert!(compare_baseline(&current_wire(&rows[8], fitting.clone()), &rows[8], 8).is_ok());
    let body = fitting["items"][0]["record"]["body"].as_str().unwrap();
    assert!(!body.is_empty(), "authored fixture must exercise unjustified empty and shorter cuts");
    for shorter in ["", &body[..body.len() - 1]] {
        let mut overcut = fitting.clone();
        overcut["items"][0]["record"]["body"] = json!(shorter);
        // Recompute counts and preserve truthful cut flags: only the unjustified
        // loss of body bytes should cause rejection, not stale accounting.
        assert!(
            compare_baseline(&current_wire(&rows[8], overcut), &rows[8], 8).is_err(),
            "unjustified fetch prefix {shorter:?}"
        );
    }
}
