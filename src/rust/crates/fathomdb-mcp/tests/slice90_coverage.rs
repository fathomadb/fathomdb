mod common;

use common::{Fixture, Harness};
use serde_json::{json, Value};

#[tokio::test(flavor = "multi_thread")]
async fn s90_orient_exposes_exact_coverage_once_in_both_formats() {
    let fixture = Fixture::new();
    fixture.store("a.sqlite", vec![]);
    for format in ["structured", "text"] {
        let harness = Harness::start(fixture.config(json!({
            "stores":{"a":{"path":"a.sqlite"}},"default_store":"a","result_format":format
        })))
        .await;
        let result = harness.call("orient", json!({})).await;
        assert_ne!(result.is_error, Some(true));
        let coverage = if let Some(payload) = result.structured_content {
            assert!(payload["projection_status"].get("vector_coverage").is_none());
            payload
                .get("vector_coverage")
                .cloned()
                .expect("orient drops stored mapped-span coverage")
        } else {
            let text = common::text_of(&result);
            let rows: Vec<_> =
                text.lines().filter_map(|line| line.strip_prefix("coverage: ")).collect();
            assert_eq!(rows.len(), 1, "orient must render coverage once: {text}");
            serde_json::from_str::<Value>(rows[0]).unwrap()
        };
        assert_eq!(coverage["population"], "stored_mapped_spans");
        assert_eq!(coverage["profile"]["key"], "default");
        assert_eq!(coverage["profile"]["dimension"], 384);
        assert_eq!(coverage["status"], "available");
        assert_eq!(coverage["records"], "0");
        assert_eq!(coverage["known_body_bytes"], "0");
        assert!(coverage["truncated_known_share"].is_null());
        assert!(coverage["unrepresented_known_byte_share"].is_null());
        harness.shutdown().await;
    }
}
