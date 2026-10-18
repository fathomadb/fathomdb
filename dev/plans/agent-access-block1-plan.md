---
status: ACTIVE
date: 2026-10-18
depends_on: owner approval of the Block 1 designs, 2026-10-18
---

# Agent access, Block 1 — build plan

## Purpose

Build Block 1 of `dev/design/agent-access/README.md`: items 1 (admission
queue), 2 (hints, stage 1), 3 (match-centred snippets) and 6 (MCP server
defects). Prompt: `dev/plans/prompts/agent-access-block1-build-prompt.md`.
Integration branch `feat/agent-access-block1`; it is not merged to `main`.

## Decisions taken in this plan

- **Stdin close (item 6.2).** The server exits 0 whenever stdin closes,
  before or after `initialize`. It exits 1 when the handshake fails for any
  other reason. `dev/interfaces/mcp.md` § 1 states both cases.
- **Vector side disabled (item 6.3).** `VectorEquivalenceMismatch` maps to
  `not_ready` with the remedy "use mode text", the adapter-only fix that
  EP-03 names. It is a named, actionable error, not a degradation, because the
  engine refuses the call before any work.
- **EP-03 scope.** Only `embedder_overloaded` is built (from `Saturated` and
  `QueuedExpired`). `SearchResult.degradations` is a `Vec` that is empty unless
  an arm degraded. In the bindings it is present only when non-empty, so the
  R-OBS-2 byte contract holds and EP-03 open question 1 needs no ruling now.
- **Snippet bindings.** EP-17 T-12 (Python and TypeScript parity) is in the
  design's test plan, so it is in Block 1 as S5d. Hints stage 2 (the SDK
  carrier) is not.
- **Provisional values.** `run_limit_ms` (60 s) and the p90 start check are
  built as configuration with those defaults.

## Slices

Each RED test is its own failing commit. Test names are the designs' names.

| Slice | Scope | Crates | RED tests |
| --- | --- | --- | --- |
| S0 | Item 6: ordered progress; exit 0 on stdin close; `VectorEquivalenceMismatch` → `not_ready` | `fathomdb-mcp` | `progress_notifications_arrive_in_order`, `stdin_closed_before_initialize_exits_zero`, `hybrid_on_dense_disabled_store_is_not_ready_use_text` |
| S1a | Queue core: queue-admission § 6.1 acquire sequence, busy object, `queue_limit`, `run_limit_ms` config, start check and window | `fathomdb-mcp` | queue-admission § 9 tests 1–12, 17–19, 21, 25–27 (26 moves to S3 if the SDK error cannot be built in a unit test) |
| S1b | Run to completion: run limit, abandoned, stuck, idle/drain/shutdown with waiters | `fathomdb-mcp` | § 9 tests 13–16, 20, 22–24 |
| S2 | EP-18 part A: idle-first reader dispatch | `fathomdb-engine` | § 9 tests 28, 29 |
| S3 | EP-03 `embedder_overloaded`: engine `degradations` (incl. frozen and `search_expand`), bindings, adapter mapping | engine, py, napi, `fathomdb-mcp` | § 9 test 30, plus the EP-03 byte test |
| S4 | Hints stage 1: engine `hints.rs`, adapter wire type and reasons, `hints` config and argument, ADR, perf-gates row | engine, `fathomdb-mcp` | hints-sidecar § 8 Block 1 tests 1–18 |
| S5a | EP-17 pure selector `snippet_for` | engine, sdk | T-4, T-5, T-11, T-17, T-18 |
| S5b | EP-17 engine point queries and sidecar | engine, sdk | T-1, T-2, T-3, T-6, T-7, T-8, T-9, T-10, T-14, T-15, T-16, T-19, T-20 |
| S5c | EP-17 adapter: `snippet_bytes`, `snippet_anchor`, head, budget re-selection | `fathomdb-mcp` | T-13 |
| S5d | EP-17 bindings parity | py, napi | T-12 |
| S6 | Close: full gate on a clean tree; MCP conformance re-run; LoCoMo agent smoke | — | — |

Order: S0; then S1a ∥ S2; S1b; S3; S4; S5a–S5d; S6. Slices that share a crate
run one after another.

## Per-slice record

| Slice | Branch | Landed at | Review | Status |
| --- | --- | --- | --- | --- |
| S0 | `feat/aa-b1-s0` (`ba992064e`..`e519273d2`) | `47c560615` | Fable-medium (codex blocked by the harness): PASS-WITH-FIXES, non-blocking; drain defect filed as a todo | landed |
| S2 | `feat/aa-b1-s2` (`7f995d843`..`54c1451ab`) | `54f762a53` | Fable-medium: PASS-WITH-FIXES, doc fixes applied | landed |
| S1a | `feat/aa-b1-s1a` | — | — | in progress |
