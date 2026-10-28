//! Shared test helpers: real database files in a temp dir, a real adapter
//! served over an in-memory duplex pipe to a real rmcp client, a raw
//! line-oriented JSON-RPC client for requests without a progress token, and a
//! spawned-binary client for stdio tests. Nothing here mocks the engine.

#![allow(dead_code)]

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fathomdb_mcp::config::Config;
use fathomdb_mcp::Adapter;
use fathomdb_sdk::{Engine, InitialState, OpenOptions, PreparedWrite, SourceId};
use rmcp::model::{CallToolRequestParams, CallToolResult, ProgressNotificationParam, Tool};
use rmcp::service::{NotificationContext, RunningService};
use rmcp::{ClientHandler, RoleClient, ServiceExt};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

/// A temp directory holding the stores and config of one test.
pub struct Fixture {
    pub dir: tempfile::TempDir,
}

impl Default for Fixture {
    fn default() -> Self {
        Self::new()
    }
}

impl Fixture {
    pub fn new() -> Self {
        Self { dir: tempfile::tempdir().expect("tempdir") }
    }

    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.path().join(name)
    }

    /// Create `file` and commit `writes` through the SDK, then close it.
    pub fn store(&self, file: &str, writes: Vec<PreparedWrite>) -> PathBuf {
        let path = self.path(file);
        let engine = Engine::open(&path, OpenOptions::default()).expect("open fixture store");
        if !writes.is_empty() {
            engine.write(&writes).expect("write fixture records");
        }
        engine.close().expect("close fixture store");
        path
    }

    /// Parse `config` as if it were a file in the fixture directory.
    pub fn config(&self, config: Value) -> Config {
        Config::from_json_str(&config.to_string(), self.dir.path()).expect("valid test config")
    }

    pub fn config_file(&self, config: Value) -> PathBuf {
        let path = self.path("mcp.json");
        std::fs::write(&path, config.to_string()).expect("write config");
        path
    }
}

pub fn node(logical_id: &str, kind: &str, body: &str) -> PreparedWrite {
    sourced_node(logical_id, kind, body, "fixture")
}

pub fn sourced_node(logical_id: &str, kind: &str, body: &str, source: &str) -> PreparedWrite {
    PreparedWrite::Node {
        kind: kind.into(),
        body: body.into(),
        source_id: SourceId::new(source).expect("source id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from: None,
        valid_until: None,
    }
}

pub fn windowed_node(
    logical_id: &str,
    body: &str,
    valid_from: Option<i64>,
    valid_until: Option<i64>,
) -> PreparedWrite {
    PreparedWrite::Node {
        kind: "note".into(),
        body: body.into(),
        source_id: SourceId::new("fixture").expect("source id"),
        logical_id: Some(logical_id.into()),
        state: InitialState::Active,
        reason: None,
        valid_from,
        valid_until,
    }
}

pub fn edge(kind: &str, from: &str, to: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("fixture").expect("source id"),
        logical_id: None,
        body: None,
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}

/// Records `progress` notifications the server sends.
#[derive(Clone, Default)]
pub struct RecordingClient {
    pub progress: Arc<Mutex<Vec<ProgressNotificationParam>>>,
}

impl ClientHandler for RecordingClient {
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) {
        self.progress.lock().unwrap().push(params);
    }
}

pub type Client = RunningService<RoleClient, RecordingClient>;

/// Serve one adapter connection over a duplex pipe and connect a client.
pub async fn connect(adapter: &Adapter) -> Client {
    let (server_io, client_io) = tokio::io::duplex(1 << 20);
    let server = adapter.connection();
    tokio::spawn(async move {
        if let Ok(running) = server.serve(server_io).await {
            let _ = running.waiting().await;
        }
    });
    RecordingClient::default().serve(client_io).await.expect("client handshake")
}

/// One adapter and one connected client.
pub struct Harness {
    pub adapter: Adapter,
    pub client: Client,
    _release: SeamRelease,
}

/// Releases every held open and call when a test ends, including by panic:
/// a blocking thread parked in a seam would otherwise keep the test
/// runtime from shutting down.
struct SeamRelease(fathomdb_mcp::seams::Seams);

impl Drop for SeamRelease {
    fn drop(&mut self) {
        self.0.release_all();
    }
}

impl Harness {
    pub async fn start(config: Config) -> Self {
        let adapter = Adapter::new(config).expect("adapter");
        let client = connect(&adapter).await;
        let release = SeamRelease(adapter.seams().clone());
        Self { adapter, client, _release: release }
    }

    pub async fn call(&self, tool: &str, args: Value) -> CallToolResult {
        call(&self.client, tool, args).await
    }

    /// Call and require success; returns `structuredContent`.
    pub async fn ok(&self, tool: &str, args: Value) -> Value {
        ok(&self.client, tool, args).await
    }

    /// Call and require a tool error; returns the `error` object.
    pub async fn err(&self, tool: &str, args: Value) -> Value {
        err(&self.client, tool, args).await
    }

    pub async fn tools(&self) -> Vec<Tool> {
        self.client.list_all_tools().await.expect("tools/list")
    }

    pub async fn shutdown(self) {
        let _ = self.client.cancel().await;
        self.adapter.shutdown().await;
    }
}

pub async fn call(client: &Client, tool: &str, args: Value) -> CallToolResult {
    let Value::Object(arguments) = args else { panic!("arguments must be an object") };
    if OUTPUT_SCHEMAS.get().is_none() {
        let tools = client.list_all_tools().await.expect("tools/list");
        record_schemas(&serde_json::to_value(tools).unwrap());
    }
    let result = client
        .call_tool(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
        .unwrap_or_else(|error| panic!("{tool}: protocol error {error:?}"));
    if let Some(structured) = &result.structured_content {
        assert_output_valid(tool, structured);
    }
    result
}

/// Every tool's declared `outputSchema`, by name. The tool list is static
/// (MR-012.1), so one listing serves every test in the binary.
static OUTPUT_SCHEMAS: std::sync::OnceLock<std::collections::HashMap<String, Value>> =
    std::sync::OnceLock::new();

/// Record output schemas from a `tools/list` `tools` array.
pub fn record_schemas(tools: &Value) {
    let schemas = tools
        .as_array()
        .expect("tools array")
        .iter()
        .map(|tool| {
            let name = tool["name"].as_str().expect("tool name").to_string();
            (name, tool["outputSchema"].clone())
        })
        .collect();
    let _ = OUTPUT_SCHEMAS.set(schemas);
}

/// MR-071.1: every `structuredContent` the suite sees validates against its
/// tool's declared `outputSchema` (draft 2020-12).
pub fn assert_output_valid(tool: &str, structured: &Value) {
    let schemas = OUTPUT_SCHEMAS.get().expect("output schemas recorded before calls");
    let schema = schemas.get(tool).unwrap_or_else(|| panic!("{tool} declares no outputSchema"));
    assert!(schema.is_object(), "{tool} declares no outputSchema");
    let compiled = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(schema)
        .unwrap_or_else(|error| panic!("{tool}: outputSchema does not compile: {error}"));
    let messages: Vec<String> = match compiled.validate(structured) {
        Ok(()) => Vec::new(),
        Err(errors) => {
            errors.map(|error| format!("{} at {}", error, error.instance_path)).collect()
        }
    };
    assert!(
        messages.is_empty(),
        "{tool}: structuredContent violates its outputSchema: {messages:?}\n{structured}"
    );
}

pub async fn ok(client: &Client, tool: &str, args: Value) -> Value {
    let result = call(client, tool, args.clone()).await;
    let structured = result.structured_content.clone().expect("structuredContent");
    assert_ne!(result.is_error, Some(true), "{tool} {args} failed: {structured}");
    structured
}

pub async fn err(client: &Client, tool: &str, args: Value) -> Value {
    let result = call(client, tool, args.clone()).await;
    let structured = result.structured_content.clone().expect("structuredContent");
    assert_eq!(result.is_error, Some(true), "{tool} {args} unexpectedly succeeded: {structured}");
    structured["error"].clone()
}

/// Call through a cloned peer (usable from spawned tasks) and require a tool
/// error; returns the `error` object.
pub async fn err_peer(peer: &rmcp::Peer<RoleClient>, tool: &str, args: Value) -> Value {
    let Value::Object(arguments) = args else { panic!("arguments must be an object") };
    let response = peer
        .call_tool_once(CallToolRequestParams::new(tool.to_string()).with_arguments(arguments))
        .await
        .unwrap_or_else(|error| panic!("{tool}: protocol error {error:?}"));
    let rmcp::model::CallToolResponse::Complete(result) = response else {
        panic!("{tool}: expected a complete result");
    };
    assert_eq!(result.is_error, Some(true), "{tool} unexpectedly succeeded");
    let structured = result.structured_content.expect("structuredContent");
    assert_output_valid(tool, &structured);
    structured["error"].clone()
}

/// The text block of a result.
pub fn text_of(result: &CallToolResult) -> String {
    assert_eq!(result.content.len(), 1, "exactly one content block");
    result.content[0].as_text().expect("text block").text.clone()
}

/// The `ref`s of a search result, in order.
pub fn refs(result: &Value) -> Vec<String> {
    result["results"]
        .as_array()
        .expect("results")
        .iter()
        .map(|hit| hit["ref"].as_str().expect("ref").to_string())
        .collect()
}

/// A raw line-oriented JSON-RPC client over a duplex pipe. Its requests carry
/// no progress token, unlike the rmcp client, which always attaches one.
pub struct RawClient {
    writer: tokio::io::WriteHalf<tokio::io::DuplexStream>,
    reader: tokio::io::BufReader<tokio::io::ReadHalf<tokio::io::DuplexStream>>,
    next_id: u64,
    /// Every server notification received so far.
    pub notifications: Vec<Value>,
}

impl RawClient {
    pub async fn connect(adapter: &Adapter) -> Self {
        let (server_io, client_io) = tokio::io::duplex(1 << 20);
        let server = adapter.connection();
        tokio::spawn(async move {
            if let Ok(running) = server.serve(server_io).await {
                let _ = running.waiting().await;
            }
        });
        let (reader, writer) = tokio::io::split(client_io);
        let mut client = Self {
            writer,
            reader: tokio::io::BufReader::new(reader),
            next_id: 1,
            notifications: Vec::new(),
        };
        client.request("initialize", initialize_params()).await;
        client.notify("notifications/initialized", json!({})).await;
        let tools = client.request("tools/list", json!({})).await;
        record_schemas(&tools["result"]["tools"]);
        client
    }

    pub async fn notify(&mut self, method: &str, params: Value) {
        let line = json!({"jsonrpc": "2.0", "method": method, "params": params}).to_string();
        self.writer.write_all(format!("{line}\n").as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
    }

    /// Send a request without waiting for its response; returns its id. A
    /// later `request` skips the response when it arrives.
    pub async fn send_request(&mut self, method: &str, params: Value) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let line =
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
        self.writer.write_all(format!("{line}\n").as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
        id
    }

    /// Cancel request `id` (`notifications/cancelled`).
    pub async fn cancel(&mut self, id: u64) {
        self.notify("notifications/cancelled", json!({"requestId": id, "reason": "test"})).await;
    }

    pub async fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let line =
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
        self.writer.write_all(format!("{line}\n").as_bytes()).await.unwrap();
        self.writer.flush().await.unwrap();
        loop {
            let mut buffer = String::new();
            let read = self.reader.read_line(&mut buffer).await.unwrap();
            assert!(read > 0, "server closed the pipe");
            let message: Value = serde_json::from_str(&buffer).unwrap();
            if message["id"] == json!(id) {
                return message;
            }
            if message.get("id").is_none() {
                self.notifications.push(message);
            }
        }
    }

    /// `tools/call` without a progress token; returns the JSON-RPC `result`.
    pub async fn call(&mut self, tool: &str, args: Value) -> Value {
        let response = self.request("tools/call", json!({"name": tool, "arguments": args})).await;
        let result =
            response.get("result").cloned().unwrap_or_else(|| panic!("protocol error {response}"));
        assert_output_valid(tool, &result["structuredContent"]);
        result
    }
}

pub fn initialize_params() -> Value {
    json!({
        "protocolVersion": "2025-06-18",
        "capabilities": {},
        "clientInfo": {"name": "fathomdb-mcp-tests", "version": "0"}
    })
}

/// The `fathomdb-mcp` binary under test, speaking JSON-RPC over its stdio.
pub struct Spawned {
    pub child: Child,
    stdin: Option<ChildStdin>,
    lines: Receiver<String>,
    pub stdout_lines: Arc<Mutex<Vec<String>>>,
    pub stderr: Arc<Mutex<String>>,
    next_id: u64,
}

impl Spawned {
    pub fn start(config: &Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_fathomdb-mcp"))
            .arg("--config")
            .arg(config)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn fathomdb-mcp");
        let stdout = child.stdout.take().unwrap();
        let stderr_pipe = child.stderr.take().unwrap();
        let stdin = child.stdin.take();
        let (sender, lines) = mpsc::channel();
        let stdout_lines = Arc::new(Mutex::new(Vec::new()));
        let recorded = stdout_lines.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                recorded.lock().unwrap().push(line.clone());
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        let stderr = Arc::new(Mutex::new(String::new()));
        let sink = stderr.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr_pipe).lines() {
                let Ok(line) = line else { break };
                let mut sink = sink.lock().unwrap();
                sink.push_str(&line);
                sink.push('\n');
            }
        });
        Self { child, stdin, lines, stdout_lines, stderr, next_id: 1 }
    }

    pub fn initialize(&mut self) -> Value {
        let response = self.request("initialize", initialize_params());
        self.notify("notifications/initialized", json!({}));
        let tools = self.request("tools/list", json!({}));
        record_schemas(&tools["result"]["tools"]);
        response
    }

    pub fn send_line(&mut self, line: &str) {
        let stdin = self.stdin.as_mut().expect("stdin open");
        stdin.write_all(line.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    pub fn notify(&mut self, method: &str, params: Value) {
        let line = json!({"jsonrpc": "2.0", "method": method, "params": params}).to_string();
        self.send_line(&line);
    }

    pub fn request(&mut self, method: &str, params: Value) -> Value {
        let line = self.request_raw(method, params);
        serde_json::from_str(&line).expect("JSON-RPC response")
    }

    /// The response line exactly as the server wrote it.
    pub fn request_raw(&mut self, method: &str, params: Value) -> String {
        let id = self.next_id;
        self.next_id += 1;
        let line =
            json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string();
        self.send_line(&line);
        loop {
            match self.lines.recv_timeout(Duration::from_secs(60)) {
                Ok(line) => {
                    let message: Value = serde_json::from_str(&line)
                        .unwrap_or_else(|_| panic!("stdout line is not JSON: {line}"));
                    if message["id"] == json!(id) {
                        return line;
                    }
                }
                Err(RecvTimeoutError::Timeout) => panic!("no response to {method}"),
                Err(RecvTimeoutError::Disconnected) => {
                    panic!("server exited; stderr: {}", self.stderr.lock().unwrap())
                }
            }
        }
    }

    /// `tools/call` with a progress token, so a call to a store that is
    /// still opening waits for it (design §4) instead of returning
    /// `not_ready` at once.
    pub fn call(&mut self, tool: &str, args: Value) -> Value {
        let token = format!("p{}", self.next_id);
        let params = json!({"name": tool, "arguments": args, "_meta": {"progressToken": token}});
        let response = self.request("tools/call", params);
        let result =
            response.get("result").cloned().unwrap_or_else(|| panic!("protocol error {response}"));
        assert_output_valid(tool, &result["structuredContent"]);
        result
    }

    /// `tools/call` with a progress token; the response line as written.
    pub fn call_raw(&mut self, tool: &str, args: Value) -> String {
        let token = format!("p{}", self.next_id);
        let params = json!({"name": tool, "arguments": args, "_meta": {"progressToken": token}});
        let line = self.request_raw("tools/call", params);
        let response: Value = serde_json::from_str(&line).expect("JSON-RPC response");
        assert_output_valid(tool, &response["result"]["structuredContent"]);
        line
    }

    /// `tools/call` without a progress token.
    pub fn call_without_progress(&mut self, tool: &str, args: Value) -> Value {
        let response = self.request("tools/call", json!({"name": tool, "arguments": args}));
        let result =
            response.get("result").cloned().unwrap_or_else(|| panic!("protocol error {response}"));
        assert_output_valid(tool, &result["structuredContent"]);
        result
    }

    /// Close stdin and wait for the process to exit.
    pub fn finish(mut self) -> (std::process::ExitStatus, Vec<String>, String) {
        drop(self.stdin.take());
        let status = wait_with_timeout(&mut self.child, Duration::from_secs(60));
        // Let the reader threads drain the pipes.
        std::thread::sleep(Duration::from_millis(100));
        let lines = self.stdout_lines.lock().unwrap().clone();
        let stderr = self.stderr.lock().unwrap().clone();
        (status, lines, stderr)
    }
}

impl Drop for Spawned {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn wait_with_timeout(child: &mut Child, timeout: Duration) -> std::process::ExitStatus {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return status;
        }
        assert!(std::time::Instant::now() < deadline, "process did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Every stdout line must be a JSON-RPC 2.0 message.
pub fn assert_json_rpc_only(lines: &[String]) {
    assert!(!lines.is_empty(), "expected protocol output");
    for line in lines {
        let message: Value =
            serde_json::from_str(line).unwrap_or_else(|_| panic!("non-JSON stdout line: {line}"));
        assert_eq!(message["jsonrpc"], "2.0", "not a JSON-RPC message: {line}");
    }
}

/// `readiness` of `store` from `list_stores`.
pub async fn readiness(client: &Client, store: &str) -> String {
    let listed = ok(client, "list_stores", json!({})).await;
    listed["stores"]
        .as_array()
        .expect("stores")
        .iter()
        .find(|row| row["name"] == store)
        .map(|row| row["readiness"].as_str().expect("readiness").to_string())
        .unwrap_or_else(|| panic!("{store} is not listed: {listed}"))
}

/// Poll `list_stores` until `store` reports `want`; false on timeout.
pub async fn wait_for_readiness(
    client: &Client,
    store: &str,
    want: &str,
    timeout: Duration,
) -> bool {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if readiness(client, store).await == want {
            return true;
        }
        if std::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Whether this process can open (and so lock) the file at `path` now.
pub fn can_open(path: &Path) -> bool {
    match Engine::open(path, OpenOptions::default()) {
        Ok(engine) => {
            engine.close().expect("close probe");
            true
        }
        Err(error) if error.kind() == fathomdb_sdk::ErrorKind::DatabaseLocked => false,
        Err(error) => panic!("unexpected open error: {error}"),
    }
}

/// A node edge whose body needs an embedder: without one, `drain` reports
/// `EmbedderRequired`.
pub fn body_edge(kind: &str, from: &str, to: &str, body: &str) -> PreparedWrite {
    PreparedWrite::Edge {
        kind: kind.into(),
        from: from.into(),
        to: to.into(),
        source_id: SourceId::new("fixture").expect("source id"),
        logical_id: Some(format!("{from}->{to}")),
        body: Some(body.into()),
        t_valid: None,
        t_invalid: None,
        confidence: None,
        extractor_model_id: None,
        temporal_fallback: None,
    }
}
