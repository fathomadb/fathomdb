---
title: FathomDB 0.8.27 Slice 135 — system qualification plan
status: APPROVED_FOR_EXECUTION
target_release: 0.8.27
planning_baseline: 316ac4769c0f4e23e9eff14e1190f66b2988f6b2
---

# Slice 135 — measure the whole system against 0.8.26

**Execution status:** the bounded [first inspectable results](results/2026-10-07-first-results/)
for the four Phase 1 areas were committed on 2026-10-07. The full Phase 1
checkpoint and dedicated correct-results phase remain ahead. The
[first-results protocol](first-results-protocol.json) was frozen before paired
candidate timing; the broader [Phase 1 protocol](phase1-protocol-draft.md)
remains a draft. The installed Python S01
[baseline-only noise pilot](results/2026-10-07-python-s01-noise-pilot/README.md)
has ten audited blocks but no candidate comparison. Its measured spread led to
the separately [frozen installed-Python S01 subset](s01-python-comparison-protocol.json)
before paired timing. The subsequent
[paired installed-Python result](results/2026-10-07-python-s01-paired/README.md)
has twenty audited blocks and descriptive p50/p95/p99 deltas. It is one SDK
workload subset; the full capability exercise and four-area checkpoint remain
open.

The [release plan](../../../plan-0.8.27.md) owns the release contract. The
repository owner approved this Slice 135 plan on 2026-10-06 and authorized
preparation in parallel with Slice 132. Protocol design, baseline
qualification, harness work and review may proceed now. The final three-SDK
feature inventory, installed-artifact tests and 0.8.27 comparison must use
the candidate after Slice 132 closes. No pre-Slice-132 receipt can qualify
the final release.

The initial [44-operation capability exercise register](phase1-capability-register.md)
tracks every accepted governed operation against installed-SDK Phase 1 evidence
or an explicit gap. Its proposed routes must be replaced by actual receipts
before the full Phase 1 checkpoint.

**Order of work:** first produce substantial, inspectable results for what
matters/Pareto path, system latency, system robustness, and logic and exception
handling. Record the Phase 1 checkpoint below. Only then start the dedicated
correct-results work, including gold construction, retrieval evaluation and
paired answer-quality runs. Existing benchmark datasets, including LongMemEval,
MuSiQue and LOCOMO where qualified, may supply Phase 1 workload shapes, query
mix, high-use paths, latency samples and fault cases. Their reference answers
may serve basic validity checks. Dedicated gold-based quality scoring remains
in Phase 2.

All five measurement families are **diagnostic in Slice 135**. No new numerical
threshold or slower timing alone blocks Slice 135. Existing accepted release
gates and product contracts still apply. Fix any confirmed product defect with
a failing test and verification before Slice 135 closes. This slice measures
and diagnoses; it ranks optimization opportunities for later work rather than
running a general optimization cycle.

This approved plan defines scope and execution order; it is not a frozen
measurement protocol or a performance verdict. Freeze the executable protocol
and reporting rule before running the final paired campaign. Preserve all
failed and invalid attempts.

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
| What matters / Pareto path | Operation frequency, aggregate elapsed time, CPU and waiting cost by operation and material condition; test coverage of the high-cost source paths; rare high-severity paths separately | Use a fixed mix drawn from query-correctness tests and qualified benchmark datasets as the 0.8.27 proxy. Add opt-in local operation metrics across Rust, Python and TypeScript so future consenting consumer traces can replace it. Combine stage timing, sampled stacks and source coverage; rank by total cost, not call count alone. Do not assume an 80/20 split exists. |
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

### Tool selection for Phase 1

Use a tool only when its output answers a named measurement question or checks
a concrete defect hypothesis. A new tool's first run is diagnostic, with a
small positive/negative fixture to show that its findings are meaningful.

| Tool | Slice 135 use | Limit |
| --- | --- | --- |
| Rust compiler, Clippy, Ruff, Pyright and TypeScript checks | Keep the existing lint/type gates; use [`rustc -C instrument-coverage`](https://doc.rust-lang.org/beta/rustc/instrument-coverage.html) plus matching LLVM tools to measure source regions/branches for test and workload runs | Coverage builds answer which code ran; run performance and acceptance checks on as-delivered builds too. The existing test-target gate is not source coverage. |
| Opt-in operation metrics and selective [`tracing::instrument`](https://docs.rs/tracing/latest/tracing/attr.instrument.html) | Prefer the planned bounded SDK metrics for call frequency and duration. Add spans only where a high-cost call still needs stage attribution; exclude data-bearing arguments and measure overhead | `#[instrument]` comes from `tracing`, not Tokio itself. Spans are not a CPU profiler or a complete operation-frequency distribution. No blanket annotation. |
| [`perf`](https://perf.wiki.kernel.org/index.php/Main_Page), [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph) and existing GDB stack sampling | Use `perf` on a permitted host for selected hot paths; a flamegraph is a view of sampled stacks. On this host `perf_event_paranoid=4`, so retain Slice 115's GDB fallback | Profiled runs are separate from unprofiled latency. Installing a flamegraph wrapper does not remove kernel profiling restrictions. |
| [Criterion.rs](https://github.com/criterion-rs/criterion.rs) | Optional microbenchmark only after Phase 1 identifies a small hot kernel and a specific optimization hypothesis | It cannot establish installed-SDK system latency or p99; do not build a broad microbenchmark suite now. |
| [Semgrep](https://semgrep.dev/docs/writing-rules/rule-ideas) | Trial one or two local rules for FathomDB-specific cross-language boundary mistakes, with seeded violations and false-positive review | Pattern/taint findings are leads, not proofs of logic correctness. Do not make a generic rule-pack gate for this slice. |
| [Kani](https://model-checking.github.io/kani/) | Optional bounded proof for one compact pure invariant, such as a codec or boundary calculation, if Phase 1 finds a high-risk gap | It does not prove the SQLite, FFI or concurrent product system. Avoid a whole-engine verification effort. |
| [cargo-deny](https://embarkstudios.github.io/cargo-deny/checks/index.html) | Record as a dependency-health follow-up only if the existing dependency/SBOM work leaves a concrete gap | Its licenses, bans, advisories and source checks do not find runtime logic defects or explain latency. |

Keep [cargo-mutants](https://github.com/sourcefrog/cargo-mutants) available for
one file when hand-chosen mutants reveal weak assertions. Use
[Miri](https://github.com/rust-lang/miri) for isolated unsafe Rust logic, not
native SQLite/FFI execution; use [Loom](https://github.com/tokio-rs/loom)
only if a small concurrent primitive can be separated from the real-database
system. Neither replaces Phase 1's actual fault/concurrency matrix.

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
   separately. Draw scenario shapes and query mixes from qualified LongMemEval,
   MuSiQue, LOCOMO or other benchmark corpora when they exercise the first
   four areas better than synthetic inputs; record the sampling and adaptation.
   Include cold and warm behavior, at least two corpus sizes, single-caller
   and bounded concurrent operation. A substantial model-embed workload uses
   the required CUDA-capable host and records actual GPU use; the small CPU
   compatibility probe remains separately labeled.
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
The controlled mix of query-correctness tests and qualified benchmark datasets
supplies a reproducible 0.8.27 proxy. Later local consumer observations may
replace it. Record the source and representativeness of each distribution.
Rank paths by
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
   concurrent operation and restart. Adapt SQLite's first-failure and
   persistent-failure patterns at selected FathomDB-owned side-effect
   boundaries. Check reopened product state and cleanup, with SQLite
   `PRAGMA integrity_check` as a supplementary structural check. Report
   failures and resources, not only pass counts.
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

### Execution bridge to the first inspectable results

The first-results milestone is a bounded, source-bound demonstration in **all
four** areas, not the complete Phase 1 checkpoint. Keep its raw evidence and
limitations, then expand the same qualified methods to every required cell
and fault case above. Do not start Phase 2 at the first-results milestone.

1. **Make the pilot executable.** Port a small representative Slice 115 query
   cell and lifecycle cell into a Slice 135 workload adapter. Qualify the
   exact 0.8.26 source build or installed artifacts, seed, model, configuration
   and timing boundary. Map matching post-Slice-132 Rust engine and
   Python/TypeScript SDK capabilities without claiming a paired 0.8.26 Rust
   SDK cell. Keep adapter tests that reject wrong identity, state, output,
   sample count and environment; the receipt validator alone produces no
   measurement. Record build provenance and artifact hashes.
2. **Pilot, then freeze.** Run baseline-only repeated blocks with raw samples,
   run order, semantic checks and environment snapshots. Estimate run-to-run
   noise and feasible sample counts. Freeze the workload manifest and weights,
   corpus/model digests, cells, warm-up, valid sample counts, uncertainty and
   multiple-cell reporting rules, invalidators and report format in the
   [Phase 1 protocol](phase1-protocol-draft.md) before examining paired
   candidate timing. Requalify if the host or artifact changes materially.
3. **Bind the final candidate and common workload.** Record the exact merged
   post-Slice-132 source and artifact hashes, supported feature/SDK mapping,
   resolved settings, operation mix, material conditions and seed/query
   digests. Use one workload manifest for the cost ranking, latency cells and
   coverage runs. Label engine-only, installed-SDK and candidate-only cells.
   Collect unprofiled timing first; run coverage and sampled stacks separately
   so their overhead cannot alter the latency comparison.
4. **Produce one inspectable result per area.** Each result must name its exact
   candidate, workload or fault fixture, command/runner, raw output,
   independent summary or state oracle, invalid attempts and limits:

   | Area | Minimum first result | Expansion before the Phase 1 checkpoint |
   | --- | --- | --- |
   | Pareto path | A fixed mixed workload's operation counts, total elapsed-cost ranking, one sampled hot-path attribution and test-versus-workload line/branch coverage for that path; show one plausible injected defect is caught. | All declared workload conditions, 80%-cost set, CPU/queue ranking, missed branches and rare severe paths. |
   | System latency | At least one valid paired 0.8.26/final-0.8.27 query cell and one whole mixed-sequence cell, with raw unprofiled samples, resource observations, semantic checks and independently recomputed supported percentiles/deltas. | E01–E12 and applicable S01–S03 cells, cold/warm and scale/concurrency conditions, tail statistics only where sampled sufficiently, plus C01 or its documented qualification failure. |
   | System robustness | One bounded concurrency case, one crash/restart boundary and one injected failure, each against a real database with before/after and reopened-state assertions, timeout and resource record. | Every declared fault/schedule row, including one-shot and persistent failures and interrupted erasure, with recovery dispositions. |
   | Logic and exception handling | Recorded lint/typecheck output, a measured hot-path branch-coverage slice, a named error/panic/cleanup boundary audit and one bounded property, state-machine or mutation probe with its outcome. | Coverage for the declared feature/platform matrix, audit of ranked and cross-language boundary paths, and dispositions for all findings and survivors. |

   The robustness fixtures and static/error audit can begin during the pilot;
   paired latency and the final cost ranking require the frozen workload and
   candidate. Preserve failing or invalid attempts rather than replacing them
   with a clean rerun.
5. **Publish and extend.** Put the four first-results sections in a dated
   measurement note with links to immutable raw receipts, profiles, coverage
   exports, fault logs and check outputs. Recompute summaries from raw files
   and mark each cell as valid, invalid, omitted or candidate-only. Record
   confirmed defects for test-first repair; rerun affected cells after a fix.
   Continue through the full four-area checkpoint below.

### Functional exercise gate for the Phase 1 checkpoint

The checkpoint must contain executed product behavior, not only benchmark
timings or a list of available tests. Run the frozen S01 text, vector and
hybrid queries where supported, with cold/warm and declared corpus-size
conditions, through each available **installed** Rust, Python and TypeScript
boundary. Run S02's whole open/write/project-to-ready/retrieve/graph and
evidence/erase/close/reopen sequence with state assertions and bounded
contention through each available installed boundary. Use the same declared
operation mix for the Pareto and coverage overlay. Pair shared Python and
TypeScript behavior with 0.8.26; label the new Rust SDK candidate-only and
pair comparable engine cells at the engine boundary.

For every accepted operation in the
[capability exercise register](phase1-capability-register.md), record the
supported SDKs and an exact-candidate, real-database positive call with an
asserted return or persisted effect, or an explicitly labeled standalone
model call where no database boundary exists. Add a negative/error condition and
reopen check where the operation has such a contract. Mark standalone model
operations and genuinely unavailable provider/platform cases separately,
with the reason and an owned follow-up. Preserve the count of supported,
executed, failed and unexecuted operations per SDK; a test name, compile
pass, engine-only run or installed smoke does not count as complete exercise
of that SDK. The checkpoint is incomplete if S01/S02's available installed
boundaries have not run, or if an accepted supported operation is left as an
unexplained execution gap. Report remaining explained gaps as residual risk,
not coverage. Store commands, package identities, raw outputs and independent
assertions so the next release can rerun the same functional gate.

**Phase 1 checkpoint:** publish an exact-candidate measurement note with four
valid result sections, raw receipt links, independently recomputed summaries,
invalid/omitted cells, coverage gaps, confirmed defects and ranked follow-ups.
The [node FTS row-error repair](results/2026-10-07-node-fts-repair/README.md)
changes candidate engine bytes after the paired S01 and S02 receipts. Rebuild
and rerun the installed candidate and affected engine cells at the repaired
SHA before treating those comparisons as checkpoint evidence.
Actual final-candidate observations in all four areas are required; a harness,
protocol or baseline alone does not meet the checkpoint. Fix any defect or
harness flaw that makes the results untrustworthy and rerun affected cells.
Record the checkpoint **before** creating new gold fixtures or running the
broader retrieval/answer-quality campaigns. Reusing an existing benchmark
dataset in Phase 1 does not require withholding its reference answers from
basic validity checks. Other confirmed product defects remain
tracked for test-first repair before Slice 135 closeout.

The [quality testing roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) records which
methods from the locally collected SQLite testing page suit this fast-changing
release and which are recommended for later release lines. It does not create
a global coverage quota or a new testing gate.

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
2. Source-ground and review the [Phase 1 protocol draft](phase1-protocol-draft.md).
   Freeze its executable cells after the baseline-only sample-size pilot and
   reporting rule are qualified. Prepare and run the verified 0.8.26 baseline
   while Slice 132 works.
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

Current status: Slice 132 is merged, the [0.8.26 baseline source is
identified](baseline-qualification.md), and the provisional
[Phase 1 receipt validator](../../../../../scripts/slice135_receipt.py)
rejects mismatched identities, environment and semantic state with independent
summary recomputation. The bounded engine-only
[noise pilot](results/2026-10-07-mixed-noise-pilot/) and
[four first results](results/2026-10-07-first-results/) are retained. The full
protocol freeze, installed-SDK capability exercise, expanded four-area runs
and Phase 1 checkpoint remain. The first-results edge FTS error was repaired
with a [permanent regression test and verification record](results/2026-10-07-edge-fts-repair/);
the full gate still has two recorded unrelated failing suites. Phase 2 awaits
the checkpoint. A candidate [installed Python wheel functional smoke](results/2026-10-07-python-wheel-qualification/README.md)
now covers a narrow graph/evidence/dependency flow. It does not satisfy the
functional exercise gate or a timed S01/S02 cell. The exact
[0.8.26 installed wheel](results/2026-10-07-python-wheel-baseline/README.md)
passed the same smoke, and a [baseline-only installed Python S01 functional
probe](results/2026-10-07-python-s01-baseline-probe/README.md) exercised 32-
and 256-row text, vector-bearing and hybrid shapes. Its timing is unqualified;
see the [verification limits](results/2026-10-07-python-s01-verification/README.md).
The [paired installed Python S01 result](results/2026-10-07-python-s01-paired/README.md)
now supplies a narrow 0.8.26 comparison with independently audited raw
calls. A [TypeScript native-suite and installed-package smoke](results/2026-10-07-ts-native-qualification/README.md)
also ran against the candidate. Neither closes the full operation map,
installed Rust/TypeScript S01/S02, or the four-area checkpoint.
The [installed-TypeScript S01 baseline noise pilot](results/2026-10-07-ts-s01-noise-pilot/README.md)
has ten independently audited blocks and matching short candidate/baseline
feasibility probes. The [TypeScript paired subset protocol](s01-ts-comparison-protocol.json)
is frozen, and a [paired TypeScript S01 diagnostic](results/2026-10-07-ts-s01-paired/README.md)
now retains 20 independently audited blocks. The full installed-SDK
functional exercise and four-area checkpoint remain open.
An interim [projection commit recovery result](results/2026-10-07-projection-recovery/README.md)
adds real-database failure, panic, redispatch and stop/reopen observations;
it does not close the full robustness matrix.
An [installed Python S02 functional feasibility pair](results/2026-10-07-python-s02-feasibility/README.md)
now exercises the whole write/project/retrieve/graph-evidence/erase/reopen
sequence against both wheels with direct canonical-row checks. Its single
attempts do not constitute a qualified S02 latency comparison.
An [installed TypeScript S02 functional feasibility pair](results/2026-10-07-ts-s02-feasibility/README.md)
now exercises the same sequence against both packages, with independently
audited artifact bytes and canonical-row checks. It also has one attempt per
version and no qualified S02 latency or contention result. A second
[node FTS row-error defect](results/2026-10-07-node-fts-repair/README.md) was
repaired on the candidate. All earlier candidate timing receipts need a
refresh against the repaired engine bytes before the Phase 1 checkpoint.
