#![no_main]

use std::path::Path;

use arbitrary::Unstructured;
use fathomdb_mcp::config::Config;
use fathomdb_mcp::seams::parse_tool_arguments_for_fuzz;
use libfuzzer_sys::fuzz_target;

const DECLARED_TOOLS: &[&str] = &[
    "list_stores",
    "use_store",
    "orient",
    "search",
    "fetch",
    "expand",
    "feedback",
    "actuate",
    "put_latest_state",
];

const FORBIDDEN_TOOLS: &[&str] = &[
    "recover",
    "restore",
    "repair",
    "fix",
    "rebuild",
    "purge",
    "erase_source",
    "migrate",
    "safe_export",
    "sql",
    "execute_sql",
    "bulk_write",
    "write_with_options",
];

fn arb_str(u: &mut Unstructured<'_>, max_len: usize) -> String {
    let raw: String = u.arbitrary().unwrap_or_default();
    raw.chars().take(max_len).collect()
}

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let base_dir = Path::new("/tmp/fathomdb-mcp-fuzz-base");

    // 1. Always fuzz Config::from_json_str directly on raw UTF-8 text.
    if let Ok(raw_text) = std::str::from_utf8(data) {
        let _ = Config::from_json_str(raw_text, base_dir);
    }

    let mut u = Unstructured::new(data);
    let mode = u.int_in_range(0..=3).unwrap_or(0);

    match mode {
        0 => {
            // Structured Config JSON exercising SC-01..SC-10 & sync_mode rules.
            let allow_writes = u.arbitrary().unwrap_or(false);
            let store_allow_writes = u.arbitrary().unwrap_or(false);
            let sync_mode_val: serde_json::Value = match u.int_in_range(0..=4).unwrap_or(0) {
                0 => serde_json::Value::Null,
                1 => serde_json::json!("normal"),
                2 => serde_json::json!("full"),
                3 => serde_json::json!(arb_str(&mut u, 16)),
                _ => serde_json::json!(u.arbitrary::<i64>().unwrap_or(0)),
            };
            let include_sync = u.arbitrary().unwrap_or(true);
            let mut store_a = serde_json::Map::new();
            store_a.insert("path".to_string(), serde_json::json!(arb_str(&mut u, 48)));
            store_a.insert("allow_writes".to_string(), serde_json::json!(store_allow_writes));
            store_a.insert("create".to_string(), serde_json::json!(u.arbitrary().unwrap_or(false)));
            store_a.insert("eager".to_string(), serde_json::json!(u.arbitrary().unwrap_or(false)));
            if include_sync {
                store_a.insert("sync_mode".to_string(), sync_mode_val.clone());
            }
            if u.arbitrary().unwrap_or(false) {
                let scope_count = u.int_in_range(0..=4).unwrap_or(1);
                let reads: Vec<String> = (0..scope_count).map(|_| arb_str(&mut u, 24)).collect();
                let write_scope = if u.arbitrary().unwrap_or(true) {
                    reads.first().cloned()
                } else {
                    Some(arb_str(&mut u, 24))
                };
                store_a.insert(
                    "scope".to_string(),
                    serde_json::json!({
                        "read": reads,
                        "write": write_scope,
                    }),
                );
            }
            if u.arbitrary().unwrap_or(false) {
                store_a
                    .insert("allowed_kinds".to_string(), serde_json::json!([arb_str(&mut u, 24)]));
            }

            let mut stores = serde_json::Map::new();
            let name_a = if u.arbitrary().unwrap_or(true) {
                "store_a".to_string()
            } else {
                arb_str(&mut u, 24)
            };
            stores.insert(name_a.clone(), serde_json::Value::Object(store_a.clone()));

            // Optional alias store sharing same path with potentially conflicting sync_mode/scope (SC-06).
            if u.arbitrary().unwrap_or(false) {
                let mut store_b = store_a;
                if u.arbitrary().unwrap_or(false) {
                    store_b.insert("sync_mode".to_string(), serde_json::json!("full"));
                }
                stores.insert("store_b".to_string(), serde_json::Value::Object(store_b));
            }

            let mut root_obj = serde_json::Map::new();
            root_obj.insert("stores".to_string(), serde_json::Value::Object(stores));
            root_obj.insert("allow_writes".to_string(), serde_json::json!(allow_writes));
            if u.arbitrary().unwrap_or(false) {
                let mut r = serde_json::Map::new();
                r.insert("path".to_string(), serde_json::json!(arb_str(&mut u, 32)));
                if u.arbitrary().unwrap_or(false) {
                    r.insert("sync_mode".to_string(), sync_mode_val);
                }
                root_obj.insert(
                    "roots".to_string(),
                    serde_json::Value::Array(vec![serde_json::Value::Object(r)]),
                );
            }
            if u.arbitrary().unwrap_or(false) {
                root_obj.insert("default_store".to_string(), serde_json::json!(name_a));
            }
            if u.arbitrary().unwrap_or(false) {
                root_obj.insert(
                    "search_mode".to_string(),
                    serde_json::json!(if u.arbitrary().unwrap_or(true) {
                        "text"
                    } else {
                        "hybrid"
                    }),
                );
            }
            if u.arbitrary().unwrap_or(false) {
                root_obj
                    .insert("result_format".to_string(), serde_json::json!(arb_str(&mut u, 16)));
            }
            if u.arbitrary().unwrap_or(false) {
                root_obj.insert(
                    "call_deadline_ms".to_string(),
                    serde_json::json!(u.arbitrary::<u64>().unwrap_or(1000)),
                );
                root_obj.insert(
                    "run_limit_ms".to_string(),
                    serde_json::json!(u.arbitrary::<u64>().unwrap_or(1000)),
                );
            }

            let text = serde_json::Value::Object(root_obj).to_string();
            let _ = Config::from_json_str(&text, base_dir);
        }
        1 => {
            // Raw JSON dispatched against any declared or forbidden MCP tool parser.
            let tool = match u.int_in_range(0..=2).unwrap_or(0) {
                0 => {
                    let idx = u.int_in_range(0..=(DECLARED_TOOLS.len() - 1)).unwrap_or(0);
                    DECLARED_TOOLS[idx].to_string()
                }
                1 => {
                    let idx = u.int_in_range(0..=(FORBIDDEN_TOOLS.len() - 1)).unwrap_or(0);
                    FORBIDDEN_TOOLS[idx].to_string()
                }
                _ => arb_str(&mut u, 32),
            };
            let rest = arb_str(&mut u, 2048);
            let res = parse_tool_arguments_for_fuzz(&tool, &rest);
            if FORBIDDEN_TOOLS.contains(&tool.as_str()) {
                assert!(res.is_err(), "forbidden MCP tool {tool} must always be refused");
            }
        }
        2 => {
            // Structured actuate operation payload fuzzing (anti-spoof scope, forbidden bulk keys,
            // locator bounds, properties typing, all 6 operation variants).
            let op_name = match u.int_in_range(0..=7).unwrap_or(0) {
                0 => "put_canonical_node",
                1 => "put_derived_node",
                2 => "put_derived_edge",
                3 => "register_source_dependency",
                4 => "transition_lifecycle",
                5 => "put_latest_state",
                6 => "recover",
                _ => "unknown_op",
            };
            let mut op = serde_json::Map::new();
            op.insert("op".to_string(), serde_json::json!(op_name));
            let inject_scope = u.arbitrary().unwrap_or(false);
            if inject_scope {
                op.insert("scope".to_string(), serde_json::json!(arb_str(&mut u, 16)));
            }
            let inject_forbidden = u.arbitrary().unwrap_or(false);
            let forbidden_key = match u.int_in_range(0..=5).unwrap_or(0) {
                0 => "bulk",
                1 => "non_strict",
                2 => "endpoint_validation",
                3 => "allow_dangling_edges",
                4 => "raw_sql",
                _ => "sql",
            };
            if inject_forbidden {
                op.insert(forbidden_key.to_string(), serde_json::json!(true));
            }
            op.insert("kind".to_string(), serde_json::json!(arb_str(&mut u, 24)));
            op.insert("body".to_string(), serde_json::json!(arb_str(&mut u, 64)));
            op.insert("source_id".to_string(), serde_json::json!(arb_str(&mut u, 24)));
            op.insert("artifact_revision_id".to_string(), serde_json::json!(arb_str(&mut u, 24)));
            op.insert("source_version_id".to_string(), serde_json::json!(arb_str(&mut u, 24)));
            if op_name == "put_derived_node" || op_name == "put_derived_edge" {
                op.insert("source_revision_id".to_string(), serde_json::json!(arb_str(&mut u, 24)));
                op.insert(
                    "canonical_source_hash".to_string(),
                    serde_json::json!(arb_str(&mut u, 70)),
                );
                if u.arbitrary().unwrap_or(false) {
                    op.insert(
                        "locator".to_string(),
                        serde_json::json!({
                            "type": if u.arbitrary().unwrap_or(true) { "utf8_bytes" } else { "whole_body" },
                            "start_inclusive": u.arbitrary::<u64>().unwrap_or(0),
                            "end_exclusive": u.arbitrary::<u64>().unwrap_or(0),
                        }),
                    );
                }
            }
            if op_name == "put_derived_edge" {
                op.insert("from".to_string(), serde_json::json!(arb_str(&mut u, 24)));
                op.insert("to".to_string(), serde_json::json!(arb_str(&mut u, 24)));
            }
            if op_name == "put_latest_state" {
                op.clear();
                op.insert("op".to_string(), serde_json::json!("put_latest_state"));
                if inject_scope {
                    op.insert("scope".to_string(), serde_json::json!("spoof"));
                }
                if inject_forbidden {
                    op.insert(forbidden_key.to_string(), serde_json::json!(true));
                }
                op.insert("collection".to_string(), serde_json::json!(arb_str(&mut u, 24)));
                op.insert("record_key".to_string(), serde_json::json!(arb_str(&mut u, 24)));
                op.insert("payload_json".to_string(), serde_json::json!(arb_str(&mut u, 64)));
            }

            let payload = serde_json::json!({
                "operation_id": arb_str(&mut u, 32),
                "expected_write_boundary": u.arbitrary::<Option<u64>>().unwrap_or(None),
                "operations": [serde_json::Value::Object(op)],
            })
            .to_string();
            let res = parse_tool_arguments_for_fuzz("actuate", &payload);
            if inject_scope || inject_forbidden || op_name == "recover" {
                assert!(
                    res.is_err(),
                    "anti-spoof scope / forbidden bulk flag / forbidden op must always be refused"
                );
            }
        }
        _ => {
            // Structured search / fetch / expand arguments with boundary integers and flags.
            let payload = serde_json::json!({
                "store": if u.arbitrary().unwrap_or(false) { Some(arb_str(&mut u, 32)) } else { None },
                "query": arb_str(&mut u, 128),
                "limit": u.arbitrary::<Option<u64>>().unwrap_or(None),
                "max_tokens": u.arbitrary::<Option<u64>>().unwrap_or(None),
                "include_superseded": u.arbitrary().unwrap_or(false),
                "include_inactive": u.arbitrary().unwrap_or(false),
                "freeze": u.arbitrary().unwrap_or(false),
                "continuation": if u.arbitrary().unwrap_or(false) { Some(arb_str(&mut u, 64)) } else { None },
            })
            .to_string();
            let _ = parse_tool_arguments_for_fuzz("search", &payload);
        }
    }
});
