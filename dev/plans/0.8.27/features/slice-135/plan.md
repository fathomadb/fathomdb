---
title: FathomDB 0.8.27 Slice 135 — system qualification plan
status: DRAFT_FOR_REVIEW
target_release: 0.8.27
planning_baseline: 316ac4769c0f4e23e9eff14e1190f66b2988f6b2
---

# Slice 135 — measure the whole system against 0.8.26

The [release plan](../../../plan-0.8.27.md) owns the release contract. The
repository owner authorized Slice 135 preparation in parallel with Slice 132
on 2026-10-06. Protocol design, baseline qualification, harness work and
review may proceed now. The final three-SDK feature inventory,
installed-artifact tests and 0.8.27 comparison must use the candidate after
Slice 132 closes. No pre-Slice-132 receipt can qualify the final release.

**Order of work:** first produce substantial, inspectable results for what
matters/Pareto path, system latency, system robustness, and logic and exception
handling. Record the Phase 1 checkpoint below. Only then start the dedicated
correct-results work, including gold construction, retrieval evaluation and
paired answer-quality runs. Basic semantic assertions remain in Phase 1 so a
timing or fault receipt cannot pass with a wrong result; they are validity
checks, not the broader correct-results campaign.

All five measurement families are **diagnostic in Slice 135**. No new numerical
threshold or slower timing alone blocks Slice 135. Existing accepted release
gates and product contracts still apply. Fix any confirmed product defect with
a failing test and verification before Slice 135 closes. This slice measures
and diagnoses; it ranks optimization opportunities for later work rather than
running a general optimization cycle.

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
| R27-135B | How does 0.8.27 compare on representative end-to-end work? | Pre-registered, paired 0.8.26/0.8.27 release-artifact cells with identical corpus, operation mix, feature set, model, settings, cache condition, concurrency, host and timing boundary. Report raw samples, p50/p95, an adequately sampled tail statistic, throughput, CPU, peak/resident memory, I/O, queue/backlog and close/resource behavior where applicable. Quantify losses and uncertainty; latency is diagnostic. |
| R27-135C | Are returned data and persisted effects correct? | After the Phase 1 checkpoint, use independent human-owned or fixture-derived gold for exact reads, ordering, pagination, projection, graph/evidence, erasure, error precedence, and restart state. Report gold-based retrieval relevance and ANN fidelity separately. Run both versions on equivalent fresh databases; never call version agreement a gold answer. Treat a confirmed correctness loss as a product defect regardless of speed. |
| R27-135D | Does the system survive representative failures and contention? | Real-database fault injection at validation, transaction/commit, projection, queue/provider, read/snapshot, close/reopen and erasure/WAL boundaries; bounded concurrent read/write and restart cases. Assert exact post-state, no orphaned work/resources, typed failure, truthful incomplete status and idempotent recovery. Every named failure mode has a before/after state oracle. |
| R27-135E | Are the most important runtime paths understood and tested? | A declared workload-derived operation-frequency distribution and per-path cost estimate. Rank by observed total time or resource consumption, not call count alone. Profile the ranked paths using matched unprofiled controls. Overlay workload and test execution coverage, inspect missed branches, and show a targeted test fails under at least one plausible defect per selected high-risk gap. The query-correctness workload mix is the 0.8.27 usage proxy; do not call it real consumer traffic. |
| R27-135F | Can defects and performance claims be independently checked? | Candidate, 0.8.26 source/artifact, runner, protocol, corpus/model and environment hashes; raw observations; invalid-attempt register; independent recomputation; explicit gate-versus-exploratory labels. Static-analysis findings and scoped panic/error audit are dispositioned. Final source or executable changes receive the full repository gate. |

These IDs are release-local. Do not edit locked `dev/acceptance.md` or change an
existing gate threshold to make the comparison pass.

## Five aspects: what and how

| Aspect | What to measure | How to measure |
| --- | --- | --- |
| What matters / Pareto path | Operation frequency, aggregate elapsed time, CPU and waiting cost by operation and material condition; test coverage of the high-cost source paths; rare high-severity paths separately | Use a fixed mix drawn from query-correctness test workloads as the 0.8.27 proxy. Add opt-in local operation metrics across Rust, Python and TypeScript so future consenting consumer traces can replace it. Combine stage timing, sampled stacks and source coverage; rank by total cost, not call count alone. Do not assume an 80/20 split exists. |
| System latency | Installed-SDK call-to-materialized-result p50/p95/p99, throughput, CPU, memory, I/O and queue delay; write-to-searchable, open/close, cold/warm and mixed-workload behavior | Pair published 0.8.26 and final 0.8.27 artifacts on the same host, data, model, settings, concurrency and cache condition. Alternate version blocks and profile separately. Use at least 1,000 valid query observations per p99 cell and 100 lifecycle observations per p50/p95 cell; omit unsupported tails. Run a separate matched native Mem0 cell. |
| System robustness | Correct final state and bounded completion with concurrent readers/writers, crash/restart near commit, provider failure, SQLite busy/full disk, close/cancellation under load and interrupted erasure; resource leaks and recoverability | Use real databases, controlled fault points, timeouts and fresh/reopened state oracles. Vary fault position and concurrency, then repeat selected cases under randomized schedules. Record exceptions, partial work, queue state, disk artifacts and recovery behavior. A component speedup that breaks a system invariant is a defect. |
| Logic and exception handling | Reachable/unreachable branches, always-running or always-failing paths, wrong condition order, off-by-one, overflow, cycles/nontermination, swallowed errors, panic/FFI escape, incomplete cleanup and error precedence | Run Clippy, Ruff, Pyright, TypeScript checks and repo guards. Collect Rust compiler-instrumented line/branch coverage and available binding coverage. Audit error/panic sites on measured and boundary paths. Use property/state-machine tests, targeted mutation probes and fault injection with explicit timeouts; disposition findings and survivors. |
| Correct results | Contract correctness, vector fidelity, retrieval effectiveness, evidence sufficiency, memory usefulness and generated answer quality | **Phase 2 only.** Use independent deterministic expected results, same-model exact-f32 neighbors, judged relevance/evidence sets and paired answer evaluation. Keep denominators and unsupported claims explicit. |

Expose opt-in operation metrics in all three SDKs with equivalent semantics and
language-native names. Specify the public contract in an ADR or interface
documentation in the same change. The proposed minimum is per-engine fixed
operation IDs, success/error counts, total duration and a bounded logarithmic
duration histogram, plus a local snapshot accessor. Default is disabled; no
query text, record content, network emission or unbounded labels. Define
enable/reset, disable, close and concurrent-snapshot behavior before coding.
Keep the existing locked `CounterSnapshot` and SQL profile shape intact and
measure the instrumentation's own overhead with enabled/disabled controls.

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
   measure run-to-run noise, then freeze sample counts, confidence method,
   multiple-cell reporting rule and invalidators before revealing paired
   candidate results. An old 15% history warning is not a Slice 135 gate.
   If noise prevents a meaningful comparison, improve the fixture or
   environment and register a new protocol before another campaign.
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
not share a boundary, so no speed ratio follows from them. Run a pinned native
Mem0 cell with matched frozen LOCOMO input, warmed external client-to-materialized
top-10 boundary, concurrency and percentile collection. Verify and reuse the
previous native index if valid; otherwise paid re-ingest has a **$20 ceiling**.
Keep answer quality, retrieval relevance, engine latency and full-client
latency distinct. Before paid re-ingest, verify checkpoint/resume, retry with
backoff, execution-window fit and an invalid-partial-run guard.

## Pareto paths, coverage and optimization

Define a *path* as a supported operation plus material conditions: corpus
size, cache state, projection/provider mode, requested feature and outcome.
The controlled mix of query-correctness test workloads supplies a reproducible
0.8.27 proxy. Later local consumer observations may replace it. Record the
source and representativeness of each distribution. Rank paths by
`frequency × measured cost`, then separately rank rare paths by failure
severity. Sampling stacks identify CPU owners; stage timers, SQLite profiles,
I/O and queue observations distinguish CPU from waiting. The current SQL
profile callback records elapsed milliseconds with placeholder step/cache
deltas; it cannot on its own identify hot Rust functions or complete cost.

Run test and workload coverage with the same feature/platform matrix using
compiler instrumentation and its coverage mapping tools. Report line and
branch coverage separately from the existing test-target gate. Map
missed high-use branches to owning invariant tests and add tests only for
meaningful behavioral gaps. Coverage percentage does not establish oracle
quality, so inject a plausible defect and require the named test to fail.
Report the paths contributing 80% of measured cost, whether or not they occupy
20% of paths. Rank optimization hypotheses by likely full-system effect;
implementation belongs to a reviewed follow-up.

## Phase 1 — four areas must produce results first

The four areas may progress concurrently where they do not share mutable data.
Each must produce inspectable results on the final post-Slice-132 candidate
before Phase 2 begins. Baseline/harness work can proceed while Slice 132 is
open; pre-Slice-132 receipts cannot qualify the final candidate.

1. **What matters/Pareto path:** publish the fixed workload mix, per-operation
   count and aggregate-cost ranking, sampled CPU/stage attribution,
   workload-to-test line/branch coverage overlay, scenario gaps and rare
   high-severity paths. Show at least one targeted test rejects a plausible
   injected defect in a high-use gap.
2. **System latency:** publish valid paired 0.8.26/0.8.27 receipts for
   representative engine and installed-SDK cells, including the mixed system
   sequence, resources and supported percentiles. Include the matched Mem0
   cell or a documented qualification failure with raw evidence. Explain
   measured bottlenecks at the whole-call boundary and rank follow-ups.
3. **System robustness:** publish a real-database fault/concurrency matrix
   with each case's setup, fault point, expected state, observed state and
   recovery result. Cover the cases in the five-aspect table, including
   concurrent operation and restart. Report failures and resources, not only
   pass counts.
4. **Logic and exception handling:** publish static-check outputs, measured
   line/branch coverage, an error/panic audit of high-cost and system-boundary
   paths, and bounded property/state-machine or mutation probes. Classify
   findings as confirmed defect, disproved concern or open risk with evidence.

Use property tests for codecs, projection, recovery and round trips; bounded
state-machine sequences for writes, supersession, erasure and replay; and
fault injection around side effects and resource shutdown. Review `unwrap`,
`expect`, panic containment and error conversion on production paths crossing
Rust, PyO3 and NAPI boundaries, including cancellation and poisoned locks.
Run existing static checks first, then use targeted mutation or fuzz probes
where state or parser complexity justifies them. Record survivors, timeouts
and untested boundaries rather than translating a clean static pass into
"no logic defects."

**Phase 1 checkpoint:** publish an exact-candidate measurement note with four
valid result sections, raw receipt links, independently recomputed summaries,
invalid/omitted cells, coverage gaps, confirmed defects and ranked follow-ups.
Actual final-candidate observations in all four areas are required; a harness,
protocol or baseline alone does not meet the checkpoint. Fix any defect or
harness flaw that makes the results untrustworthy and rerun affected cells.
Record the checkpoint **before** creating gold fixtures or running the broader
retrieval/answer-quality campaigns. Other confirmed product defects remain
tracked for test-first repair before Slice 135 closeout.

## Phase 2 — correct results

Begin only after the Phase 1 checkpoint. Use two independent oracle classes:
deterministic database fixtures with specified results, state and errors, and
qualified external gold judgments. Version agreement is not a gold answer.
Report class denominators, abstentions and claim limitations; existing
supersession human gold is limited. Vector fidelity and semantic relevance
remain separate measures.

| Dimension | What to measure | How to measure |
| --- | --- | --- |
| Contract correctness | Exact eligible rows, order, filters, lifecycle, validity, provenance, bounded graph results, erasure and persisted state | Independently specified fixtures, a small reference implementation where useful, real database/reopen checks and paired version runs |
| Vector fidelity | Top-K overlap with exact full-precision nearest neighbors for the same model and eligible corpus | Exact-f32 truth and [ANN-Benchmarks methodology](https://ann-benchmarks.com/); preserve the existing fidelity gate separately |
| Retrieval effectiveness | Recall@K, nDCG@10 and optionally MRR@10 for relevant facts | Qualified relevance judgments and per-class denominators; [ranked IR evaluation](https://nlp.stanford.edu/IR-book/html/htmledition/evaluation-of-ranked-retrieval-results-1.html) and [BEIR](https://arxiv.org/abs/2104.08663) guide method, not an automatic dataset claim |
| Evidence sufficiency | Whether context contains a complete, valid supporting evidence set | Qualified evidence-set judgments drawing on [HotpotQA](https://aclanthology.org/D18-1259/), [MuSiQue](https://aclanthology.org/2022.tacl-1.31/) or [KILT](https://aclanthology.org/2021.naacl-main.200/) only when their mapping is valid |
| Memory usefulness | Extraction, multi-session, temporal, knowledge-update and abstention accuracy | Qualified [LongMemEval](https://arxiv.org/abs/2410.10813) and existing LOCOMO cells, by query class |
| Generated answer quality | Reference-answer correctness, faithfulness to evidence and citation quality, separately | Fresh paired 0.8.26/0.8.27 answerer/judge run; [ALCE](https://arxiv.org/abs/2305.14627) and [ARES](https://aclanthology.org/2024.naacl-long.20/) inform scoring. Diagnostic only, with a separate **$20 ceiling**. |

FathomDB's data-plane boundary means generated answers are an external
consumer outcome, not a database return value. Before any priced run,
validate a small sample, checkpoint completed cells, verify resume skips
completed calls, back off on 429/5xx, limit concurrency, fit the execution
window and reject incomplete results. Report spend and completeness.

## Execution and closeout

1. Finish the source-grounded harness and Phase 1 workload inventory. Write tests that
   reject missing/mismatched artifact hashes, insufficient samples, semantic
   failures, changed features, invalid environment and an intentionally
   degraded output. Keep measurement adapters separate from product behavior.
2. Freeze and review the Phase 1 protocol, baseline-only sample-size pilot and
   reporting rule. Prepare and run the verified 0.8.26 baseline while Slice 132 works.
3. After Slice 132 closes, rebase or merge its reviewed candidate into this
   branch, resolve any approved capability changes, freeze the final candidate,
   and run the four Phase 1 campaigns plus applicable accepted release gates.
4. Publish the Phase 1 checkpoint. Then freeze the qualified gold/answer
   protocol and run Phase 2's paired correct-results campaign.
5. Diagnose each loss as measurement artifact, bottleneck or product defect.
   Repair confirmed defects with a RED test and separately reviewed change,
   then repeat affected paired cells and shared-system cells.
6. Retain raw receipts, independent summary recomputation, verdicts and
   limits. Run the checks matching changed files, including the full gate for
   source, test or executable-script changes. Only complete, exact-SHA Phase 1
   and Phase 2 receipts can close the slice and feed Slice 140/150.

Current status: Phase 1 protocol and code work can start in this worktree.
Phase 1 final-candidate results await completed Slice 132. Phase 2 work awaits
the recorded Phase 1 checkpoint.
