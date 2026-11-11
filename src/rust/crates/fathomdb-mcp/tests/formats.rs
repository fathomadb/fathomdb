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
    assert_eq!(text_of(&listed), "stores: 1\n\"a\" open=false readiness=closed default");
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
