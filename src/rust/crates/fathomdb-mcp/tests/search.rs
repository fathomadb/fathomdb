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
    assert_eq!(found["verdict"], "unavailable");
    assert_eq!(found["state"], json!({"status": "unavailable"}));
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
    assert_eq!(found["degradation"], json!([{"arm": "vector", "reason": "embedder_unavailable"}]));
    // MR-032.3: never `complete` while an arm that could contribute is degraded.
    assert_eq!(found["completeness"], "partial");
    let empty = harness.ok("search", json!({"query": "zebra", "mode": "hybrid"})).await;
    assert_eq!(empty["completeness"], "empty_not_ready", "MR-030.3");
    let text_empty = harness.ok("search", json!({"query": "zebra"})).await;
    assert_eq!(text_empty["completeness"], "empty");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn filters_are_hybrid_only_and_constrain_hybrid_results() {
    let (_fixture, harness) = harness_with(
        vec![
            node("note:1", "note", "river crossing one"),
            node("doc:1", "doc", "river crossing two"),
        ],
        json!({}),
    )
    .await;
    for filter in [
        json!({"kind": "note"}),
        json!({"source_type": "x"}),
        json!({"status": "open"}),
        json!({"created_after": 1}),
        json!({"attributes": {"a": "b"}}),
    ] {
        let mut args = json!({"query": "river", "mode": "text"});
        for (key, value) in filter.as_object().unwrap() {
            args[key] = value.clone();
        }
        let error = harness.err("search", args).await;
        assert_eq!(error["code"], "bad_argument", "{filter}: {error}");
    }
    let found =
        harness.ok("search", json!({"query": "river", "mode": "hybrid", "kind": "doc"})).await;
    assert_eq!(refs(&found), vec!["l:doc:1"]);
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
    let (_fixture, harness) = harness_with(long_notes, json!({"idle_timeout_secs": 0})).await;
    for max_tokens in [256_u64, 300, 512, 1000, 2500, 4000, 20_000] {
        for explain in [false, true] {
            for mode in ["text", "hybrid"] {
                let result = harness
                    .call(
                        "search",
                        json!({"query": "river", "limit": 40, "max_tokens": max_tokens, "explain": explain, "mode": mode}),
                    )
                    .await;
                assert_ne!(result.is_error, Some(true));
                assert_fits(&result, max_tokens);
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
        json!({"query": "river", "include_superseded": true}),
        json!({"query": "river", "colour": "blue"}),
    ] {
        let error = harness.err("search", args.clone()).await;
        assert_eq!(error["class"], "invalid_request", "{args}: {error}");
        assert_eq!(error["code"], "bad_argument", "{args}: {error}");
    }
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
/// embedder diverges from its references and disables the dense side.
#[cfg(feature = "default-embedder")]
fn dense_disabled_store(fixture: &Fixture) {
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
    // Embed them now: once the dense side is disabled nothing embeds them,
    // and the adapter's shutdown drain would wait out its whole deadline.
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
async fn hybrid_on_dense_disabled_store_is_not_ready_use_text() {
    let fixture = Fixture::new();
    dense_disabled_store(&fixture);
    let config = fixture.config(json!({
        "stores": {"a": {"path": "a.sqlite"}},
        "default_store": "a",
        "use_default_embedder": true,
        // The open re-runs the equivalence check, embedding every probe; an
        // unoptimized build takes minutes.
        "call_deadline_ms": 900000
    }));
    let harness = Harness::start(config).await;
    let orient = harness.ok("orient", json!({})).await;
    assert_eq!(orient["dense_disabled"]["disabled"], true, "precondition: {orient}");

    let error = harness.err("search", json!({"query": "river", "mode": "hybrid"})).await;
    assert_eq!(error["class"], "not_ready", "{error}");
    assert_eq!(error["code"], "vector_disabled", "{error}");
    assert_eq!(error["detail"], "VectorEquivalenceMismatchError", "{error}");
    let remedy = error["remedy"].as_str().expect("remedy");
    assert!(remedy.contains("mode") && remedy.contains("text"), "{remedy}");

    let found = harness.ok("search", json!({"query": "river", "mode": "text"})).await;
    assert!(!refs(&found).is_empty(), "text search still answers: {found}");
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
    writes.extend((0..12).map(|i| node(&format!("note:{i:02}"), "note", "zanzibar again")));
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
