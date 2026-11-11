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
