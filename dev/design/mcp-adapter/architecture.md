---
title: MCP adapter architecture proposal (rmcp)
date: 2026-10-17
status: PROPOSED
desc: Architecture for an MCP adapter over FathomDB built on rmcp, aligned with FathomDB's principles. Architecture only; tool schemas, field names and envelopes belong to a later design and interface document.
inputs:
  - dev/design/mcp-adapter/agent-needs.md
  - dev/design/mcp-adapter/requirements.md
  - dev/notes/mcp-rust-sdk-selection.md
  - dev/adr/ADR-0.6.0-cli-scope.md
  - dev/adr/ADR-0.6.0-async-surface.md
  - dev/adr/ADR-0.6.0-crate-topology.md
  - dev/adr/ADR-0.6.0-json-schema-policy.md
  - dev/adr/ADR-0.6.0-database-lock-mechanism-reader-pool-revision.md
  - dev/adr/ADR-0.8.25-eligibility-and-frozen-reads.md
  - src/rust/crates/fathomdb-sdk/src/{engine,read,graph,guard,options}.rs
  - src/rust/crates/fathomdb-engine/src/{frozen_read,search_types}.rs
---

# MCP adapter architecture proposal (rmcp)

> Scope: the shape of the system — components, boundaries, process and thread
> topology, data flow, trust model, and where each requirement is met. It does
> not fix tool names, arguments, schemas or envelopes; those belong to a design
> and a `dev/interfaces/mcp.md` written after acceptance. "MR-###" refers to
> `requirements.md`, "AN-###" to `agent-needs.md`. SDK facts are as of the
> 0.8.27 tree.

## 1. Principles this architecture holds to

| FathomDB principle (source) | Consequence here |
| --- | --- |
| Governed surface is the contract; public surface changes need an ADR (`AGENTS.md` §1) | The adapter is a client of `fathomdb-sdk` and adds no engine semantics. Missing capability arrives as SDK verbs under their own ADRs (§7), usable by every SDK, never as adapter-private SQL. |
| Application query verbs stay out of the operator CLI (ADR-0.6.0-cli-scope) | A separate crate and binary; the operator CLI is unchanged. See §1.1: the adopting ADR must address this ADR explicitly. |
| Sync engine surface (ADR-0.6.0-async-surface) | Async lives only inside the adapter's protocol edge; engine calls cross into blocking threads. |
| One process owns a database file (sidecar lock ADRs) | Two deployment modes, embedded and standalone (§3). A file is never open in two processes at once. How the MCP server and an application using an SDK share one database is out of scope (§10). |
| Deterministic, explainable retrieval; no model in the read path | The adapter shapes and labels. It never re-ranks with a model, summarises, or rewrites results. |
| Frozen reads are reproduce-or-fail, not retained snapshots (ADR-0.8.25) | Consistency is reported (state identity and drift labels), not held (§5). |
| JSON Schema 2020-12 (ADR-0.6.0-json-schema-policy) | Tool schemas are generated as 2020-12 (rmcp uses schemars 1.x). |
| Local-first, no egress by default | stdio only; no listener; any network transport needs its own ADR. |
| TDD against a real database; no agent-generated oracles | Every requirement is met by a test that drives the real engine through the real protocol (§9). |

### 1.1 Relationship to ADR-0.6.0-cli-scope

That ADR rejected "Operator CLI + read-only application query" because it
"introduces a second public query surface." An MCP adapter is also a query
surface, so the proposal does not claim the ADR is untouched. Its argument,
for the adopting ADR to accept or reject:

- cli-scope governs the **operator** binary: what an operator can do to a
  database by hand. The adapter is an **application-layer** client, of the
  same kind as any program built on `fathomdb-sdk`, and exposes only governed
  SDK reads.
- The rejected option's harm was a parallel query surface with its own
  semantics to keep in sync. MR-070 removes that harm: the adapter has no query
  semantics of its own, and its manifest cannot reach engine internals.

The adopting ADR either records this distinction in cli-scope's Consequences
or amends cli-scope. It also amends ADR-0.6.0-crate-topology for the new crate.

## 2. Components

```text
            MCP client (agent harness)
                       │  JSON-RPC over stdio, or an in-memory transport
┌──────────────────────┼─────────────────────────────────────────────┐
│ fathomdb-mcp         ▼                                             │
│  ┌──────────────────────────────┐                                  │
│  │ Protocol edge (rmcp)         │  handshake, tools/list, dispatch, │
│  │                              │  cancellation, progress           │
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐   ┌───────────────────────────┐  │
│  │ Session                      │──▶│ Launch policy             │  │
│  │  state identity, last cursor │   │  read-only / write,       │  │
│  │  validity instant, defaults  │   │  origin class for writes, │  │
│  └──────────────┬───────────────┘   │  scope, admission cap     │  │
│  ┌──────────────▼───────────────┐   └───────────────────────────┘  │
│  │ Tool handlers                │                                  │
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐                                  │
│  │ Response shaper (no rmcp)    │  references, snippets, budget,   │
│  │                              │  continuation, labels, ordering  │
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐                                  │
│  │ Engine bridge                │  readiness gate, admission,      │
│  │                              │  blocking hand-off, deadlines,   │
│  │                              │  error mapping                   │
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐                                  │
│  │ Store registry               │  name → file, lazy open,         │
│  │                              │  idle close, open-store cap      │
│  └───┬──────────┬──────────┬────┘                                  │
└──────┼──────────┼──────────┼───────────────────────────────────────┘
       ▼          ▼          ▼
    Engine A   Engine B   Engine C   (fathomdb-sdk, governed, sync; one per file)
```

- **Protocol edge.** The only module that names `rmcp` types. rmcp is pinned
  with `=`; an upgrade touches this module only.
- **Session.** One per MCP connection. Holds an optional default store and, per
  store, the last state identifier and cursor it reported. It also holds the
  validity instant (now, or a pinned as-of instant) and effective defaults. It
  holds no result sets.
- **Launch policy.** Fixed by the operator: read-only or write-enabled, the
  origin class stamped on writes, the scope (MR-014), the admission cap. No tool
  can change it (MR-011, MR-014, MR-060).
- **Tool handlers.** Validate, call the bridge, pass governed results to the
  shaper. No ranking, filtering or eligibility logic (MR-070).
- **Response shaper.** Pure functions from governed results to agent-shaped
  output. It does not depend on rmcp, so it can be property-tested on its own.
- **Engine bridge.** Owns the per-store readiness gate (§3), admission control
  and deadlines (§3), the hand-off to blocking threads, and the mapping from SDK
  `ErrorKind` to the MR-054 classes.
- **Store registry.** Maps the store names the agent uses to database files the
  operator made addressable (§3.1). It opens engines on first use, closes idle
  ones, and enforces the open-store cap. In embedded mode the host registers its
  own open engines under names, and the registry never opens or closes those.

## 3. Deployment, startup and threads

### Modes

| Mode | Who opens each `Engine` | Transport | Use |
| --- | --- | --- | --- |
| **Standalone** | The `fathomdb-mcp` binary, through its store registry | stdio | Harness-launched access to any number of operator-addressable stores: Claude Code, OpenClaw, Hermes. |
| **Embedded** | A Rust host application, which registers its open `Engine`s by name | Any rmcp transport the host chooses | A Rust program that serves its own engines over MCP (MR-004). |

The deliverable is a Rust MCP server. Embedded mode is the same server used as a
Rust library, and nothing else (owner decision, 2026-10-17).

### 3.1 Store addressing

The adapter is never tied to one database file (owner directive, 2026-10-17;
MR-005).

- **The operator defines what is addressable.** The launch configuration names
  stores explicitly (name → file), or names a root directory whose FathomDB
  database files are addressable by relative name, or both. The agent addresses
  stores only by name. A name never resolves outside the registry: names with
  path separators, `..` or absolute paths are rejected before any filesystem
  access. Creating a new file needs explicit per-name permission.
- **Every tool takes a store.** Each call names a store, or relies on the
  session default. An orient-family tool lists the addressable stores with their
  open/closed and readiness state, and never their paths.
- **Lazy open, idle close, one at a time.** A store opens on its first call and
  closes once idle for the configured timeout (default 60 s; 0 never closes),
  which releases its sidecar lock. **This release supports one open store at a
  time** (owner decision, 2026-10-17): using another store closes the current
  one first (§3.2). The registry still keeps a set of open stores with a limit
  and least-recently-used choice, with the limit fixed at 1, so supporting more
  later is configuration, not restructuring. Each open `Engine` owns its own
  embedder provider and executors (ADR-0.8.27 runtime topology), so every open
  store costs model memory and threads; that cost is what the limit bounds.
- **Lock conflicts are per store.** A store whose file another process holds
  answers with the typed lock error and its remedy. Other stores keep working.
- **Per-store state.** State identity, cursors, continuations and feedback all
  name their store and are rejected on any other (MR-005.6).
- **Shared configuration.** Launch policy (read-only, origin class, admission
  cap) applies to every store unless the registry entry overrides it. Overrides
  may only narrow, never widen.

### 3.2 Store lifecycle and reconnection

Each store moves through `closed → opening → ready → draining → closed`. A
failed open returns to `closed` and records its error, so the next call retries
(MR-006.6).

- **Idle.** Every call resets the idle clock. A store with calls in flight is
  never idle, and that includes abandoned calls whose blocking threads are still
  running (§3, admission). A timeout of 0 disables the clock.
- **Draining and close.** On idle expiry or a store switch, the store stops
  admitting calls, waits for in-flight calls to return, then calls
  `Engine::drain` and `Engine::close` on a blocking thread. Calls that arrive
  while it drains wait or return busy, and never run against another store.
- **Switching.** With the limit at 1, a call to store B while A is open drains
  and closes A, then opens B. The switch is serialised: nothing is ever open on
  two files at once.

What a reopen must restore, and where it lives:

| State | Owner | Survives close? | On reopen |
| --- | --- | --- | --- |
| Frozen-read key, visibility generation, write cursor | Database file | Yes | Nothing to do: tokens, state identifiers and cursors stay valid. |
| Continuation authentication key | Adapter process | Yes, within the process | Nothing to do. After a process restart, old continuations fail as invalid request with a remedy. |
| Session's last state identifier and cursor, per store | Adapter session | Yes | Compared on the next read. If another process changed the store while it was closed, the response is labelled `state_advanced` (MR-006.3). |
| Telemetry sink (`enable_telemetry`) | Engine instance | No | Re-applied from launch policy. Each sink has a new nonce, so pre-close feedback returns `not_recorded` (MR-006.5). |
| Diagnostics subscriber (`attach_subscriber`), profiling, slow-query threshold | Engine instance | No | Re-applied from launch policy (MR-006.2). |
| Embedder warm-up, open report, readiness | Engine instance | No | Recomputed; the store reports not-ready, phase `reopening`, until done (MR-006.4). |
| Admission count, idle clock | Adapter bridge | Reset | Start from zero for the new engine. |

All re-applied settings come from one per-store "apply on open" list in the
bridge, so first open and reopen take the same path. A test asserts each entry
is active after reopen.

**On the rmcp side, reconnection needs no protocol refresh.** That is a
deliberate result of the design, not an omission:

- Tool names, schemas and descriptions are static and do not depend on which
  store is open (MR-012.1), so no `notifications/tools/list_changed` is sent.
- Server capabilities are fixed at `initialize`.
- Stores are listed through a tool, not as MCP resources, so there is no
  resource list to invalidate.
- MCP sessions are independent of engine lifetime: the MCP connection stays up
  while stores close and reopen underneath it.

The only rmcp traffic that reconnection causes is progress notifications during
a reopen, for calls that supplied a progress token (MR-006.4). If a later version
exposes stores or spilled results as MCP resources, then `resources/list_changed`
on open and close and `resources/updated` on state changes would become
required, and this section must be revised.

### Startup

1. The edge serves `initialize` and `tools/list` at once. The tool list is
   static (MR-012.1) and the registry is configuration, so neither needs an
   engine.
2. A store's `Engine::open` runs on a blocking thread when the store is first
   used, or eagerly at launch for stores the operator marks as eager. Open takes
   the sidecar lock first, then migrates and warms the embedder.
3. Until a store's open completes, the readiness gate answers calls to it with
   not-ready naming the phase (MR-003.2). If open fails, the gate answers every
   call to that store with the typed open error and remedy, and logs it to
   stderr (MR-003.3).

This keeps a slow embedder load or a model download from tripping the harness's
handshake timeout.

### Threads, cancellation and admission

- A small tokio runtime runs the edge: owned by the binary in standalone mode,
  supplied by the host in embedded mode. Engine calls run on blocking threads.
- SDK calls have no interrupt, deadline or progress hook. **Cancellation is
  cooperative at return only:** a cancelled or timed-out call stops waiting and
  answers the client (MR-033.1), but its blocking thread runs to completion and
  still holds a reader-pool slot.
- So that abandoned work cannot exhaust the engine, the bridge admits at most
  N engine calls in flight, and an abandoned call counts against N until its
  thread actually returns. In standalone mode N defaults below the reader-pool
  size (8 today). In embedded mode the host sets N, since its own reads draw on
  the same pool. Excess calls wait up to the deadline, then return busy
  (MR-033.2). Interruptible search is an engine follow-up (§7).
- The bridge's open and call paths carry a test seam behind an adapter-only
  feature, for MR-003 and MR-033 tests, so no engine test hook is needed.
- Writes serialise on the engine's single writer; the adapter adds no
  concurrency of its own.

### Protocol channel purity (MR-002)

stdout belongs to the edge alone. The binary installs a stderr tracing
subscriber before starting the open, routes embedder and download progress to
it, and converts handler panics into JSON-RPC errors while continuing to serve.

## 4. Tool families

Families, not schemas. The "today" column is what the 0.8.27 SDK provides; the
engine column is what §7 must add.

| Family | Purpose | Today in `fathomdb-sdk` | Needs from §7 | MR |
| --- | --- | --- | --- | --- |
| Orient | Capped summary of contents and readiness | `read.projections`, `read.projection_status`, `read.embedding_readiness` | Kind enumeration, per-kind and per-class counts | MR-022 |
| Search | Relevance search, filters, budget, as-of; explain in the same call | `search` and `search_text_only` (reader pool), filter grammar, `ReadView.valid_as_of`, `explain=true` | Exact phrase predicate; bounded count; raw per-arm scores; currency fields on hits; scope | MR-020–023, MR-030–032, MR-040–041 |
| Fetch | Full content for many references | `read.get`, `read.get_many` (logical ids only); `search_with_evidence` and `resolve_evidence` for passages | Fetch by content or passage id | MR-040.2, MR-042, MR-052 |
| Expand | Bounded neighbourhood from an explicit reference | `graph.expand`, `graph.neighbors` (one read each; `search_expand_frozen` is writer-serialised and not a default) | — | MR-053 |
| Explain | Why a hit ranked | `explain` sidecar | Raw per-arm scores (shared with Search) | MR-031 |
| Changes | Governed changes since a cursor | Nothing suitable: `read.mutations` reads the op-store; `read.crossed_boundary_since` is keyed by wall-clock | Governed change feed keyed by write cursor and visibility generation | MR-055 |
| Remember *(write policy only)* | Record, replace, retract | `write` (same logical id replaces), `transition` to `Deleted` | First-class origin field; exact-duplicate rule | MR-060–062 |
| Stores | List addressable stores; set the session default | Store registry (adapter), `open_report` | — | MR-005 |
| Feedback | "This hit was used" | `record_feedback` with label source `agent`, behind opt-in telemetry | — | MR-024 |

Feedback is recorded and has no other effect. What FathomDB does with it is
deliberately undecided (owner, 2026-10-17), so nothing in ranking, defaults or
determinism may depend on it until a separate decision says so. The engine's
feedback verb keys hits by numeric id and a telemetry query id. The adapter
keeps that mapping inside the authenticated response context, so the agent
names a response and references, never internal ids.

Few tools with typed options, `search` over `list`, and a concise default with a
detailed mode, per `dev/notes/agent-tools-regrade-of-query-interfaces.md`.

## 5. Consistency model

The engine's frozen read contexts reproduce one state or fail with drift; they
do not retain history (ADR-0.8.25). Two facts shape the adapter:

- The state binding includes the global canonical write boundary, so **any
  commit, by anyone, drifts every frozen context.** Under a live writer, holding
  one context per session would make most reads fail.
- Eligibility (filters) is baked into the frozen token, and only the search
  family takes a frozen context; `read.get`, `read.list` and `graph.*` take a
  `ReadView` only.

- `freeze_read_context` takes the **writer-connection lock** and counts every
  canonical row. It is a coordination tool for multi-read plans, not a cheap
  probe. Calling it per read would serialise the adapter behind the single
  writer, and behind the host's writes in embedded mode, which defeats the
  reader pool.

So the adapter **reports** consistency rather than holding it:

1. **The visibility generation is the state.** The engine's read-visibility
   generation is advanced by triggers on canonical, source, dependency,
   projection, vector and op-store tables, and explicitly by projection commits
   that change visibility. Because of that it moves both when records change and
   when indexing changes what is searchable. That is what an agent needs, since
   either changes search output. The identifier is derived from it alone. The
   validity instant is a separate field, so an unpinned session's identifier does
   not change merely because time passed (MR-050).
2. **Cause is classified beside it, not proved.** The state read also returns the
   canonical write boundary and the projection cursor. A label's cause is
   `canonical` when the write boundary changed, `index` when only the projection
   cursor changed, and `unspecified` otherwise (for example an in-place
   lifecycle update). These fields are informational: the write boundary is a
   `MAX` over four tables and can fall after a purge (MR-051).
3. **One engine addition is a prerequisite:** a content-free read of those three
   values on a reader connection (§7). Without it, the identifier is reported
   `unavailable` and drift labelling is off. Nothing approximates it.
4. **One SDK read per response.** Each handler issues exactly one governed read,
   bracketed by two state reads. The proof rests on the generation alone, which
   is monotonic (it aborts rather than wrap): equal generations in the brackets
   prove that no visibility-changing commit happened during the read. On a
   mismatch the bridge retries once. A second mismatch returns a drift error and
   no data (MR-050.3). The frozen composites (`search_frozen`,
   `search_expand_frozen`) take the writer lock through `freeze_read_context`,
   so no default tool uses them. The default expand tool expands from an
   explicit reference with `graph.expand` or `graph.neighbors`, which is one
   read.
5. **Labels, not failures.** When a response's state differs from the session's
   previous one, it is answered on the new state and labelled `state_advanced`
   with its cause (MR-051). The agent's own writes produce the same label.
6. **Continuations re-derive.** A continuation is an opaque value authenticated
   by the adapter. It carries the state identifier and the references already
   returned, which the ranked-result cap bounds, and no record content
   (MR-041.5). On an unchanged state, the page continues
   exactly. On an advanced state, the ranked run is repeated on the new state,
   references already returned are dropped, and the page is labelled
   `state_advanced` with the caveat that results which moved earlier may be
   missing (MR-041.4). A continuation fails as drift only when it cannot be
   re-derived.
7. **Validity instant per call.** It is resolved per call, or pinned for as-of
   sessions (MR-023), so records whose validity starts later appear once it
   arrives.

A scope-limited drift signal, one that ignores commits outside the session's
eligibility, would cut `state_advanced` noise further under a busy writer. It
is an engine follow-up (§7).

## 6. Trust and scope model

- **Origin class is stored by the engine, outside content.** The only safe home
  is a first-class field beside `source_id`, written from the writer's
  entitlement and never from the body. A body-derived projection is rejected as
  an interim: any ingested document could carry a body field claiming `owner`
  (MR-010.4). Until the field exists, every result reports `unclassified`.
- **The write family waits for engine-stamped origin and scope.** The first
  release is read-only. Write tools ship only once the engine stamps both origin
  class and scope from launch policy, never from the body (MR-011, MR-014.5,
  MR-060). A body-path scope has the same hole as a body-path origin: an agent
  or an ingested document could write itself into another scope. Waiting also
  gives the read path a measured quality baseline (MR-021) before agents start
  writing.
- **Scope is enforced in the engine on every read path, or not offered.** Today
  eligibility reaches only the search family; point reads, neighbours, list
  predicates, aggregates and change queries take none. The adapter does not
  filter in its own code (MR-070), so a scope can only be honoured once the
  engine applies it everywhere. Until then the adapter fails closed and refuses
  to start with a scope configured (MR-014.4). The preferred engine form is a
  scope member of `ReadView`, so every view-taking verb honours it natively. A
  reference outside scope reads as not found.
- **Content stays data.** Tool descriptions, server instructions and error
  templates are compile-time constants. Kind names in errors come from the
  engine's kind enumeration, set by writers and not by content. In write mode,
  adapter writes are confined to the operator's launch allow-list of kinds, so
  an agent cannot mint a name that other sessions would see echoed. Field names
  come from the projection registry (MR-012.2, MR-054.3). The text rendering
  marks content as quoted data.
- **Least privilege.** Read-only by default; write tools absent, not refusing.
  No SQL, admin, purge or erasure tools: erasure stays an application and
  operator action through the SDK and CLI.
- **No egress.** stdio only. A network transport needs its own ADR reviewing the
  rmcp HTTP advisories (origin checks, DNS rebinding, session handling).

## 7. Engine additions the adapter depends on

Each is an SDK verb or field under its own ADR, available to all three SDKs,
and each is read-side unless noted. The adapter reports the dependent signal as
`unavailable` until it lands.

| Addition | For | Note |
| --- | --- | --- |
| **Content-free state read on a reader connection** | MR-050, MR-051, MR-041.2/.4 | **The first-release prerequisite.** Returns the existing visibility generation, the canonical write boundary and the projection cursor, with no writer lock and no row count. These are values the engine already maintains, so this is a small read verb. |
| First-class origin field, filterable, on every hit path | MR-010, MR-011 | A data-model addition: an additive column under the no-data-migration rule. Gates the write family. |
| Scope applied on every read path (preferably a `ReadView` member), stamped on write from policy | MR-014 | Point reads, neighbours, list, aggregates and change feed included. Gates scope binding and the write family. |
| Validity and supersession fields on search hits | MR-013 | `NodeRecord` has them; `SearchHit` does not. |
| Kind enumeration; per-kind and per-class counts; time bounds | MR-022, MR-054 | Read-side aggregates. |
| Bounded match count and saturation on search and list | MR-030 | Read-side. |
| Raw per-arm similarity and BM25 scores | MR-031 | Fused scores are rank-only (reciprocal-rank sums); only the cross-encoder score is calibrated today. |
| Exact phrase predicate composed with relevance search | MR-020 | Lexical; distinct from the `Json` predicate ADR-0.8.11 D3 rejects on `search`. |
| Fetch by content or passage id | MR-042.2 | `read.get` takes logical ids only. |
| Ranked pagination beyond the ranked-result cap (100) | MR-041.2 | Until then continuation re-runs at a larger limit within the cap. |
| Governed change feed keyed by write cursor and visibility generation | MR-055 | No suitable verb exists. |
| Exact-duplicate rule on write | MR-062 | Engine-owned normalization. |
| Interruptible search; scope-limited drift signal | MR-033, MR-051 | Quality improvements; not prerequisites. |
| Search returns its own telemetry query id | MR-024.6 | `last_telemetry_query_id` is engine-wide; until this lands the adapter serialises telemetry-captured searches per store. |

Sequencing follows from the table. The first release is read-only and
unscoped, needs only the state read, and meets reach, hygiene,
readiness, deadlines, as-of, fetch by logical id, expand, budget, determinism,
state identity, drift labels and typed errors. The other rows land
independently. `requirements.md` § First-release scope lists the exact split.

## 8. Versioning and change control

- **rmcp** is pinned exactly and confined to the edge; each rmcp major is a
  scoped migration gated by the conformance and determinism suites, excluded
  from automated version-bump pull requests.
- **MCP protocol version** is negotiated by rmcp; the adapter declares its
  minimum and is tested at both ends of its range.
- **Tool contract** is versioned by the interface document. Additive fields
  follow the "new optional field with default" rule; a change to a field's
  meaning, the reference format, the estimator, or a default budget is breaking
  for agents and needs an interface update in the same change.
- **Workspace.** `fathomdb-mcp` joins Axis W lockstep versioning.

## 9. Verification architecture

- **In-process protocol tests.** rmcp's async read/write transport over a duplex
  pipe connects a real client to a real server on a real database file. No mock
  engine.
- **Process tests.** Spawn the binary for stdio purity (MR-002), startup and
  readiness (MR-003), the lock conflict (MR-004.2) and the listener check
  (MR-001.2).
- **Conformance.** Official MCP conformance scenarios for stdio and tools.
- **Schema conformance.** Every structured output validated against its declared
  output schema (MR-071), which also guards rmcp issue #1374.
- **Property tests.** The shaper's budget, continuation (no gaps or duplicates
  across pages), ordering and snippet functions under `proptest`.
- **Determinism.** Two process lifetimes, same database, pinned validity
  instant, byte comparison (MR-043); run-to-run, never against agent-written
  goldens.
- **Concurrency.** A live writer during a read session for the drift labels
  (MR-051) and a blocked engine call for deadlines and admission (MR-033).
- **Retrieval quality.** The MR-021 evaluation runs the adapter's default path,
  so the number reported is the number agents get.

## 10. Open questions for the adopting ADR

Resolved by the owner on 2026-10-17:

- The deliverable is a Rust MCP server; embedded mode is that server used as a
  Rust library.
- The adapter addresses many stores and is never tied to one file (§3.1).
- Distribution is prebuilt binaries, one per platform and accelerator variant
  the workspace already builds.
- "This hit was used" feedback is accepted and recorded; its use is undecided
  (MR-024).
- The budget estimator starts as serialized bytes ÷ 4 (MR-041).

- One open store at a time in this release, with the registry built for more
  (§3.1, §3.2).
- The idle timeout is configurable, defaults to 60 s, and 0 means never close.
- Reconnection restores all engine-level state and needs no MCP-level refresh
  (§3.2).

Out of scope for this effort, and left open:

- How the MCP server and an application using a FathomDB SDK share one
  database. The sidecar lock allows one opener per file; this proposal neither
  solves nor assumes a sharing protocol.
