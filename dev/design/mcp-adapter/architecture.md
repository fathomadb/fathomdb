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
| One process owns a database file (sidecar lock ADRs) | Two deployment modes, embedded and standalone (§3). Sharing a file between an application and a separately launched adapter is impossible by design, and the architecture says so rather than working around it. |
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
└─────────────────┼──────────────────────────────────────────────────┘
                  ▼
      fathomdb-sdk  (governed, sync)  ──▶  fathomdb-engine
```

- **Protocol edge.** The only module that names `rmcp` types. rmcp is pinned
  with `=`; an upgrade touches this module only.
- **Session.** One per MCP connection. Holds the last state identifier and
  cursor it reported, the validity instant (now, or a pinned as-of instant), and
  effective defaults. It holds no result sets.
- **Launch policy.** Fixed by the operator: read-only or write-enabled, the
  origin class stamped on writes, the scope (MR-014), the admission cap. No tool
  can change it (MR-011, MR-014, MR-060).
- **Tool handlers.** Validate, call the bridge, pass governed results to the
  shaper. No ranking, filtering or eligibility logic (MR-070).
- **Response shaper.** Pure functions from governed results to agent-shaped
  output. It does not depend on rmcp, so it can later be extracted into a crate
  that the Python and TypeScript SDKs reuse; agent-shaped output then need not
  be MCP-only.
- **Engine bridge.** Owns the readiness gate (§3), admission control and
  deadlines (§3), the hand-off to blocking threads, and the mapping from SDK
  `ErrorKind` to the MR-054 classes.

## 3. Deployment, startup and threads

### Modes

| Mode | Who opens the `Engine` | Transport | Use |
| --- | --- | --- | --- |
| **Standalone** | The `fathomdb-mcp` binary, as sole opener | stdio | A store owned by the agent itself: harness-launched memory for Claude Code, OpenClaw, Hermes. |
| **Embedded** | The host application, which lends its open `Engine` | Any rmcp transport the host chooses | A store owned by an application (for example Memex) that its agents also use (MR-004). |

The common topology "the application owns the file and a harness also launches
a standalone adapter on it" is excluded by the sidecar lock. AN-018 is met for
application-owned stores only through embedded mode. Embedded mode is a Rust API
first; whether the Python and TypeScript bindings expose it is a **blocking**
question for the adopting ADR (§10), because most consumers are Python or
TypeScript applications.

### Startup

1. The edge serves `initialize` and `tools/list` at once; the tool list is
   static (MR-012.1), so it does not need the engine.
2. `Engine::open` runs on a blocking thread in parallel. It takes the sidecar
   lock first, then migrates and warms the embedder.
3. Until open completes, the readiness gate answers tool calls with not-ready
   naming the phase (MR-003.2). If open fails, the gate answers every call with
   the typed open error and remedy, and logs it to stderr (MR-003.3).

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
  N engine calls in flight, with N below the reader-pool size (8 today) and
  owned by the ADR. Excess calls wait up to the deadline, then return busy
  (MR-033.2). Interruptible search is an engine follow-up (§7).
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
| Search | Relevance search, filters, scope, budget, as-of | `search`, `search_text_only`, `search_frozen`, filter grammar, `ReadView.valid_as_of` | Exact phrase predicate; bounded count; raw per-arm scores; currency fields on hits | MR-020–023, MR-030–032, MR-040–041 |
| Fetch | Full content for many references | `read.get`, `read.get_many` (logical ids only); `search_with_evidence` and `resolve_evidence` for passages | Fetch by content or passage id | MR-040.2, MR-042, MR-052 |
| Expand | Bounded neighbourhood | `graph.expand`, `graph.neighbors`, `search_expand_frozen` | — | MR-053 |
| Explain | Why a hit ranked | `explain` sidecar | Raw per-arm scores (shared with Search) | MR-031 |
| Changes | Governed changes since a cursor | Nothing suitable: `read.mutations` reads the op-store; `read.crossed_boundary_since` is keyed by wall-clock | Governed change feed keyed by write cursor and visibility generation | MR-055 |
| Remember *(write policy only)* | Record, replace, retract | `write` (same logical id replaces), `transition` to `Deleted` | First-class origin field; exact-duplicate rule | MR-060–062 |

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

So the adapter **reports** consistency rather than holding it:

1. Each response carries a state identifier computed from the state binding and
   validity instant only, not from the token or its eligibility. Equal
   identifiers mean the same state, whatever the filters (MR-050).
2. Each call reads one state. Search-family calls freeze and consume a context
   within the call, so a single response never mixes states (MR-050.3). For
   non-frozen verbs, the bridge obtains the binding with `freeze_read_context`
   before and after the read; if it moved, it retries once and otherwise answers
   with a drift label. A cheaper binding read is an engine nicety, not a
   prerequisite.
3. When a call's state differs from the session's previous one, the response is
   answered on the new state and labelled `state_advanced` (MR-051). The
   agent's own writes are labelled the same way, so they are visible to it at
   once.
4. Continuations embed the state identifier. A continuation on an old state
   fails as drift with a remedy (MR-051.3), so pages never mix states.
5. The validity instant is resolved per call (or pinned for as-of sessions,
   MR-023), so records whose validity starts later appear when it arrives.

A scope-limited drift signal, one that ignores commits outside the session's
eligibility, would reduce `state_advanced` noise under a busy writer; it is an
engine follow-up (§7), not a prerequisite.

## 6. Trust and scope model

- **Origin class is stored by the engine, outside content.** The only safe home
  is a first-class field beside `source_id`, written from the writer's
  entitlement and never from the body. A body-derived projection is rejected as
  an interim: any ingested document could carry a body field claiming `owner`
  (MR-010.4). Until the field exists, every result reports `unclassified`.
- **The write family waits for the origin field.** The first release is
  read-only; write tools ship only once adapter writes can be stamped by policy
  (MR-011, MR-060). This also gives the read path a measured quality baseline
  (MR-021) before agents start writing.
- **Scope uses the engine's eligibility.** The operator's scope is expressed over
  declared exact projections, which ADR-0.8.25 anticipates for owner and scope.
  The bridge joins it into every call's eligibility, so it reaches the
  engine before truncation and no tool argument can remove it (MR-014). A
  reference outside scope reads as not found.
- **Content stays data.** Tool descriptions, server instructions and error
  templates are compile-time constants. Kind and field names in errors come
  only from operator or registry declarations, never from agent-written records
  (MR-012.2, MR-054.3). The text rendering marks content as quoted data.
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
| First-class origin field, filterable, on every hit path | MR-010, MR-011 | The one data-model addition; additive column under the no-data-migration rule. Gates the write family. |
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

Sequencing follows from the table: a first read-only release meets the reach,
hygiene, scope, as-of, fetch-by-logical-id, expand, budget, determinism, drift
and error requirements with the SDK as it is; the rest arrive as the engine
rows land.

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

1. **Blocking:** embedded mode through the Python and TypeScript bindings, since
   most consumers are Python or TypeScript applications and the lock rules out a
   second opener.
2. Distribution of the binary: crates.io only, or bundled with the wheel and npm
   package so harness configuration needs no Rust toolchain.
3. Whether agents may send "this hit was used" feedback through the adapter to
   the existing `record_feedback` and opt-in telemetry.
4. The estimator: the proposal fixes serialized bytes ÷ 4 (portable,
   model-agnostic); a bundled tokenizer is the alternative.
