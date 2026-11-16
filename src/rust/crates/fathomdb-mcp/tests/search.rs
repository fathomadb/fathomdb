//! Search (design §9 order 6): text and hybrid paths, filters, as-of and
//! the validity instant (MR-023), snippet cap (MR-040.1), completeness and
//! degradation (MR-030.1, MR-032), continuations (MR-041.2/.5), budget
//! (MR-041.1/.3) and references (MR-042.1/.3).

mod common;

use std::time::Duration;

use common::{node, refs, text_of, wait_for_readiness, windowed_node, Fixture, Harness};
use fathomdb_sdk::PreparedWrite;
use serde_json::{json, Value};

fn river_notes(count: usize) -> Vec<PreparedWrite> {
    (0..count)
        .map(|i| node(&format!("note:{i:03}"), "note", &format!("river crossing entry number {i}")))
        .collect()
}

async fn harness_with(writes: Vec<PreparedWrite>, extra: Value) -> (Fixture, Harness) {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", writes);
    let mut config = json!({"stores": {"a": {"path": "a.sqlite"}}, "default_store": "a"});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    let harness = Harness::start(fixture.config(config)).await;
    (fixture, harness)
}

fn tokens(bytes: usize) -> u64 {
    (bytes as u64).div_ceil(4)
}

#[tokio::test(flavor = "multi_thread")]
async fn text_mode_returns_lexical_hits_with_references_and_provenance() {
    let (_fixture, harness) = harness_with(
        vec![
            node("note:1", "note", "alpha river crossing"),
            node("doc:2", "doc", "unrelated words entirely"),
        ],
        json!({}),
    )
    .await;
    let result = harness.call("search", json!({"query": "river"})).await;
    let found = result.structured_content.clone().unwrap();
    assert_eq!(found["mode"], "text");
    assert_eq!(found["store"], "a");
    assert!(found.get("verdict").is_none(), "unavailable verdict is omitted");
    assert!(found.get("state").is_none(), "unavailable envelope state is omitted");
    assert_eq!(found["degradation"], json!([]));
    let hit = &found["results"][0];
    assert_eq!(hit["ref"], "l:note:1", "MR-042.1: the logical id verbatim");
    assert_eq!(hit["kind"], "note");
    assert_eq!(hit["snippet"], "alpha river crossing");
    assert_eq!(hit["snippet_cut"], false);
    assert_eq!(hit["source_id"], "fixture");
    assert_eq!(hit["arm"], "text");
    assert!(hit["score"].is_number());
    assert_eq!(found["results"].as_array().unwrap().len(), 1);
    assert_eq!(found["completeness"], "complete");
    let response_id = found["response_id"].as_str().unwrap();
    assert!(response_id.starts_with("r1-") && response_id.len() == 3 + 16, "{response_id}");
    let text = text_of(&result);
    assert!(text.contains("l:note:1") && text.contains("\"alpha river crossing\""), "{text}");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn hybrid_mode_without_an_embedder_returns_lexical_hits_and_names_the_vector_arm() {
    let (_fixture, harness) =
        harness_with(vec![node("note:1", "note", "alpha river crossing")], json!({})).await;
    let found = harness.ok("search", json!({"query": "river", "mode": "hybrid"})).await;
    assert_eq!(found["mode"], "hybrid");
    assert_eq!(refs(&found), vec!["l:note:1"]);
    // MR-032.1: the vector arm and why it did not run.
    assert_eq!(
        found["degradation"],
        json!([{"arm": "vector", "reason": "embedder_absent"}, {"arm": "vector", "reason": "embedder_unavailable"}])
    );
    // MR-032.3: never `complete` while an arm that could contribute is degraded.
    assert_eq!(found["completeness"], "partial");
    let empty = harness.ok("search", json!({"query": "zebra", "mode": "hybrid"})).await;
    assert_eq!(empty["completeness"], "empty_not_ready", "MR-030.3");
    let text_empty = harness.ok("search", json!({"query": "zebra"})).await;
    assert_eq!(text_empty["completeness"], "empty");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn exact_kind_filters_constrain_both_routes() {
    let (_fixture, harness) = harness_with(
        vec![
            node("note:1", "note", "river crossing one"),
            node("doc:1", "doc", "river crossing two"),
        ],
        json!({}),
    )
    .await;
    for mode in ["text", "hybrid"] {
        let found = harness.ok("search", json!({"query":"river","mode":mode,"kind":"doc"})).await;
        assert_eq!(refs(&found), vec!["l:doc:1"], "{mode}");
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn as_of_selects_the_validity_window_and_is_reported() {
    let t1 = 1_700_000_000_i64;
    let t2 = 1_800_000_000_i64;
    let (_fixture, harness) = harness_with(
        vec![windowed_node("note:w", "windowed river record", Some(t1), Some(t2))],
        json!({}),
    )
    .await;
    for mode in ["text", "hybrid"] {
        let inside =
            harness.ok("search", json!({"query": "river", "mode": mode, "as_of": t1 + 10})).await;
        assert_eq!(refs(&inside), vec!["l:note:w"], "{mode}");
        assert_eq!(inside["validity_instant"], t1 + 10, "MR-023.2");
        let before =
            harness.ok("search", json!({"query": "river", "mode": mode, "as_of": t1 - 1})).await;
        assert!(refs(&before).is_empty(), "{mode}: not before the window");
        let after =
            harness.ok("search", json!({"query": "river", "mode": mode, "as_of": t2})).await;
        assert!(refs(&after).is_empty(), "{mode}: the upper bound is exclusive");
    }
    let now = harness.ok("search", json!({"query": "river"})).await;
    let instant = now["validity_instant"].as_i64().expect("never null");
    let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
        as i64;
    assert!((wall - instant).abs() <= 5, "unpinned calls are answered at now");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn snippets_are_capped_in_bytes() {
    let long: String = "é".repeat(500) + " river";
    let (_fixture, harness) = harness_with(
        vec![node("note:long", "note", &format!("river {long}"))],
        json!({"snippet_bytes": 50}),
    )
    .await;
    let found = harness.ok("search", json!({"query": "river"})).await;
    let snippet = found["results"][0]["snippet"].as_str().unwrap();
    assert!(snippet.len() <= 50, "MR-040.1 in bytes: {} bytes", snippet.len());
    assert_eq!(found["results"][0]["snippet_cut"], true);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_default_snippet_cap_is_300_bytes() {
    let body = format!("river {}", "wörd ".repeat(400));
    let (_fixture, harness) = harness_with(vec![node("note:long", "note", &body)], json!({})).await;
    let found = harness.ok("search", json!({"query": "river"})).await;
    for hit in found["results"].as_array().unwrap() {
        assert!(hit["snippet"].as_str().unwrap().len() <= 300);
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn completeness_reports_limit_reached_and_complete() {
    let (_fixture, harness) = harness_with(river_notes(5), json!({})).await;
    let exact = harness.ok("search", json!({"query": "river", "limit": 5})).await;
    assert_eq!(exact["completeness"], "limit_reached");
    let fewer = harness.ok("search", json!({"query": "river", "limit": 6})).await;
    assert_eq!(fewer["completeness"], "complete", "MR-030.1");
    assert_eq!(fewer["continuation"], Value::Null);
    harness.shutdown().await;
}

async fn follow(harness: &Harness, first: Value, max_tokens: Option<u64>) -> Vec<String> {
    let mut collected = refs(&first);
    let mut page = first;
    let mut rounds = 0;
    while let Some(handle) = page["continuation"].as_str().map(str::to_string) {
        let mut args = json!({"continuation": handle});
        if let Some(max_tokens) = max_tokens {
            args["max_tokens"] = json!(max_tokens);
        }
        page = harness.ok("search", args).await;
        collected.extend(refs(&page));
        rounds += 1;
        assert!(rounds < 200, "continuations must terminate");
    }
    collected
}

#[tokio::test(flavor = "multi_thread")]
async fn continuations_page_without_gaps_or_duplicates_up_to_the_ranked_cap() {
    let (_fixture, harness) = harness_with(river_notes(130), json!({"idle_timeout_secs": 0})).await;
    let all = harness
        .ok("search", json!({"query": "river", "limit": 100, "max_tokens": 1_000_000}))
        .await;
    let expected = refs(&all);
    assert_eq!(expected.len(), 100);

    let first = harness.ok("search", json!({"query": "river", "limit": 10})).await;
    assert_eq!(first["completeness"], "limit_reached");
    let continuation = first["continuation"].as_str().unwrap();
    assert!(continuation.starts_with("c1-") && continuation.len() == 3 + 16);
    let paged = follow(&harness, first, None).await;
    assert_eq!(paged, expected, "MR-041.2: same order, no gaps, no duplicates");

    // Budget cuts page the same ranked list.
    let cut = harness.ok("search", json!({"query": "river", "limit": 20, "max_tokens": 600})).await;
    assert_eq!(cut["completeness"], "cut_by_budget");
    assert_eq!(cut["budget"]["cut"], true);
    let budgeted = follow(&harness, cut, Some(600)).await;
    assert_eq!(budgeted, expected);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn continuations_are_stale_after_a_reopen_tampered_or_mixed_with_arguments() {
    let (_fixture, harness) = harness_with(river_notes(30), json!({"idle_timeout_secs": 1})).await;
    let first = harness.ok("search", json!({"query": "river", "limit": 10})).await;
    let handle = first["continuation"].as_str().unwrap().to_string();

    let mixed = harness.err("search", json!({"continuation": handle, "limit": 5})).await;
    assert_eq!(mixed["code"], "bad_argument");
    let mut tampered = handle.clone();
    let last = tampered.pop().unwrap();
    tampered.push(if last == '0' { '1' } else { '0' });
    let error = harness.err("search", json!({"continuation": tampered})).await;
    assert_eq!(error["code"], "stale_handle", "MR-041.5");
    assert_eq!(error["remedy"], "re-run the search");

    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    let error = harness.err("search", json!({"continuation": handle})).await;
    assert_eq!(error["code"], "stale_handle", "a new epoch invalidates continuations");
    harness.shutdown().await;
}

fn assert_fits(result: &rmcp::model::CallToolResult, max_tokens: u64) {
    let structured = result.structured_content.clone().unwrap();
    let text = text_of(result);
    let whole = serde_json::to_string(&structured).unwrap().len() + text.len();
    assert!(tokens(whole) <= max_tokens, "MR-041.1: {} > {max_tokens}", tokens(whole));
    let mut without = structured.clone();
    without.as_object_mut().unwrap().remove("budget");
    let used = tokens(serde_json::to_string(&without).unwrap().len() + text.len());
    assert_eq!(structured["budget"]["used_tokens"], used, "used_tokens excludes budget itself");
    assert_eq!(structured["budget"]["max_tokens"], max_tokens);
}

// Independent mandatory zero-progress payload: authored envelope/handle widths
// and exact trace fields from a same-state SDK control read before adapter open.
// No production shaper or rendered MCP payload supplies this oracle.
fn mandatory_search_size(mode: &str, explain: bool, max_tokens: u64, sdk_trace: &Value) -> u64 {
    let mut p = json!({"store":"a","epoch":1,"validity_instant":1900000000,"degradation":[],"budget":{"max_tokens":max_tokens,"used_tokens":0,"cut":true},"mode":mode,"response_id":"r1-0000000000000000","results":[],"completeness":"cut_by_budget","continuation":null});
    if mode == "hybrid" {
        p["degradation"] = json!([{"arm":"vector","reason":"embedder_absent"},{"arm":"vector","reason":"embedder_unavailable"}]);
    }
    if explain {
        p["explanation"] = if mode == "text" {
            json!({"status":"unavailable","reason":"text_mode"})
        } else {
            json!({"trace":sdk_trace,"per_hit":[]})
        };
    }
    for _ in 0..4 {
        let text = serde_json::to_string(&p).unwrap();
        let mut without = p.clone();
        without.as_object_mut().unwrap().remove("budget");
        let used = tokens(serde_json::to_string(&without).unwrap().len() + text.len());
        if p["budget"]["used_tokens"] == used {
            return tokens(2 * text.len());
        }
        p["budget"]["used_tokens"] = json!(used);
    }
    panic!("authored lower-bound count must converge");
}

#[tokio::test(flavor = "multi_thread")]
async fn every_response_fits_its_budget() {
    let long_notes: Vec<PreparedWrite> = (0..40)
        .map(|i| {
            node(
                &format!("note:{i}"),
                "note",
                &format!("river {i} {}", "long body text ".repeat(40)),
            )
        })
        .collect();
    let fixture = Fixture::new();
    let path = fixture.store("a.sqlite", long_notes);
    let sdk = fathomdb_sdk::Engine::open(&path, Default::default()).unwrap();
    let mut options = fathomdb_sdk::SearchOptions::default();
    options.limit = 40;
    options.explain = true;
    options.view.valid_as_of = Some(1_900_000_000);
    let control = sdk.search("river", options).unwrap();
    assert_eq!(control.results.len(), 40, "authored unchanged lexical fixture");
    assert!(control.soft_fallback.is_none());
    assert_eq!(
        control
            .degradations
            .iter()
            .map(|entry| (entry.arm, entry.reason.as_str()))
            .collect::<Vec<_>>(),
        vec![(fathomdb_sdk::SoftFallbackBranch::Vector, "embedder_absent")],
        "the same-state None-provider hybrid control reports exactly its per-call absence"
    );
    let trace = &control.explanation.as_ref().unwrap().trace;
    let sdk_trace = json!({"query_chars":trace.query_chars,"k":trace.k,"rerank_depth":trace.rerank_depth,"pool_n":trace.pool_n,"alpha":trace.alpha,"use_graph_arm":trace.use_graph_arm,"recency":trace.recency,"embedder_id":trace.embedder_id,"ce_active":trace.ce_active,"vector_hits":trace.vector_hits,"text_hits":trace.text_hits,"graph_hits":trace.graph_hits,"dropped_edge_hits":trace.dropped_edge_hits});
    sdk.close().unwrap();
    let harness = Harness::start(fixture.config(
        json!({"stores":{"a":{"path":"a.sqlite"}},"default_store":"a","idle_timeout_secs":0}),
    ))
    .await;
    for max_tokens in [256_u64, 300, 512, 1000, 2500, 4000, 20_000] {
        for explain in [false, true] {
            for mode in ["text", "hybrid"] {
                let result = harness
                    .call(
                        "search",
                        json!({"query": "river", "limit": 40, "max_tokens": max_tokens, "explain": explain, "mode": mode,"as_of":1_900_000_000}),
                    )
                    .await;
                if result.is_error == Some(true) {
                    let error = &result.structured_content.as_ref().unwrap()["error"];
                    assert_eq!(error["code"], "bad_argument");
                    assert!(mandatory_search_size(mode,explain,max_tokens,&sdk_trace)>max_tokens,"refusal requires independent mandatory-overflow proof: {mode}/{explain}/{max_tokens}: {error}");
                } else {
                    assert_fits(&result, max_tokens);
                }
            }
        }
    }
    // MR-041.3: the documented default applies when max_tokens is omitted.
    let result = harness.call("search", json!({"query": "river", "limit": 40})).await;
    assert_fits(&result, 4000);
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_arguments_are_typed_invalid_requests() {
    let (_fixture, harness) = harness_with(river_notes(2), json!({})).await;
    for args in [
        json!({}),
        json!({"query": ""}),
        json!({"query": "   "}),
        json!({"query": "river", "limit": 0}),
        json!({"query": "river", "limit": 101}),
        json!({"query": "river", "max_tokens": 255}),
        json!({"query": "river", "mode": "vector"}),
        json!({"query": "river", "include_superseded": "true"}),
    ] {
        let error = harness.err("search", args.clone()).await;
        assert_eq!(error["class"], "invalid_request", "{args}: {error}");
        assert_eq!(error["code"], "bad_argument", "{args}: {error}");
    }
    let unknown = harness.err("search", json!({"query":"river","colour":"blue"})).await;
    assert_eq!(unknown["class"], "invalid_request");
    assert_eq!(unknown["code"], "unknown_argument");
    assert_eq!(unknown["argument"], "colour");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn explain_attaches_a_per_hit_explanation_without_run_varying_ids() {
    let (_fixture, harness) = harness_with(river_notes(3), json!({})).await;
    let hybrid =
        harness.ok("search", json!({"query": "river", "mode": "hybrid", "explain": true})).await;
    let explanation = &hybrid["explanation"];
    assert!(explanation["trace"]["k"].is_number(), "{explanation}");
    let per_hit = explanation["per_hit"].as_array().unwrap();
    assert_eq!(per_hit.len(), hybrid["results"].as_array().unwrap().len());
    for (entry, hit) in per_hit.iter().zip(hybrid["results"].as_array().unwrap()) {
        assert_eq!(entry["ref"], hit["ref"]);
    }
    let serialized = serde_json::to_string(&hybrid).unwrap();
    assert!(!serialized.contains("correlation_id"), "engine correlation ids vary run to run");
    let text = harness.ok("search", json!({"query": "river", "explain": true})).await;
    assert_eq!(text["explanation"], json!({"status": "unavailable", "reason": "text_mode"}));
    let plain = harness.ok("search", json!({"query": "river"})).await;
    assert!(plain.get("explanation").is_none());
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn identical_requests_give_identical_output_and_references_survive_restarts() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", river_notes(12));
    let config = json!({"stores": {"a": {"path": "a.sqlite"}}, "default_store": "a"});
    let args = json!({"query": "river", "limit": 5, "as_of": 1_900_000_000});
    let first = Harness::start(fixture.config(config.clone())).await;
    let one = first.call("search", args.clone()).await;
    let two = first.call("search", args.clone()).await;
    assert_eq!(one.structured_content, two.structured_content);
    assert_eq!(text_of(&one), text_of(&two));
    first.shutdown().await;
    let second = Harness::start(fixture.config(config)).await;
    let three = second.call("search", args).await;
    // MR-042.3: the same records yield the same references after a restart.
    assert_eq!(
        refs(one.structured_content.as_ref().unwrap()),
        refs(three.structured_content.as_ref().unwrap())
    );
    assert_eq!(one.structured_content, three.structured_content);
    second.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn the_default_snippet_cap_is_300_bytes_from_the_record_start() {
    let body = format!("river {}", "abcdefghij".repeat(40));
    assert!(body.len() > 300);
    let (_fixture, harness) = harness_with(vec![node("note:long", "note", &body)], json!({})).await;
    let found = harness.ok("search", json!({"query": "river"})).await;
    let hit = &found["results"][0];
    let snippet = hit["snippet"].as_str().unwrap();
    assert!(snippet.len() <= 300, "MR-040.1 default cap, in bytes");
    assert!(snippet.starts_with("river"), "{snippet:?}");
    assert_eq!(hit["snippet_cut"], true);
    harness.shutdown().await;
}

/// A store whose open-time vector-equivalence check fails, through the real
/// engine path: references persisted by the default embedder are negated and
/// the cached verdict cleared, so the next open re-runs the check, finds the
/// embedder diverges from its references and refuses the open.
#[cfg(feature = "default-embedder")]
fn vector_mismatch_store(fixture: &Fixture) {
    let path = fixture.path("a.sqlite");
    let options = fathomdb_sdk::OpenOptions { use_default_embedder: true, ..Default::default() };
    // The first open declares a vector projection and writes nodes, which
    // registers their kind; the second, seeing the kind registered, persists
    // the probe references.
    let engine = fathomdb_sdk::Engine::open(&path, options.clone()).expect("open with embedder");
    let spec = fathomdb_sdk::ProjectionSpec {
        name: "text".into(),
        roles: [fathomdb_sdk::ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(fathomdb_sdk::ProjectionFts { tokenizer: None }),
        vector: Some(fathomdb_sdk::ProjectionVector { embedder: None, dense_readiness: None }),
        source: None,
    };
    engine.configure_projections(&[spec], &[]).expect("declare a vector projection");
    engine.write(&river_notes(3)).expect("write notes");
    // Persist all work before corrupting references for the next open.
    engine.drain(600_000).expect("drain the vector work");
    engine.close().expect("close");
    let engine = fathomdb_sdk::Engine::open(&path, options).expect("reopen with embedder");
    engine.close().expect("close");

    let conn = rusqlite::Connection::open(&path).expect("raw connection");
    let references: Vec<(i64, Vec<u8>)> = conn
        .prepare("SELECT probe_ordinal, reference_vec FROM _fathomdb_embed_probe")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert!(!references.is_empty(), "the reopen persisted probe references");
    for (ordinal, mut vector) in references {
        // Little-endian f32: flip the sign bit of every component.
        for component in vector.chunks_exact_mut(4) {
            component[3] ^= 0x80;
        }
        conn.execute(
            "UPDATE _fathomdb_embed_probe SET reference_vec = ?1 WHERE probe_ordinal = ?2",
            rusqlite::params![vector, ordinal],
        )
        .unwrap();
    }
    conn.execute(
        "DELETE FROM _fathomdb_open_state WHERE key = 'vector_equivalence_verified_fingerprint'",
        [],
    )
    .unwrap();
}

#[cfg(feature = "default-embedder")]
#[tokio::test(flavor = "multi_thread")]
async fn vector_mismatch_refuses_adapter_open_until_none_is_explicit() {
    let fixture = Fixture::new();
    vector_mismatch_store(&fixture);
    let config = fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a",
        "use_default_embedder": true,
        // The open re-runs the equivalence check, embedding every probe; an
        // unoptimized build takes minutes.
        "call_deadline_ms": 900000
    }));
    let harness = Harness::start(config).await;
    for (tool, args) in [
        ("orient", json!({})),
        ("search", json!({"query": "river", "mode": "hybrid"})),
        ("search", json!({"query": "river", "mode": "text"})),
    ] {
        let error = harness.err(tool, args).await;
        assert_eq!(error["class"], "unavailable", "{error}");
        assert_eq!(error["code"], "open_failed", "{error}");
        assert_eq!(error["detail"], "VectorEquivalenceMismatchError", "{error}");
    }
    harness.shutdown().await;

    let config = fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a",
        "use_default_embedder": false
    }));
    let harness = Harness::start(config).await;
    let found = harness.ok("search", json!({"query": "river", "mode": "text"})).await;
    assert!(!refs(&found).is_empty(), "deliberate None-provider text access: {found}");
    harness.shutdown().await;
}

/// A body of filler words with `term` at byte `at` (a multiple of 6).
fn long_body(at: usize, term: &str, len: usize) -> String {
    assert_eq!(at % 6, 0);
    let mut body = "lorem ".repeat(at / 6);
    body.push_str(term);
    while body.len() < len {
        body.push_str(" ipsum");
    }
    body
}

/// T-13 (match-centred-snippets.md § 7.3): hits carry `snippet_anchor`, and a
/// long record's snippet is centred on the matched term, not its prefix; a
/// record that fits is whole. Every response still fits its budget
/// (MR-041.1).
#[tokio::test(flavor = "multi_thread")]
async fn snippets_are_centred_on_the_match_with_an_anchor() {
    let long = long_body(3000, "zanzibar", 4096);
    let (_fixture, harness) = harness_with(
        vec![node("note:long", "note", &long), node("note:short", "note", "a short zanzibar note")],
        json!({}),
    )
    .await;
    let found = harness.ok("search", json!({"query": "zanzibar"})).await;
    let hits = found["results"].as_array().unwrap();
    let hit = |reference: &str| hits.iter().find(|hit| hit["ref"] == reference).unwrap().clone();

    let long_hit = hit("l:note:long");
    let snippet = long_hit["snippet"].as_str().unwrap();
    assert_eq!(long_hit["snippet_anchor"], "match", "{long_hit}");
    assert!(snippet.contains("zanzibar"), "{snippet:?}");
    assert!(snippet.len() <= 300);
    assert_eq!(long_hit["snippet_cut"], true);
    assert!(long_hit.get("highlights").is_none(), "highlights are opt-in");

    let short_hit = hit("l:note:short");
    assert_eq!(short_hit["snippet_anchor"], "whole");
    assert_eq!(short_hit["snippet"], "a short zanzibar note");
    assert_eq!(short_hit["snippet_cut"], false);

    for max_tokens in [256_u64, 300, 400, 600, 1000] {
        let result =
            harness.call("search", json!({"query": "zanzibar", "max_tokens": max_tokens})).await;
        assert_fits(&result, max_tokens);
    }
    harness.shutdown().await;
}

/// The highlights opt-in (owner decision 3): `highlights: true` adds, per hit,
/// byte ranges into `snippet` of the matched words; it is presentation, so a
/// continuation call may pass it and the page's references are unchanged.
#[tokio::test(flavor = "multi_thread")]
async fn highlights_are_opt_in_byte_ranges_into_the_snippet() {
    let long = long_body(3000, "zanzibar", 4096);
    let mut writes = vec![node("note:long", "note", &long)];
    writes.extend(
        (0..12).map(|i| node(&format!("note:{i:02}"), "note", &format!("zanzibar again {i}"))),
    );
    let (_fixture, harness) = harness_with(writes, json!({})).await;

    let plain = harness.ok("search", json!({"query": "zanzibar", "limit": 5})).await;
    assert!(plain["results"].as_array().unwrap().iter().all(|hit| hit.get("highlights").is_none()));

    let lit =
        harness.ok("search", json!({"query": "zanzibar", "limit": 5, "highlights": true})).await;
    assert_eq!(refs(&lit), refs(&plain), "presentation only");
    for hit in lit["results"].as_array().unwrap() {
        let snippet = hit["snippet"].as_str().unwrap();
        let ranges = hit["highlights"].as_array().unwrap_or_else(|| panic!("{hit}"));
        assert!(!ranges.is_empty(), "{hit}");
        for range in ranges {
            let (a, b) = (range[0].as_u64().unwrap() as usize, range[1].as_u64().unwrap() as usize);
            assert_eq!(&snippet[a..b].to_lowercase(), "zanzibar", "{hit}");
        }
    }

    let next = harness
        .ok("search", json!({"continuation": lit["continuation"], "highlights": true}))
        .await;
    assert!(next["results"][0]["highlights"].is_array(), "{next}");
    harness.shutdown().await;
}

/// Review fix (S5c P2): the engine's snippet point-query cap covers the
/// whole run window (offset + limit), so a long record ranked past 20, on a
/// later continuation page, is still centred on its match.
#[tokio::test(flavor = "multi_thread")]
async fn long_hits_past_rank_20_are_centred_on_continuation_pages() {
    let writes: Vec<PreparedWrite> = (0..30)
        .map(|i| {
            node(
                &format!("note:{i:02}"),
                "note",
                &format!("{} doc{i}", long_body(3000, "zanzibar", 4096)),
            )
        })
        .collect();
    let (_fixture, harness) = harness_with(writes, json!({})).await;
    let mut page =
        harness.ok("search", json!({"query": "zanzibar", "limit": 10, "max_tokens": 20000})).await;
    let mut seen = 0;
    loop {
        for hit in page["results"].as_array().unwrap() {
            assert_eq!(hit["snippet_anchor"], "match", "rank {}: {hit}", seen + 1);
            seen += 1;
        }
        let Some(next) = page["continuation"].as_str().map(str::to_string) else { break };
        page = harness.ok("search", json!({"continuation": next, "max_tokens": 20000})).await;
    }
    assert_eq!(seen, 30);
    harness.shutdown().await;
}

/// Review fix (S5c P2): `highlights` is presentation only. With short hits
/// ranked above long ones, asking for highlights (which makes fitting hits
/// use point queries) must not change any hit's snippet text or anchor.
#[tokio::test(flavor = "multi_thread")]
async fn highlights_do_not_change_snippet_text_or_anchor() {
    let mut writes: Vec<PreparedWrite> = (0..25)
        .map(|i| node(&format!("note:s{i:02}"), "note", &format!("zanzibar note {i}")))
        .collect();
    writes.extend((0..5).map(|i| {
        node(
            &format!("note:l{i}"),
            "note",
            &format!("{} doc{i}", long_body(3000, "zanzibar", 4096)),
        )
    }));
    let (_fixture, harness) = harness_with(writes, json!({})).await;
    let args = |highlights: bool| json!({"query": "zanzibar", "limit": 30, "max_tokens": 20000, "highlights": highlights});
    let plain = harness.ok("search", args(false)).await;
    let lit = harness.ok("search", args(true)).await;
    let view = |page: &Value| -> Vec<(Value, Value, Value)> {
        page["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| (hit["ref"].clone(), hit["snippet"].clone(), hit["snippet_anchor"].clone()))
            .collect()
    };
    assert_eq!(view(&plain).len(), 30);
    assert_eq!(view(&lit), view(&plain));
    harness.shutdown().await;
}

/// Owner decision 2 (match-centred-snippets.md § 10): MCP keeps the record
/// head. A LoCoMo-style record whose match is near byte 3000 gets a snippet
/// that begins with the header and still holds the match.
#[tokio::test(flavor = "multi_thread")]
async fn match_snippets_keep_the_record_head() {
    let header = "(unknown date) [8 May 2023] Caroline: ";
    let mut body = header.to_string();
    while body.len() < 2995 {
        body.push_str("we talked ");
    }
    body.push_str("zanzibar");
    body.push_str(&" again".repeat(200));
    let (_fixture, harness) =
        harness_with(vec![node("note:locomo", "note", &body)], json!({})).await;
    let found = harness.ok("search", json!({"query": "zanzibar"})).await;
    let hit = &found["results"][0];
    let snippet = hit["snippet"].as_str().unwrap();
    assert_eq!(hit["snippet_anchor"], "match", "{hit}");
    assert!(snippet.starts_with("(unknown date) [8 May 2023] Caroline:"), "{snippet:?}");
    assert!(snippet.contains("zanzibar"), "{snippet:?}");
    assert!(snippet.len() <= 300);
    harness.shutdown().await;
}

/// Block 1 close (match-centred-snippets.md § 6.5): the per-term snippet
/// pass draws on a per-search pool sized by the run window, so a
/// continuation page of a multi-term search shows the same snippets as an
/// unpaged search over the same window (offset + limit).
#[tokio::test(flavor = "multi_thread")]
async fn mcp_multi_term_continuation_pages_anchor_alike() {
    let writes: Vec<PreparedWrite> = (0..30)
        .map(|i| {
            let mut body = "lorem ".repeat(84);
            body.push_str("zanzibar ");
            body.push_str(&"lorem ".repeat(250));
            body.push_str("quokka ");
            body.push_str(&"lorem ".repeat(250));
            body.push_str("marmot ");
            body.push_str(&"ipsum ".repeat(100));
            node(&format!("note:{i:02}"), "note", &format!("{body}doc{i}"))
        })
        .collect();
    let (_fixture, harness) = harness_with(writes, json!({})).await;
    let query = "zanzibar quokka marmot";
    let view = |page: &Value| -> Vec<(Value, Value, Value)> {
        page["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| (hit["ref"].clone(), hit["snippet"].clone(), hit["snippet_anchor"].clone()))
            .collect()
    };
    let mut page =
        harness.ok("search", json!({"query": query, "limit": 10, "max_tokens": 20000})).await;
    let mut offset = 0;
    let mut pages = 0;
    loop {
        let paged = view(&page);
        if paged.is_empty() {
            break;
        }
        assert_eq!(paged.len(), 10, "page {pages}: {page}");
        let window = offset + paged.len();
        let unpaged = harness
            .ok("search", json!({"query": query, "limit": window, "max_tokens": 20000}))
            .await;
        assert_eq!(paged, view(&unpaged)[offset..window], "page {pages} (offset {offset})");
        offset = window;
        pages += 1;
        let Some(next) = page["continuation"].as_str().map(str::to_string) else { break };
        page = harness.ok("search", json!({"continuation": next, "max_tokens": 20000})).await;
    }
    assert_eq!((pages, offset), (3, 30));
    harness.shutdown().await;
}

/// Review fix (C1 P2-1, match-centred-snippets.md § 6.5): the per-term pool
/// is visible from MCP. Under `run cat`, approximate identity picks the
/// earlier `running … runs` window and exact identity the `running … cat`
/// window, so a snippet shows which pass it got. The pool holds the run
/// window's worth of queries and each pass costs 2, so in a search over a
/// window of W only ranks 1..=W/2 get the exact pass; a continuation page
/// equals the same ranks of the unpaged search over its window.
#[tokio::test(flavor = "multi_thread")]
async fn mcp_per_term_pool_scales_with_the_run_window() {
    let filler = |n: usize| "lorem ".repeat(n);
    let writes: Vec<PreparedWrite> = (0..30)
        .map(|i| {
            let body = format!(
                "{}running {}runs {}running {}cat {}doc{i}",
                filler(160),
                filler(5),
                filler(300),
                filler(5),
                filler(300)
            );
            node(&format!("note:{i:02}"), "note", &body)
        })
        .collect();
    let (_fixture, harness) = harness_with(writes, json!({})).await;
    let query = "run cat";
    let view = |page: &Value| -> Vec<(Value, Value, Value)> {
        page["results"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| (hit["ref"].clone(), hit["snippet"].clone(), hit["snippet_anchor"].clone()))
            .collect()
    };
    let exact = |snippet: &Value| {
        let text = snippet.as_str().unwrap();
        assert!(text.contains("running"), "{text:?}");
        text.contains("cat")
    };
    let mut page =
        harness.ok("search", json!({"query": query, "limit": 10, "max_tokens": 20000})).await;
    let mut offset = 0;
    loop {
        let paged = view(&page);
        if paged.is_empty() {
            break;
        }
        assert_eq!(paged.len(), 10, "offset {offset}: {page}");
        let window = offset + paged.len();
        let unpaged = view(
            &harness
                .ok("search", json!({"query": query, "limit": window, "max_tokens": 20000}))
                .await,
        );
        assert_eq!(unpaged.len(), window);
        for (rank, (_, snippet, anchor)) in unpaged.iter().enumerate() {
            assert_eq!(anchor, "match", "window {window}, rank {}", rank + 1);
            assert_eq!(exact(snippet), rank < window / 2, "window {window}, rank {}", rank + 1);
        }
        assert_eq!(paged, unpaged[offset..window], "offset {offset}");
        offset = window;
        let Some(next) = page["continuation"].as_str().map(str::to_string) else { break };
        page = harness.ok("search", json!({"continuation": next, "max_tokens": 20000})).await;
    }
    assert_eq!(offset, 30);
    harness.shutdown().await;
}

/// Marker exhaustion supplies no spans; both presentations retain the exact
/// engine record start, including its UTF-8 budget and combining clusters.
#[tokio::test(flavor = "multi_thread")]
async fn record_start_prefix_is_exact_in_both_result_formats() {
    let ascii = "alpha beta gamma delta epsilon zeta eta theta iota kappa lambda ".repeat(100);
    let nfd = "e\u{301}".repeat(200);
    let markers = " needle \u{E000}\u{E001}\u{F8FE}\u{F8FF}\u{FDD0}\u{FDD1}";
    let expected = [
        ("l:ascii", "alpha beta gamma delta epsilon zeta eta theta iota kappa …".to_string()),
        ("l:nfd", format!("{} …", "e\u{301}".repeat(19))),
    ];
    for format in ["structured", "text"] {
        let (_fixture, harness) = harness_with(
            vec![
                node("ascii", "note", &format!("{ascii}{markers}")),
                node("nfd", "note", &format!("{nfd}{markers}")),
            ],
            json!({"snippet_bytes":62,"result_format":format}),
        )
        .await;
        let result = harness
            .call(
                "search",
                json!({"query":"needle","limit":2,"highlights":true,"max_tokens":20000}),
            )
            .await;
        assert_ne!(result.is_error, Some(true));
        if format == "structured" {
            let value = result.structured_content.as_ref().unwrap();
            assert_eq!(value["results"].as_array().unwrap().len(), 2);
            for (reference, text) in &expected {
                let hit = value["results"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|hit| hit["ref"] == *reference)
                    .unwrap();
                assert_eq!(hit["snippet"], *text);
                assert_eq!(hit["snippet_anchor"], "start");
                assert_eq!(hit["snippet_cut"], true);
                assert_eq!(hit["highlights"], json!([]));
            }
        } else {
            assert!(result.structured_content.is_none());
            let rendered = text_of(&result);
            for (reference, text) in &expected {
                let line = rendered
                    .lines()
                    .find(|line| line.starts_with(&format!("\"{reference}\" ")))
                    .unwrap();
                assert!(line.contains("anchor=start cut"), "{line}");
                assert!(line.ends_with(&format!(" | {text}")), "{line}");
                assert!(line.contains(" hl=[]"), "{line}");
            }
        }
        harness.shutdown().await;
    }
}
