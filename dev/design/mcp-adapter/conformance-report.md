---
title: MCP conformance report for fathomdb-mcp
date: 2026-10-17
status: PROPOSED
desc: Results of the official MCP conformance suite, the MCP Inspector CLI and a reference-SDK session check against the stdio fathomdb-mcp server, with each scenario classified and rmcp and bridge behavior recorded. Evidence for MR-001.3.
inputs:
  - dev/interfaces/mcp.md
  - dev/design/mcp-adapter/design.md
  - dev/design/mcp-adapter/requirements.md
  - src/rust/crates/fathomdb-mcp/src/edge.rs
  - src/rust/crates/fathomdb-mcp/src/main.rs
---

# MCP conformance report for `fathomdb-mcp`

## 1. Summary

- Date: 2026-10-17.
- Server commit: `4159c7415847c520c1ed365c196d495ad016f57d` (branch
  `exp/mcp-conformance`). The binary was built with
  `cargo build --release -p fathomdb-mcp` and no extra features, so the
  default embedder is off.
- Official suite: 32 server scenarios ran. Classified results:
  - 3 applicable and passed;
  - 0 applicable and failed;
  - 29 not applicable.
- The suite only reaches servers over Streamable HTTP. Its results passed
  through the `supergateway` stdio-to-HTTP bridge (§3).
- Direct stdio checks with no bridge also passed:
  - the MCP Inspector CLI ran `initialize`, `tools/list` and a `tools/call` of
    all seven tools;
  - a reference TypeScript SDK client ran a 15-check single session;
  - 14 raw JSON-RPC lifecycle probes ran.
- Defects found in `fathomdb-mcp`: none. No fix commits were made.
- Several rmcp 3.5.1 and bridge behaviors are recorded in §6 and §7. None was
  patched around.
- MR-001.3 asks the adapter to pass the official server scenarios that apply
  to stdio and the tools capability. It does: the three applicable scenarios
  pass.

## 2. Tools and versions

| Tool | Package and version | Role |
| --- | --- | --- |
| Conformance suite | `@modelcontextprotocol/conformance` 0.1.16 (npm `latest`; uses `@modelcontextprotocol/sdk` 1.32.1) | Official server scenarios |
| Bridge | `supergateway` 4.2.0 (uses `@modelcontextprotocol/sdk` 1.30.0) | stdio to Streamable HTTP, for the suite only |
| Inspector | `@modelcontextprotocol/inspector` 2.11.0, `--cli` mode (uses `@modelcontextprotocol/client` 2.3.1) | Second check, stdio direct |
| Reference client | `@modelcontextprotocol/sdk` 1.32.1 (`Client` + `StdioClientTransport`) | Single-session check, stdio direct |
| Runtime | Node v26.8.2, npm 12.0.2; rustc 1.95.0 | |
| Server | `fathomdb-mcp` 0.8.27 on rmcp `=3.5.1` | Under test |

All npm packages were installed into a scratch directory outside the
repository. Nothing was installed into the repository.

## 3. Transport path

- **Conformance suite.** The path was suite (HTTP client) → `supergateway
  --stateful` on `127.0.0.1:8765/mcp` → the stdio child `fathomdb-mcp
  --config mcp.json`. In stateful mode the bridge spawns one server process
  per HTTP session, so each scenario ran against a fresh server. The bridge
  only reframes JSON-RPC messages between HTTP and stdio. Scenarios that test
  HTTP transport behavior (§5, group (c)) therefore test the bridge, not this
  server. No HTTP transport was added to the crate; the contract is stdio only
  (`dev/interfaces/mcp.md` §1, MR-001.2).
- **Inspector, SDK session check and raw probes.** These spawned the release
  binary over stdio directly. No bridge was involved.

## 4. Fixture and commands

The test store is a real file written through `fathomdb-sdk` by a scratch
binary. It holds three nodes (`note:1`, `note:2` and `doc:1`, source
`conformance`) and two edges (`note:1 -mentions-> note:2` and
`note:2 -cites-> doc:1`). The config is:

```json
{"stores": {"demo": {"path": "demo.sqlite"}}, "default_store": "demo"}
```

In the commands below, `$BIN` is the release `fathomdb-mcp` binary and
`$CFG` is that config file.

```sh
# Install (scratch dir)
npm install @modelcontextprotocol/conformance@0.1.16 \
  @modelcontextprotocol/inspector@2.11.0 supergateway@4.2.0

# Bridge
supergateway --stdio "$BIN --config $CFG" --outputTransport streamableHttp \
  --stateful --host 127.0.0.1 --port 8765 --logLevel debug

# Official suite: every server scenario, active and pending
conformance server --url http://127.0.0.1:8765/mcp --suite all -o results-all

# Inspector CLI, stdio direct (one server process per invocation)
mcp-inspector --cli --method initialize --format json -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/list --strict --format json -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name list_stores --format json -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name use_store \
  --tool-args-json '{"store":"demo"}' --format json -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name orient \
  --tool-args-json '{"store":"demo"}' --format json -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name search \
  --tool-args-json '{"store":"demo","query":"alpha river"}' --format json -q -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name fetch \
  --tool-args-json '{"store":"demo","refs":["l:note:1","l:doc:1","l:missing:9","bogus"]}' \
  --format json -q -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name expand \
  --tool-args-json '{"store":"demo","refs":["l:note:1"],"max_depth":2,"direction":"both"}' \
  --format json -q -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name feedback \
  --tool-args-json '{"store":"demo","response_id":"r1-1943bde14d0a7dc5","used":["l:note:1"],"unused":["l:doc:1"]}' \
  --format json -q -- "$BIN" --config "$CFG"
# Error paths: unknown store, wrong argument type, unknown tool
mcp-inspector --cli --method tools/call --tool-name search \
  --tool-args-json '{"store":"nope","query":"alpha"}' --format json -q -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name search \
  --tool-args-json '{"query":123}' --format json -q -- "$BIN" --config "$CFG"
mcp-inspector --cli --method tools/call --tool-name drop_store \
  --tool-args-json '{}' --format json -q -- "$BIN" --config "$CFG"

# Reference-SDK single session and raw JSON-RPC probes (scratch scripts)
node session-check.mjs "$BIN" "$CFG"
python3 -I raw_probe.py "$BIN" "$CFG"
```

The suite reported 9 passed and 24 failed checks across the 32 scenarios.
§5 classifies each scenario against what this server declares. A raw "pass"
is not taken as applicable by itself.

## 5. Official suite: scenario table

Group (a) is applicable and passed, (b) is applicable and failed, and (c) is
not applicable. "Suite" is the raw verdict the tool printed.

| Scenario | Suite | Group | Note |
| --- | --- | --- | --- |
| `server-initialize` | pass | (a) | Two-phase handshake. Server answered `protocolVersion` 2025-11-25, `capabilities {"tools":{}}`, `serverInfo` and `instructions`. |
| `ping` | pass | (a) | Empty `{}` result. |
| `tools-list` | pass | (a) | Seven tools, each with name, description and `inputSchema`. |
| `logging-set-level` | fail (-32601) | (c) | Logging is not declared; `get_info` declares tools only (design §8; interface §1 sends diagnostics to stderr). Method-not-found is the correct answer. |
| `completion-complete` | pass | (c) | Completions are not declared (design §8). The pass is vacuous: rmcp 3.5.1's default `complete` handler returns an empty result (§6.1). |
| `tools-call-simple-text` | fail (-32602) | (c) | Needs the reference "everything" fixture tool `test_simple_text`. Our tool set is fixed at seven (MR-006.7). Each tool returns a text block, as the Inspector run shows. |
| `tools-call-image` | fail (-32602) | (c) | Fixture tool `test_image_content`. The server returns no image content. |
| `tools-call-audio` | fail (-32602) | (c) | Fixture tool `test_audio_content`. The server returns no audio content. |
| `tools-call-embedded-resource` | fail (-32602) | (c) | Fixture tool `test_embedded_resource`. Resources are not declared (design §8). |
| `tools-call-mixed-content` | fail (-32602) | (c) | Fixture tool `test_multiple_content_types`. Results are one text block plus `structuredContent` (interface §3). |
| `tools-call-with-logging` | fail (-32601) | (c) | Calls `logging/setLevel` first; logging is not declared (design §8). |
| `tools-call-error` | fail (-32602) | (c) | Fixture tool `test_error_handling`. The same behavior was checked on our own tools: `isError: true` with a text message and schema-valid `structuredContent` (Inspector and SDK checks, §8). |
| `tools-call-with-progress` | fail (-32602) | (c) | Fixture tool `test_tool_with_progress`. This server sends progress only while a call waits on an opening store (design §8). That path is covered by the crate's Rust tests (`tests/lifecycle.rs`), not by a fixture tool. |
| `tools-call-sampling` | fail (-32602) | (c) | Fixture tool `test_sampling`. The server never requests sampling. |
| `tools-call-elicitation` | fail (-32602) | (c) | Fixture tool `test_elicitation`. The server never elicits. |
| `json-schema-2020-12` | fail (tool not found) | (c) | Fixture tool `json_schema_2020_12_tool`. The property it checks does hold for our tools: every `inputSchema` keeps `$schema` (`https://json-schema.org/draft/2020-12/schema`) and `additionalProperties: false`, and `expand` and `search` keep `$defs` (Inspector `tools/list`). |
| `elicitation-sep1034-defaults` | fail (-32602) | (c) | Fixture tool `test_elicitation_sep1034_defaults`. The server never elicits. |
| `elicitation-sep1330-enums` | fail (-32602) | (c) | Fixture tool `test_elicitation_sep1330_enums`. The server never elicits. |
| `server-sse-polling` | no checks (warnings) | (c) | Streamable HTTP SSE resumability through the `test_reconnection` fixture tool. It tests the bridge's HTTP layer; the server is stdio only (interface §1). Its warnings (no priming event, no `retry` field) are the bridge's (§7). |
| `server-sse-multiple-streams` | pass | (c) | Concurrent POST SSE streams. This is the bridge's HTTP layer, not the server. |
| `resources-list` | pass | (c) | Resources are not declared (design §8). The pass is vacuous: rmcp's default `list_resources` returns an empty list (§6.1). |
| `resources-read-text` | fail (-32601) | (c) | Resources are not declared. |
| `resources-read-binary` | fail (-32601) | (c) | Resources are not declared. |
| `resources-templates-read` | fail (-32601) | (c) | Resources are not declared. |
| `resources-subscribe` | fail (-32601) | (c) | Resources are not declared. |
| `resources-unsubscribe` | fail (-32601) | (c) | Resources are not declared. |
| `prompts-list` | pass | (c) | Prompts are not declared (design §8). The pass is vacuous: rmcp's default `list_prompts` returns an empty list (§6.1). |
| `prompts-get-simple` | fail (-32601) | (c) | Prompts are not declared. |
| `prompts-get-with-args` | fail (-32601) | (c) | Prompts are not declared. |
| `prompts-get-embedded-resource` | fail (-32601) | (c) | Prompts are not declared. |
| `prompts-get-with-image` | fail (-32601) | (c) | Prompts are not declared. |
| `dns-rebinding-protection` | fail (1 of 2 checks) | (c) | A Host/Origin check on a localhost HTTP listener. The server has no listener (MR-001.2). The failing check is the bridge's (§7). |

Counts: (a) 3, (b) 0, (c) 29.

## 6. rmcp 3.5.1 behavior

None of these is a defect in `fathomdb-mcp`. None was patched around.

1. **Undeclared list and complete methods answer with empty results.**
   rmcp's default `ServerHandler` returns an empty `CompleteResult`,
   `ListPromptsResult`, `ListResourcesResult` and
   `ListResourceTemplatesResult` (rmcp `src/handler/server.rs`, default
   `complete`, `list_prompts`, `list_resources` and `list_resource_templates`).
   It does this even though the server declares none of those capabilities.
   `resources/read`, `prompts/get`, `logging/setLevel` and
   `resources/subscribe` answer -32601. The empty answers break no rule of the
   spec, and they explain the three vacuous suite passes in §5.
2. **A malformed JSON line gets no reply.** A line that is not JSON gets no
   -32700 Parse error. The line is dropped and the session keeps serving: a
   later `ping` is answered. JSON-RPC 2.0 §5.1 expects a parse-error response
   with `id: null`. The adapter has no framing code of its own
   (`rmcp::transport::stdio()` in `main.rs`).
3. **Malformed `tools/call` params get the wrong error code.** A request
   missing `name`, or with a non-object `arguments`, gets -32601 with message
   `tools/call` instead of -32602 Invalid params. rmcp fails to decode the
   typed request before any handler runs. Well-formed calls with bad argument
   values reach the adapter and get the typed `invalid_request/bad_argument`
   tool error (design §8).
4. **Pre-initialize requests use the inline-lifecycle path.** A `tools/list`
   sent before `initialize` is treated as a 2026-07-28 inline-lifecycle
   request. It gets -32602 "request _meta is missing … protocolVersion,
   clientCapabilities". A `ping` before `initialize` is answered `{}`.
5. **`tools/list` ignores an unknown cursor.** It returns the full list where
   the pagination utility says an invalid cursor SHOULD get -32602. The
   handler comes from the `#[tool_handler]` macro. With seven tools and no
   pagination, this has no practical effect.
6. **Version negotiation is correct.** A requested 2025-06-18 or 2025-03-26 is
   echoed back. An unknown version (1999-01-01) gets the server's latest
   supported version, 2025-11-25.

Schema notes come from schemars 1, not rmcp, and are not protocol defects:

- Integer fields carry `format` values `uint32`, `uint64` and `int64`. These
  are not registered 2020-12 formats. The TS SDK's AJV logs `unknown format
  "uint32" ignored`, and in 2020-12 an unknown format is only an annotation.
  Every `structuredContent` returned in these runs validated against its
  `outputSchema`; the TS clients validate it on each call.
- `inspector --strict` reported 51 portability warnings and 0 errors, so it
  exited 0. Each warning is an optional field written as `"type": [T,
  "null"]`. Clients that map tool schemas onto a single-`type` dialect, such as
  the OpenAPI subset used for Gemini function declarations, may reject or
  loosen these fields. This is a candidate follow-up. It does not violate the
  contract.

## 7. Bridge (`supergateway` 4.2.0) behavior

- `dns-rebinding-protection`: the bridge accepted a request with Host and
  Origin `evil.example.com` and answered HTTP 200. It does not validate
  Host/Origin.
- `server-sse-polling`: the bridge sends no SSE priming event and no `retry`
  field. These are warnings, not failures.
- With `--stateful`, every session's `fathomdb-mcp` child stayed alive after
  the run, until the bridge's 30-minute session timeout or its own exit. The
  suite does not end its sessions.

## 8. Direct stdio checks (no bridge)

**Inspector CLI 2.11.0.**

- `initialize` returned `protocolVersion` 2025-11-25, `capabilities
  {"tools":{}}` and the server info and instructions.
- `tools/list --strict` returned the seven tools, with exit code 0 (§6).
- `tools/call` returned `isError: false` for each tool, with a text block and
  `structuredContent` that validated against the tool's `outputSchema`:
  - `list_stores`, `use_store` and `orient`;
  - `search`: three results;
  - `fetch`: two records plus `not_found` and `bad_argument` per-item errors;
  - `expand`: two targets at hops 1 and 2;
  - `feedback`: `not_recorded` / `response_expired`, because each Inspector
    invocation is a new process and session.
- Error paths:
  - an unknown store gave `isError: true` with `unknown_store` and
    candidates;
  - an integer `query` gave `isError: true` with `bad_argument`;
  - an unknown tool was reported as not found.

**Reference SDK session (`@modelcontextprotocol/sdk` 1.32.1), 15 of 15
passed:**

- the handshake and declared capabilities (tools only), and `ping`;
- `tools/list` (seven tools, all with `outputSchema`);
- in one session: `list_stores`; `use_store` then `orient` with no `store`
  (the session default applied); `search` with a progress token; a
  continuation page; `fetch`; `expand`; `feedback` on that session's
  `response_id` (`not_recorded` / `telemetry_disabled`, since no
  `telemetry_dir` was configured);
- tool errors carried `isError` with schema-valid `structuredContent`, for an
  unknown store and for a wrong argument type;
- an unknown tool gave JSON-RPC -32602 and an unknown method gave -32601;
- after an aborted call (`notifications/cancelled`), the session kept serving.

**Raw JSON-RPC probes, 14 probes:**

- version negotiation, as in §6.6;
- a string request `id` was echoed;
- `tools/call` with `arguments` omitted succeeded;
- an unknown notification, and a cancel for an unknown request id, were
  ignored and serving continued;
- a JSON-RPC batch got -32600 and serving continued (batching was removed in
  2025-06-18);
- closing stdin after `initialize` exited 0 with empty stderr;
- the rest are the rmcp behaviors in §6.2 to §6.5.

## 9. Defects and fixing commits

None. No scenario fell into group (b). The direct checks found no defect in
`fathomdb-mcp` either, so this run made no `test(mcp): RED` or
`fix(mcp): GREEN` commits.

Two points came from reading the code. They were not reproduced and are
recorded for review:

- **Progress ordering.** `edge.rs` sends each progress notification from its
  own `tokio::spawn` task. Two ticks issued back to back could reach the
  transport out of order, for example when the phase changes from draining to
  opening without the 1 s wait. The progress utility requires the value to
  increase with each notification. No probe or test showed a reordering.
- **Exit code on early disconnect.** When the client closes stdin before
  `initialize`, the process exits 1 and logs `MCP initialization failed`.
  Interface §1 gives 0 for normal shutdown (stdin closed) and 1 for any other
  startup failure. A disconnect before the handshake falls in the second
  class.

## 10. What remains manual

- Progress notifications against a store that is slow to open. The suite has
  no applicable scenario, and the crate's seam-driven Rust tests cover it.
- Hybrid search and telemetry-recorded feedback. These need a
  `default-embedder` build, a `telemetry_dir` and a downloaded model; this
  binary had none of them.
- Real host clients: Claude Code, Claude Desktop and other MCP hosts that
  launch stdio servers.
- Rerunning this report when rmcp, the suite or the contract changes. The
  suite is marked unstable upstream, and its scenario set changes between
  releases.
