mod common;

use common::Harness;
use serde_json::{json, Value};

#[tokio::test(flavor = "multi_thread")]
async fn s90_unknown_argument_is_schema_derived_and_precedes_bad_type() {
    let harness = Harness::start(common::structured_config()).await;
    let tools = serde_json::to_value(harness.tools().await).unwrap();
    for tool in tools.as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        let mut accepted: Vec<_> =
            tool["inputSchema"]["properties"].as_object().unwrap().keys().cloned().collect();
        accepted.sort();
        let error = harness.err(name, json!({"zzz": 1, "aaa": 2, "hints": "wrong"})).await;
        assert_eq!(error["code"], "unknown_argument", "{name}: {error}");
        assert_eq!(error["argument"], "aaa");
        assert_eq!(error["accepted"], json!(accepted));
        assert_eq!(error["message"], "unknown tool argument");
        let known_only = harness.err(name, json!({"hints": "wrong"})).await;
        assert_eq!(known_only["code"], "bad_argument");
        assert!(known_only.get("accepted").is_none());
    }
    for (key, expected) in [
        ("x".repeat(64), "x".repeat(64)),
        ("é".repeat(33), format!("{}…", "é".repeat(30))),
        (format!("{}é", "x".repeat(63)), format!("{}…", "x".repeat(61))),
    ] {
        let args: Value = json!({key: true});
        let error = harness.err("list_stores", args).await;
        assert_eq!(error["argument"], expected);
        assert!(error["argument"].as_str().unwrap().len() <= 64);
    }
    harness.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn s90_unknown_argument_text_preserves_bounded_json_details() {
    let harness = Harness::start(Default::default()).await;
    let result = harness.call("fetch", json!({"limit": 1, "refs": []})).await;
    let text = common::text_of(&result);
    assert!(text.starts_with("error invalid_request/unknown_argument "), "{text}");
    assert!(text.contains("argument=\"limit\" accepted=["), "{text}");
    assert!(!text.contains("hints:"));
    harness.shutdown().await;
}
