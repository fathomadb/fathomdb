---
title: FathomDB 0.8.27 Slice 135 — system qualification plan
status: DRAFT_FOR_REVIEW
target_release: 0.8.27
planning_baseline: 316ac4769c0f4e23e9eff14e1190f66b2988f6b2
---

# Slice 135 — system qualification against 0.8.26

The [release plan](../../../plan-0.8.27.md) owns the release contract: the
completed 0.8.27 candidate must preserve or improve performance against
published 0.8.26 without losing supported function. The repository owner
authorized Slice 135 preparation in parallel with Slice 132 on 2026-10-06.
Protocol design, baseline qualification, harness work and review may proceed
now. The final three-SDK feature inventory, installed-artifact tests and
0.8.27 comparison must use the candidate after Slice 132 closes. No
pre-Slice-132 receipt can qualify the final release.

This is a draft for review, not a frozen measurement protocol or a performance
verdict. Freeze the executable protocol and decision rule before running the
final paired campaign. Preserve all failed and invalid attempts.

## Existing evidence and gaps

| Evidence already present | What it establishes | What remains for this slice |
| --- | --- | --- |
| Slice 115 frozen protocol, runner and twelve-path raw receipt | Repeatable current-engine characterization with semantic checks and selected sampled stacks | Re-run equivalent cells on 0.8.26 and the final 0.8.27 candidate; seven observations do not estimate p99 reliably |
| Slice 90 D27 runtime qualification | Accepted configuration and default-runtime gate on its exact candidate | Recheck final-candidate contract and do not silently change accepted defaults |
| 0.8.26 performance gauntlet and append-only perf history | Historical retrieval, scale, fidelity and some latency evidence | Matched final-candidate runs; historical cells with different hosts or boundaries are context only |
| Engine counters, SQL profile callback, search telemetry | Aggregate operation counts, opt-in statement elapsed time and search outcomes | Representative call mix, end-to-end stage timing and code-path frequency; SQL statements are not a call graph |
| Test-target/feature-matrix gate, behavioral and property tests | Every registered Rust test target has an execution route, with deep invariant suites | Measured line/branch coverage and coverage of frequently exercised production paths |
| Clippy, Rust type checks, Ruff, standard Pyright, TypeScript checks and repo guards | Static type, lint, boundary and build diagnostics | Logic, fault, panic and resource-path evidence; static passes alone cannot prove these |
| LOCOMO, SEARCH-01, AC-073/075, MuSiQue and corpus claim matrix | Separate gold-based relevance, fidelity or answer-quality evidence for named claims | An explicit correctness matrix for this candidate; supersession gold remains limited |

The code-grounded inventory above is the starting point. Before implementation,
verify every claimed harness behavior against its current source and a small
deliberately invalid fixture. A green test of the harness is insufficient if it
cannot reject an actual regression or malformed receipt.

## Questions and acceptance

| ID | Question | Required evidence |
| --- | --- | --- |
| R27-135A | Does the final candidate preserve the accepted product contract? | A live, installed Rust/Python/TypeScript capability and feature inventory after Slice 132; exact public input, result, error, default and supported-platform comparisons. Document intentional F27-01 and Slice 132 contract differences rather than treating byte equality as correctness. No accepted capability disappears to improve timing. |
| R27-135B | Is 0.8.27 at least as performant on representative end-to-end work? | Pre-registered, paired 0.8.26/0.8.27 release-artifact cells with identical corpus, operation mix, feature set, model, settings, cache condition, concurrency, host and timing boundary. Report raw samples, p50/p95, an adequately sampled tail statistic, throughput, CPU, peak/resident memory, I/O, queue/backlog and close/resource behavior where applicable. A material regression or inconclusive comparison blocks closeout. |
| R27-135C | Are returned data and persisted effects correct? | Independent human-owned or fixture-derived gold for exact reads, ordering, pagination, projection, graph/evidence, erasure, error precedence, and restart state. Report gold-based retrieval relevance and ANN fidelity separately. Run both versions on equivalent fresh databases; never call version agreement a gold answer. Fail on a correctness loss even when faster. |
| R27-135D | Does the system survive representative failures and contention? | Real-database fault injection at validation, transaction/commit, projection, queue/provider, read/snapshot, close/reopen and erasure/WAL boundaries; bounded concurrent read/write and restart cases. Assert exact post-state, no orphaned work/resources, typed failure, truthful incomplete status and idempotent recovery. Every named failure mode has a before/after state oracle. |
| R27-135E | Are the most important runtime paths understood and tested? | A declared workload-derived operation-frequency distribution and per-path cost estimate. Rank by observed total time or resource consumption, not call count alone. Profile the ranked paths using matched unprofiled controls. Overlay workload and test execution coverage, inspect missed branches, and show a targeted test fails under at least one plausible defect per selected high-risk gap. No claim of real-world Pareto coverage without representative usage evidence. |
| R27-135F | Can defects and performance claims be independently checked? | Candidate, 0.8.26 source/artifact, runner, protocol, corpus/model and environment hashes; raw observations; invalid-attempt register; independent recomputation; explicit gate-versus-exploratory labels. Static-analysis findings and scoped panic/error audit are dispositioned. Final source or executable changes receive the full repository gate. |

These IDs are release-local. Do not edit locked `dev/acceptance.md` or change an
existing gate threshold to make the comparison pass.

## Workload and measurement design

1. **Freeze product and workload identity.** Use the published 0.8.26 artifact
   or its verified release commit and the exact post-Slice-132 candidate. Run
   fresh databases because 0.8.26 is fresh-database-only. Hash the same seed,
   queries, expected outputs, model and resolved configuration. Give each
   version the same supported capability and materialized-result boundary.
2. **Cover the system.** Start with Slice 115's open, close, write,
   write-to-ready, text/vector/hybrid search, graph, evidence and erasure
   cells. Add one representative mixed sequence and existing scale/latency
   release selectors, keeping engine-only and installed-SDK intervals labeled
   separately. Include cold and warm behavior, at least two corpus sizes,
   single-caller and bounded concurrent operation. A substantial model-embed
   workload uses the required CUDA-capable host and records actual GPU use;
   the small CPU compatibility probe remains separately labeled.
3. **Pre-register before comparison.** Use baseline-only pilot repetitions to
   measure run-to-run noise, then freeze the sample count, equivalence margin,
   maximum acceptable tail/throughput/resource deltas, confidence method,
   multiple-cell decision rule and invalidators before revealing paired
   candidate results. An old 15% history warning is not the release margin.
   If noise prevents a meaningful decision, improve the fixture or environment
   and register a new protocol before another campaign.
4. **Alternate versions in blocks.** Hold host, storage, governor, software,
   background load, warm-up and process isolation constant. Alternate run
   order to reduce drift. Keep setup outside the operation timer except named
   open/close and end-to-end cells. Mutation samples start from identical
   independently validated states. Do not correct profiled latency by an
   inferred overhead factor; profile runs are separate attribution evidence.
5. **Report each cell honestly.** Provide paired deltas and uncertainty for
   comparable cells, plus exact acceptance-gate results. Seven samples may
   describe a maximum but may not be labeled a reliable p99. A cell without
   the pre-registered count, semantic pass or environment controls is invalid,
   not a zero or a pass.

Competitor comparisons are a separate protocol. The retained Mem0 timings do
not share a boundary, so no speed ratio follows from them. A new competitor
cell needs a pinned native comparator, matched input, warmed client boundary,
concurrency, result materialization and percentile collection. Keep answer
quality, retrieval relevance, engine latency and full-client latency distinct.

## Pareto paths, coverage and optimization

Define a *path* as a supported operation plus material conditions: corpus
size, cache state, projection/provider mode, requested feature and outcome.
Opt-in, privacy-preserving local observation may supply operation frequencies;
the controlled mixed workload supplies a reproducible minimum. Record the
source and representativeness of each distribution. Rank paths by
`frequency × measured cost`, then separately rank rare paths by failure
severity. Sampling stacks identify CPU owners; stage timers, SQLite profiles,
I/O and queue observations distinguish CPU from waiting. The current SQL
profile callback records elapsed milliseconds with placeholder step/cache
deltas; it cannot on its own identify hot Rust functions or complete cost.

Run test and workload coverage with the same feature/platform matrix. Report
line and branch coverage separately from the existing test-target gate. Map
missed high-use branches to owning invariant tests and add tests only for
meaningful behavioral gaps. Coverage percentage does not establish oracle
quality, so inject a plausible defect and require the named test to fail.
For hot-path changes, retain a RED correctness test, measure the whole
end-to-end workload before and after, and reject a local speedup that worsens
total latency, throughput, resources or correctness.

## Correctness, robustness and defect finding

Use two independent oracle classes. Deterministic database fixtures specify
exact rows, ordering, identities, projection/queue state and errors; close
and reopen for at-rest assertions. Gold corpora specify evidence or answers
where qualified, with class denominators and abstention. Approximate-vector
fidelity uses same-model exact-f32 truth and is not semantic relevance.
Unsupported gold claims, especially supersession, stay explicitly limited.

Use property tests for codecs, projection, recovery and round trips; bounded
state-machine sequences for writes, supersession, erasure and replay; and
fault injection around side effects and resource shutdown. Review `unwrap`,
`expect`, panic containment and error conversion on production paths crossing
Rust, PyO3 and NAPI boundaries, including cancellation and poisoned locks.
Run existing static checks first, then use targeted mutation or fuzz probes
where state or parser complexity justifies them. Record survivors, timeouts
and untested boundaries rather than translating a clean static pass into
"no logic defects."

## Execution and closeout

1. Finish the source-grounded harness and gold inventory. Write tests that
   reject missing/mismatched artifact hashes, insufficient samples, semantic
   failures, changed features, invalid environment and an intentionally
   degraded output. Keep measurement adapters separate from product behavior.
2. Freeze and review the protocol, oracles, sample-size pilot and decision
   rule. Prepare and run the verified 0.8.26 baseline while Slice 132 works.
3. After Slice 132 closes, rebase or merge its reviewed candidate into this
   branch, resolve any approved capability changes, freeze the final candidate,
   and run the paired campaign plus applicable accepted release gates.
4. Diagnose each loss as measurement artifact, real bottleneck or correctness
   defect. Repair with a RED test and separately reviewed change, then repeat
   the affected paired cells and all shared-system cells. No feature trade-off
   is inferred from a fast result.
5. Retain raw receipts, summary recomputation, qualification verdicts and
   limits. Run the checks matching changed files, including the full gate for
   any source, test or executable-script change. Only a complete, exact-SHA
   receipt can close this slice and feed Slice 140/150.

Current status: protocol and code work can start in this worktree. Final
0.8.27 qualification awaits the completed Slice 132 candidate.
