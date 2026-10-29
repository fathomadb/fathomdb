---
title: Agent access, Block 1 — finish plan and 0.8.29 consolidation
date: 2026-10-27
status: ACTIVE
desc: Consolidates Block 1 and 0.8.28 onto a new dev/0.8.29 branch, then clears the 19 open Block 1 follow-up todos there, then designs and builds Block 2. Owner rulings of 2026-10-27 recorded. This copy, on dev/0.8.29, is the plan of record.
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
the todos there, and then runs Block 2 there.

## Where this stands

A fresh session starts here. Verify each line from git before acting on it.

- **Plan of record.** This file on `dev/0.8.29`. The copy on
  `feat/agent-access-block1` stopped at `d5768dc07` and is not updated.
- **Phase 0: done** on 2026-10-28; `dev/0.8.29` head `e336cfc09`. Step 4,
  the 0.8.29 release state and board, waits until 0.8.28 publishes; see
  Phase 0.
- **Phase 1: W4 has started.** The AC-013 regression investigation began on
  2026-10-29; see W4. No other workstream has started.
- **Phase 2: not started.** B2-8 (seq 296) still needs owner approval.

## Owner rulings (2026-10-27)

- **Compact text block.** All four decisions in design § 7 are approved,
  with `text` as the default. Two tests are added to design § 5: a
  cross-mode equivalence test, and MCP conformance under both settings.
- **Priced runs.** Approved when needed: the LoCoMo agent smoke (seq 305)
  and the item-4 re-run.
- **Destination.** `dev/0.8.29`. Block 1 does not merge to `main` on its
  own; `main` gets it when 0.8.29 ships.

B2-8 (seq 296) is asked in Phase 2, where it is needed.

## Phase 0 — consolidate `dev/0.8.29` (done 2026-10-28)

The 0.8.28 timing question was settled by the order of the work. 0.8.28 was
consolidated first, on `dev/0.8.28`, and `dev/0.8.29` was cut from it.

1. **`dev/0.8.28`.** Cut from `release/0.8.28` at `19704c6a7`, then `main`
   merged in (`6784421d6`). The todos ledger had forked: `main` and the
   release line both numbered new rows from 287. `main`'s numbering was kept,
   and the release-only rows were appended again as seq 320–349, each
   carrying a `source-seq:<old>` ref. The merge body has the full map.
   `388202442` updated the docs that cited the old numbers. Cite todos by TC
   id; seq numbers differ between branches.
2. **`dev/0.8.29`.** Cut from `dev/0.8.28` at `c02cff1f9`, then
   `feat/agent-access-block1` merged in at `d5768dc07` (`b2eca660d`).
   - Four fixes followed: `fa6890c34`, `feee12482`, `507dee704` and
     `e336cfc09`. The last restores the FTS rowid column. Without it,
     projected search silently fell back to the full sort.
   - The decision-index row for the hints-sidecar ADR moved to 66, under a
     new "Phase 0.8.29" heading. The owner approved this on 2026-10-29.
3. **Ledger rows.** Taken by union. Todos run 1..349 and the steward ledger
   1..346, both identical to `dev/0.8.28`.
4. **Release state and board: deferred.** `scripts/release-current.py`
   requires exactly one live release, and 0.8.28 is still unpublished.
   Adding `release-state-0.8.29.json` now would fail that check. Add it when
   0.8.28 publishes. Until then, this plan is the 0.8.29 ladder.
5. **Gate at `e336cfc09`.** All green except AC-013:
   - `agent-verify` passed;
   - the release `AGENT_LONG` perf gates: 16 pass, AC-013 fails;
   - feature-complete with CUDA: 166 pass, 0 fail;
   - TypeScript: 612 of 612;
   - `fathomdb-mcp` tests: 191 of 191;
   - MCP conformance: 9 pass, 24 fail, identical in every scenario to the
     Block 1 baseline.

   The AC-013 baseline for W4 is p50 123 ms, p99 135–139 ms and seed time
   3.9–4.2 s, against an 80 ms p50 limit.

`dev/0.8.28` will be merged in again after 0.8.28 Slice 42 lands. Every
workstream cuts its branch from the `dev/0.8.29` tip and lands back on it.

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

**W4 re-scoped on 2026-10-29.** The 81–82 ms Block 1 delta is now dwarfed by
a 0.8.28 regression. Release bisect, AC-013 p50 in ms:

| Code at | p50 (ms) |
| --- | --- |
| `main` | 79 |
| Before the Slice 40 merge (`869d6a9e1^1`) | 78 |
| After the Slice 40 merge (`869d6a9e1`) | 119 |
| Before Unit I (`39255751d`) | 80 |
| Slice 40 Unit I (`65e6eff7e`) | 122 |

Unit I removed `SQLITE_ENABLE_MEMORY_MANAGEMENT` (ADR-0.8.28 page cache).
That gave each connection its own 2,000 KiB cache, where before all
connections shared one larger pool. W4 now runs first, ahead of W1, in
three steps:

1. Measure AC-013 through the MCP server and its queue as well as at the
   engine. Fix `scripts/perf-experiments/run-ac013.sh` and the
   perf-canonical workflow, which leave out `--features test-hooks`.
2. Find the cause with a controlled sweep that has a stated decision rule.
3. Bring the fix options to the owner. A fix that changes the per-connection
   cache amends the ADR, so the owner decides.

Todo TC-1b1f69e6 (seq 350 on `dev/0.8.29`) tracks the regression; seq 306
(TC-016ff172) remains the headroom item.

**Order:**

- W4 steps 1–2 first (owner, 2026-10-29). Then W1, which steadies the
  gates the others rely on.
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
