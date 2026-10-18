---
title: ADR-0.8.29-agent-hints-sidecar
date: 2026-10-18
target_release: 0.8.29
desc: A FathomDB-authored hints sidecar of suggested next calls — engine subset plus MCP adapter wire type, trust rule, defaults, budget accounting and versioning.
blast_radius: fathomdb-engine (new pure hints module), fathomdb-sdk (re-export), fathomdb-mcp (output, tools, config); dev/interfaces/mcp.md, dev/interfaces/rust.md
status: accepted (repository owner, 2026-10-18)
---

# ADR-0.8.29 — Agent hints sidecar

**Status:** accepted. The repository owner approved the design on 2026-10-18
(`dev/design/agent-access/hints-sidecar.md` § 10, "Owner decisions,
2026-10-18"). The design is the specification; this record states the
decisions it rests on.

## Context

In the LoCoMo MCP runs, agents acted on explicit instructions but ignored
flags. They answered from prefix snippets marked `snippet_cut: true` and
rarely fetched, and they called `list_stores` although a default store was set.
When the guidance was explicit, the guided arm fetched on 147 of 150 questions
(`dev/experiments/mcp-agent-loop/README.md`). A response that names the next
call worth making, in a form an agent can execute without inference, applies
that finding.

Precedent: the HITL-ratified explanation sidecar added an optional field on
`SearchResult`, `None` and zero-cost when off, with `SearchHit` untouched
(`dev/design/0.8.8-explain-and-telemetry-adr.md`).

## Decisions

- **(a) Split.** `fathomdb-engine` owns a pure engine subset
  (`fathomdb-engine/src/hints.rs`: no SQL, I/O, clock or locks) with the
  reasons `no_results`, `degraded` and `more_results{limit}`, reachable by
  every SDK. `fathomdb-mcp` owns the agent-facing wire type, converts the
  engine subset (attaching continuations) and adds the adapter-only reasons
  `snippet_cut`, `body_cut`, `cut_by_budget` and `default_store_set`. The one
  agent vocabulary table is in `dev/interfaces/mcp.md`.
- **(b) Trust.** Every hint string is a closed-enum word, an adapter-minted
  handle, an operator-configured store name, or a ref copied from the same
  response. Refs longer than 128 bytes are left out of hints and counted. In
  text blocks every ref is JSON-quoted, in hint lines and in hit lines. A
  cut-ref hint lists every cut ref, up to 50. Hint text lines come from
  compile-time templates.
- **(c) Defaults and precedence.** Hints are on by default for MCP. The SDK
  option is off until the 0.8.29 tests decide its default (owner ruling on
  question 3, recorded on `dev/ROADMAP.md`). Precedence is per call, then per
  process, then built in; there is no operator lock.
- **(d) Budget.** Hints share the response budget (bytes ÷ 4) with no
  separate allowance. With hints on, a page may keep fewer results; every
  response stays internally consistent, and hints are not part of the
  canonical request, so the continuation handle does not depend on the
  setting except through the kept count.
- **(e) Versioning.** The enums are closed per interface revision. Clients
  ignore a hint whose `action` or `reason` they do not know. There is no
  per-response version field.
- **(f) Block 2.** `state` and `verdict` become optional and are omitted
  while they can only be `unavailable`; they are re-added additively when the
  state read, or a calibrated verdict with its ADR, lands. `orient` lists what
  is missing in `not_in_this_release`. This decision is applied with the
  Block 2 code, not in stage 1.

## Stage 2

The `hints: bool` option on `search`, `search_text_only` and `search_frozen`
across Rust, Python and TypeScript, and `SearchResult.hints:
Option<Vec<Hint>>` filled from the engine subset. It is an option, not an
operation, so the governed-operation parity manifest is unchanged. Its default
is off until the 0.8.29 tests decide (decision (c)).

## Consequences

- Agents get executable next calls. MCP output grows by about 150 estimated
  tokens on a fully cut 10-hit page.
- A stated overhead gate joins the devloop tier of `dev/design/perf-gates.md`
  with the tests that measure it.
- Quoting refs in hit lines changes text-block bytes with hints off too; that
  change lands with its interface update.

## Options considered

- **SDK-first placement, moved down later.** Rejected: the Python and
  TypeScript bindings depend on `fathomdb-engine` directly, so a rule all
  three SDKs share has to live there.
- **Hints off by default for MCP.** Rejected: the evidence is that agents
  act on explicit actions and skip flags; off is one argument away.
- **A separate token allowance for hints.** Rejected: it would let a
  response exceed `max_tokens`.
