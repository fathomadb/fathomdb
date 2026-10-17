//! Feedback (design §9 order 8): not recorded without telemetry or for text
//! searches, recorded with label source `agent` and the right query id under
//! concurrent hybrid searches, store and reference checks, store_reopened,
//! unique ids per sink file across epochs, and no effect on later output.
//! MR-024.1–.7, MR-006.5.

mod common;

use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use common::{node, refs, text_of, wait_for_readiness, Fixture, Harness};
use serde_json::{json, Value};

fn notes() -> Vec<fathomdb_sdk::PreparedWrite> {
    let mut writes = Vec::new();
    for (i, word) in
        ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta"].iter().enumerate()
    {
        writes.push(node(&format!("note:{word}"), "note", &format!("{word} river crossing {i}")));
        writes.push(node(&format!("note:{word}2"), "note", &format!("{word} second record")));
    }
    writes
}

async fn start(fixture: &Fixture, telemetry: bool, extra: Value) -> Harness {
    fixture.store("a.sqlite", notes());
    fixture.store("b.sqlite", notes());
    let mut config = json!({
        "stores": {"a": {"path": "a.sqlite"}, "b": {"path": "b.sqlite"}},
        "default_store": "a",
        "idle_timeout_secs": 0
    });
    if telemetry {
        config["telemetry_dir"] = json!("telemetry");
    }
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    Harness::start(fixture.config(config)).await
}

fn sink(fixture: &Fixture, store: &str, epoch: u64) -> std::path::PathBuf {
    fixture.path("telemetry").join(format!("{store}.{}.{epoch}.jsonl", std::process::id()))
}

fn records(path: &Path) -> Vec<Value> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn of_type<'a>(records: &'a [Value], kind: &str) -> Vec<&'a Value> {
    records.iter().filter(|record| record["type"] == kind).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn without_telemetry_feedback_is_not_recorded_and_not_an_error() {
    let fixture = Fixture::new();
    let harness = start(&fixture, false, json!({})).await;
    let found = harness.ok("search", json!({"query": "alpha", "mode": "hybrid"})).await;
    let feedback = harness
        .ok("feedback", json!({"response_id": found["response_id"], "used": [refs(&found)[0]]}))
        .await;
    assert_eq!(feedback["status"], "not_recorded");
    assert_eq!(feedback["reason"], "telemetry_disabled", "MR-024.2");
    assert_eq!(feedback["store"], "a");
    assert!(!fixture.path("telemetry").exists());
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn text_searches_are_never_captured_so_their_feedback_is_text_mode() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({})).await;
    let found = harness.ok("search", json!({"query": "alpha"})).await;
    let feedback = harness
        .ok("feedback", json!({"response_id": found["response_id"], "used": [refs(&found)[0]]}))
        .await;
    assert_eq!(feedback["status"], "not_recorded");
    assert_eq!(feedback["reason"], "text_mode", "MR-024.1");
    assert!(of_type(&records(&sink(&fixture, "a", 1)), "feedback").is_empty());
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn hybrid_feedback_is_recorded_against_its_own_query_with_label_source_agent() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({})).await;
    let found = harness.ok("search", json!({"query": "alpha", "mode": "hybrid"})).await;
    let hit_refs = refs(&found);
    assert!(hit_refs.len() >= 2, "{found}");
    let feedback = harness
        .ok(
            "feedback",
            json!({"response_id": found["response_id"], "used": [hit_refs[0]], "unused": [hit_refs[1]]}),
        )
        .await;
    assert_eq!(feedback["status"], "recorded", "MR-024.1/.4: {feedback}");
    assert!(feedback.get("reason").is_none());
    let all = records(&sink(&fixture, "a", 1));
    let events = of_type(&all, "event");
    let labels = of_type(&all, "feedback");
    assert_eq!(events.len(), 1);
    assert_eq!(labels.len(), 1);
    let (event, label) = (events[0], labels[0]);
    assert_eq!(label["query_id"], event["query_id"]);
    assert_eq!(label["label_source"], "agent");
    assert_eq!(event["result_stable_ids"], json!(hit_refs));
    let ids = event["result_ids"].as_array().unwrap();
    assert_eq!(label["relevant_ids"], json!([ids[0]]));
    assert_eq!(label["irrelevant_ids"], json!([ids[1]]));
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn concurrent_hybrid_searches_credit_feedback_to_the_right_search() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({"admission_cap": 7})).await;
    harness.ok("orient", json!({})).await;
    // Seven concurrent searches: the highest admission cap (7) admits all.
    let words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta"];
    let mut tasks = Vec::new();
    for word in words {
        let peer = harness.client.peer().clone();
        tasks.push(tokio::spawn(async move {
            let args = json!({"query": word, "mode": "hybrid"});
            let Value::Object(arguments) = args else { unreachable!() };
            let response = peer
                .call_tool_once(
                    rmcp::model::CallToolRequestParams::new("search").with_arguments(arguments),
                )
                .await
                .unwrap();
            let rmcp::model::CallToolResponse::Complete(result) = response else { unreachable!() };
            result.structured_content.unwrap()
        }));
    }
    let mut responses = Vec::new();
    for task in tasks {
        responses.push(task.await.unwrap());
    }
    let mut expected: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for found in &responses {
        let hit_refs = refs(found);
        let feedback = harness
            .ok("feedback", json!({"response_id": found["response_id"], "used": [hit_refs[0]]}))
            .await;
        assert_eq!(feedback["status"], "recorded", "{feedback}");
        expected.insert(hit_refs[0].clone(), hit_refs);
    }
    let all = records(&sink(&fixture, "a", 1));
    let events: BTreeMap<String, &Value> = of_type(&all, "event")
        .into_iter()
        .map(|event| (event["query_id"].as_str().unwrap().to_string(), event))
        .collect();
    let labels = of_type(&all, "feedback");
    assert_eq!(labels.len(), words.len());
    for label in labels {
        let event = events[label["query_id"].as_str().unwrap()];
        let stable: Vec<String> = event["result_stable_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap().to_string())
            .collect();
        let position = event["result_ids"]
            .as_array()
            .unwrap()
            .iter()
            .position(|id| *id == label["relevant_ids"][0])
            .expect("MR-024.6: the used id belongs to the credited search");
        let used_ref = &stable[position];
        assert_eq!(expected[used_ref], stable, "MR-024.6: credited to the search that answered");
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn feedback_checks_the_store_and_the_references() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({})).await;
    let found = harness.ok("search", json!({"query": "alpha", "mode": "hybrid"})).await;
    let error = harness
        .err("feedback", json!({"response_id": found["response_id"], "used": ["l:note:theta"]}))
        .await;
    assert_eq!(error["class"], "invalid_request", "MR-024.5: refs not in that response");
    let error = harness
        .err(
            "feedback",
            json!({"store": "b", "response_id": found["response_id"], "used": [refs(&found)[0]]}),
        )
        .await;
    assert_eq!(error["code"], "wrong_store", "MR-024.5");
    let unknown =
        harness.ok("feedback", json!({"response_id": "r1-0000000000000000", "used": []})).await;
    assert_eq!(unknown["reason"], "response_expired");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn response_ids_expire_after_256_newer_responses() {
    let fixture = Fixture::new();
    let mut writes = Vec::new();
    for i in 0..260 {
        writes.push(node(&format!("note:{i}"), "note", &format!("token{i} body")));
    }
    let harness = {
        fixture.store("a.sqlite", writes);
        Harness::start(fixture.config(json!({
            "stores": {"a": {"path": "a.sqlite"}},
            "default_store": "a",
            "telemetry_dir": "telemetry"
        })))
        .await
    };
    let first = harness.ok("search", json!({"query": "token0", "mode": "hybrid"})).await;
    for i in 1..=256 {
        harness.ok("search", json!({"query": format!("token{i}")})).await;
    }
    let feedback = harness
        .ok("feedback", json!({"response_id": first["response_id"], "used": [refs(&first)[0]]}))
        .await;
    assert_eq!(feedback["reason"], "response_expired");
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn feedback_from_before_a_reopen_is_refused_and_ids_stay_unique_per_sink() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({"idle_timeout_secs": 1})).await;
    let first = harness.ok("search", json!({"query": "alpha", "mode": "hybrid"})).await;
    let recorded = harness
        .ok("feedback", json!({"response_id": first["response_id"], "used": [refs(&first)[0]]}))
        .await;
    assert_eq!(recorded["status"], "recorded");
    assert!(wait_for_readiness(&harness.client, "a", "closed", Duration::from_secs(5)).await);
    let closed = harness
        .ok("feedback", json!({"response_id": first["response_id"], "used": [refs(&first)[0]]}))
        .await;
    assert_eq!(closed["reason"], "store_reopened");

    let second = harness.ok("search", json!({"query": "alpha", "mode": "hybrid"})).await;
    assert_eq!(second["epoch"], 2);
    let stale = harness
        .ok("feedback", json!({"response_id": first["response_id"], "used": [refs(&first)[0]]}))
        .await;
    assert_eq!(stale["status"], "not_recorded");
    assert_eq!(stale["reason"], "store_reopened", "MR-006.5");
    let again = harness
        .ok("feedback", json!({"response_id": second["response_id"], "used": [refs(&second)[0]]}))
        .await;
    assert_eq!(again["status"], "recorded");

    // MR-024.7: each sink file pairs every feedback record with exactly one
    // capture event, and nothing from the refused feedback was written.
    for epoch in [1, 2] {
        let all = records(&sink(&fixture, "a", epoch));
        let labels = of_type(&all, "feedback");
        assert_eq!(labels.len(), 1, "epoch {epoch}");
        for label in labels {
            let joined = of_type(&all, "event")
                .into_iter()
                .filter(|event| event["query_id"] == label["query_id"])
                .count();
            assert_eq!(joined, 1, "epoch {epoch}: one capture event per feedback");
        }
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn feedback_never_changes_later_output() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({})).await;
    let args = json!({"query": "alpha", "mode": "hybrid", "as_of": 1_900_000_000});
    let before = harness.call("search", args.clone()).await;
    let structured = before.structured_content.clone().unwrap();
    for _ in 0..3 {
        harness
            .ok(
                "feedback",
                json!({"response_id": structured["response_id"], "used": [refs(&structured)[0]]}),
            )
            .await;
    }
    let after = harness.call("search", args).await;
    assert_eq!(before.structured_content, after.structured_content, "MR-024.3");
    assert_eq!(text_of(&before), text_of(&after));
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn feedback_with_no_used_refs_on_a_hybrid_response_is_recorded() {
    let fixture = Fixture::new();
    let harness = start(&fixture, true, json!({})).await;
    let found = harness.ok("search", json!({"query": "alpha", "mode": "hybrid"})).await;
    let hit_refs = refs(&found);
    let feedback = harness
        .ok(
            "feedback",
            json!({"response_id": found["response_id"], "used": [], "unused": [hit_refs[0]]}),
        )
        .await;
    assert_eq!(feedback["status"], "recorded", "{feedback}");
    let all = records(&sink(&fixture, "a", 1));
    let labels = of_type(&all, "feedback");
    assert_eq!(labels.len(), 1);
    assert_eq!(labels[0]["relevant_ids"], json!([]));
    let event = of_type(&all, "event")[0];
    assert_eq!(labels[0]["irrelevant_ids"], json!([event["result_ids"][0]]));
    harness.shutdown().await;
}
