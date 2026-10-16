//! Slice 41 (0.8.28) phase 2 AC41-31/33 — `full_ids` through the Rust SDK.
//!
//! The shared, hand-derived `full_ids_cases` and `full_ids_errors` run through
//! `fathomdb_sdk` alone, field for field including `forced`, as the Python and
//! TypeScript suites do.

use fathomdb_sdk::{
    pack_context, ContextItem, ContextOrder, ContextPack, ContextPart, ContextPolicy,
    ContextSource, ContextTier, ErrorKind,
};
use serde_json::{json, Value};

const FIXTURES: &str = include_str!("../../../../conformance/context-pack-fixtures.json");

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).expect("fixture JSON")
}

fn opt_str(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

fn item_from(v: &Value) -> ContextItem {
    ContextItem {
        id: v["id"].as_str().expect("id").to_string(),
        parts: v["parts"]
            .as_array()
            .expect("parts")
            .iter()
            .map(|p| ContextPart {
                role: opt_str(&p["role"]),
                text: p["text"].as_str().expect("text").into(),
            })
            .collect(),
        group: opt_str(&v["group"]),
        position: v["position"].as_u64(),
        order_key: opt_str(&v["order_key"]),
        header: opt_str(&v["header"]),
    }
}

fn items_from(v: &Value) -> Vec<ContextItem> {
    v.as_array().map(|a| a.iter().map(item_from).collect()).unwrap_or_default()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().expect("array").iter().map(|s| s.as_str().expect("string").to_string()).collect()
}

fn policy_from(v: &Value) -> ContextPolicy {
    let mut policy = ContextPolicy::default();
    if let Some(map) = v.as_object() {
        for (key, value) in map {
            let n = || value.as_u64().expect("count") as usize;
            match key.as_str() {
                "budget_chars" => policy.budget_chars = n(),
                "item_chars" => policy.item_chars = n(),
                "full_items" => policy.full_items = n(),
                "compact_chars" => policy.compact_chars = n(),
                "keep_roles" => policy.keep_roles = strings(value),
                "full_ids" => policy.full_ids = strings(value),
                "expand_groups" => policy.expand_groups = value.as_bool().expect("bool"),
                "order" => {
                    policy.order = if value == "chronological" {
                        ContextOrder::Chronological
                    } else {
                        ContextOrder::Rank
                    }
                }
                other => panic!("fixture policy field {other}"),
            }
        }
    }
    policy
}

fn to_json(pack: &ContextPack) -> Value {
    json!({
        "text": pack.text,
        "chars_used": pack.chars_used,
        "text_chars": pack.text_chars,
        "items": pack.items.iter().map(|e| json!({
            "id": e.id,
            "source": match e.source { ContextSource::Hit => "hit", ContextSource::Context => "context" },
            "tier": match e.tier {
                ContextTier::Full => "full",
                ContextTier::Compact => "compact",
                ContextTier::Omitted => "omitted",
            },
            "chars": e.chars,
            "truncated": e.truncated,
            "forced": e.forced,
            "rendered_index": e.rendered_index,
            "parts": e.parts.iter().map(|p| json!({"role": p.role, "chars": p.chars, "of_chars": p.of_chars})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// AC41-33: every `full_ids` case through the Rust SDK.
#[test]
fn every_full_ids_case_matches_through_the_rust_sdk() {
    let cases = fixtures()["full_ids_cases"].as_array().expect("full_ids_cases").clone();
    assert!(!cases.is_empty());
    for c in cases {
        let r = &c["request"];
        let pack = pack_context(
            &items_from(&r["hits"]),
            &policy_from(&r["policy"]),
            &items_from(&r["context"]),
        )
        .unwrap_or_else(|e| panic!("case {}: {e}", c["name"]));
        assert_eq!(to_json(&pack), c["expected"], "case {}", c["name"]);
    }
}

/// AC41-31: typed `full_ids` refusals are `InvalidArgument` naming the field.
#[test]
fn invalid_full_ids_are_invalid_argument_naming_the_field() {
    for e in fixtures()["full_ids_errors"].as_array().expect("full_ids_errors") {
        if e["typed"] != true {
            continue;
        }
        let r = &e["request"];
        let err = pack_context(
            &items_from(&r["hits"]),
            &policy_from(&r["policy"]),
            &items_from(&r["context"]),
        )
        .expect_err("must be refused");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument, "case {}", e["name"]);
        let needle = e["message_contains"].as_str().expect("needle");
        assert!(err.to_string().contains(needle), "case {}: {err}", e["name"]);
        if let Some(excluded) = e["message_excludes"].as_str() {
            assert!(!err.to_string().contains(excluded), "case {}: {err}", e["name"]);
        }
    }
}
