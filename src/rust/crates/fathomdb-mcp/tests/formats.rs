//! The compact wire contract, through real MCP connections. Each connection
//! captures and validates its own schemas; no global schema cache is used.
mod common;

use common::Harness;
use fathomdb_mcp::config::Config;
use rmcp::ServerHandler;

#[tokio::test]
async fn default_discovery_has_no_output_schema() {
    let text = Harness::start(Config::default()).await;
    let structured = Harness::start(
        Config::from_json_str(r#"{"result_format":"structured"}"#, std::path::Path::new("."))
            .unwrap(),
    )
    .await;
    let a = text.tools().await;
    let b = structured.tools().await;
    let mut names: Vec<_> = a.iter().map(|tool| tool.name.as_ref()).collect();
    names.sort();
    assert_eq!(
        names,
        ["expand", "feedback", "fetch", "list_stores", "orient", "search", "use_store"]
    );
    for tool in &a {
        assert!(
            tool.output_schema.is_none(),
            "default {} must not advertise outputSchema",
            tool.name
        );
        let other = b.iter().find(|other| other.name == tool.name).unwrap();
        assert!(other.output_schema.is_some());
        assert_eq!(tool.input_schema, other.input_schema);
        assert!(text.adapter.connection().get_tool(&tool.name).unwrap().output_schema.is_none());
        assert!(structured
            .adapter
            .connection()
            .get_tool(&tool.name)
            .unwrap()
            .output_schema
            .is_some());
    }
    text.shutdown().await;
    structured.shutdown().await;
}

use common::{node, text_of, Fixture};
use rmcp::model::{CallToolRequestParams, CallToolResult};
use serde_json::{json, Value};

async fn wire(harness: &Harness, tool: &str, args: Value) -> CallToolResult {
    let tools = harness.tools().await;
    let result = harness
        .client
        .call_tool(
            CallToolRequestParams::new(tool.to_string())
                .with_arguments(args.as_object().unwrap().clone()),
        )
        .await
        .unwrap();
    if let Some(payload) = &result.structured_content {
        if let Some(schema) = &tools.iter().find(|entry| entry.name == tool).unwrap().output_schema
        {
            let schema = Value::Object(schema.as_ref().clone());
            let compiled = jsonschema::JSONSchema::options()
                .with_draft(jsonschema::Draft::Draft202012)
                .compile(&schema)
                .unwrap();
            assert!(compiled.is_valid(payload), "{tool}: {payload}");
        }
    }
    result
}

fn wire_config(fixture: &Fixture, extra: Value) -> Config {
    let mut config = json!({"stores":{"a":{"path":"a.sqlite"}},"default_store":"a","hints":false});
    for (key, value) in extra.as_object().unwrap() {
        config[key] = value.clone();
    }
    Config::from_json_str(&config.to_string(), fixture.dir.path()).unwrap()
}

#[tokio::test]
async fn list_stores_text_has_one_block_and_quoted_names() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("one", "note", "river")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let listed = wire(&harness, "list_stores", json!({})).await;
    assert!(listed.structured_content.is_none(), "default results must carry only text");
    assert_eq!(
        text_of(&listed),
        "stores: 1 default=\"a\"\n\"a\" open=false readiness=closed default"
    );
    for (tool, args) in [
        ("use_store", json!({"store":"a"})),
        ("orient", json!({})),
        ("search", json!({"query":"river"})),
        ("fetch", json!({"refs":["l:one"]})),
        ("expand", json!({"refs":["l:one"]})),
        ("feedback", json!({"response_id":"expired","used":[]})),
    ] {
        let result = wire(&harness, tool, args).await;
        assert_ne!(result.is_error, Some(true), "{tool}: {}", text_of(&result));
        assert!(result.structured_content.is_none(), "{tool} default must carry only text");
        assert!(!text_of(&result).is_empty());
    }
    let error = wire(&harness, "search", json!({"query":""})).await;
    assert_eq!(error.is_error, Some(true));
    assert!(error.structured_content.is_none());
    text_of(&error);
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_search_rows_keep_quotes_and_scores() {
    let fixture = Fixture::new();
    let id = "odd space\"\\|é\r\n😀";
    let body = "river \r\n| café e\u{301} 😀";
    let path = fixture.store("a.sqlite", vec![node(id, "note", body)]);
    let engine = fathomdb_sdk::Engine::open(path, Default::default()).unwrap();
    let mut options = fathomdb_sdk::TextSearchOptions::default();
    options.view.valid_as_of = Some(1_900_000_000);
    let expected = engine.search_text_only("river", options).unwrap();
    assert_eq!(expected.results.len(), 1);
    let score = expected.results[0].score;
    engine.close().unwrap();
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let result = wire(&harness, "search", json!({"query":"river","as_of":1_900_000_000})).await;
    let text = text_of(&result);
    assert!(text.starts_with("search store=\"a\" epoch=1 validity_instant=1900000000 mode=text hits=1 completeness=complete kind=\"note\" response_id=r1-"), "{text}");
    let quoted = serde_json::to_string(&format!("l:{id}")).unwrap();
    let flattened = body.replace(['\r', '\n'], " ");
    assert_eq!(
        text.lines().nth(1).unwrap(),
        format!("{quoted} arm=text score={score:.3} src=\"fixture\" | {flattened}")
    );
    assert_eq!(text.lines().count(), 2, "one LF-delimited hit line");
    assert!(text.lines().next().unwrap().contains(" tokens="));
    harness.shutdown().await;
    let structured =
        Harness::start(wire_config(&fixture, json!({"result_format":"structured"}))).await;
    let result = wire(&structured, "search", json!({"query":"river","as_of":1_900_000_000})).await;
    assert_eq!(result.structured_content.unwrap()["results"][0]["score"].as_f64().unwrap(), score);
    structured.shutdown().await;
}

#[tokio::test]
async fn compact_search_signal_highlights_and_text_explanation() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("unicode", "note", "é\r\nriver café 😀")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let result =
        wire(&harness, "search", json!({"query":"river","highlights":true,"explain":true})).await;
    let text = text_of(&result);
    assert!(text.lines().any(|line| line.contains(" hl=4-9 | é  river café 😀")), "{text}");
    let explanation = text.lines().find_map(|line| line.strip_prefix("explanation: ")).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(explanation).unwrap(),
        json!({"status":"unavailable","reason":"text_mode"})
    );
    harness.shutdown().await;
}

#[tokio::test]
async fn hosted_continuation_reports_best_effort() {
    let fixture = Fixture::new();
    let path = fixture.store(
        "host.sqlite",
        (0..3).map(|i| node(&format!("host{i}"), "note", &format!("river {i}"))).collect(),
    );
    let engine = std::sync::Arc::new(fathomdb_sdk::Engine::open(path, Default::default()).unwrap());
    let harness = Harness::start(Config::default()).await;
    harness.adapter.register_engine("host", engine.clone()).unwrap();
    let result =
        wire(&harness, "search", json!({"store":"host","query":"river","limit":1,"hints":false}))
            .await;
    let text = text_of(&result);
    let header = text.lines().next().unwrap();
    assert!(header.contains(" continuation=c1-"), "{header}");
    assert!(header.contains(" continuation_exact=false "), "{header}");
    assert!(!header.contains("state="), "epoch is not a new freshness assertion");
    harness.shutdown().await;
    engine.close().unwrap();
}

#[tokio::test]
async fn compact_orient_reports_real_projection_status_without_a_provider() {
    let fixture = Fixture::new();
    let engine = std::sync::Arc::new(
        fathomdb_sdk::Engine::open(fixture.path("host.sqlite"), Default::default()).unwrap(),
    );
    let spec = fathomdb_sdk::ProjectionSpec {
        name: "text".into(),
        roles: [fathomdb_sdk::ProjectionRole::Searchable].into_iter().collect(),
        fts: Some(fathomdb_sdk::ProjectionFts { tokenizer: None }),
        vector: None,
        source: None,
    };
    engine.configure_projections(&[spec], &[]).unwrap();
    engine.write(&[node("one", "note", "river")]).unwrap();
    let status = fathomdb_sdk::read::projection_status(&engine).unwrap();
    assert!(!status.runtime_embedder_available);
    assert_eq!(status.runtime_unavailability_reason.as_str(), "no_runtime");
    let harness = Harness::start(Config::default()).await;
    harness.adapter.register_engine("host", engine.clone()).unwrap();
    let text = text_of(&wire(&harness, "orient", json!({"store":"host"})).await);
    assert!(
        text.contains(
            "projection \"text\" roles=searchable fts=true vector=false dense=not_declared"
        ),
        "{text}"
    );
    assert!(text.contains("affected_kinds=[]"), "{text}");
    assert!(text.contains("runtime_embedder: available=false reason=no_runtime"), "{text}");
    assert!(text.contains("vector_unsupported_kinds=[]"), "{text}");
    harness.shutdown().await;
    engine.close().unwrap();
}

fn quoted_prefix(input: &str) -> (String, &str) {
    let mut decoder = serde_json::Deserializer::from_str(input).into_iter::<String>();
    let value = decoder.next().unwrap().unwrap();
    (value, &input[decoder.byte_offset()..])
}

fn fetch_frames(text: &str) -> Vec<Value> {
    let (_, mut remaining) = text.split_once('\n').expect("fetch header line");
    while remaining.starts_with("hint: ") {
        let (_, tail) = remaining.split_once('\n').expect("hint line before frames");
        remaining = tail;
    }
    let mut items = Vec::new();
    while !remaining.is_empty() {
        let (header, tail) = remaining.split_once('\n').expect("item header line");
        if let Some(fields) = header.strip_prefix("record ") {
            let (reference, fields) = quoted_prefix(fields);
            let (kind, fields) = quoted_prefix(fields.strip_prefix(" kind=").unwrap());
            let mut fields = fields.strip_prefix(" bytes=").unwrap().split_whitespace();
            let bytes: usize = fields.next().unwrap().parse().unwrap();
            let cut = fields.next() == Some("cut");
            assert_eq!(
                tail.as_bytes().get(bytes),
                Some(&b'\n'),
                "one framing LF after exactly n bytes"
            );
            let body = std::str::from_utf8(&tail.as_bytes()[..bytes]).unwrap();
            items.push(json!({"ref":reference,"kind":kind,"body":body,"cut":cut}));
            remaining = &tail[bytes + 1..];
        } else {
            let fields =
                header.strip_prefix("error ").expect("length-framed record or per-item error");
            let (reference, fields) = quoted_prefix(fields);
            items.push(json!({"ref":reference,"error":fields.trim()}));
            remaining = tail;
        }
    }
    items
}

#[tokio::test]
async fn compact_fetch_frames_exact_utf8_in_request_order() {
    let fixture = Fixture::new();
    let id = "odd \" | é\r\n😀";
    let body = "a\r\nrecord \"l:fake\" kind=\"x\" bytes=4\n😀\n";
    fixture.store("a.sqlite", vec![node(id, "note", body), node("small", "note", "small body")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let reference = format!("l:{id}");
    let result = wire(&harness,"fetch",json!({"refs":[reference,"l:missing","bad","h:x",reference,"p:7","l:small"],"as_of":1900000000,"max_tokens":100000})).await;
    let text = text_of(&result);
    assert!(
        text.starts_with("fetch store=\"a\" epoch=1 validity_instant=1900000000 items=7 tokens="),
        "{text}"
    );
    let items = fetch_frames(&text);
    assert_eq!(items.len(), 7);
    assert_eq!(items[0], json!({"ref":reference,"kind":"note","body":body,"cut":false}));
    assert_eq!(items[4], items[0], "duplicate refs preserve a separate ordered outcome");
    for (index, reference, code) in [
        (1, "l:missing", "not_found/not_found"),
        (2, "bad", "invalid_request/bad_argument"),
        (3, "h:x", "invalid_request/unsupported_ref"),
        (5, "p:7", "invalid_request/unsupported_ref"),
    ] {
        assert_eq!(items[index]["ref"], reference);
        assert!(items[index]["error"].as_str().unwrap().starts_with(code), "{}", items[index]);
    }
    assert_eq!(items[6], json!({"ref":"l:small","kind":"note","body":"small body","cut":false}));
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_error_fetch_items_keep_message_and_remedy() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("one", "note", "river")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let text = text_of(
        &wire(&harness, "fetch", json!({"refs":["l:missing","h:x","bad"],"max_tokens":100000}))
            .await,
    );
    let items = fetch_frames(&text);
    assert_eq!(items.len(), 3);
    let error = items[0]["error"].as_str().unwrap();
    assert!(
        error.contains(
            " message=\"no record is visible for this reference under the requested view\""
        ),
        "{error}"
    );
    assert!(
        error.contains(
            " remedy=\"check the reference, as_of, include_superseded and include_inactive\""
        ),
        "{error}"
    );
    for item in &items {
        assert!(item["error"].as_str().unwrap().contains(" message="), "{item}");
        assert!(item["error"].as_str().unwrap().contains(" remedy="), "{item}");
    }
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_error_unknown_store_keeps_candidates() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("one", "note", "river")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let result = wire(&harness, "search", json!({"store":"aa","query":"river"})).await;
    assert_eq!(result.is_error, Some(true));
    assert!(result.structured_content.is_none());
    let text = text_of(&result);
    assert!(text.starts_with("error invalid_request/unknown_store message="), "{text}");
    assert!(text.contains(" candidates=[\"a\"]"), "{text}");
    assert!(text.contains(" remedy="), "{text}");
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_error_sdk_invalid_filter_keeps_typed_detail() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("one", "note", "river")]);
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let result = wire(
        &harness,
        "search",
        json!({"query":"nohit","mode":"hybrid","attributes":{"unknown":"x"}}),
    )
    .await;
    assert_eq!(result.is_error, Some(true));
    let text = text_of(&result);
    assert!(text.starts_with("error invalid_request/bad_argument message="), "{text}");
    assert!(text.contains(" detail=\"InvalidFilterError\""), "{text}");
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_expand_has_one_ordered_line_per_target() {
    let fixture = Fixture::new();
    let id = "leaf \" | é\r\n😀";
    let body = "é\r\nbody | expand store=fake";
    fixture.store(
        "a.sqlite",
        vec![node("hub", "topic", "hub"), node(id, "note", body), common::edge("link", "hub", id)],
    );
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let text = text_of(
        &wire(&harness, "expand", json!({"refs":["l:hub"],"as_of":1900000000,"max_tokens":100000}))
            .await,
    );
    assert!(text.starts_with("expand store=\"a\" epoch=1 validity_instant=1900000000 targets=1 complete=true tokens="),"{text}");
    let reference = serde_json::to_string(&format!("l:{id}")).unwrap();
    assert_eq!(
        text.lines().nth(1).unwrap(),
        format!("{reference} kind=\"note\" hop=1 edge=\"link\" | é  body | expand store=fake")
    );
    assert_eq!(text.lines().count(), 2);
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_use_store_quotes_name_and_preserves_session_action() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("a", "note", "river a")]);
    fixture.store("b.sqlite", vec![node("b", "note", "river b")]);
    let harness = Harness::start(wire_config(
        &fixture,
        json!({"stores":{"a":{"path":"a.sqlite"},"b":{"path":"b.sqlite"}}}),
    ))
    .await;
    let result = wire(&harness, "use_store", json!({"store":"b"})).await;
    assert_eq!(text_of(&result), "default store: \"b\"");
    let result = wire(&harness, "search", json!({"query":"river"})).await;
    let text = text_of(&result);
    assert!(text.starts_with("search store=\"b\" "), "{text}");
    assert!(text.lines().any(|line| line.starts_with("\"l:b\" ")), "{text}");
    harness.shutdown().await;
}

#[tokio::test]
async fn compact_feedback_unique_ref_keeps_recorded_envelope() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![node("one", "note", "river crossing")]);
    let harness = Harness::start(wire_config(&fixture, json!({"telemetry_dir":"telemetry"}))).await;
    let search = text_of(&wire(&harness, "search", json!({"query":"river","mode":"hybrid"})).await);
    let response = search
        .lines()
        .next()
        .unwrap()
        .split_whitespace()
        .find_map(|word| word.strip_prefix("response_id="))
        .unwrap();
    let result = wire(&harness, "feedback", json!({"response_id":response,"used":["l:one"]})).await;
    let text = text_of(&result);
    assert!(text.starts_with("feedback store=\"a\" epoch=1 validity_instant="), "{text}");
    assert!(text.contains(" status=recorded tokens="), "{text}");
    assert!(!text.contains(" reason="), "{text}");
    let expired =
        text_of(&wire(&harness, "feedback", json!({"response_id":"expired","used":[]})).await);
    assert!(expired.contains(" status=not_recorded reason=response_expired tokens="), "{expired}");
    harness.shutdown().await;
}

#[tokio::test]
async fn structured_text_is_exact_final_payload_json_for_all_tools_and_errors() {
    let fixture = Fixture::new();
    fixture.store(
        "a.sqlite",
        vec![
            node("one", "note", "river"),
            node("two", "note", "another river"),
            common::edge("link", "one", "two"),
        ],
    );
    let harness =
        Harness::start(wire_config(&fixture, json!({"result_format":"structured"}))).await;
    let mut mismatches = Vec::new();
    for (tool, args) in [
        ("list_stores", json!({})),
        ("use_store", json!({"store":"a"})),
        ("orient", json!({})),
        ("search", json!({"query":"river","explain":true})),
        ("fetch", json!({"refs":["l:one","bad"]})),
        ("expand", json!({"refs":["l:one"]})),
        ("feedback", json!({"response_id":"expired","used":[]})),
    ] {
        let result = wire(&harness, tool, args).await;
        assert_ne!(result.is_error, Some(true), "{tool}");
        let payload = result.structured_content.as_ref().unwrap();
        if text_of(&result) != serde_json::to_string(payload).unwrap() {
            mismatches.push(format!("{tool} success"));
        }
    }
    for tool in ["list_stores", "use_store", "orient", "search", "fetch", "expand", "feedback"] {
        let result = wire(&harness, tool, json!({"unexpected":true})).await;
        assert_eq!(result.is_error, Some(true), "{tool}");
        let payload = result.structured_content.as_ref().unwrap();
        if text_of(&result) != serde_json::to_string(payload).unwrap() {
            mismatches.push(format!("{tool} error"));
        }
    }
    assert!(
        mismatches.is_empty(),
        "text must be the exact serialized final payload: {mismatches:?}"
    );
    harness.shutdown().await;
}

fn checked_wire_budget(result: &CallToolResult, format: &str, max: u64) -> Result<(), String> {
    if result.is_error == Some(true) {
        return Err(format!("unexpected refusal: {}", text_of(result)));
    }
    let text = text_of(result);
    if format == "structured" {
        let payload = result.structured_content.as_ref().ok_or("missing payload")?;
        if text != serde_json::to_string(payload).unwrap() {
            return Err("text differs from final JSON".into());
        }
        let mut p = payload.clone();
        p.as_object_mut().unwrap().remove("budget");
        let expected = (serde_json::to_string(&p).unwrap().len() + text.len()).div_ceil(4) as u64;
        if payload["budget"]["used_tokens"] != expected {
            return Err(format!(
                "structured displayed count {} expected {expected}",
                payload["budget"]["used_tokens"]
            ));
        }
        if (2 * text.len()).div_ceil(4) as u64 > max {
            return Err("both final JSON copies overflow".into());
        }
    } else {
        if result.structured_content.is_some() {
            return Err("text carried structuredContent".into());
        }
        let count = text
            .lines()
            .next()
            .unwrap()
            .split_ascii_whitespace()
            .find_map(|word| word.strip_prefix("tokens="))
            .ok_or("missing count")?;
        let (used, applied) = count.split_once('/').unwrap();
        let used = used.parse::<u64>().unwrap();
        if applied.parse::<u64>().unwrap() != max {
            return Err("max changed".into());
        }
        let expected = text.len().div_ceil(4) as u64;
        if used != expected {
            return Err(format!(
                "text count {used} expected {expected} for {} UTF8 bytes",
                text.len()
            ));
        }
        if used > max {
            return Err("final text overflows".into());
        }
    }
    Ok(())
}

#[tokio::test]
async fn final_wire_counts_include_their_own_digits_and_every_tool_field() {
    let mut failures = Vec::new();
    for format in ["text", "structured"] {
        for body_len in [1, 40, 400, 4000] {
            let fixture = Fixture::new();
            let body = format!("river {}\r\n😀", "é\\\"|".repeat(body_len));
            fixture.store(
                "a.sqlite",
                vec![
                    node("n", "note", &body),
                    node("to", "note", "river target"),
                    common::edge("link", "n", "to"),
                ],
            );
            let harness = Harness::start(wire_config(
                &fixture,
                json!({"result_format":format,"default_max_tokens":20000,"hints":true}),
            ))
            .await;
            for (tool, args) in [
                ("orient", json!({})),
                (
                    "search",
                    json!({"query":"river","limit":1,"max_tokens":20000,"highlights":true,"explain":true}),
                ),
                ("fetch", json!({"refs":["l:n","l:missing","h:x"],"max_tokens":20000})),
                ("expand", json!({"refs":["l:n"],"max_tokens":20000})),
                ("feedback", json!({"response_id":"expired","used":[]})),
            ] {
                let result = wire(&harness, tool, args).await;
                if let Err(error) = checked_wire_budget(&result, format, 20000) {
                    failures.push(format!("{format}/{body_len}/{tool}: {error}"));
                }
            }
            harness.shutdown().await;
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn budgeted_tools_refuse_mandatory_metadata_overflow() {
    let fixture = Fixture::new();
    let engine = std::sync::Arc::new(
        fathomdb_sdk::Engine::open(fixture.path("many.sqlite"), Default::default()).unwrap(),
    );
    let specs = (0..30)
        .map(|i| fathomdb_sdk::ProjectionSpec {
            name: format!("projection_{i:02}"),
            roles: [fathomdb_sdk::ProjectionRole::Rankable].into(),
            fts: None,
            vector: None,
            source: None,
        })
        .collect::<Vec<_>>();
    engine.configure_projections(&specs, &[]).unwrap();
    let mut failures = Vec::new();
    for format in ["text", "structured"] {
        let config = Config::from_json_str(
            &json!({"result_format":format,"default_max_tokens":256}).to_string(),
            fixture.dir.path(),
        )
        .unwrap();
        let harness = Harness::start(config).await;
        harness.adapter.register_engine("a", engine.clone()).unwrap();
        let result = wire(&harness, "orient", json!({"store":"a"})).await;
        if result.is_error != Some(true) {
            failures.push(format!("{format} orient returned oversized mandatory metadata"));
        } else if let Some(payload) = result.structured_content {
            assert_eq!(payload["error"]["code"], "bad_argument");
        }
        harness.shutdown().await;
    }
    engine.close().unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn final_budget_cuts_keep_fetch_outcomes_and_expand_suffix_bounds() {
    let fixture = Fixture::new();
    let body = format!("river {}\r\n😀", "é\\\"|".repeat(600));
    let mut writes = vec![node("root", "note", "river root")];
    for i in 0..8 {
        writes.push(node(&format!("n{i}"), "note", &body));
        writes.push(common::edge("link", "root", &format!("n{i}")));
    }
    fixture.store("a.sqlite", writes);
    let mut failures = Vec::new();
    for format in ["text", "structured"] {
        let harness =
            Harness::start(wire_config(&fixture, json!({"result_format":format,"hints":true})))
                .await;
        for (tool, args) in [
            ("search", json!({"query":"river","limit":8,"max_tokens":256})),
            ("fetch", json!({"refs":["l:n0","l:missing","l:n0","h:x"],"max_tokens":512})),
            ("expand", json!({"refs":["l:root"],"max_tokens":256})),
        ] {
            let result = wire(&harness, tool, args).await;
            if result.is_error == Some(true) {
                if let Some(payload) = result.structured_content {
                    assert_eq!(payload["error"]["code"], "bad_argument");
                }
                continue;
            }
            let max = if tool == "fetch" { 512 } else { 256 };
            if let Err(error) = checked_wire_budget(&result, format, max) {
                failures.push(format!("{format}/{tool}: {error}"));
            }
            if tool == "fetch" {
                if let Some(payload) = &result.structured_content {
                    assert_eq!(payload["items"].as_array().unwrap().len(), 4);
                    for i in [0, 2] {
                        assert!(body
                            .starts_with(payload["items"][i]["record"]["body"].as_str().unwrap()));
                    }
                    assert_eq!(payload["items"][1]["error"]["code"], "not_found");
                    assert_eq!(payload["items"][3]["error"]["code"], "unsupported_ref");
                } else {
                    let frames = fetch_frames(&text_of(&result));
                    assert_eq!(frames.len(), 4);
                }
            }
            if let Some(payload) = result.structured_content {
                if tool == "expand" && payload["targets"].as_array().unwrap().len() < 8 {
                    assert_eq!(payload["bound"], "budget");
                }
            }
        }
        harness.shutdown().await;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn body_cut_hint_budget_fits_complete_target_format_fetch() {
    let fixture = Fixture::new();
    let body = format!("river {}\r\n😀", "é\\\"|".repeat(600));
    let mut writes = vec![node("root", "note", "river root")];
    for i in 0..6 {
        writes.push(node(&format!("n{i}"), "note", &body));
        writes.push(common::edge("link", "root", &format!("n{i}")));
    }
    fixture.store("a.sqlite", writes);
    let mut failures = Vec::new();
    for format in ["text", "structured"] {
        let harness =
            Harness::start(wire_config(&fixture, json!({"result_format":format,"hints":true})))
                .await;
        for (tool, args) in [
            ("fetch", json!({"refs":["l:n0","l:n1","l:n2"],"max_tokens":1000,"as_of":1900000000})),
            ("expand", json!({"refs":["l:root"],"max_tokens":1000,"as_of":1900000000})),
        ] {
            let cut = wire(&harness, tool, args).await;
            assert_ne!(cut.is_error, Some(true), "{}", text_of(&cut));
            let (refs, max) = if let Some(payload) = &cut.structured_content {
                let hint = payload["hints"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|hint| hint["reason"] == "body_cut")
                    .unwrap();
                (hint["refs"].clone(), hint["max_tokens"].as_u64().unwrap())
            } else {
                let text = text_of(&cut);
                let line = text
                    .lines()
                    .find(|line| line.starts_with("hint: fetch ") && line.contains("(body_cut: "))
                    .unwrap();
                let input = line.strip_prefix("hint: fetch ").unwrap();
                let mut decoder = serde_json::Deserializer::from_str(input).into_iter::<Value>();
                let refs = decoder.next().unwrap().unwrap();
                let max = input[decoder.byte_offset()..]
                    .split_ascii_whitespace()
                    .find_map(|word| word.strip_prefix("max_tokens="))
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();
                (refs, max)
            };
            let uncut =
                wire(&harness, "fetch", json!({"refs":refs,"max_tokens":max,"as_of":1900000000}))
                    .await;
            if let Err(error) = checked_wire_budget(&uncut, format, max) {
                failures.push(format!("{format}/{tool} hinted fetch: {error}"));
            }
            if let Some(payload) = &uncut.structured_content {
                for item in payload["items"].as_array().unwrap() {
                    assert_eq!(item["record"]["body"], body);
                    assert_eq!(item["record"]["body_cut"], false);
                }
            } else {
                for frame in fetch_frames(&text_of(&uncut)) {
                    assert_eq!(frame["body"], body);
                    assert_eq!(frame["cut"], false);
                }
            }
        }
        harness.shutdown().await;
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[tokio::test]
async fn compact_filtered_pages_traverse_the_sdk_prefix_when_each_whole_hit_fits() {
    let fixture = Fixture::new();
    let writes = (0..130)
        .map(|i| node(&format!("n{i:03}"), "note", &format!("river crossing {i:03} café")))
        .collect();
    let path = fixture.store("a.sqlite", writes);
    let sdk = fathomdb_sdk::Engine::open(&path, Default::default()).unwrap();
    let mut options = fathomdb_sdk::TextSearchOptions::default();
    options.view.valid_as_of = Some(1_900_000_000);
    options.limit = 100;
    let mut filter = fathomdb_sdk::SearchFilter::default();
    filter.kind = Some("note".into());
    options.filter = Some(filter);
    let control = sdk.search_text_only("river", options).unwrap();
    assert_eq!(control.results.len(), 100);
    for hit in &control.results {
        let body = format!("river crossing {} café", &hit.id.value[1..]);
        let one=format!("search store=\"a\" epoch=1 validity_instant=1900000000 mode=text hits=1 completeness=cut_by_budget kind=\"note\" continuation=c1-0000000000000000 response_id=r1-0000000000000000 tokens=256/256 cut\n{} arm=text score={:.3} src=\"fixture\" | {body}",serde_json::to_string(&hit.id.to_prefixed()).unwrap(),hit.score);
        assert!(
            one.len().div_ceil(4) <= 256,
            "independent complete one-hit specimen establishes progress"
        );
    }
    let expected = control.results.iter().map(|hit| hit.id.to_prefixed()).collect::<Vec<_>>();
    sdk.close().unwrap();
    let harness = Harness::start(wire_config(&fixture, json!({}))).await;
    let mut args =
        json!({"query":"river","kind":"note","as_of":1900000000,"limit":5,"max_tokens":256});
    let mut collected = Vec::new();
    for _ in 0..101 {
        let result = wire(&harness, "search", args).await;
        checked_wire_budget(&result, "text", 256).unwrap();
        let text = text_of(&result);
        let header = text.lines().next().unwrap();
        let page = text.lines().skip(1).map(|row| quoted_prefix(row).0).collect::<Vec<_>>();
        if collected.len() < expected.len() {
            assert!(!page.is_empty(), "the independent budget specimen proves a whole hit fits");
        }
        collected.extend(page);
        assert_eq!(collected, expected[..collected.len()]);
        if let Some(next) =
            header.split_ascii_whitespace().find_map(|word| word.strip_prefix("continuation="))
        {
            args = json!({"continuation":next,"max_tokens":256});
        } else {
            break;
        }
    }
    assert_eq!(collected, expected, "full traversal follows only from the whole-hit fit proof");
    harness.shutdown().await;
}
