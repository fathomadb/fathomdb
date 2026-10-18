# Prompt — build agent-access Block 1

You are the main thread, the orchestrator, for the agent-access Block 1
build.

**Setup:**
- Integration branch: `feat/agent-access-block1`, cut from
  `origin/exp/agent-access`.
- Integration worktree:
  `/home/coreyt/projects/fathomdb-worktrees/agent-access-block1`.

**Owner rulings in force:**
- The owner approved the Block 1 designs on 2026-10-18.
- **Do not merge to `main`.**
- Do not start Block 2 (rows B2-1 to B2-7) until Block 1 is complete and
  every test passes.

## Rules, front-loaded

1. Read `AGENTS.md` and the memory index, then the entries below. Follow
   them over any default.
   - `tdd-red-must-fail-per-finding.md`: every behaviour change gets its own
     RED commit with recorded failure evidence. No big-bang GREEN, and no
     "not RED" coverage tests.
   - `slice-40-41-fable-review-gates.md`:
     - Fable-medium review of the requirements, acceptance criteria and
       tests before code.
     - TDD.
     - Fable-medium code review after.
   - `orchestration-execution-traps.md`, `agent-worktree-stale-base-trap.md`
     and `verify-the-agents-fix-not-just-its-diagnosis.md`.
   - `full-gate-run-frequency.md`: scoped checks while working. Run
     `./scripts/agent-verify.sh` once per code phase, on a final clean tree.
   - `agent-access-block1-design-state.md`.
2. **Topology.**
   - You plan, verify from git and land. You write no product code
     yourself.
   - Each slice goes to one `implementer` subagent, in a fresh worktree cut
     from the current tip of `feat/agent-access-block1`. Run
     `scripts/preflight.sh --worktree <wt>` before each spawn.
   - One writer per checkout. Run slices that touch the same crate one
     after another, not in parallel.
   - Review gate: codex § 9 review of each slice branch is the reviewer of
     record. If codex is unavailable, use an independent adversarial
     Fable-medium review subagent, so the build doesn't stall.
   - Land by cherry-pick or merge into `feat/agent-access-block1`, then
     verify from git.
3. **Contracts.**
   - Public surface is contract. Update `dev/interfaces/mcp.md`, the
     interface doc for any engine `pub` change, and the design docs in the
     same slice.
   - No mocked database: integration tests hit a real store.
   - No schema migrations; none are expected.
   - No backward-compatibility shims.
4. **Never:**
   - `--no-verify`, or a force-push;
   - a `v*` tag;
   - prettier or unguarded `markdownlint --fix` on markdown;
   - destructive operations without asking the owner;
   - the K620 GPU. Use the RTX 3090 `cuda:0` only.
5. **Python.** Each worktree uses its own checkout-owned `.venv`
   (`AGENTS.md` § 3). If one is missing, report the preflight failure. Never
   borrow another checkout's `.venv`, and never `pip install -e` from a
   worktree.
6. **Push.** Push the integration branch and the slice branches freely, as
   explicit refs.

## Read before planning

These are all on the integration branch.

- **`dev/design/agent-access/README.md`:** the work list. Block 1 is items
  1, 2, 3 and 6. Items 4 and 5 are evaluations that run after this build.
  Block 2 is the table.
- **`dev/design/agent-access/queue-admission.md`:**
  - § 6.1 is the adapter queue: fair semaphore, FIFO depth 32, a 10 s wait
    deadline, run to completion under `run_limit_ms`, the p90 start check,
    and the busy codes.
  - § 6.2 is EP-18 part A: try an idle reader first, otherwise round-robin.
  - § 8 lists the artefacts to change, and § 9 is the 30-test RED plan.
  - The p90 start check and `run_limit_ms` (60 s) are **provisional**. Build
    them as designed, as configuration with those defaults. A parallel
    simulation (`dev/plans/prompts/queue-run-to-completion-test-prompt.md`)
    may change the defaults later. Do not wait for it.
- **`dev/design/agent-access/hints-sidecar.md`:**
  - § 3 (placement) and § 3.2 (the split) define stage 1, which is Block 1:
    the pure engine subset in `fathomdb-engine`, plus the adapter wire type
    and the adapter-only reasons.
  - Also read § 4 (cost budget), § 5 (configuration: MCP on by default) and
    § 8 (tests).
  - Stage 2, the SDK carrier, is **not** in Block 1. Its SDK default is
    decided by the 0.8.29 tests.
  - File the § 11 draft ADR under `dev/adr/` as accepted (owner approval,
    2026-10-18) in the first hints slice.
- **`dev/design/agent-access/match-centred-snippets.md`:**
  - Build § 7 (the design), § 6 (text handling) and the § 8 RED plan.
  - The § 10 decisions apply: 300 bytes, a head snapped to a word boundary,
    highlights off by default, the exact per-term `highlight()` rule, and
    the prefix fallback for vector-only hits.
  - Also read the engine proposal,
    `dev/design/mcp-adapter/engine-proposals/EP-17-match-centred-snippets.md`.
- **`dev/design/mcp-adapter/engine-proposals/EP-03-hybrid-search-degradation.md`:**
  build only the `embedder_overloaded` reason.
- **Background:** `dev/design/agent-access/queue-lock-trace.md` and
  `dev/experiments/queue-sim/REPORT.md`.

## Proposed slices

Confirm or adjust this ladder in a short plan file,
`dev/plans/agent-access-block1-plan.md`. Each slice must name its RED tests,
taken from the designs' test plans.

| Slice | Scope | Crates |
| --- | --- | --- |
| S0 | Item 6 defects: ordered progress notifications; the stdin-close exit code (decide the contract and update `mcp.md` § 1); the vector-disabled `internal` error becomes a degradation or a named error | `fathomdb-mcp` |
| S1 | Admission queue, adapter side: queue-admission § 6.1 and the adapter tests in § 9; the `mcp.md` contract change | `fathomdb-mcp` |
| S2 | EP-18 part A: idle-first reader dispatch (§ 6.2; the two engine tests in § 9) | `fathomdb-engine` |
| S3 | EP-03: `embedder_overloaded` | `fathomdb-engine`, adapter mapping |
| S4 | Hints stage 1: engine subset `no_results`, `degraded`, `more_results`; the adapter wire type and reasons `snippet_cut`, `body_cut`, `cut_by_budget`, `default_store_set`; configuration; the ADR | engine, `fathomdb-mcp` |
| S5 | EP-17 match-centred snippets: the engine `snippet_for` selector and the point `highlight()` queries; the adapter options; a hint for vector-only hits if the hints design names one | engine, `fathomdb-mcp` |
| S6 | Close: full `./scripts/agent-verify.sh` on a clean tree, and a re-run of the MCP conformance checks and the LoCoMo agent smoke from `dev/experiments/mcp-agent-loop/` | — |

**Order:**
- S0 first. Then S1 and S2, in either order. S3 goes after S2 because both
  touch the engine. S4 goes before S5, since snippet cuts feed the hints.
  S6 is last.
- Slices in different crates may run in parallel, each in its own worktree.
- The Block 2 rows must remain additions to these shapes, not redesigns.
  Check this at each review.

## Per-slice loop

1. Write the slice brief: scope, RED test list, acceptance criteria and the
   docs to update.
2. Get a Fable-medium review of the brief and its tests. Fix what it finds.
3. Spawn the implementer: RED commits with failure evidence, then GREEN,
   then refactor. Tests are read-only during fix-to-spec.
4. Verify from git:
   - every RED commit fails on its own;
   - every GREEN commit passes;
   - the scoped lint, typecheck and tests pass.
5. Run the codex § 9 review, or the Fable-medium fallback. Each fix gets its
   own RED commit.
6. Land on `feat/agent-access-block1` and push.
7. Record the slice's commits and status in the plan file.

## Done means

- All Block 1 slices are landed and every test in the designs' test plans
  exists and passes.
- The full gate is green on a clean tree.
- The interface and design docs match the code.
- The plan file records each slice's commits.
- A short report to the owner covers:
  - what landed;
  - any deviations from the designs, and why;
  - the gate result, quoted;
  - what Block 2 and item 4/5 need next.

Do not merge to `main`.
