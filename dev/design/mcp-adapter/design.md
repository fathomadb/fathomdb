---
title: MCP adapter design (production read+write, 0.8.32)
date: 2026-11-26
status: ACTIVE
desc: Module structure, lifecycle implementation, rmcp confinement, opt-in scoped write safety (SC-01..SC-10, AC32-MCP-SAFETY-01..09), frozen evidence, and 3-channel distribution design for fathomdb-mcp. Contract in dev/interfaces/mcp.md; semantics in requirements.md.
inputs:
  - dev/adr/ADR-0.8.32-mcp-read-write-and-distribution.md
  - dev/adr/ADR-0.8.32-caller-scope-isolation.md
  - dev/design/mcp-adapter/requirements.md
  - dev/design/mcp-adapter/architecture.md
  - dev/interfaces/mcp.md
  - src/rust/crates/fathomdb-sdk/src/{engine,read,graph,options,error}.rs
  - src/rust/crates/fathomdb-engine/src/{telemetry,read,graph_expand/execution,actuation}.rs
---

# MCP adapter design (`fathomdb-mcp`)

## 1. Scope

Graduated in `0.8.32` (`dev/adr/ADR-0.8.32-mcp-read-write-and-distribution.md`)
as FathomDB's production read+write MCP server over `fathomdb-sdk`, preserving
`MR-070.2` (only `fathomdb-sdk` in runtime dependencies; zero `rusqlite` or
direct `fathomdb-engine` linkage) and `rmcp = "=3.5.1"` confinement to
`src/edge.rs` and `src/main.rs`.

- **Default read-only posture (`allow_writes: false`):** Read-only profiles
  expose the exact `7`-tool surface (`list_stores`, `use_store`, `orient`,
  `search`, `fetch`, `expand`, `feedback`), integrating bounded exact
  frozen-evidence retrieval via optional parameters on `search`, `expand`, and
  `fetch`.
- **Per-store write opt-in & `SC-01`–`SC-10`:** Write-enabled profiles expose
  `9` tools (adding `actuate` and `put_latest_state`) only when at least one
  store enables `allow_writes: true` with an explicit non-empty writable caller
  `ScopePolicy`. Startup conflicts (`SC-01`–`SC-10`) fail fast before opening
  any database or `.lock` file.
- **Pin C SDK seams:** Uses `EP-07` (`OpenOptions.create_if_missing`, closing
  the check-then-open race), `EP-01` (`read::visibility_state`, populating live
  `Envelope.state`), and atomic `ActuationOperationV1::PutLatestState` inside
  `ActuationBatchV1`.
- **Write safety & drain fence (`AC32-MCP-SAFETY-01`–`09`):** Enforces
  per-canonical-path write serialization, idempotent `(store, scope, operation_id)`
  replay, CAS `expected_write_boundary`, scope-keyed continuations/sessions,
  strict-only atomic rollback, blocking cancellation/shutdown drain fencing,
  template-only secret-scrubbed errors, and zero operator escape hatches.

Tool names, arguments, results, configuration, and errors are owned by
`dev/interfaces/mcp.md`.

## 2. Crate and dependencies

`src/rust/crates/fathomdb-mcp`, a publishable workspace member on Axis W
(`publish = true`, `default = []` on `crates.io`). Prebuilt 5-target GitHub
Release archives and lockstep PyPI `fathomdb-mcp` wheels (`src/python-mcp/`,
exposed via `fathomdb[mcp]`) build with `--features default-embedder`
(`Gate1-D5`). One library target (embedded use, MR-004) and one binary target,
`fathomdb-mcp`.

| Dependency | Version | Features | Why |
| --- | --- | --- | --- |
| `fathomdb-sdk` | workspace | `default-embedder` optional, forwarded | The only FathomDB dependency (MR-070.2). |
| `rmcp` | `=3.5.1` | `server`, `macros`, `transport-io` (dev: `client`, `transport-async-rw`) | Protocol edge. |
| `tokio` | 1 | `rt-multi-thread`, `macros`, `sync`, `time`, `io-std` | rmcp runtime; blocking hand-off. |
| `serde`, `serde_json` | 1 | `derive` | Config and results. |
| `schemars` | 1 (the version rmcp uses) | — | Tool input and output schemas, draft 2020-12 (MR-071). |
| `tracing`, `tracing-subscriber` | workspace-compatible | `fmt` to stderr | MR-002. |
| `thiserror` | 1 or 2, matching the workspace | — | Error type. |
| `sha2` | already in the lockfile | — | Deterministic response and continuation handles (MR-043). |

Features: `default-embedder` (forwards to the SDK), `test-seams` (adapter-only
delay and block hooks, MR-003.1, MR-033.1). Dev-dependencies add
`jsonschema` (MR-071.1) and `proptest`. A manifest test asserts that no
`fathomdb-*` crate other than `fathomdb-sdk`, and no `rusqlite`, appears in
`[dependencies]` (MR-070.2).

## 3. Modules

| Module | Responsibility | rmcp types? |
| --- | --- | --- |
| `actuate` | Governed opt-in write (`actuate` / `put_latest_state`) validation, CAS boundary checks, idempotent replay, and strict SDK actuation dispatch | No |
| `config` | Parse and validate the JSON config (`SC-01`–`SC-10`); resolve the registry of names to paths and `ScopePolicy` | No |
| `error` | Adapter error taxonomy, secret-scrubbed templates, and `fathomdb_sdk::Error` mapping | No |
| `hints` | Deterministic, data-free `hints` sidecar generation (`MR-072`) | No |
| `hooks` | Internal lifecycle/telemetry observer hooks | No |
| `names` | Store-name grammar and resolution, rejecting paths (MR-005.3) | No |
| `output` | Compact text and structured JSON envelope formatting and budget rendering | No |
| `registry` | Store slots, lifecycle state machine, epochs, idle clock, open limit (fixed at 1), LRU choice; owns `Engine`s | No |
| `bridge` | Readiness gate, admission counter, deadline, `spawn_blocking`, apply-on-open list, `ErrorKind` mapping | No |
| `session` | Default store, per-store handle tables (continuations, frozen contexts, response ids) | No |
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
  in_flight: Arc<AtomicUsize>, idle: Arc<Notify>, last_used, last_error }` per
  addressable name. The registry also holds `open_set` (names whose state is not
  `closed`) and `limit = 1`.
- **Mutex discipline.** One registry mutex (a `std::sync::Mutex`) guards state
  transitions only. It is **never held across an `.await`** or a blocking call.
  Checking that a slot is `ready` and incrementing `in_flight` happen in one
  critical section, so the idle task cannot mark a slot `draining` between
  them. Dropping a guard decrements `in_flight` without taking the mutex and
  calls `idle.notify_waiters()` when the count reaches zero.
- **Acquire.** A call does `registry.acquire(name)`, which returns a guard that
  holds an increment of `in_flight` and reset `last_used`.
  - `ready`: proceed.
  - `opening`, `reopening`, `draining`: wait (below).
  - `closed`: if `open_set.len() == limit`, start draining the LRU member whose
    `in_flight == 0`. If none is idle, return `busy`. Then transition this slot
    to `opening` (`reopening` when `epoch > 0`), spawn the open, and wait.
  - Every call waits, up to its call deadline, for an open or reopen in
    progress to finish, whether or not it carries a progress token. Only calls
    that carry a token receive progress notifications while they wait
    (MR-006.4). At the deadline the call returns `not_ready` with the phase
    (owner decision, 2026-10-17, FIX-1 CR-1).
- **Open.** On a blocking thread, `Engine::open(path, OpenOptions)` passes
  `create_if_missing(entry.create)` (`EP-07`), failing atomically with
  `DatabaseNotFound` → `unavailable/missing_file` before lock/database creation
  or embedder/CUDA initialization when `create: false` and the file is missing
  or zero-length (closing the pre-`0.8.32` check-then-open race), followed by
  the apply-on-open list (§5). Success sets `ready`, `epoch += 1`. Failure
  records `last_error` and sets `closed`. The next acquire retries (MR-006.6)
  but returns the recorded error first if the retry also fails.
- **Idle.** A single tokio task ticks every second (skipped when
  `idle_timeout_secs == 0`). It drains a `ready` slot when `in_flight == 0` and
  `last_used` is older than the timeout.
- **Drain.** Under the mutex, move `ready → draining`, then release it. Await
  `idle.notified()` until `in_flight == 0`, without holding the mutex. Then on
  a blocking thread call `engine.drain(deadline_ms)`, and call `engine.close()`
  **whether or not `drain` succeeded**. `drain` returns `EmbedderRequired` when
  vector work is pending without an embedder, and `Scheduler` on timeout; a
  failure is logged to stderr, never allowed to keep the file locked. Drop the
  `Engine`, then under the mutex move `draining → closed`. A switch awaits the drain before opening the new
  store, so two files are never open at once (MR-005.7).
- **Abandoned calls** keep their guard (and so `in_flight`) until the blocking
  thread returns. That is how admission (MR-033.2) and idle (MR-005.4) both see
  them.
- **Shutdown.** When stdin closes, the edge stops accepting calls and the
  registry drains every open slot before exit. It waits at most
  `call_deadline_ms` + 5 s; a store whose abandoned call is still running
  after that is left to process exit.

## 5. Apply-on-open list (MR-006.2)

One ordered list, run after every successful open, first open or reopen:

1. Read `read::embedding_readiness` into the slot's open info:
   `usable_embedder` decides whether hybrid searches carry the
   `{arm: "vector", reason: "embedder_unavailable"}` degradation. It is sampled
   once per open epoch, not per call; a host engine is sampled once, at
   registration. `dense_disabled()` is read live on each hybrid search.
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

- **Validity instant.** Every store-bound call resolves `as_of` (or now, in
  epoch seconds, read once at the start of the call) and passes it as
  `ReadView.valid_as_of`. The value is reported as `validity_instant` (MR-023.2).
- **search.**
  - `mode=text` → `engine.search_text_only(query, options)`, `options` a
    `TextSearchOptions::default()` with the complete closed `filter`, `view`,
    `limit` and snippet options set. Predicates are applied in the engine before
    caps/ranking; the adapter never filters governed results. This path never captures telemetry, so
    its responses carry no query id, and feedback on them returns `text_mode`.
  - `mode=hybrid` → `engine.search(query, options)`, `options` a
    `SearchOptions::default()` with `filter`, `view`, `limit` and `explain`
    set.
    When telemetry is enabled, `explain` is forced on, and the query id is taken
    from `result.explanation.correlation_id` when it starts with `q`. The
    explanation is removed from output unless the caller asked for it
    (MR-024.6).
  - Search forwards include_superseded and include_inactive independently through
    the same ReadView in both modes;
    continuations retain its canonical value and refuse explicit overrides.
    Distinct historical revisions retain their ordered occurrences. The selected
    atomic feedback ambiguity guard is implemented; fetch remains latest-eligible
    logical lookup. Supplied search flags must be booleans; null is rejected
    before SDK dispatch, including on continuations. No out-of-window flag is exposed.
  - The shaper builds `results` from the engine's snippet sidecar (EP-17:
    `snippet`, `snippet_cut` and `snippet_anchor` from `snippets[i]`, requested
    with `max_bytes: snippet_bytes`), then `completeness`: `empty` or
    `empty_not_ready` (empty with a degradation entry, or `dense_disabled` in
    hybrid), `complete` (count < limit and no degradation), `limit_reached`, or
    `cut_by_budget`. It adds `degradation` from `soft_fallback` and
    `dense_disabled_reason`, and fixes `verdict: unavailable`.
  - Then the budget pass: drop trailing results until
    the selected format's final-byte predicate holds (interface §3). If any were dropped, set
    `cut_by_budget` and store a continuation.
- **Continuations & frozen-context sessions.** Unfrozen continuations store
  `{store, epoch, args, offset}` under `c<epoch>-<hash>`, with the hash as in
  the interface §4 (SHA-256 over store, canonical arguments and offset, first 16
  hex digits). Continuing re-runs the same SDK call with
  `limit = min(100, offset + page)` and slices from `offset`. That is exact
  while the store is unchanged (MR-041.2, within the ranked cap). An epoch
  mismatch on an unfrozen continuation returns `invalid_request/stale_handle`.
  By contrast, cached `FrozenReadContextV1` sessions (`ADR-0.8.32` `#103` §1.4)
  are durable across MCP close/reopen (`expire_idle` or `max_open_stores = 1`
  LRU eviction) and MUST NOT reject calls via global `visibility != entry.minted_state`
  pre-gates, premature `entry.epoch != guard.epoch` checks, or global
  `after != visibility` / `after != minted_state` post-error checks across
  either reused or freshly minted `frozen_ctx` handles. Instead, `search`,
  `fetch` (`resolve_evidence_many` and `graph_refs` `resolve_graph_evidence`),
  and `expand` pass `frozen_ctx` directly to `fathomdb-sdk` so Schema 39
  partitioned `validate_snapshot` decides drift; `ErrorKind::FrozenRead(StateDrifted)`
  maps directly to `state_drift`, and cold-path `ErrorKind::Evidence` while a
  `frozen_ctx` is active is classified strictly through `MR-070.2`-compliant
  frozen SDK probes:
  - `search` (`engine.search_with_evidence`): probes
    `engine.search_frozen(&ev_req.query, &frozen_ctx, opts)` with matching
    `opts.use_graph_arm = ev_req.use_graph_arm` and `opts.limit = 1`;
  - `fetch` (`engine.resolve_evidence_many`): runs a two-step probe —
    `engine.search_frozen("probe", &frozen_ctx, opts)` with
    `use_graph_arm = false, limit = 1` (`SearchNoGraph`/`EvidenceNode`), and
    if `Ok(_)`, `use_graph_arm = true, limit = 1` (`GraphOrEdge`/`EvidenceGraph`
    TOCTOU `edge_any`);
  - `fetch` `graph_refs` (`engine.resolve_graph_evidence`): maps
    `ErrorKind::FrozenRead(StateDrifted)` immediately to whole-call
    `FetchEvidenceExec::Drifted` and probes `ErrorKind::Evidence` via
    `engine.search_frozen("probe", &frozen_ctx, opts)` with
    `use_graph_arm = true, limit = 1` (`GraphOrEdge`), escalating drift to
    whole-call `FetchEvidenceExec::Drifted` and converting only non-drift
    evidence errors to per-item `ResolvedFetchSlot` errors;
  - `expand` (`graph::expand`): maps `ErrorKind::FrozenRead(StateDrifted)`
    directly and probes `ErrorKind::Evidence` via
    `engine.search_expand_frozen("probe", &frozen_ctx, 0, SearchExpandOptions { search_limit: 1, view: None })` (`GraphOrEdge`).
  Live `read::visibility_state` calls remain exclusively for populating
  `Envelope.state` (`EP-01`).
- **Response ids.** `r<epoch>-<hash>` (store, canonical arguments, result
  references) maps to `{mode, query_id?, ref → write_cursor}` for feedback. LRU
  of 256 per store. Handles are content-derived, so identical requests on
  identical state produce identical output (MR-043).
- **fetch.** Supports two modes: (1) logical lookup — reject non-`l:` refs per
  item with `unsupported_ref`, call `read::get_many(engine, ids, Some(&view))`
  once (`None` yields per-item `not_found`), and budget bodies with `body_cut`;
  (2) frozen evidence resolution — when `evidence_refs` or `graph_evidence_refs`
  are supplied with a `frozen_context` handle, invoke
  `engine.resolve_evidence_many` and/or `engine.resolve_graph_evidence` under
  the bound `FrozenReadContextV1` with the cold-path drift probes above,
  returning exact source bytes, revision identity, and typed drift, erasure,
  and truncation outcomes.
- **expand.** Supports current (`GraphReadContextV1::Current`) and frozen
  (`GraphReadContextV1::Frozen` via `frozen_context` / `include_evidence_refs`)
  one-call `graph::expand` execution with bounded depth/work/targets, returning
  `GraphExpandResultV1.targets`, optional opaque graph evidence refs,
  `complete`, and `degradation_codes`.
- **actuate / put_latest_state (opt-in write tools).** Exposed in `tools/list`
  only when at least one configured store enables `allow_writes: true` with a
  non-empty bound writable caller `ScopePolicy`; refused deterministically with
  `writes_disabled` (zero DB mutation) on read-only profiles or read-only
  stores. Serialized per canonical store path with CAS
  `expected_write_boundary`, idempotent `(store, scope, operation_id)` replay,
  strict-only atomic rollback via `engine.actuate` (including atomic
  `ActuationOperationV1::PutLatestState`), and blocking shutdown/cancellation
  drain fencing (`AC32-MCP-SAFETY-01`–`09`).
- **orient.** `read::projections`, `read::projection_status`,
  `read::embedding_readiness`, `dense_disabled`. Kinds and counts are
  `unavailable`.
- **feedback.** Look up `response_id`. Return `invalid_request/wrong_store`
  if only another store's table holds it, and `not_recorded` with
  `response_expired` if no table does. Refs not in the response are
  `invalid_request`. Then return `not_recorded` with `store_reopened` if the
  store is not ready at the response's epoch (closed or reopened since the
  response: either way its sink no longer holds that query),
  `telemetry_disabled` if telemetry is off or its sink could not be enabled,
  and `text_mode` if the response came from the text path. Otherwise map refs
  to write cursors and call
  `engine.record_feedback(query_id, used, unused, "agent")`.
- **list_stores / use_store.** Registry and session only; no engine.

Final accounting converges compact text including its displayed count and
structured JSON including both final copies within four renders. The edge
refuses any mandatory overflow. Search retains whole leading hits, with
hints-off fallback and truthful zero-progress tiny-budget pages; complete
traversal requires progress-capable budgets. Fetch keeps byte-framed UTF-8
prefixes/outcomes; expand cuts bodies then trailing targets. Sufficient body-cut
budgets measure complete fetch in the selected target format. The interface
owns exact grammar and serialized-field presentation classifications.

## 8. rmcp usage

- `#[tool_router]` on an `FathomServer` struct holding the shared registry and
  config and a per-connection `Session`. Each `#[tool]` takes the raw argument
  object (`JsonObject`) and parses it in the handler into a typed
  `Args: Deserialize + JsonSchema` (`deny_unknown_fields`). A parse failure is
  then a typed `invalid_request/bad_argument` tool error; with rmcp 3.5.1's
  `Parameters<Args>` it would be an unstructured text error. The input schema
  is attached explicitly from `Args`. Each tool returns `CallToolResult`:
  the configured compact text, or structured content with its exact final JSON
  text copy. `is_error` is set for tool errors. Default text omits output schemas;
  structured attaches them from the same connection's configured router.
- Structured-mode output schemas: attached explicitly as `anyOf[<result>, {error}]` with a
  root `"type": "object"`, because tool errors also carry structured content.
  rmcp 3.5.1 derives an output schema only from a `Json<T>` return. The shared
  test clients validate every structured result in the suite against it,
  which also guards rmcp issue #1374.
- Per-call task: `ServerHandler::call_tool` runs each call on its own detached
  tokio task. A handler panic becomes a JSON-RPC internal error for that
  request while the connection keeps serving (MR-002.2), because rmcp 3.5.1
  does not catch handler panics. A client's `notifications/cancelled` stops
  the client waiting, but it does not stop the engine call already running
  on its blocking thread. That call keeps its admission slot until it
  returns, exactly like a call abandoned at its deadline (§6).
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
| 1 | `tests/config.rs` & `tests/startup_conflicts.rs` | Config grammar, defaults, `SC-01`–`SC-10` fail-fast startup conflicts (including `allow_writes` requiring non-empty writable `ScopePolicy`, rejecting legacy `"write"` key, and read-only `roots`), name grammar, root resolution, path rejection (MR-005.3) |
| 2 | `tests/manifest.rs` | MR-070.2 dependency lint; rmcp confinement |
| 3 | `tests/protocol.rs` | Handshake, static `tools/list` across read-only (7 tools) and write-enabled (9 tools) configs (MR-012.1, MR-006.7), stdio purity by spawning the binary (MR-002.1), diagnostics on stderr (MR-002.3), panic isolation (MR-002.2), no listener (MR-001.2) |
| 4 | `tests/lifecycle.rs` | Lazy open, `not_ready` during open with test seam (MR-003.1/.2), open failure without exit and retry (MR-003.3, MR-006.6), lock conflict (MR-004.2), idle close at the timeout and never at 0 (MR-005.4), one-open-at-a-time switch (MR-005.7), reopen re-applies settings (MR-006.1/.2), epoch handling, close after a failed drain releases the lock (F-44), and a stall test: a call blocked past its deadline while idle expiry fires does not delay a call to another store beyond its deadline (F-42) |
| 4b | `tests/stores.rs` | Three stores return their own records (MR-005.1), listing has no paths (MR-005.2), no creation without `create` (MR-005.5), handle from another store rejected (MR-005.6), embedded mode with two host engines (MR-004.1) |
| 5 | `tests/bridge.rs` | A panic on the blocking thread is `internal/panic` and frees its admission permit |
| 5b | `tests/admission.rs` | Admission queue (MR-033.2; `dev/design/agent-access/queue-admission.md` § 9): FIFO wait for a permit, `busy/admission` reasons `queue_full` and `deadline`, no start check, queued and cancelled calls, `busy/store_switch`, progress while queued; started calls run to completion, `timeout/deadline` at the run limit from the permit and the adapter keeps serving (MR-033.1), abandoned and stuck counts, cancel of a running call, shutdown with waiters |
| 6 | `tests/search.rs` | Text and hybrid paths, closed direct-text filters, `as_of` and `validity_instant` (MR-023), snippet cap (MR-040.1), completeness values (MR-030.1, MR-032), continuation exact on an unchanged store and stale on a new epoch (MR-041.2/.5), budget fits (MR-041.1/.3), references contain the logical id and are stable across restarts (MR-042.1/.3) |
| 7 | `tests/fetch_expand.rs` & `tests/mcp_frozen_evidence.rs` | Fetch order and per-item errors (MR-052), `l:` only (MR-042.2), malformed versus missing reference (MR-054.2), same content as SDK (MR-040.2, MR-070.1), `as_of` on fetch (MR-023.1), expand bounds (MR-053), scoped frozen-evidence equivalence across `search`/`fetch`/`expand`, reopen durability, and `ADR #103` partitioned drift isolation |
| 8 | `tests/feedback.rs` | `telemetry_disabled` (MR-024.2), `text_mode`, recorded with the right query id under concurrent hybrid searches (MR-024.1/.6), available in read-only mode (MR-024.4), wrong store or foreign refs rejected (MR-024.5), `store_reopened` (MR-006.5), unique ids per sink file across epochs (MR-024.7), output unchanged after feedback (MR-024.3) |
| 8b | `tests/readonly.rs` & `tests/mcp_write_safety.rs` | Read-only default mutation invariance (MR-060.1) and opt-in write safety (`AC32-MCP-SAFETY-01`–`09`: scoped writes, CAS `expected_write_boundary`, idempotent `operation_id` replay, `put_latest_state`, strict rollback, blocking drain fence, secret-scrubbed errors) |
| 9 | `tests/determinism.rs` | Two process lifetimes, pinned `as_of`, byte-identical output (MR-043.1) |
| 10 | `tests/schema.rs` | Every structured result validates against its `outputSchema`; schemas are draft 2020-12 (MR-071) |
| — | `src/shape/*` unit + `proptest` | Budget never exceeded, snippet on char boundary, continuation pages have no gaps or duplicates, ordering stable |

Not covered by automated tests in this release: MR-001.3 (official
conformance suite, run manually once and recorded), MR-021 (evaluation run).

## 10. Workspace touch points

- `Cargo.toml`: `fathomdb-mcp` is a publishable workspace member (`publish = true`,
  `default = []` on `crates.io`; `--features default-embedder` on 5-target GH
  Release archives and lockstep `src/python-mcp/` PyPI wheels).
- `fathomdb-engine` / `fathomdb-sdk` / bindings: backed by `ADR-0.8.32` `#99`
  (`Pin C`: `PutLatestState`, `EP-07` `create_if_missing`, `EP-01`
  `read::visibility_state`), `#102` (`ScopePolicy`), and `#103` (Schema 39
  partitioned frozen-read additive stability) while keeping `fathomdb-mcp`
  runtime dependencies strictly confined to `fathomdb-sdk` (`MR-070.2`).
- `scripts/agent-*.sh` & `.github/workflows/release.yml`: covered by workspace
  tests, `MR-070.2` manifest lint, `rmcp` confinement checks, and 3-channel
  packaging/smoke gates.
