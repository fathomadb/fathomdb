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
---

# MCP adapter architecture proposal (rmcp)

> Scope: the shape of the system — components, boundaries, process and thread
> topology, data flow, trust model, and where each requirement is met. It does
> not fix tool names, argument names, schemas or wire envelopes; those belong
> to a design and a `dev/interfaces/mcp.md` written after this is accepted.
> "MR-###" refers to `requirements.md`, "AN-###" to `agent-needs.md`.

## 1. Principles this architecture holds to

| FathomDB principle (source) | Consequence here |
| --- | --- |
| Governed surface is the contract; public surface changes need an ADR (`AGENTS.md` §1) | The adapter is a client of `fathomdb-sdk`. It adds no engine semantics of its own. New engine capability arrives as SDK verbs under their own ADRs, never as adapter-private SQL. |
| No application query verbs on the `fathomdb` CLI (ADR-0.6.0-cli-scope) | A separate crate and binary. The operator CLI is untouched; this ADR is not re-opened. |
| Sync engine surface (ADR-0.6.0-async-surface) | Async lives only inside the adapter, at the protocol edge. Engine calls cross into blocking threads. |
| One process owns a database file (sidecar lock ADR) | Two deployment modes: embedded in the owning process, or a standalone process that is the sole opener (§3). |
| Deterministic, explainable retrieval; no model in the read path | The adapter shapes and labels; it never re-ranks with a model, summarises, or rewrites results. |
| Frozen reads are reproduce-or-fail, not retained snapshots (ADR-0.8.25) | Session consistency is a frozen read context per session with explicit drift, not a long-lived snapshot. |
| JSON Schema 2020-12 (ADR-0.6.0-json-schema-policy) | Tool input and output schemas are generated as 2020-12 (rmcp uses schemars 1.x). |
| Local-first, no egress by default | stdio only; no listener; no network transport without its own ADR. |
| Stale > missing; TDD with a real database | Every requirement is met by a test that drives the real engine through the real protocol in-process. |

## 2. Components

```text
            MCP client (agent harness)
                       │  JSON-RPC over stdio, or an in-memory transport
┌──────────────────────┼─────────────────────────────────────────────┐
│ fathomdb-mcp         ▼                                             │
│  ┌──────────────────────────────┐                                  │
│  │ Protocol edge (rmcp)         │  handshake, tools/list, call      │
│  │  - tool router               │  dispatch, cancellation, progress │
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐   ┌───────────────────────────┐  │
│  │ Session                      │──▶│ Policy (launch-time)      │  │
│  │  - frozen read context       │   │  read-only / write,       │  │
│  │  - change cursor             │   │  origin class for writes, │  │
│  │  - budget defaults           │   │  allowed origin classes   │  │
│  └──────────────┬───────────────┘   └───────────────────────────┘  │
│  ┌──────────────▼───────────────┐                                  │
│  │ Tool handlers                │  one per tool family (§4)        │
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐                                  │
│  │ Response shaper              │  handles, snippets, budget,      │
│  │                              │  continuation, completeness,     │
│  │                              │  degradation, deterministic order│
│  └──────────────┬───────────────┘                                  │
│  ┌──────────────▼───────────────┐                                  │
│  │ Engine bridge                │  blocking-thread hand-off,       │
│  │                              │  error mapping to MR-053 classes │
│  └──────────────┬───────────────┘                                  │
└─────────────────┼──────────────────────────────────────────────────┘
                  ▼
      fathomdb-sdk  (governed verbs; sync)  ──▶  fathomdb-engine
```

- **Protocol edge.** The only module that names `rmcp` types. Owns transport
  selection, the tool router, cancellation and progress notifications. `rmcp`
  is pinned with `=`; replacing or upgrading it touches this module only.
- **Session.** One per MCP connection. Holds the frozen read context, the last
  change cursor, and effective defaults. Holds no result sets; the store's own
  cursors and the frozen context carry continuity (AN-011, MR-050).
- **Policy.** Fixed at launch by the operator, never by the agent: read-only or
  write-enabled, the origin class stamped on writes, and the origin classes
  readable by default. The agent cannot widen policy through any tool (MR-011,
  MR-060).
- **Tool handlers.** Thin: validate, call the bridge, hand the governed result
  to the shaper. No ranking, filtering or eligibility logic of their own
  (MR-070).
- **Response shaper.** Turns governed results into agent-shaped output: handles
  (MR-042), snippets (MR-040), budget enforcement and continuation (MR-041),
  completeness and degradation labels (MR-030, MR-032), confidence verdict
  (MR-031), and canonical ordering and serialization (MR-043). Pure functions
  of their inputs, so they are property-testable.
- **Engine bridge.** Runs each SDK call on a blocking thread, applies the
  per-call deadline, and maps the SDK `ErrorKind` taxonomy onto the MR-053
  error classes with candidate suggestions.

## 3. Deployment and process topology

The database sidecar lock means one process owns a file. The architecture
therefore offers the same server in two shapes, from one crate:

| Mode | Who opens the `Engine` | Transport | Use |
| --- | --- | --- | --- |
| **Standalone** | The `fathomdb-mcp` binary, as sole opener | stdio | Harness-launched memory for one agent (Claude Code, OpenClaw, Hermes). |
| **Embedded** | The host application, which lends its open `Engine` | Any rmcp transport the host selects (in-memory, stdio, or its own) | An application such as Memex exposing its store to agents without a second opener (MR-003). |

- In standalone mode a second opener fails fast with the existing typed lock
  error (MR-003.2); the adapter never retries or steals the lock.
- Embedded mode is a Rust library API first. Exposure through the Python and
  TypeScript bindings is an open question (§10), not part of this proposal.

**Threads.** A small tokio runtime owned by the adapter in standalone mode
(borrowed from the host in embedded mode) runs the protocol edge. Engine calls
run on blocking threads, so concurrent read requests use the engine's existing
reader pool and writes serialize on its single writer. The adapter adds no
engine-level concurrency of its own.

**Protocol channel purity (MR-002).** stdout belongs to the protocol edge alone.
The binary installs a tracing subscriber on stderr before opening the engine,
routes any embedder or download progress to it, and converts handler panics to
JSON-RPC errors. A process-level test spawns the binary and parses every stdout
byte.

## 4. Tool families

Families, not schemas. Each family is one or a few tools; exact names and
arguments are design work.

| Family | Purpose | Governed verbs underneath | Requirements |
| --- | --- | --- | --- |
| Orient | One capped summary of what the store holds and what is ready | `read.projections`, `read.projection_status`, `read.embedding_readiness`, plus **[engine]** counts | MR-022 |
| Search | Relevance search with exact constraints, filters, origin-class restriction, budget | `search`, `search_text_only`, `search_frozen`, filter grammar | MR-020, MR-030–032, MR-040–041 |
| Fetch | Full content for one or many handles | `read.get`, `read.get_many` | MR-040.2, MR-051 |
| Expand | Bounded neighbourhood of a handle | `graph.expand`, `graph.neighbors`, `search_expand_frozen` | MR-052 |
| Explain | Why a hit ranked, with per-arm contributions | `explain` sidecar; negative explain is **[engine]**, later | MR-031 |
| Changes | What changed since a cursor | `read.mutations`, `read.crossed_boundary_since`, write cursors | MR-054 |
| Session | Refresh the frozen context; report snapshot identity | `freeze_read_context` | MR-050 |
| Remember *(write policy only)* | Record, supersede, retract | `write`, `transition` | MR-060–062 |

Consolidation follows the tool-design evidence in
`dev/notes/agent-tools-regrade-of-query-interfaces.md`: few tools with typed
options, `search` over `list`, and a concise default with a detailed mode.

## 5. Data flow of a search

1. The handler validates arguments against the input schema; unknown kinds or
   fields fail here with candidates (MR-053), using the store's own catalogue.
2. The session supplies its frozen read context; the bridge calls the frozen
   search verb on a blocking thread. Drift returns a typed drift result
   (MR-050.2), never newer data.
3. The governed result arrives with soft fallbacks, per-arm explain data and
   cursor. The shaper:
   - builds handles from identities (logical id when present);
   - cuts snippets and enforces the budget, emitting a continuation that encodes
     the snapshot and position;
   - derives completeness from the limit, the **[engine]** bounded count, and
     index readiness; derives degradation entries from soft fallbacks;
   - computes the confidence verdict from per-arm scores and their attainable
     maxima under the documented rule;
   - orders and serializes canonically.
4. The edge returns `structuredContent` plus a compact text rendering of the
   same data, both deterministic.

The shaper never drops a record the governed call returned except by limit or
budget, and every such drop is reported (MR-030, MR-041).

## 6. Trust model

- **Origin class is stored, not inferred.** Each record carries one origin class
  from a closed set. Two delivery stages, each behind its own ADR:
  1. *Interim:* the adapter stamps the class into a declared exact projection
     convention that the operator configures, which the ADR-0.8.25 rationale
     already anticipates ("declared exact projections let applications map
     owner/scope without FathomDB owning their semantics"). Records without it
     read as `unclassified`.
  2. *First-class:* an engine-level origin field beside `source_id`, filterable
     in the filter grammar and present on every hit path, so every SDK, not only
     the adapter, honours it. **[engine]**
- **Policy sets the class on writes.** The write tools take no origin argument;
  the session's policy decides (MR-011).
- **Content stays data.** Tool descriptions, server instructions and error
  templates are compile-time constants (MR-012). Results carry content only in
  designated fields; the text rendering marks them as quoted data.
- **Least privilege by default.** Read-only launch; write tools absent rather
  than refusing (MR-060). No SQL, no admin, no erasure tools: erasure stays an
  application and operator action through the SDK and CLI.
- **No egress.** stdio only. Any network transport needs its own ADR reviewing
  the rmcp HTTP advisories (origin checks, DNS rebinding, session handling).

## 7. Where engine work is needed

The adapter alone meets most requirements. These need SDK verbs first, each
under its own ADR, and each usable by every SDK, not only the adapter:

| Engine addition | For | Note |
| --- | --- | --- |
| Bounded match count and saturation on search and list | MR-030 | Read-side; no data-model change. |
| Attainable maximum per arm on explain | MR-031 | Extends the existing explain sidecar additively. |
| Exact-literal constraint composed with relevance search | MR-020 | Today `Json` is typed-rejected on `search` by design (ADR-0.8.11 D3); a lexical phrase constraint is a different predicate and needs its own decision. |
| Per-kind and per-origin counts, time bounds | MR-022 | Read-side aggregates. |
| First-class origin class | MR-010 | The only data-model addition; additive column under the no-data-migration rule. |
| Duplicate detection rule on write | MR-062 | Content-hash match against current records; policy owned by the engine. |

Until each lands, the adapter reports the corresponding signal as
`unavailable` rather than approximating it, consistent with MR-032.

## 8. Versioning and change control

- **rmcp** is pinned exactly and confined to the protocol edge; each rmcp major
  is a scoped migration with the conformance and determinism suites as the gate.
- **MCP protocol version** is negotiated by rmcp; the adapter declares the
  minimum it supports and tests at both ends of its range.
- **Tool contract** is versioned by the FathomDB interface document. Additive
  fields follow the existing "new optional field with default" rule; any change
  to a field's meaning, a handle format, or a default budget is a breaking
  change for agents and needs an interface update in the same change.
- **Workspace axis.** `fathomdb-mcp` joins Axis W lockstep versioning.

## 9. Verification architecture

- **In-process protocol tests.** rmcp's async read/write transport over a duplex
  pipe connects a real client to a real server over a real database file. No
  mock engine (`AGENTS.md` §10).
- **Process tests.** Spawn the binary for stdio purity (MR-002), the lock
  conflict (MR-003.2), and the listener check (MR-001.2).
- **Conformance.** The official MCP conformance scenarios for stdio and tools
  (MR-001.3).
- **Schema conformance.** Every structured output in the suite validated against
  its declared output schema (MR-071), which also guards rmcp issue #1374.
- **Property tests.** The shaper's budget, continuation (no gaps, no
  duplicates), ordering and snippet functions under `proptest`.
- **Determinism.** Two process lifetimes, same database, byte-compared outputs
  (MR-043); comparisons are run-to-run, never against agent-generated goldens.
- **Retrieval quality.** The MR-021 evaluation runs the adapter's default path,
  not the SDK's, so the number reported is the number agents get.

## 10. Open questions for the adopting ADR

1. Embedded mode across the Python and TypeScript bindings: expose a server
   entry point, or document stdio-only for those hosts.
2. Budget estimator: a fixed bytes-to-tokens ratio (portable, model-agnostic)
   versus a bundled tokenizer (accurate for one family, heavier). The proposal
   assumes the former.
3. Whether a session may pin a validity instant other than "now" for
   point-in-time questions, and how drift interacts with it.
4. Whether the write family ships in the first release or follows after the
   read family has a measured quality baseline (MR-021).
5. Distribution of the binary: crates.io only, or bundled with the Python wheel
   and npm package so harness configuration needs no Rust toolchain.
6. Interaction with the opt-in telemetry and `record_feedback`: whether "this
   hit was used" feedback from agents is accepted through the adapter.
