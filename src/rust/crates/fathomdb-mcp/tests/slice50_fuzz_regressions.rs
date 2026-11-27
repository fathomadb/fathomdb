//! Slice 50 (`WP-50C`, `AC32-FUZZ-03`) — Deterministic, zero-fuzz-dep regressions for
//! Surface 6 (`fuzz_mcp_jsonrpc`): `Config::from_json_str` (`SC-01`–`SC-10`, `sync_mode`)
//! and strict MCP tool argument deserialization / anti-spoof / forbidden-tool checks.

use std::path::Path;

use fathomdb_mcp::config::Config;
use fathomdb_mcp::seams::parse_tool_arguments_for_fuzz;

#[test]
fn fuzz_regression_mcp_config_malformed_and_sync_mode_conflicts() {
    let base = Path::new("/tmp/fathomdb-mcp-regressions");

    // Malformed / non-object / unknown-key JSON inputs must fail closed.
    for raw in [
        "",
        "{",
        "[]",
        "null",
        "42",
        r#"{"unknown_top_level": true}"#,
        r#"{"max_open_stores": 2}"#,
        r#"{"admission_cap": 0}"#,
        r#"{"admission_cap": 8}"#,
        r#"{"call_deadline_ms": 5000, "run_limit_ms": 1000}"#,
        r#"{"search_mode": "hybrid", "use_default_embedder": false}"#,
        r#"{"result_format": "xml"}"#,
    ] {
        assert!(Config::from_json_str(raw, base).is_err(), "must reject config: {raw}");
    }

    // SC-03: sync_mode on read-only store fails closed.
    assert!(Config::from_json_str(
        r#"{"stores":{"s1":{"path":"/tmp/a.sqlite","allow_writes":false,"sync_mode":"full"}}}"#,
        base
    )
    .is_err());

    // SC-05: sync_mode on roots fails closed.
    assert!(
        Config::from_json_str(r#"{"roots":[{"path":"/tmp/r","sync_mode":"full"}]}"#, base).is_err()
    );

    // Invalid sync_mode spelling fails closed.
    assert!(Config::from_json_str(
        r#"{"allow_writes":true,"stores":{"s1":{"path":"/tmp/a.sqlite","allow_writes":true,"scope":{"read":["a"],"write":"a"},"sync_mode":"extra"}}}"#,
        base
    )
    .is_err());

    // SC-06: conflicting sync_mode across canonical path aliases fails closed.
    assert!(Config::from_json_str(
        r#"{
            "allow_writes": true,
            "stores": {
                "a": {"path": "/tmp/same.sqlite", "allow_writes": true, "scope": {"read": ["t1"], "write": "t1"}, "sync_mode": "normal"},
                "b": {"path": "/tmp/same.sqlite", "allow_writes": true, "scope": {"read": ["t1"], "write": "t1"}, "sync_mode": "full"}
            }
        }"#,
        base
    )
    .is_err());

    // Valid write-enabled full sync_mode config succeeds.
    assert!(Config::from_json_str(
        r#"{
            "allow_writes": true,
            "stores": {
                "a": {"path": "/tmp/valid_full.sqlite", "allow_writes": true, "scope": {"read": ["t1"], "write": "t1"}, "sync_mode": "full"}
            }
        }"#,
        base
    )
    .is_ok());
}

#[test]
fn fuzz_regression_mcp_tool_args_anti_spoof_and_forbidden_tools() {
    // All 13 forbidden operator/bulk tool names must be refused unconditionally.
    for tool in [
        "recover",
        "restore",
        "repair",
        "fix",
        "rebuild",
        "purge",
        "erase_source",
        "migrate",
        "safe_export",
        "sql",
        "execute_sql",
        "bulk_write",
        "write_with_options",
    ] {
        assert!(parse_tool_arguments_for_fuzz(tool, "{}").is_err());
    }

    // Anti-spoof scope & forbidden bulk/raw-SQL keys must be refused across actuate ops,
    // including put_latest_state (regression pinned from fuzz_mcp_jsonrpc campaign harness fix).
    for forbidden in [
        "scope",
        "bulk",
        "non_strict",
        "endpoint_validation",
        "allow_dangling_edges",
        "raw_sql",
        "sql",
    ] {
        let payload = serde_json::json!({
            "operation_id": "op-1",
            "operations": [{
                "op": "put_latest_state",
                "collection": "c",
                "record_key": "k",
                "payload_json": "{}",
                forbidden: true,
            }]
        })
        .to_string();
        assert!(
            parse_tool_arguments_for_fuzz("actuate", &payload).is_err(),
            "actuate put_latest_state must refuse forbidden key {forbidden}"
        );
    }

    // Strict deny_unknown_fields + out-of-range search/fetch/expand limits fail closed.
    assert!(parse_tool_arguments_for_fuzz("search", r#"{"limit": 0}"#).is_err());
    assert!(parse_tool_arguments_for_fuzz("search", r#"{"limit": 101}"#).is_err());
    assert!(parse_tool_arguments_for_fuzz("search", r#"{"max_tokens": 10}"#).is_err());
    assert!(parse_tool_arguments_for_fuzz("search", r#"{"include_superseded": null}"#).is_err());
    assert!(parse_tool_arguments_for_fuzz("expand", r#"{"refs": []}"#).is_err());
    assert!(parse_tool_arguments_for_fuzz("use_store", r#"{"store": "../bad"}"#).is_err());
}
