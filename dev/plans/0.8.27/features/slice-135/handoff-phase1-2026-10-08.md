---
title: Slice 135 direct-execution hand-off — full Phase 1 checkpoint
status: READY_TO_CONTINUE
target_release: 0.8.27
---

# Slice 135 direct-execution hand-off

Continue Slice 135 until the **full Phase 1 checkpoint** is written for the
exact post-Slice-132 0.8.27 candidate. Work directly as the Slice 135 agent.
**Do not use Steward or Orchestrator**, and do not defer execution to those
roles. Follow the [approved plan](plan.md), [broader protocol draft](phase1-protocol-draft.md),
[capability register](phase1-capability-register.md), and repository
`AGENTS.md`. Ask the owner only for a real missing decision or permission;
continue independent work meanwhile. Dedicated gold-answer correctness
scoring is Phase 2 and must wait until this four-area checkpoint exists.

## Starting state to verify from Git

- Worktree: `/home/coreyt/projects/fathomdb-worktrees/release-0.8.27-slice-135`;
  branch: `llm/0.8.27-slice-135`. Do not create a replacement worktree merely
  to continue. Verify `git status` and product source before using old
  receipts. Do not mutate a concurrent writer's checkout.
- Comparison baseline: exact 0.8.26 source
  `f99e002f0d2e4002f3694c9f8d4986b56089edaa`. Latest measured product
  candidate: `224e44c593c13d86ece648adabe445723db04070`, with a clean
  detached build checkout at `/tmp/slice135-candidate-224e44`.
  The Slice 135 branch has later documentation-only commits. Compare product
  trees and build hashes, never branch labels or version strings alone; the
  candidate package still reports a 0.8.26 prerelease version.
- The candidate includes the corrected embedder-close landing `96796fe04`
  and Tegra install-route landing `c23e2d23f`. Close now drops the engine-owned
  model after workers drain, changing memory timing at `close()` relative to
  0.8.26. Slice 150 must verify the published `0.8.26+tegra` install route.
- The full Phase 1 protocol is **not frozen**. Individual E01–E12 and Python
  S01/S02 protocols are frozen to exact binaries/wheel and audited. Prior
  candidate receipts are historical after the search error repairs.

## Evidence already in hand

- [E01–E12 paired engine](results/2026-10-08-e12-224e-paired/README.md): 20
  alternating blocks, twelve cells, independent raw/order/state/resource
  audit. Median five-pair p50 change: vector stage +15.67%, hybrid +14.54%,
  populated open +12.14%. Query leads survive four warning-free pairs;
  populated-open has only one warning-free pair. This is an engine boundary.
- [Installed Python S01](results/2026-10-08-python-s01-224e-paired/README.md):
  20 blocks and 60,120 independently checked materialized calls, 32/256-row
  text/vector/hybrid cells. Median vector p50 is -3.425%/-0.045% at the two
  sizes; use the receipt for exact values and warnings. The 256-row vector
  value is not a meaningful speed conclusion.
- [Installed Python S02](results/2026-10-08-python-s02-224e-paired/README.md):
  100 valid fresh-process sequences per version with raw, order and reopened
  SQLite audits; pooled whole-sequence p50 +0.287% and p95 +0.206%.
  Close/reopened-close and reopened-open are attribution leads. Candidate
  peak RSS is materially lower; five pairs do not prove equivalence.
- [Selected engine coverage overlay](results/2026-10-08-e12-current-coverage-refresh/README.md)
  is at older source `8c2455b6c`: 20 targeted test binaries hit all 888
  workload-hit engine branch IDs and 8,120/8,133 workload-hit lines. Refresh
  on `224e44c59`; it is selected-route overlap, not production traffic or
  assertion strength. Earlier four-of-eleven operation ranking reached
  84.12% of synthetic elapsed cost.
- [Capability register](phase1-capability-register.md): 44 governed
  operations. At earlier source `d465cd56d`, installed Python and TypeScript
  each had 41 selected positive cases executed, zero failed, three
  provider/model cases unavailable; Rust had 42 selected cases. Selected
  calls do not cover all filters, errors, cancellation or concurrency.
  Rust has no same-SDK 0.8.26 peer; compare common engine behavior separately.
- Real-database robustness receipts cover projection recovery, provider
  terminal failure, persistent SQLite-full, interrupted erasure, contention,
  close and reopen. Test-first repairs fixed several suppressed row errors
  in search/graph/vector/evidence/fallback. Read the current plan's status
  section for exact receipts; do not infer a final matrix from pass counts.
- [C01 qualification failure](results/2026-10-07-c01-qualification/README.md):
  pinned LOCOMO corpus and nonempty Mem0/Qdrant volumes exist, but the exact
  external harness, config and output root are missing. Claim no matched
  comparator timing until recovered and qualified.

## Execution sequence

1. Confirm product/source identity and available disk before any expensive
   build. Refresh selected E01–E12 workload/test coverage at `224e44c59`
   using separate instrumented workload and targeted-test runs. Audit the
   line/branch overlay and remaining 13 historical gaps. Search in-crate
   `#[cfg(test)]` modules as well as `tests/` before calling a path untested.
2. Attribute the engine vector/hybrid/populated-open leads against Python's
   installed whole-call results. Align work, corpus, lifecycle and timing
   boundaries; inspect CPU/queue/resource traces separately from unprofiled
   latency. Rebuild exact-source TypeScript packages and refresh S01/S02/S03
   where supported. Refresh the Rust SDK as candidate-only and Python S03 or
   S02-L where source impact or protocol requires. Do not hide a slow valid
   pair or equate distinct call boundaries.
3. Finish baseline pilots and negative fixtures for the common mixed
   workload, determine supported sample counts and reporting rules, and
   freeze one executable, hashed broader Phase 1 protocol before remaining
   candidate timing. Use its operation mix for Pareto, system latency and
   coverage. Report the smallest observed set contributing 80% of elapsed
   cost, separately rank CPU/queue cost, and inspect rare severe paths.
4. Complete the real-database robustness matrix: named crash positions around
   commits/queue transitions, one-shot and persistent provider/SQLite busy or
   permission faults, more interrupted-erasure positions, close/cancellation
   under load, FFI boundaries and resource/reopen oracles. Complete static,
   error-conversion and panic review of hot and boundary paths with targeted
   mutation/property/state probes. Disposition each finding or survivor.
5. Reconcile exact-candidate positive and negative SDK exercise across all
   44 operations, the two off-ladder landings, and invalid/omitted cells.
   Independently recompute raw receipts. Write a single dated four-area
   checkpoint report with source/artifact hashes, protocols, measured
   results, limitations, confirmed defects, residual risk and ranked
   follow-ups. Run the needed full workspace gate at this checkpoint.
   Keep Phase 2 unopened until this record is complete.

## Keep the work efficient and trustworthy

- Use scoped validation for plan/receipt-only edits (`agent-lint-md.sh` and
  applicable release-state checks). Avoid repeated full `agent-verify` runs;
  run it when source, tests or executable scripts change as required by
  `AGENTS.md`, and at the checkpoint. A scoped pass is not a full green claim.
- Follow test-first RED/GREEN for confirmed code defects. Read the **whole**
  relevant test before asserting what it proves. Run focused adjacent cases
  before wider gates; do not edit tests just to force a pass.
- Use separate wheel/venv and npm artifact directories. Never `pip install
  -e` or `maturin develop` from this worktree: the shared `.venv` can be
  repointed. Verify real command exit status; a piped or background wrapper's
  trailing success can hide test failure. Do not substitute a compiled
  engine test for an installed SDK test.
- Keep exact SHA, command, corpus/model, environment, raw samples, semantic
  and reopened-state checks, invalid attempts, audit output and negative
  controls. Query p99 requires at least 1,000 valid samples; lifecycle p99
  is unsupported. Host-only swap drift is a warning; measured-child swaps
  invalidate. Preserve warning-free sensitivity, not only pooled medians.
- Disk was about 48 GiB free at hand-off. Reuse pinned build artifacts and
  avoid redundant instrumented builds. Do not delete raw archives. Their
  summaries are tracked, but some raw directories are local and untracked.
  Gitleaks policy/retention was explicitly deprioritized until the end;
  automatic approval review rejected a path-scoped policy change. Resolve
  retention then without delaying the four measurement areas.
- SQLite's locally collected [testing roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
  guides selected fault, differential and property methods. Avoid a broad
  test ecosystem that makes a rapidly changing 0.8.x system change-resistant.
  Use Semgrep, Kani, Criterion or flamegraphs only for a concrete uncovered
  hypothesis; existing Rust lint/type checks, branch coverage, real-database
  faults and independent receipts are the core methods.

The hand-off is complete only when the four-area checkpoint exists and can be
recomputed from retained evidence, with all omitted and unavailable cells
explicit. Do not claim release performance equivalence from these interim
diagnostics or mark Slice 135 complete before its Phase 2 work and later
closeout are also finished.
