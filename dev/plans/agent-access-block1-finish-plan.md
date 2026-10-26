---
title: Agent access, Block 1 — finish plan
date: 2026-10-26
status: PROPOSED
desc: Clears every open Block 1 follow-up todo on feat/agent-access-block1 so Block 2 starts from a clean, gated base.
inputs:
  - dev/plans/agent-access-block1-plan.md
  - dev/plans/agent-access-block1-close-plan.md
  - dev/design/agent-access/README.md
  - dev/design/agent-access/compact-text-block.md
---

# Agent access, Block 1 — finish plan

## Purpose

Block 1 is built and closed at `b5f539880`, but it left 19 open todos on
`main`. This plan clears them on `feat/agent-access-block1`. Block 2 starts
after it.

## Owner questions

These change the plan. Everything else proceeds on the defaults below.

1. **Compact text block, design § 7.** Approve the four decisions: default
   `text`, a server-wide setting, the field keep/drop list, and fetch
   included? W6 waits for this.
2. **Priced runs.** Authorise the LoCoMo agent smoke (seq 305) and the item-4
   re-run after W6 (about $1.23 last time)? Without this, the close skips
   both and says so.
3. **Destination.** Does finished Block 1 merge to `main` for 0.8.29, or stay
   on its branch? A merge needs the CHANGELOG entry (seq 308) and lifts the
   standing "do not merge to `main`" rule.

Not asked here, and left open: B2-8 as a Block 2 row (seq 296), which is
Block 2 scope.

## Rules

- Same as the Block 1 build: TDD with one failing RED commit per behaviour,
  an implementer per slice in its own worktree cut from the integration tip,
  and engine and MCP slices serialized.
- **Review gate.** Try codex first, at `/usr/lib/chatgpt/resources/codex`
  (it is not on PATH). Fall back to a Fable-medium review if it fails.
- The full gate runs once per workstream, on a clean tip.
- GPU work uses the RTX 3090 `cuda:0` only.

## Workstreams

| W | Scope | Todos | Kind |
| --- | --- | --- | --- |
| W1 | Test harness: make the `tc57`, `slice135` and `slice50_evidence` tests parallel-safe; remove the `fathomdb-mcp` dead_code warning; add the `before_enqueue` hook and build queue test 23 | 309, 290, 307, 300 | code |
| W2 | Engine defects: drain never finishes on a dense-disabled store with pending vector rows; EP-03 degradations for an arm timeout and an absent embedder | 289, 301 | code |
| W3 | SDK surface: facade re-exports of the snippet and hint types; document Py/TS snippet offsets as UTF-8 bytes; binding parity test for `SnippetOptions` fields | 297, 310 | code |
| W4 | Perf: AC-013 headroom. First confirm the layout cause (S4 built with hints compiled out), then recover headroom. Skip record-start segmentation only if its proptest holds | 306, 292 | code + measure |
| W5 | Measurements, no product code: EP-18 part A real-engine probe; `latency.py` rerun; characterise the 5-call text slowdown; then re-check the overflow-chunking triggers against the new numbers | 298, 299, 304, 303 | measure |
| W6 | Compact text block, after question 1: optional codex re-review, fix the item-4 prompt sentence, build the 15 § 5 RED tests; item-4 re-run if question 2 allows | 316 | code |
| W7 | Fusion identical-body merge: ADR draft and Memex duplicate census; no code change | 295 | design |

**Order:**

- W1 goes first, because it steadies the gates the others rely on.
- Then W2, W3 and W4 run one after another, since all three touch the engine.
- W6 follows W1, because both touch `fathomdb-mcp`.
- W5 and W7 run in parallel with the code workstreams, each in its own
  worktree.

## Close

On the final tip:

- `./scripts/agent-verify.sh`;
- `AGENT_LONG=1 ./scripts/check.sh` in release, where AC-013 must now pass;
- the feature-complete gate;
- the TypeScript tests;
- MCP conformance;
- the LoCoMo smoke, if question 2 allows.

Then:

- mark each todo done with its commit;
- record the result in `dev/plans/agent-access-block1-plan.md`;
- do seq 308 and the merge if question 3 says so.
