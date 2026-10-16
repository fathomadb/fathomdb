//! Slice 41 (0.8.28) AC41-20/21 — the Rust SDK context pack.
//!
//! The shared, hand-derived fixture runs through `fathomdb_sdk` alone (this
//! consumer never names `fathomdb_engine`), so the Rust SDK matches the Python
//! and TypeScript suites field for field. Errors are `InvalidArgument` with a
//! message naming the field.

use fathomdb_sdk::{
    context_item_from_hit, decode_round_body, encode_round_body, pack_context, ContextHitOptions,
    ContextItem, ContextOrder, ContextPack, ContextPart, ContextPolicy, ContextSource, ContextTier,
    Engine, ErrorKind, InitialState, OpenOptions, PreparedWrite, SourceId, TextSearchOptions,
};
use serde_json::{json, Value};

const FIXTURES: &str = include_str!("../../../../conformance/context-pack-fixtures.json");

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).expect("fixture JSON")
}

fn opt_str(v: &Value) -> Option<String> {
    v.as_str().map(str::to_string)
}

fn parts_from(v: &Value) -> Vec<ContextPart> {
    v.as_array()
        .map(|a| {
            a.iter()
                .map(|p| ContextPart {
                    role: opt_str(&p["role"]),
                    text: p["text"].as_str().expect("text").into(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn item_from(v: &Value) -> ContextItem {
    ContextItem {
        id: v["id"].as_str().expect("id").to_string(),
        parts: parts_from(&v["parts"]),
        group: opt_str(&v["group"]),
        position: v["position"].as_u64(),
        order_key: opt_str(&v["order_key"]),
        header: opt_str(&v["header"]),
    }
}

fn items_from(v: &Value) -> Vec<ContextItem> {
    v.as_array().map(|a| a.iter().map(item_from).collect()).unwrap_or_default()
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
                "keep_roles" => {
                    policy.keep_roles = value
                        .as_array()
                        .expect("keep_roles")
                        .iter()
                        .map(|r| r.as_str().expect("role").to_string())
                        .collect()
                }
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
            "rendered_index": e.rendered_index,
            "parts": e.parts.iter().map(|p| json!({"role": p.role, "chars": p.chars, "of_chars": p.of_chars})).collect::<Vec<_>>(),
        })).collect::<Vec<_>>(),
    })
}

/// AC41-21: every fixture case, through the Rust SDK.
#[test]
fn every_fixture_case_matches_through_the_rust_sdk() {
    for c in fixtures()["pack_cases"].as_array().expect("cases") {
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

/// AC41-21: the SDK default policy is the documented product default.
#[test]
fn default_policy_matches_the_documented_defaults() {
    let d = fixtures()["defaults"].clone();
    assert_eq!(ContextPolicy::default(), policy_from(&d));
    assert_eq!(ContextPolicy::default().order, ContextOrder::Rank);
}

/// AC41-11: typed-expressible invalid inputs are `InvalidArgument` naming the
/// field.
#[test]
fn invalid_inputs_are_invalid_argument_naming_the_field() {
    for e in fixtures()["pack_errors"].as_array().expect("errors") {
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
    }
    for e in fixtures()["encode_errors"].as_array().expect("encode") {
        let err = encode_round_body(&parts_from(&e["parts"])).expect_err("refused");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);
        assert!(err.to_string().contains(e["message_contains"].as_str().expect("n")), "{err}");
    }
    for e in fixtures()["decode_errors"].as_array().expect("decode") {
        let err = decode_round_body(e["body"].as_str().expect("body")).expect_err("refused");
        assert_eq!(err.kind(), ErrorKind::InvalidArgument);
    }
}

/// AC41-16 through the SDK: the hand-written round bodies.
#[test]
fn round_body_fixtures_through_the_rust_sdk() {
    for c in fixtures()["round_bodies"].as_array().expect("bodies") {
        let parts = parts_from(&c["parts"]);
        let body = encode_round_body(&parts).expect("encode");
        assert_eq!(body, c["body"].as_str().expect("body"));
        assert_eq!(decode_round_body(&body).expect("decode"), parts);
    }
}

fn node(logical_id: &str, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "round".to_string(),
        body,
        source_id: SourceId::new("session-7").expect("source id"),
        logical_id: Some(logical_id.to_string()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

/// AC41-20: a tagged hit decodes into parts; an untagged hit is one role-none
/// part; the item id is the hit's prefixed id; a malformed tagged body raises.
#[test]
fn context_item_from_hit_decodes_tagged_and_keeps_untagged() {
    let dir = tempfile::tempdir().expect("tempdir");
    let engine =
        Engine::open(dir.path().join("pack.sqlite"), OpenOptions::default()).expect("open");
    let tagged = encode_round_body(&[
        ContextPart { role: Some("user".into()), text: "my cat is named tom".into() },
        ContextPart { role: Some("assistant".into()), text: "tom is a fine name".into() },
    ])
    .expect("encode");
    engine
        .write(&[
            node("round:1", tagged),
            node("round:2", "plain kettle note".to_string()),
            node("round:3", "\u{1e}Xbroken kazoo".to_string()),
        ])
        .expect("write");
    engine.drain(10_000).expect("drain");

    let hit = engine.search_text_only("cat", TextSearchOptions::default()).expect("search").results;
    assert_eq!(hit.len(), 1);
    let item = context_item_from_hit(
        &hit[0],
        ContextHitOptions {
            group: Some("session-7".into()),
            position: Some(1),
            order_key: Some("2023/01/02".into()),
            header: Some("[Session date: 2023/01/02]".into()),
        },
    )
    .expect("from hit");
    assert_eq!(item.id, "l:round:1");
    assert_eq!(
        item.parts,
        vec![
            ContextPart { role: Some("user".into()), text: "my cat is named tom".into() },
            ContextPart { role: Some("assistant".into()), text: "tom is a fine name".into() },
        ]
    );
    assert_eq!((item.group.as_deref(), item.position), (Some("session-7"), Some(1)));
    assert_eq!(item.header.as_deref(), Some("[Session date: 2023/01/02]"));
    let policy = ContextPolicy { full_items: 0, compact_chars: 3, ..ContextPolicy::default() };
    let pack = pack_context(&[item], &policy, &[]).expect("pack");
    assert_eq!(pack.text, "[Session date: 2023/01/02]\nmy cat is named tom\ntom");

    let plain =
        engine.search_text_only("kettle", TextSearchOptions::default()).expect("search").results;
    let item = context_item_from_hit(&plain[0], ContextHitOptions::default()).expect("untagged");
    assert_eq!(item.parts, vec![ContextPart { role: None, text: "plain kettle note".into() }]);
    let pack = pack_context(&[item], &policy, &[]).expect("pack");
    assert_eq!(pack.text, "plain kettle note");

    let broken =
        engine.search_text_only("kazoo", TextSearchOptions::default()).expect("search").results;
    let err =
        context_item_from_hit(&broken[0], ContextHitOptions::default()).expect_err("malformed");
    assert_eq!(err.kind(), ErrorKind::InvalidArgument);
    engine.close().expect("close");
}
