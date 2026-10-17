//! Slice 41 (0.8.28) phase 2 unit F — the `full_ids` full-render override.
//!
//! The fixture cases (`full_ids_cases`, `full_ids_errors` in
//! `src/conformance/context-pack-fixtures.json`) carry hand-derived expected
//! output and run here through the typed API and the JSON wire. The
//! properties pin AC41-32: an empty `full_ids` is phase 1, and forcing only
//! changes forced items and items selected after the first of them.

use std::collections::{HashMap, HashSet};

use fathomdb_engine::{
    pack_context, pack_context_json, ContextItem, ContextOrder, ContextPack, ContextPackEntry,
    ContextPart, ContextPolicy, ContextSource, ContextTier,
};
use proptest::prelude::*;
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
                text: p["text"].as_str().expect("text").to_string(),
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
    let Some(map) = v.as_object() else { return policy };
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
                policy.order = match value.as_str().expect("order") {
                    "rank" => ContextOrder::Rank,
                    "chronological" => ContextOrder::Chronological,
                    other => panic!("fixture order {other}"),
                }
            }
            other => panic!("fixture policy field {other}"),
        }
    }
    policy
}

fn entry_to_json(entry: &ContextPackEntry) -> Value {
    json!({
        "id": entry.id,
        "source": match entry.source { ContextSource::Hit => "hit", ContextSource::Context => "context" },
        "tier": match entry.tier {
            ContextTier::Full => "full",
            ContextTier::Compact => "compact",
            ContextTier::Omitted => "omitted",
        },
        "chars": entry.chars,
        "truncated": entry.truncated,
        "forced": entry.forced,
        "rendered_index": entry.rendered_index,
        "parts": entry.parts.iter().map(|p| json!({
            "role": p.role, "chars": p.chars, "of_chars": p.of_chars,
        })).collect::<Vec<_>>(),
    })
}

fn pack_to_json(pack: &ContextPack) -> Value {
    json!({
        "text": pack.text,
        "chars_used": pack.chars_used,
        "text_chars": pack.text_chars,
        "items": pack.items.iter().map(entry_to_json).collect::<Vec<_>>(),
    })
}

/// AC41-28, AC41-29, AC41-30: every `full_ids` case through the typed API and
/// the JSON wire equals the hand-derived expectation.
#[test]
fn every_full_ids_case_matches_through_typed_api_and_wire() {
    let cases = fixtures()["full_ids_cases"].as_array().expect("full_ids_cases").clone();
    assert_eq!(cases.len(), 4);
    for c in cases {
        let name = c["name"].as_str().expect("name");
        let r = &c["request"];
        let typed = pack_context(
            &items_from(&r["hits"]),
            &policy_from(&r["policy"]),
            &items_from(&r["context"]),
        )
        .unwrap_or_else(|e| panic!("case {name}: {e}"));
        assert_eq!(pack_to_json(&typed), c["expected"], "typed API, case {name}");
        let wire = pack_context_json(&r.to_string()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let wire: Value = serde_json::from_str(&wire).expect("wire JSON");
        assert_eq!(wire, c["expected"], "JSON wire, case {name}");
    }
}

/// AC41-31: unknown, repeated and empty ids are refused naming
/// `policy.full_ids[i]`; a REQ41-P8 refusal wins over `full_ids`.
#[test]
fn invalid_full_ids_are_refused_naming_the_field() {
    for e in fixtures()["full_ids_errors"].as_array().expect("full_ids_errors") {
        let name = e["name"].as_str().expect("name");
        let r = &e["request"];
        let contains = e["message_contains"].as_str().expect("message_contains");
        let wire = pack_context_json(&r.to_string()).expect_err(name);
        assert!(wire.message().contains(contains), "{name}: {}", wire.message());
        if let Some(excluded) = e["message_excludes"].as_str() {
            assert!(!wire.message().contains(excluded), "{name}: {}", wire.message());
        }
        if e["typed"] == true {
            let typed = pack_context(
                &items_from(&r["hits"]),
                &policy_from(&r["policy"]),
                &items_from(&r["context"]),
            )
            .expect_err(name);
            assert_eq!(typed.message(), wire.message(), "{name}");
        }
    }
}

/// The default policy forces nothing.
#[test]
fn default_full_ids_is_empty() {
    assert!(ContextPolicy::default().full_ids.is_empty());
}

// --- properties ---------------------------------------------------------------

const ALPHABET: &[&str] = &["a", "b", " ", "\n", "é", "😀"];
const ROLES: &[Option<&str>] = &[None, Some("user"), Some("assistant"), Some("tool")];

fn text_strategy() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::sample::select(ALPHABET), 0..12).prop_map(|v| v.concat())
}

fn parts_strategy() -> impl Strategy<Value = Vec<ContextPart>> {
    prop::collection::vec(
        (prop::sample::select(ROLES), text_strategy())
            .prop_map(|(role, text)| ContextPart { role: role.map(str::to_string), text }),
        0..4,
    )
}

#[derive(Debug, Clone)]
struct Request {
    hits: Vec<ContextItem>,
    context: Vec<ContextItem>,
    policy: ContextPolicy,
    /// One flag per input item (hits then context): list it in `full_ids`.
    force: Vec<bool>,
}

fn request_strategy() -> impl Strategy<Value = Request> {
    let groups = prop::option::of(prop::sample::select(vec!["g0", "g1", "g2"]));
    let hit = (parts_strategy(), groups.clone());
    let ctx = (parts_strategy(), groups);
    let policy = (1usize..60, 1usize..30, 0usize..5, 0usize..8, any::<bool>(), any::<bool>());
    (
        prop::collection::vec(hit, 0..6),
        prop::collection::vec(ctx, 0..8),
        policy,
        prop::collection::vec(any::<bool>(), 14),
    )
        .prop_map(|(hits, ctx, (budget, item, full, compact, expand, chrono), force)| {
            let hits: Vec<ContextItem> = hits
                .into_iter()
                .enumerate()
                .map(|(i, (parts, group))| ContextItem {
                    id: format!("h{i}"),
                    parts,
                    group: group.map(str::to_string),
                    ..ContextItem::default()
                })
                .collect();
            let mut next: HashMap<Option<&str>, u64> = HashMap::new();
            let context: Vec<ContextItem> = ctx
                .into_iter()
                .enumerate()
                .map(|(i, (parts, group))| {
                    let slot = next.entry(group).or_insert(0);
                    let position = *slot;
                    *slot += 1;
                    ContextItem {
                        id: format!("c{i}"),
                        parts,
                        group: group.map(str::to_string),
                        position: Some(position),
                        order_key: Some(format!("2023/01/0{}", i % 3)),
                        ..ContextItem::default()
                    }
                })
                .collect();
            let force = force[..hits.len() + context.len()].to_vec();
            Request {
                hits,
                context,
                policy: ContextPolicy {
                    budget_chars: budget,
                    item_chars: item,
                    full_items: full,
                    compact_chars: compact,
                    expand_groups: expand,
                    order: if chrono { ContextOrder::Chronological } else { ContextOrder::Rank },
                    ..ContextPolicy::default()
                },
                force,
            }
        })
}

/// Selection order as input indices (hits first, context at `hits.len() + j`),
/// written from REQ41-P3: hits in order, then each hit group's context items
/// by position.
fn selection(req: &Request) -> Vec<usize> {
    let mut out: Vec<usize> = (0..req.hits.len()).collect();
    if req.policy.expand_groups {
        let mut groups: Vec<&str> = Vec::new();
        for hit in &req.hits {
            if let Some(group) = hit.group.as_deref() {
                if !groups.contains(&group) {
                    groups.push(group);
                }
            }
        }
        for group in groups {
            let mut members: Vec<usize> = (0..req.context.len())
                .filter(|&j| req.context[j].group.as_deref() == Some(group))
                .collect();
            members.sort_by_key(|&j| req.context[j].position);
            out.extend(members.into_iter().map(|j| req.hits.len() + j));
        }
    }
    out
}

fn ids(req: &Request) -> Vec<String> {
    req.hits.iter().chain(req.context.iter()).map(|item| item.id.clone()).collect()
}

fn pack_with(req: &Request, full_ids: Vec<String>) -> ContextPack {
    let policy = ContextPolicy { full_ids, ..req.policy.clone() };
    pack_context(&req.hits, &policy, &req.context).expect("valid request")
}

type Shape = (ContextTier, usize, bool, Vec<(usize, usize)>);

fn shape(entry: &ContextPackEntry) -> Shape {
    (
        entry.tier,
        entry.chars,
        entry.truncated,
        entry.parts.iter().map(|p| (p.chars, p.of_chars)).collect(),
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// AC41-32, first clause: with `full_ids` empty nothing is forced, and
    /// listing only ids that cannot change the outcome (selected inside
    /// `full_items`, or not selected) gives the same pack.
    #[test]
    fn prop_empty_full_ids_is_phase_one(req in request_strategy()) {
        let base = pack_with(&req, Vec::new());
        prop_assert!(base.items.iter().all(|entry| !entry.forced));
        let sel = selection(&req);
        let all = ids(&req);
        let selected: HashSet<usize> = sel.iter().copied().collect();
        let no_effect: Vec<String> = (0..all.len())
            .filter(|&g| match sel.iter().position(|&s| s == g) {
                Some(index) => index < req.policy.full_items,
                None => !selected.contains(&g),
            })
            .map(|g| all[g].clone())
            .collect();
        prop_assert_eq!(pack_with(&req, no_effect), base);
    }

    /// AC41-32, second clause, against the same request without `full_ids`:
    /// (i) the first effectively forced item renders at least as many chars;
    /// (ii) every item selected before it, which includes every index below
    /// `full_items`, is unchanged; (iii) so the first non-forced item that
    /// changes is selected after the first forced item. `forced` is true
    /// exactly for rendered forced items past `full_items`, which are full.
    #[test]
    fn prop_forcing_changes_only_forced_and_later_items(req in request_strategy()) {
        let all = ids(&req);
        let full_ids: Vec<String> =
            (0..all.len()).filter(|&g| req.force[g]).map(|g| all[g].clone()).collect();
        let forced_ids: HashSet<&str> = full_ids.iter().map(String::as_str).collect();
        let base = pack_with(&req, Vec::new());
        let forced = pack_with(&req, full_ids.clone());
        let sel = selection(&req);
        let effective: Vec<usize> = sel
            .iter()
            .enumerate()
            .filter(|(index, g)| *index >= req.policy.full_items && forced_ids.contains(all[**g].as_str()))
            .map(|(index, _)| index)
            .collect();
        let Some(&first) = effective.first() else {
            prop_assert_eq!(forced, base);
            return Ok(());
        };
        let g_first = sel[first];
        prop_assert!(forced.items[g_first].chars >= base.items[g_first].chars);
        for &g in &sel[..first] {
            prop_assert_eq!(shape(&forced.items[g]), shape(&base.items[g]));
            prop_assert!(!forced.items[g].forced);
        }
        let selected: HashSet<usize> = sel.iter().copied().collect();
        for (g, entry) in forced.items.iter().enumerate() {
            if !selected.contains(&g) {
                prop_assert_eq!(entry.tier, ContextTier::Omitted);
                prop_assert!(!entry.forced);
                continue;
            }
            let index = sel.iter().position(|&s| s == g).expect("selected");
            let expect_forced = forced_ids.contains(all[g].as_str())
                && index >= req.policy.full_items
                && entry.tier != ContextTier::Omitted;
            prop_assert_eq!(entry.forced, expect_forced, "item {}", all[g].clone());
            if expect_forced {
                prop_assert_eq!(entry.tier, ContextTier::Full);
            }
        }
        prop_assert!(forced.chars_used <= req.policy.budget_chars);
    }
}
