//! Slice 41 (0.8.28) — context pack and the role-tagged round body.
//!
//! The fixture cases (`src/conformance/context-pack-fixtures.json`) carry
//! hand-derived expected output; they run here through both the typed API and
//! the JSON wire the bindings use, and through the Python and TypeScript SDKs
//! in their own suites. The properties pin the budget, monotonicity, ordering
//! and round-trip invariants. The engine tests check that a tagged corpus
//! ranks exactly like its plain twin (no mocking: real databases).

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use fathomdb_embedder::NoopEmbedder;
use fathomdb_engine::{
    decode_round_body, decode_round_body_json, encode_round_body, encode_round_body_json,
    pack_context, pack_context_json, ContextItem, ContextOrder, ContextPack, ContextPackEntry,
    ContextPart, ContextPolicy, ContextSource, ContextTier, EmbedderChoice, Engine, InitialState,
    PreparedWrite, SourceId,
};
use fathomdb_schema::SQLITE_SUFFIX;
use proptest::prelude::*;
use serde_json::{json, Value};
use tempfile::TempDir;

const FIXTURES: &str = include_str!("../../../../conformance/context-pack-fixtures.json");

fn fixtures() -> Value {
    serde_json::from_str(FIXTURES).expect("fixture JSON")
}

fn case(name: &str) -> Value {
    fixtures()["pack_cases"]
        .as_array()
        .expect("pack_cases")
        .iter()
        .find(|c| c["name"] == name)
        .unwrap_or_else(|| panic!("no fixture case {name}"))
        .clone()
}

// --- test-side decoding of fixture requests into the typed API ----------------

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
            "keep_roles" => {
                policy.keep_roles = value
                    .as_array()
                    .expect("keep_roles")
                    .iter()
                    .map(|r| r.as_str().expect("role").to_string())
                    .collect();
            }
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

fn typed_request(request: &Value) -> (Vec<ContextItem>, ContextPolicy, Vec<ContextItem>) {
    (items_from(&request["hits"]), policy_from(&request["policy"]), items_from(&request["context"]))
}

fn pack_to_json(pack: &ContextPack) -> Value {
    json!({
        "text": pack.text,
        "chars_used": pack.chars_used,
        "text_chars": pack.text_chars,
        "items": pack.items.iter().map(entry_to_json).collect::<Vec<_>>(),
    })
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
        "rendered_index": entry.rendered_index,
        "parts": entry.parts.iter().map(|p| json!({
            "role": p.role, "chars": p.chars, "of_chars": p.of_chars,
        })).collect::<Vec<_>>(),
    })
}

/// Run one fixture case through the typed API and the JSON wire; both must
/// equal the hand-derived expectation exactly.
fn assert_case(name: &str) {
    let c = case(name);
    let (hits, policy, context) = typed_request(&c["request"]);
    let typed = pack_context(&hits, &policy, &context).expect("typed pack_context");
    assert_eq!(pack_to_json(&typed), c["expected"], "typed API, case {name}");
    let wire = pack_context_json(&c["request"].to_string()).expect("pack_context_json");
    let wire: Value = serde_json::from_str(&wire).expect("wire JSON");
    assert_eq!(wire, c["expected"], "JSON wire, case {name}");
}

// --- fixture cases ------------------------------------------------------------

/// AC41-01: the top tier renders every part; compact items keep the user part
/// whole and cut the rest to `compact_chars`.
#[test]
fn top_tier_full_rest_compact_keeps_user_text() {
    assert_case("tiers_basic");
}

/// AC41-02: non-kept parts share one allowance in part order.
#[test]
fn compact_allowance_is_shared_across_non_kept_parts_in_order() {
    assert_case("shared_allowance_and_empty_parts");
}

/// AC41-02 (R0 F6): an empty non-kept part is dropped; an empty kept part and
/// an empty full-tier part stay.
#[test]
fn empty_non_kept_part_is_dropped_but_empty_kept_and_full_parts_stay() {
    assert_case("shared_allowance_and_empty_parts");
    assert_case("full_tier_keeps_empty_parts");
}

/// AC41-02: `full_items = 0` renders every item compact.
#[test]
fn zero_full_items_renders_every_item_compact() {
    let hits: Vec<ContextItem> = (0..4)
        .map(|i| ContextItem {
            id: format!("h{i}"),
            parts: vec![
                ContextPart { role: Some("user".into()), text: format!("q{i}") },
                ContextPart { role: Some("assistant".into()), text: "answer".into() },
            ],
            ..ContextItem::default()
        })
        .collect();
    let policy = ContextPolicy { full_items: 0, compact_chars: 2, ..ContextPolicy::default() };
    let pack = pack_context(&hits, &policy, &[]).expect("pack");
    assert!(pack.items.iter().all(|e| e.tier == ContextTier::Compact), "{:?}", pack.items);
    assert_eq!(pack.text, "q0\nan\n\nq1\nan\n\nq2\nan\n\nq3\nan");
}

/// AC41-03: tiers follow the selection index into the expansion items.
#[test]
fn tiers_follow_selection_index_into_expansion() {
    assert_case("tiers_follow_selection_index");
}

/// AC41-04: the last item is cut to the remaining budget, later items are
/// omitted, and headers are not charged.
#[test]
fn budget_cuts_last_item_and_omits_the_rest() {
    assert_case("budget_cut_with_headers");
}

/// AC41-04: an item that spends the budget exactly leaves the next
/// whitespace-only item omitted.
#[test]
fn exactly_spent_budget_omits_following_whitespace_item() {
    assert_case("exact_budget_then_whitespace_item");
}

/// AC41-05: `item_chars` caps a full-tier item; joins count in the item, not
/// in a part.
#[test]
fn item_chars_caps_full_tier_and_sets_truncated() {
    assert_case("item_chars_cap");
    assert_case("item_chars_cut_at_join");
}

/// AC41-06: groups expand in order of first hit; members by position, ties by
/// input order; expansion off selects hits only.
#[test]
fn expansion_orders_groups_by_first_hit_and_members_by_position() {
    assert_case("expansion_order");
    assert_case("expansion_off");
}

/// AC41-07: chronological order sorts without changing tiers or cuts.
#[test]
fn chronological_order_sorts_without_changing_cuts() {
    assert_case("chronological_order");
    let c = case("chronological_order");
    let (hits, mut policy, context) = typed_request(&c["request"]);
    let chrono = pack_context(&hits, &policy, &context).expect("chronological");
    policy.order = ContextOrder::Rank;
    let rank = pack_context(&hits, &policy, &context).expect("rank");
    for (a, b) in chrono.items.iter().zip(&rank.items) {
        assert_eq!((a.tier, a.chars, &a.parts), (b.tier, b.chars, &b.parts), "item {}", a.id);
    }
    assert_eq!(chrono.chars_used, rank.chars_used);
}

/// AC41-08: a whitespace-only rendered item is omitted, spends nothing, and
/// still takes its selection slot.
#[test]
fn whitespace_only_item_is_omitted_and_keeps_its_slot() {
    assert_case("whitespace_item_keeps_its_slot");
}

/// AC41-09: one provenance entry per input item, hits then context in input
/// order; `rendered_index` numbers the rendered items 0..n.
#[test]
fn provenance_has_one_entry_per_input_in_input_order() {
    assert_case("nothing_to_render");
    for c in fixtures()["pack_cases"].as_array().expect("cases") {
        let (hits, policy, context) = typed_request(&c["request"]);
        let pack = pack_context(&hits, &policy, &context).expect("pack");
        let input: Vec<&str> =
            hits.iter().chain(context.iter()).map(|item| item.id.as_str()).collect();
        let got: Vec<&str> = pack.items.iter().map(|e| e.id.as_str()).collect();
        assert_eq!(got, input, "case {}", c["name"]);
        let mut rendered: Vec<usize> = pack.items.iter().filter_map(|e| e.rendered_index).collect();
        rendered.sort_unstable();
        assert_eq!(rendered, (0..rendered.len()).collect::<Vec<_>>(), "case {}", c["name"]);
        for e in &pack.items {
            assert_eq!(e.rendered_index.is_some(), e.tier != ContextTier::Omitted, "{}", e.id);
            let source = if hits.iter().any(|h| h.id == e.id) {
                ContextSource::Hit
            } else {
                ContextSource::Context
            };
            assert_eq!(e.source, source, "{}", e.id);
        }
    }
}

/// AC41-10: counts and cuts are Unicode scalar values.
#[test]
fn cuts_count_scalar_values_on_astral_and_combining_text() {
    assert_case("scalar_values_on_non_ascii_text");
    let c = case("scalar_values_on_non_ascii_text");
    let text = c["expected"]["text"].as_str().expect("text");
    assert_eq!(text.chars().count(), 17);
    assert_eq!(text.encode_utf16().count() as u64, c["utf16_length"].as_u64().expect("utf16"));
}

/// AC41-12: a large `full_items` with expansion and chronological order is
/// the Slice 40 renderer; expected text derived by hand.
#[test]
fn large_full_items_matches_hand_derived_slice40_text() {
    assert_case("slice40_renderer_shape");
}

/// AC41-20 (core part): role `None` is always kept; `keep_roles` is honoured.
#[test]
fn role_none_and_custom_keep_roles() {
    assert_case("role_none_and_custom_keep_roles");
}

/// AC41-11: every invalid input is refused with a message naming the field,
/// through the JSON wire and, where expressible, the typed API.
#[test]
fn invalid_inputs_raise_invalid_argument_naming_the_field() {
    for e in fixtures()["pack_errors"].as_array().expect("pack_errors") {
        let needle = e["message_contains"].as_str().expect("needle");
        let err = pack_context_json(&e["request"].to_string())
            .expect_err(&format!("case {} must be refused", e["name"]));
        assert!(err.message().contains(needle), "case {}: {:?} lacks {needle}", e["name"], err);
        if e["typed"] == true {
            let (hits, policy, context) = typed_request(&e["request"]);
            let err = pack_context(&hits, &policy, &context)
                .expect_err(&format!("typed case {} must be refused", e["name"]));
            assert!(err.message().contains(needle), "typed {}: {:?}", e["name"], err);
        }
    }
    let err = pack_context_json("{not json").expect_err("malformed JSON");
    assert!(!err.message().is_empty());
    let err = pack_context_json(r#"{"hits": [{"id": "\ud800", "parts": []}]}"#)
        .expect_err("a lone surrogate escape is refused");
    assert!(!err.message().is_empty());
}

// --- round body ---------------------------------------------------------------

fn parts_from(v: &Value) -> Vec<ContextPart> {
    v.as_array()
        .expect("parts")
        .iter()
        .map(|p| ContextPart {
            role: opt_str(&p["role"]),
            text: p["text"].as_str().expect("t").into(),
        })
        .collect()
}

fn strip_markers(body: &str) -> String {
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == '\u{1e}' {
            chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// AC41-16 (fixtures): hand-written encodings; decode inverts; markers
/// removed give the plain join; untagged bodies decode to one role-none part.
#[test]
fn round_body_fixtures_encode_and_decode() {
    let f = fixtures();
    for c in f["round_bodies"].as_array().expect("round_bodies") {
        let parts = parts_from(&c["parts"]);
        let body = encode_round_body(&parts).expect("encode");
        assert_eq!(body, c["body"].as_str().expect("body"), "case {}", c["name"]);
        assert_eq!(decode_round_body(&body).expect("decode"), parts, "case {}", c["name"]);
        assert_eq!(strip_markers(&body), c["plain"].as_str().expect("plain"), "case {}", c["name"]);
        let wire = encode_round_body_json(&c["parts"].to_string()).expect("encode json");
        assert_eq!(wire, body, "case {}", c["name"]);
        let wire_body = serde_json::to_string(&body).expect("body JSON");
        let decoded: Value =
            serde_json::from_str(&decode_round_body_json(&wire_body).expect("dj")).unwrap();
        assert_eq!(decoded, c["parts"], "case {}", c["name"]);
    }
    for c in f["untagged_bodies"].as_array().expect("untagged") {
        let body = c["body"].as_str().expect("body");
        assert_eq!(decode_round_body(body).expect("decode"), parts_from(&c["parts"]));
    }
}

/// AC41-17: a part text with a marker or role code, an unknown role and an
/// empty part list are refused.
#[test]
fn encode_refuses_marker_characters_in_text() {
    for e in fixtures()["encode_errors"].as_array().expect("encode_errors") {
        let needle = e["message_contains"].as_str().expect("needle");
        let err = encode_round_body(&parts_from(&e["parts"]))
            .expect_err(&format!("case {} must be refused", e["name"]));
        assert!(err.message().contains(needle), "case {}: {:?}", e["name"], err);
        let err = encode_round_body_json(&e["parts"].to_string()).expect_err("json refuses");
        assert!(err.message().contains(needle), "json case {}: {:?}", e["name"], err);
    }
}

/// AC41-17: a body that starts with U+001E but does not decode is refused.
#[test]
fn decode_refuses_malformed_tagged_body() {
    for e in fixtures()["decode_errors"].as_array().expect("decode_errors") {
        let needle = e["message_contains"].as_str().expect("needle");
        let body = e["body"].as_str().expect("body");
        let err =
            decode_round_body(body).expect_err(&format!("case {} must be refused", e["name"]));
        assert!(err.message().contains(needle), "case {}: {:?}", e["name"], err);
        let wire_body = serde_json::to_string(body).expect("body JSON");
        assert!(decode_round_body_json(&wire_body).is_err(), "json case {}", e["name"]);
    }
}

// --- properties -----------------------------------------------------------------

const ALPHABET: &[&str] = &["a", "b", "Z", " ", "\n", "\t", "é", "e\u{301}", "日", "😀", "🇫"];
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
}

fn request_strategy() -> impl Strategy<Value = Request> {
    let groups = prop::option::of(prop::sample::select(vec!["g0", "g1", "g2"]));
    let hit = (
        parts_strategy(),
        groups.clone(),
        prop::option::of(0u64..5),
        prop::option::of(prop::sample::select(vec!["2023/01/01", "2023/01/02", "2023/01/03"])),
        prop::option::of(prop::sample::select(vec!["[H]", "[Session date: x]"])),
    );
    let ctx = (
        parts_strategy(),
        groups,
        prop::option::of(prop::sample::select(vec!["2023/01/01", "2023/01/02"])),
    );
    let policy =
        (1usize..60, 1usize..30, 0usize..5, 0usize..8, any::<bool>(), any::<bool>(), any::<bool>());
    (prop::collection::vec(hit, 0..6), prop::collection::vec(ctx, 0..8), policy).prop_map(
        |(hits, ctx, (budget, item, full, compact, expand, chrono, keep_assistant))| {
            let hits = hits
                .into_iter()
                .enumerate()
                .map(|(i, (parts, group, position, order_key, header))| ContextItem {
                    id: format!("h{i}"),
                    parts,
                    group: group.map(str::to_string),
                    position,
                    order_key: order_key.map(str::to_string),
                    header: header.map(str::to_string),
                })
                .collect();
            // Distinct positions per group (R0 F10): position = index within group.
            let mut next: HashMap<Option<&str>, u64> = HashMap::new();
            let context = ctx
                .into_iter()
                .enumerate()
                .map(|(i, (parts, group, order_key))| {
                    let slot = next.entry(group).or_insert(0);
                    let position = *slot;
                    *slot += 1;
                    ContextItem {
                        id: format!("c{i}"),
                        parts,
                        group: group.map(str::to_string),
                        position: Some(position),
                        order_key: order_key.map(str::to_string),
                        header: None,
                    }
                })
                .collect();
            let mut keep_roles = vec!["user".to_string()];
            if keep_assistant {
                keep_roles.push("assistant".to_string());
            }
            Request {
                hits,
                context,
                policy: ContextPolicy {
                    budget_chars: budget,
                    item_chars: item,
                    full_items: full,
                    compact_chars: compact,
                    keep_roles,
                    expand_groups: expand,
                    order: if chrono { ContextOrder::Chronological } else { ContextOrder::Rank },
                },
            }
        },
    )
}

fn header_cost(pack: &ContextPack, req: &Request) -> usize {
    let headers: HashMap<&str, &Option<String>> = req
        .hits
        .iter()
        .chain(req.context.iter())
        .map(|item| (item.id.as_str(), &item.header))
        .collect();
    pack.items
        .iter()
        .filter(|e| e.rendered_index.is_some())
        .map(|e| headers[e.id.as_str()].as_ref().map_or(0, |h| h.chars().count() + 1))
        .sum()
}

proptest! {
    /// AC41-13: the budget bounds item text, and the text length is exactly
    /// chars plus header lines plus blank-line separators.
    #[test]
    fn prop_chars_used_within_budget_and_text_chars_formula(req in request_strategy()) {
        let pack = pack_context(&req.hits, &req.policy, &req.context).expect("pack");
        prop_assert!(pack.chars_used <= req.policy.budget_chars);
        prop_assert_eq!(pack.chars_used, pack.items.iter().map(|e| e.chars).sum::<usize>());
        prop_assert_eq!(pack.text_chars, pack.text.chars().count());
        let rendered = pack.items.iter().filter(|e| e.rendered_index.is_some()).count();
        let expected = if rendered == 0 {
            0
        } else {
            pack.chars_used + header_cost(&pack, &req) + 2 * (rendered - 1)
        };
        prop_assert_eq!(pack.text_chars, expected);
        for e in &pack.items {
            prop_assert!(e.chars <= req.policy.item_chars);
            if e.tier == ContextTier::Omitted {
                prop_assert_eq!(e.chars, 0);
                prop_assert!(!e.truncated);
            }
        }
    }

    /// AC41-14: raising the budget never removes a rendered item or shortens a
    /// rendered part.
    #[test]
    fn prop_raising_budget_never_removes_rendered_text(req in request_strategy(), extra in 1usize..40) {
        let small = pack_context(&req.hits, &req.policy, &req.context).expect("small");
        let mut bigger = req.policy.clone();
        bigger.budget_chars += extra;
        let large = pack_context(&req.hits, &bigger, &req.context).expect("large");
        for (s, l) in small.items.iter().zip(&large.items) {
            if s.tier != ContextTier::Omitted {
                prop_assert_eq!(s.tier, l.tier, "item {}", s.id.clone());
                prop_assert!(l.chars >= s.chars);
                for (sp, lp) in s.parts.iter().zip(&l.parts) {
                    prop_assert!(lp.chars >= sp.chars);
                }
            }
        }
    }

    /// AC41-15: output is deterministic and independent of `context` input
    /// order when `(group, position)` pairs are distinct.
    #[test]
    fn prop_deterministic_and_context_order_independent(req in request_strategy()) {
        let a = pack_context(&req.hits, &req.policy, &req.context).expect("a");
        let b = pack_context(&req.hits, &req.policy, &req.context).expect("b");
        prop_assert_eq!(&a, &b);
        let reversed: Vec<ContextItem> = req.context.iter().rev().cloned().collect();
        let r = pack_context(&req.hits, &req.policy, &reversed).expect("r");
        prop_assert_eq!(&a.text, &r.text);
        let by_id = |p: &ContextPack| -> BTreeMap<String, ContextPackEntry> {
            p.items.iter().map(|e| (e.id.clone(), e.clone())).collect()
        };
        prop_assert_eq!(by_id(&a), by_id(&r));
    }

    /// AC41-16: decode inverts encode; markers removed give the plain join.
    #[test]
    fn prop_round_body_round_trips_and_strips_to_plain(
        parts in prop::collection::vec(
            (prop::sample::select(vec![None, Some("user"), Some("assistant"), Some("system"), Some("tool")]),
             text_strategy())
                .prop_map(|(role, text)| ContextPart { role: role.map(str::to_string), text }),
            1..5,
        )
    ) {
        let body = encode_round_body(&parts).expect("encode");
        prop_assert_eq!(decode_round_body(&body).expect("decode"), parts.clone());
        let plain: Vec<&str> = parts.iter().map(|p| p.text.as_str()).collect();
        prop_assert_eq!(strip_markers(&body), plain.join("\n"));
    }
}

// --- engine: a tagged corpus ranks like its plain twin (AC41-18) ------------------

const WORDS: &[&str] = &[
    "cat", "dog", "oslo", "vitamin", "c", "d3", "the", "of", "kitchen", "garden", "trip", "paris",
    "doctor", "sister", "brother", "cat's", "coffee", "tea", "budget", "piano",
];

/// A deterministic 200-round corpus: multi-part rounds, query terms as the very
/// first token of a part, empty parts, assistant-only terms, and byte-identical
/// duplicate rounds.
fn corpus() -> Vec<Vec<ContextPart>> {
    let mut state: u64 = 0x5EED_0041;
    let mut next = move |n: usize| {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((state >> 33) as usize) % n
    };
    let mut rounds: Vec<Vec<ContextPart>> = Vec::new();
    for i in 0..200 {
        if i % 50 == 49 {
            let twin: Vec<ContextPart> = rounds[i - 7].clone();
            rounds.push(twin);
            continue;
        }
        let mut sentence = |len: usize| -> String {
            (0..len).map(|_| WORDS[next(WORDS.len())]).collect::<Vec<_>>().join(" ")
        };
        let user = sentence(3 + i % 5);
        let assistant = if i % 13 == 0 { String::new() } else { sentence(4 + i % 7) };
        let mut parts = vec![
            ContextPart { role: Some("user".into()), text: user },
            ContextPart { role: Some("assistant".into()), text: assistant },
        ];
        if i % 9 == 0 {
            parts.push(ContextPart {
                role: Some("assistant".into()),
                text: format!("zebra {}", sentence(2)),
            });
        }
        if i % 17 == 0 {
            parts.push(ContextPart { role: Some("tool".into()), text: "onlyassistantterm".into() });
        }
        rounds.push(parts);
    }
    rounds
}

fn plain_body(parts: &[ContextPart]) -> String {
    parts.iter().map(|p| p.text.as_str()).collect::<Vec<_>>().join("\n")
}

fn node(logical_id: String, body: String) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "doc".to_string(),
        body,
        source_id: SourceId::new("test:slice41").expect("source id"),
        logical_id: Some(logical_id),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

fn write_corpus(engine: &Engine, tagged: bool) {
    let writes: Vec<PreparedWrite> = corpus()
        .iter()
        .enumerate()
        .map(|(i, parts)| {
            let body =
                if tagged { encode_round_body(parts).expect("encode") } else { plain_body(parts) };
            node(format!("R{i:03}"), body)
        })
        .collect();
    engine.write(&writes).expect("write corpus");
    engine.drain(30_000).expect("drain");
}

const QUERIES: &[&str] = &[
    "cat",
    "dog",
    "oslo",
    "vitamin c",
    "d3",
    "the cat",
    "of the kitchen",
    "garden trip",
    "paris",
    "doctor",
    "sister brother",
    "cat's",
    "coffee or tea",
    "budget",
    "piano",
    "zebra",
    "onlyassistantterm",
    "how many doctors did I see",
    "c",
    "the",
    "kitchen garden",
    "trip to paris",
    "my sister's piano",
    "coffee budget",
    "dog doctor",
    "vitamin d3",
    "tea",
    "brother",
    "garden",
    "oslo trip",
    "what is the budget",
    "cat dog",
    "piano lessons",
    "zebra cat",
    "paris coffee",
    "c d3",
    "kitchen",
    "sister",
    "doctor's",
    "where did I travel",
];

type Ranked = Vec<(String, String, u64)>;

fn ranked(result: &fathomdb_engine::SearchResult) -> Ranked {
    result
        .results
        .iter()
        .map(|h| (h.id.to_prefixed(), format!("{:?}", h.branch), h.score.to_bits()))
        .collect()
}

fn open_noop(dir: &TempDir, name: &str) -> fathomdb_engine::OpenedEngine {
    Engine::open_with_choice(
        dir.path().join(format!("{name}{SQLITE_SUFFIX}")),
        EmbedderChoice::Caller(Arc::new(NoopEmbedder::default())),
    )
    .expect("open")
}

/// AC41-18: equal ids, order and score bits from `search_text_only` at limits
/// 10 and 100 for a tagged corpus and its plain twin.
#[test]
fn tagged_and_plain_corpora_rank_identically() {
    let dir = TempDir::new().expect("tempdir");
    let plain = open_noop(&dir, "slice41_plain");
    let tagged = open_noop(&dir, "slice41_tagged");
    write_corpus(&plain.engine, false);
    write_corpus(&tagged.engine, true);
    let mut non_empty = 0;
    for q in QUERIES {
        for limit in [10, 100] {
            let p = plain.engine.search_text_only_with_limit(q, limit).expect("plain search");
            let t = tagged.engine.search_text_only_with_limit(q, limit).expect("tagged search");
            assert_eq!(ranked(&p), ranked(&t), "query {q:?} limit {limit}");
            if !p.results.is_empty() {
                non_empty += 1;
            }
        }
    }
    assert!(non_empty >= 60, "the query set must exercise ranking ({non_empty} non-empty)");
    plain.engine.close().expect("close");
    tagged.engine.close().expect("close");
}

/// AC41-18 (hybrid, after AC41-19): with the default embedder and the dense
/// projection declared, hybrid `search` ids, order and fused-score bits match.
#[cfg(feature = "default-embedder")]
#[test]
fn tagged_and_plain_corpora_fuse_identically() {
    use fathomdb_engine::{ProjectionRole, ProjectionSpec, ProjectionVector};
    use std::collections::BTreeSet;

    let dense = ProjectionSpec {
        name: "dense".to_string(),
        roles: BTreeSet::from([ProjectionRole::Searchable]),
        fts: None,
        vector: Some(ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    };
    let dir = TempDir::new().expect("tempdir");
    let open = |name: &str| {
        Engine::open_with_choice(
            dir.path().join(format!("{name}{SQLITE_SUFFIX}")),
            EmbedderChoice::Default,
        )
    };
    let (plain, tagged) = match (open("slice41_hplain"), open("slice41_htagged")) {
        (Ok(p), Ok(t)) => (p, t),
        (p, t) => {
            eprintln!("SKIP: default embedder unavailable ({:?} / {:?})", p.err(), t.err());
            return;
        }
    };
    plain.engine.configure_projections(std::slice::from_ref(&dense), &[]).expect("configure");
    tagged.engine.configure_projections(std::slice::from_ref(&dense), &[]).expect("configure");
    write_corpus(&plain.engine, false);
    write_corpus(&tagged.engine, true);
    let mut vector_hits = 0;
    for q in QUERIES {
        let p = plain.engine.search_with_limit(q, 30).expect("plain search");
        let t = tagged.engine.search_with_limit(q, 30).expect("tagged search");
        assert_eq!(ranked(&p), ranked(&t), "query {q:?}");
        vector_hits +=
            p.results.iter().filter(|h| format!("{:?}", h.branch).contains("Vector")).count();
    }
    assert!(vector_hits > 0, "the hybrid case must exercise the vector arm");
    plain.engine.close().expect("close");
    tagged.engine.close().expect("close");
}
