//! Exit codes of the built binary around the MCP handshake
//! (`dev/interfaces/mcp.md` § 1): stdin closing before or after
//! `initialize` is a normal shutdown.

mod common;

use common::{node, Fixture, Spawned};
use serde_json::json;

fn config(fixture: &Fixture) -> std::path::PathBuf {
    fixture.store("a.sqlite", vec![node("note:a", "note", "alpha river crossing")]);
    fixture.config_file(json!({"stores": {"a": {"path": "a.sqlite"}}, "default_store": "a"}))
}

#[test]
fn stdin_closed_before_initialize_exits_zero() {
    let fixture = Fixture::new();
    let server = Spawned::start(&config(&fixture));
    let (status, lines, stderr) = server.finish();
    assert_eq!(status.code(), Some(0), "stdin closed with no bytes written; stderr: {stderr}");
    assert!(lines.is_empty(), "nothing on stdout: {lines:?}");
}
