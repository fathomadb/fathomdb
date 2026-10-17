---
title: MCP adapter design (first release, adapter-only)
date: 2026-10-17
status: PROPOSED
desc: Module structure, lifecycle implementation, rmcp usage and TDD test plan for the first fathomdb-mcp release. Contract in dev/interfaces/mcp.md; semantics in requirements.md; shape in architecture.md.
inputs:
  - dev/design/mcp-adapter/requirements.md
  - dev/design/mcp-adapter/architecture.md
  - dev/interfaces/mcp.md
  - src/rust/crates/fathomdb-sdk/src/{engine,read,graph,options,error}.rs
  - src/rust/crates/fathomdb-engine/src/{telemetry,read,graph_expand/execution}.rs
---

# MCP adapter design (first release, adapter-only)

## 1. Scope

Builds `fathomdb-mcp` over `fathomdb-sdk` 0.8.27 with **no engine or SDK
change** (owner decision, 2026-10-17: evaluate the MCP work first, then propose
engine additions). In scope: every requirement in the "SDK as-is" row of
`requirements.md` § First-release scope. Out of scope here: the "needs the state
read" row (reported `unavailable`), and every `[engine]` item. Tool names,
arguments, results, configuration and errors are fixed by
`dev/interfaces/mcp.md`.

Corrections found while grounding this design, carried back to the
requirements: `NodeRecord` exposes only `logical_id`, `kind`, `body` and
`write_cursor`, and reads only active rows. So MR-013 currency is `[engine]` on
fetch as well as on search hits, and fetch cannot report a record's lifecycle
state. `graph.expand` accepts `max_depth ≤ 3`, `result_limit 1..=50` and
`max_work_units 1..=10_000`.

## 2. Crate and dependencies

`src/rust/crates/fathomdb-mcp`, a workspace member on Axis W, `publish = false`
until the evaluation is done. One library target (embedded use, MR-004) and one
binary target, `fathomdb-mcp`.

| Dependency | Version | Features | Why |
| --- | --- | --- | --- |
| `fathomdb-sdk` | workspace | `default-embedder` optional, forwarded | The only FathomDB dependency (MR-070.2). |
| `rmcp` | `=3.5.1` | `server`, `macros`, `transport-io` (dev: `client`, `transport-async-rw`) | Protocol edge. |
| `tokio` | 1 | `rt-multi-thread`, `macros`, `sync`, `time`, `io-std` | rmcp runtime; blocking hand-off. |
| `serde`, `serde_json` | 1 | `derive` | Config and results. |
| `schemars` | 1 (the version rmcp uses) | — | Tool input and output schemas, draft 2020-12 (MR-071). |
| `tracing`, `tracing-subscriber` | workspace-compatible | `fmt` to stderr | MR-002. |
| `thiserror` | 1 or 2, matching the workspace | — | Error type. |

Features: `default-embedder` (forwards to the SDK), `test-seams` (adapter-only
delay and block hooks, MR-003.1, MR-033.1). Dev-dependencies add
`jsonschema` (MR-071.1) and `proptest`. A manifest test asserts that no
`fathomdb-*` crate other than `fathomdb-sdk`, and no `rusqlite`, appears in
`[dependencies]` (MR-070.2).

## 3. Modules

| Module | Responsibility | rmcp types? |
| --- | --- | --- |
| `config` | Parse and validate the JSON config; resolve the registry of names to paths | No |
| `names` | Store-name grammar and resolution, rejecting paths (MR-005.3) | No |
| `registry` | Store slots, lifecycle state machine, epochs, idle clock, open limit (fixed at 1), LRU choice; owns `Engine`s | No |
| `bridge` | Readiness gate, admission counter, deadline, `spawn_blocking`, apply-on-open list, `ErrorKind` mapping | No |
| `session` | Default store, per-store handle tables (continuations, response ids) | No |
| `shape` | References, snippets, budget, completeness, degradation, canonical ordering, text rendering. Pure functions | No |
| `tools` | One function per tool: validate, call bridge, call shape | No |
| `edge` | `ServerHandler` with `#[tool_router]`; converts tool outcomes to `CallToolResult`; progress notifications | **Yes, only here** |
| `seams` (feature `test-seams`) | Injectable delays for open and calls | No |
| `main.rs` | CLI, stderr subscriber, config load, `serve(stdio())` | Yes |

The "only `edge` and `main` name rmcp" rule is checked by a test that greps the
crate's other source files for `rmcp::`.

## 4. Store lifecycle (MR-005, MR-006)

```text
closed ──open()──▶ opening ──ok──▶ ready ──idle/switch──▶ draining ──▶ closed
   ▲                  │                                                  │
   └──────err (record error; next call retries)───┘◀─────────────────────┘
```

- **Slot.** One `StoreSlot { name, path, state, epoch, engine: Option<Arc<Engine>>,
  in_flight: usize, last_used: Instant, last_error: Option<OpenFailure> }` per
  addressable name, behind one async mutex in `registry`. The registry also
  holds `open_set` (names whose state is not `closed`) and `limit = 1`.
- **Acquire.** A call does `registry.acquire(name)`, which returns a guard that
  increments `in_flight` and resets `last_used`, and decrements on drop.
  - `ready`: proceed.
  - `opening`, `reopening`, `draining`: return `not_ready` with the phase.
  - `closed`: if `open_set.len() == limit`, start draining the LRU member whose
    `in_flight == 0`. If none is idle, return `busy`. Then transition this slot
    to `opening` (`reopening` when `epoch > 0`), spawn the open, and return
    `not_ready`.
  - Calls that carry a progress token wait, up to the deadline, for the open to
    finish and receive progress notifications (MR-006.4). Calls without one
    return `not_ready` immediately.
- **Open.** On a blocking thread: `Engine::open(path, OpenOptions)`, then the
  apply-on-open list (§5). Success sets `ready`, `epoch += 1`. Failure records
  `last_error` and sets `closed`. The next acquire retries (MR-006.6) but returns
  the recorded error first if the retry also fails.
- **Idle.** A single tokio task ticks every second (skipped when
  `idle_timeout_secs == 0`). It drains a `ready` slot when `in_flight == 0` and
  `last_used` is older than the timeout.
- **Drain.** Set `draining`, wait for `in_flight == 0`, then on a blocking
  thread call `engine.drain(deadline_ms)` and `engine.close()`, drop the
  `Engine`, and set `closed`. A switch awaits the drain before opening the new
  store, so two files are never open at once (MR-005.7).
- **Abandoned calls** keep their guard (and so `in_flight`) until the blocking
  thread returns. That is how admission (MR-033.2) and idle (MR-005.4) both see
  them.
- **Shutdown.** When stdin closes, the edge stops accepting calls and the
  registry drains every open slot before exit.

## 5. Apply-on-open list (MR-006.2)

One ordered list, run after every successful open, first open or reopen:

1. Read `open_report()` and `dense_disabled()` into the slot's readiness.
2. If `telemetry_dir` is set: `enable_telemetry("<dir>/<store>.<pid>.<epoch>.jsonl")`.
   The engine's query ids are `q0-<seq>` and restart on each enable, so a fresh
   file per epoch keeps them unique per file (MR-024.7).
3. Record the epoch in the slot.

A test opens a store, forces an idle close, reopens, and asserts each item is
re-applied (telemetry file for the new epoch exists and receives the next
search event).

## 6. Calls through the bridge

`bridge.call(slot_guard, deadline, f)`:

1. Reject with `busy` if `in_flight` already exceeds `admission_cap`. The
   guard counts, so abandoned calls count too.
2. `tokio::task::spawn_blocking(move || f(&engine))`, raced against
   `tokio::time::sleep(deadline)`. On timeout, return `timeout` and leave the
   task running. Its guard is moved into the closure so it is released only
   when the closure returns.
3. Map `fathomdb_sdk::Error` by `ErrorKind` to the interface's error classes.
   The message is the adapter's own template, never the SDK message text (it
   may quote record content, MR-012.2). The SDK kind name is kept in `code`
   detail.
4. Catch panics from `spawn_blocking` (`JoinError::is_panic`) as
   `internal/panic` (MR-002.2).

## 7. Tool implementations

- **search.**
  - `mode=text` → `engine.search_text_only(query, TextSearchOptions{view, limit})`.
  - `mode=hybrid` → `engine.search(query, SearchOptions{filter, view, limit, explain})`.
  - When telemetry is enabled, `explain` is forced on, and the query id is taken
    from `result.explanation.correlation_id` when it starts with `q`. The
    explanation is removed from output unless the caller asked for it
    (MR-024.6).
  - The shaper builds `results` (snippet cut at `snippet_chars` on a character
    boundary, with `snippet_cut`), then `completeness`: `empty` or
    `empty_not_ready` (empty with a degradation entry, or `dense_disabled` in
    hybrid), `complete` (count < limit and no degradation), `limit_reached`, or
    `cut_by_budget`. It adds `degradation` from `soft_fallback` and
    `dense_disabled_reason`, and fixes `verdict: unavailable`.
  - Then the budget pass: drop trailing results until
    `ceil(bytes(structured + text) / 4) ≤ max_tokens`. If any were dropped, set
    `cut_by_budget` and store a continuation.
- **Continuations.** The session stores `{store, epoch, args, offset}` under
  `c<epoch>-<n>`. Continuing re-runs the same SDK call with
  `limit = min(100, offset + page)` and slices from `offset`. That is exact
  while the store is unchanged (MR-041.2, within the ranked cap). An epoch
  mismatch returns `invalid_request/stale_handle`.
- **Response ids.** `r<epoch>-<n>` maps to `{query_id?, ref → write_cursor}`
  for feedback. LRU of 256 per store.
- **fetch.** Reject non-`l:` refs per item with `unsupported_ref`. Call
  `read::get_many(engine, ids, Some(&view))` once (one SDK read). `None` gives a
  per-item `not_found`. Bodies are budgeted like snippets, with `body_cut`.
- **expand.** As in the interface doc. One `graph::expand` call. Map
  `GraphExpandResultV1.targets`, `complete` and `degradation_codes`.
- **orient.** `read::projections`, `read::projection_status`,
  `read::embedding_readiness`, `dense_disabled`. Kinds and counts are
  `unavailable`.
- **feedback.** Look up `response_id`. Return `not_recorded` with
  `response_expired` if absent, `store_reopened` if the epoch differs,
  `telemetry_disabled` if no `query_id` was captured. Otherwise map refs to
  write cursors and call `engine.record_feedback(query_id, used, unused, "agent")`.
  Refs not in the response are `invalid_request`.
- **list_stores / use_store.** Registry and session only; no engine.

## 8. rmcp usage

- `#[tool_router]` on an `FathomServer` struct holding `Arc<Registry>`,
  `Arc<Config>` and a per-connection `Session`. Each `#[tool]` takes
  `Parameters<Args>` (`Args: Deserialize + JsonSchema`) and returns
  `CallToolResult`. Structured content comes from the shaper's `serde_json::Value`
  and the text block from its renderer; `is_error` is set for tool errors.
- Output schemas: each tool's result type derives `JsonSchema`, and the router
  attaches it as `outputSchema`. The MR-071 test validates every structured
  result in the suite against it, which also guards rmcp issue #1374.
- `get_info` declares the tools capability only (no resources, prompts or
  logging) and static instructions text.
- Progress: when the request carries a progress token and the store is opening,
  the edge sends `notify_progress` at 1 s intervals until ready or deadline.
- Transport: `rmcp::transport::stdio()` in `main`; tests use
  `tokio::io::duplex` with the client side of rmcp.

## 9. TDD plan

Each requirement gets a failing test before its code. Tests drive the real
server over a duplex transport against real database files in a temp dir; no
mock engine. A shared test helper builds a config, starts the server, and
returns an rmcp client.

| Order | Test file | Covers |
| --- | --- | --- |
| 1 | `tests/config.rs` | Config grammar, defaults, invalid values (`max_open_stores ≠ 1`, `write: true`, `scope` non-null), name grammar, root resolution, path rejection (MR-005.3) |
| 2 | `tests/manifest.rs` | MR-070.2 dependency lint; rmcp confinement |
| 3 | `tests/protocol.rs` | Handshake, static `tools/list` across configs (MR-012.1, MR-006.7), stdio purity by spawning the binary (MR-002.1), panic isolation (MR-002.2), no listener (MR-001.2) |
| 4 | `tests/lifecycle.rs` | Lazy open, `not_ready` during open with test seam (MR-003.1/.2), open failure without exit and retry (MR-003.3, MR-006.6), lock conflict (MR-004.2), idle close at the timeout and never at 0 (MR-005.4), one-open-at-a-time switch (MR-005.7), reopen re-applies settings (MR-006.1/.2), epoch handling |
| 5 | `tests/bridge.rs` | Deadline returns `timeout` and keeps serving (MR-033.1); admission cap counts abandoned calls (MR-033.2) |
| 6 | `tests/search.rs` | Text and hybrid paths, filters rejected in text mode, `as_of` (MR-023), snippet cap (MR-040.1), completeness values (MR-030.1, MR-032), continuation exact on an unchanged store and stale on a new epoch (MR-041.2/.5), budget fits (MR-041.1/.3) |
| 7 | `tests/fetch_expand.rs` | Fetch order and per-item errors (MR-052), `l:` only (MR-042.2), same content as SDK (MR-040.2, MR-070.1), expand bounds (MR-053) |
| 8 | `tests/feedback.rs` | `telemetry_disabled`, recorded with the right query id under concurrent searches (MR-024.6), `store_reopened` (MR-006.5), unique ids per sink file across epochs (MR-024.7), output unchanged after feedback (MR-024.3) |
| 9 | `tests/determinism.rs` | Two process lifetimes, pinned `as_of`, byte-identical output (MR-043.1) |
| 10 | `tests/schema.rs` | Every structured result validates against its `outputSchema`; schemas are draft 2020-12 (MR-071) |
| — | `src/shape/*` unit + `proptest` | Budget never exceeded, snippet on char boundary, continuation pages have no gaps or duplicates, ordering stable |

Not covered by automated tests in this release: MR-001.3 (official
conformance suite, run manually once and recorded), MR-021 (evaluation run).

## 10. Workspace touch points

- `Cargo.toml`: add the member and the `fathomdb-mcp` workspace dependency
  entry.
- `scripts/agent-*.sh`: none expected; the new crate joins `cargo test
  --workspace`. If module-boundary or clippy configuration needs an entry, add
  it in the same change.
- No change to `fathomdb-engine`, `fathomdb-sdk`, the bindings or the CLI.
