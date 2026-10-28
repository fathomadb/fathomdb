---
title: Agent access, Block 1 — finish plan and 0.8.29 consolidation
date: 2026-10-27
status: PROPOSED
desc: Consolidates Block 1 and 0.8.28 onto a new dev/0.8.29 branch, then clears the 19 open Block 1 follow-up todos there, then designs and builds Block 2. Owner rulings of 2026-10-27 recorded.
inputs:
  - dev/plans/agent-access-block1-plan.md
  - dev/plans/agent-access-block1-close-plan.md
  - dev/design/agent-access/README.md
  - dev/design/agent-access/compact-text-block.md
---

# Agent access, Block 1 — finish plan and 0.8.29 consolidation

## Purpose

Block 1 closed at `b5f539880` on `feat/agent-access-block1` with 19 open
todos. This plan moves it to a new integration branch, `dev/0.8.29`, clears
the todos there, and then runs Block 2 there. No work starts until the owner
says go.

## Owner rulings (2026-10-27)

- **Compact text block.** All four decisions in design § 7 are approved,
  with `text` as the default. Two tests are added to design § 5: a
  cross-mode equivalence test, and MCP conformance under both settings.
- **Priced runs.** Approved when needed: the LoCoMo agent smoke (seq 305)
  and the item-4 re-run.
- **Destination.** `dev/0.8.29`. Block 1 does not merge to `main` on its
  own; `main` gets it when 0.8.29 ships.

## Open question

1. **0.8.28 timing.** `release/0.8.28` is not on `main`, and it has 419
   commits that Block 1 lacks. Its one remaining slice, 42, is a test fix and
   an investigation document. A dry-run merge with Block 1 conflicts in 15
   files, including `search.rs`, `search_api.rs`, the SDK `lib.rs`, the
   Python `lib.rs` and TypeScript `binding.ts`.
   - **Recommended:** merge `release/0.8.28` into `dev/0.8.29` in Phase 0,
     before any workstream, and merge it again after Slice 42 lands. The
     conflicts are resolved once, against today's code, and W4 measures
     AC-013 on the combined build.
   - Alternative: wait until 0.8.28 lands on `main`. The conflicts then grow
     with every workstream.

B2-8 (seq 296) is asked in Phase 4, where it is needed.

## Phase 0 — consolidate `dev/0.8.29`

1. Cut `dev/0.8.29` from `main` at `b815048ce`, in its own worktree.
2. Merge `feat/agent-access-block1`. It is 2 ledger commits behind `main`.
3. Merge `release/0.8.28` (open question 1). Take each side's ledger rows
   by union, never by picking one file.
4. Add `dev/plans/release-state-0.8.29.json` and
   `dev/plans/runs/STATUS-0.8.29.md`, with this plan as the ladder.
5. Gate: `./scripts/agent-verify.sh`, `AGENT_LONG=1 ./scripts/check.sh` in
   release, the TypeScript tests and MCP conformance. Record the AC-013
   numbers as the W4 baseline.

Every workstream after this cuts its branch from the `dev/0.8.29` tip and
lands back on it.

## Phase 1 — Block 1 follow-ups (19 todos)

Same rules as the Block 1 build: TDD with one failing RED commit per
behaviour, one implementer per workstream in its own worktree, codex review
first (`/usr/lib/chatgpt/resources/codex`) with Fable-medium as the fallback,
and the full gate once per workstream on a clean tip.

| W | Scope | Todos | Kind |
| --- | --- | --- | --- |
| W1 | Test harness: make `tc57` and `slice50_evidence` parallel-safe; remove the `fathomdb-mcp` dead_code warning; add the `before_enqueue` hook and build queue test 23. The `slice135` half of 309 is 0.8.28 Slice 42 Unit A, so it arrives with the 0.8.28 merge | 309, 290, 307, 300 | code |
| W2 | Engine defects: drain never finishes on a dense-disabled store with pending vector rows; EP-03 degradations for an arm timeout and an absent embedder | 289, 301 | code |
| W3 | SDK surface: facade re-exports of the snippet and hint types; document Py/TS snippet offsets as UTF-8 bytes; binding parity test for `SnippetOptions` fields | 297, 310 | code |
| W4 | Perf: AC-013 headroom, measured from the Phase 0 baseline. First confirm the layout cause (S4 with hints compiled out), then recover headroom. Skip record-start segmentation only if its proptest holds | 306, 292 | code + measure |
| W5 | Measurements, no product code: EP-18 part A real-engine probe; `latency.py` rerun; the 5-call text slowdown; then re-check the overflow-chunking triggers against the new numbers | 298, 299, 304, 303 | measure |
| W6 | Compact text block, `text` default: fix the item-4 prompt sentence; build the 17 § 5 RED tests; release-notes line for the default change | 316 | code |
| W7 | Fusion identical-body merge: ADR draft and Memex duplicate census; no code | 295 | design |
| W8 | Close: CHANGELOG entry for the `#[non_exhaustive]` break and the `text` default; LoCoMo smoke; item-4 re-run against the built server | 308, 305 | docs + priced |

**Order:**

- W1 first; it steadies the gates the others rely on.
- W2, W3 and W4 one after another; all three touch the engine.
- W6 after W1, in parallel with W2–W4; it touches only `fathomdb-mcp`.
  Landings stay serialized.
- W5 and W7 in parallel with the code work.
- W8 last, on the final tip.

**Phase 1 exit:** `agent-verify`, `AGENT_LONG` in release (AC-013 must
pass), the feature-complete gate, TypeScript tests, MCP conformance under
`text` and under `structured`, and the LoCoMo smoke. Mark each todo done with
its commit, and record the result in `dev/plans/agent-access-block1-plan.md`.

## Phase 2 — Block 2 (agent feedback signals)

Design can start during Phase 1. Code starts after the Phase 1 exit.

1. **Owner approval of B2-8** (seq 296) as a Block 2 row: the long-record
   guard, which reports the embedded extent in `orient` and the snippet.
2. **Design** B2-1 to B2-8 in one document, building on the shapes Block 1
   defined (hints sidecar, queue). Check B2-7 against the built queue first;
   it may already be met. B2-1 must agree with compact text, which drops the
   `state` and `verdict` placeholders under `text` but keeps them under
   `structured`.
3. Fable-medium review and fixes, then owner review. No code before it.
4. Build in its own worktree off `dev/0.8.29`, with the same gate as Phase 1.

## Other 0.8.29 work

There is no other built 0.8.29 work on any branch, so none needs merging.
These items are placed in 0.8.29 but not started, and are planned
separately:

- **CUDA context-reset survival** (seq 281). Design depends on the 0.8.28
  D28-08 instrumentation data (seq 285), which arrives with the 0.8.28
  merge.
- **Candidate-selection review checkpoint** (ROADMAP § 0.8.29): an owner
  decision on whether to run the experiments.
- **Placed experiments** (ROADMAP § 0.8.29): coalescing of concurrent work,
  and the SDK hints default.

Out of scope and left as they are: `dev/0.8.30` and
`llm/deepseek-approach-scale-note`, which are live work in other sessions,
and three stale 0.8.27/0.8.28 branches (`llm/0.8.28-filtered-body-fts-roadmap`,
`llm/sdk-python-driver`, `llm/sdk-rust-driver`).
