---
title: FathomDB 0.8.27 Slice 135 — system qualification plan
status: APPROVED_FOR_EXECUTION
target_release: 0.8.27
planning_baseline: 316ac4769c0f4e23e9eff14e1190f66b2988f6b2
---

# Slice 135 — measure the whole system against 0.8.26

**Execution status:** the bounded [first inspectable results](results/2026-10-07-first-results/)
exist for all four Phase 1 areas. The [E01–E12 paired engine subset](e12-comparison-protocol.json)
and the Python and TypeScript S01 subsets have frozen protocols and audited
paired runs. Integrated-candidate E01–E12, Python S01/S02 and TypeScript S01/S02 refreshes now
have separate frozen subsets and audited raw receipts. The
[44-operation capability register](phase1-capability-register.md)
records executed Rust, Python and TypeScript cases and explicit gaps, including
an [integrated-candidate installed Python functional
receipt](results/2026-10-07-python-integrated-candidate/README.md) and
[TypeScript functional receipt](results/2026-10-07-ts-integrated-candidate/README.md).
These receipts do not yet constitute the full Phase 1 checkpoint. The paired
TypeScript S02 subset has host paging warnings; the Rust S02 candidate-only
result is linked below. The [installed Python S02 bounded-contention
pair](results/2026-10-07-python-s02-contention-paired/README.md) now has
independently audited raw receipts. An [installed TypeScript contention
feasibility pair](results/2026-10-07-ts-s02-contention-feasibility/README.md)
passed independent functional checks. Its [five-block baseline-only noise
pilot](results/2026-10-08-ts-s02-contention-baseline-pilot/README.md) now has
independently audited receipts. The
[TypeScript contention protocol](s02-ts-contention-comparison-protocol.json)
froze before candidate timing. Its [paired campaign](results/2026-10-08-ts-s02-contention-paired/README.md)
passed independent audit over 100 fresh-process sequences per version;
the [Rust SDK candidate-only contention campaign](results/2026-10-08-rust-s02-contention/README.md)
also passed ten fresh-process runs with independent reopened-state checks.
The [installed Python S03 functional feasibility](results/2026-10-08-python-s03-feasibility/README.md)
now exercises fixture-grounded filter, temporal, graph, evidence and 32/256-row
memory-load shapes on both exact installed versions, with four independently
audited real databases. The [S03 baseline-only noise pilot](results/2026-10-08-python-s03-baseline-noise/README.md)
then passed ten audited blocks and 7,000 fixture-checked observations with
no environment invalidators. The [S03 Python paired subset](results/2026-10-08-python-s03-paired/README.md)
then froze before candidate timing and passed 20 independently audited blocks
and 14,000 observations, with no environment warnings. Its 256-row evidence
p50 increase is an attribution lead. Other S03 bindings, the broader workload
mix and the robustness and coverage matrices remain. C01 has a documented
[qualification failure](results/2026-10-07-c01-qualification/README.md).
The [broader Phase 1 protocol](phase1-protocol-draft.md) is still a draft.
The full verification gate is still open. A clean-checkout attempt through
`90d1fa1cb` passed lint, typecheck and strict security, then the test stage
found an invalid fake-wheel fixture (repaired at `1f8d03cc7`) and a separate
real-repository `steward-orient` output-cap failure (4,635 bytes against its
4,096-byte cap). The already-failed test stage was stopped before completing
the workspace tests. The local worktree's untracked raw archives also contain
temporary Cargo manifests that trip the pinned-override scanner; the clean
checkout made them available to the Markdown link check without scanning
their build files. None of these attempts is a full-gate green claim.
An additional [vector row-error defect](results/2026-10-07-vector-row-repair/README.md)
has a real-database RED/GREEN repair committed at `3f29d649d`. Its source
change supersedes prior candidate timing for the final checkpoint. The
[vector-repaired E01–E12 paired refresh](results/2026-10-07-e12-vector-repaired-paired/README.md)
is independently audited. The repaired-source installed
[Python S01](results/2026-10-07-python-s01-vector-repaired-paired/README.md),
[Python S02](results/2026-10-07-python-s02-vector-repaired-paired/README.md),
[TypeScript S01](results/2026-10-07-ts-s01-vector-repaired-paired/README.md)
and [TypeScript S02](results/2026-10-07-ts-s02-vector-repaired-paired/README.md)
subsets are independently audited. Rust S02 now has a candidate-only
[timing result](results/2026-10-07-rust-s02-candidate-timing/README.md);
bounded contention remains.
Dedicated correct-results work remains Phase 2.

The [release plan](../../../plan-0.8.27.md) owns the release contract. The
repository owner approved this Slice 135 plan on 2026-10-06 and authorized
preparation in parallel with Slice 132. Protocol design, baseline
qualification, harness work and review may proceed now. The final three-SDK
feature inventory, installed-artifact tests and 0.8.27 comparison must use
the candidate after Slice 132 closes. No pre-Slice-132 receipt can qualify
the final release.

The candidate must include the off-ladder embedder-close and published
`0.8.26+tegra` install-route fixes, measured at or after `ef4bb42da` as
recorded in the release plan and board. Count the engine-owned embedder memory
released at `close()` as an intended lifecycle change relative to 0.8.26.
Add repeated open/close cycles and post-close memory observations to the
matched workload; keep caller-owned and module-level model lifetimes distinct.

The [44-operation capability exercise register](phase1-capability-register.md)
tracks every accepted governed operation against executed SDK evidence or an
explicit gap. Its remaining supported positive-path gaps and contract
conditions need execution or a checkpoint risk disposition.

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
Include an [off-ladder landing audit](off-ladder-qualification-handoff.md):
confirm the exact candidate includes the corrected embedder-close change
`96796fe04` and the reported 0.8.26+Tegra Pages pin/install/docs change
`c23e2d23f`; exercise their affected close/reopen and installed Tegra
boundaries where available; and give Slice 150 an explicit integrated
qualification owner. The release plan and board now record the landings;
the release-state verification obligation remains for its shared-state
writer. Do not infer that a local commit hash
or a prior slice's verification covers the final candidate. The
[focused embedder-close receipt](results/2026-10-07-off-ladder-embedder-close/README.md)
checks the known landing's lock and provider ownership regressions; the
remaining concurrency, installed and final-candidate checks are still owed.
The [node FTS row-error repair](results/2026-10-07-node-fts-repair/README.md)
and [graph traversal row-error repair](results/2026-10-07-graph-arm-row-repair/README.md)
change candidate engine bytes after the paired S01 and S02 receipts. Rebuild
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

Historical execution chronology (early 2026-10-07): Slice 132 is merged, the [0.8.26 baseline source is
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
A [candidate-only external Rust SDK S02 consumer](results/2026-10-07-rust-sdk-s02-feasibility/README.md)
now has a real-database source-bound functional receipt, including vector,
evidence, graph, erasure and reopen assertions. It does not replace a
published-crate installation or close the full operation map.
A third confirmed swallowed-row-error defect in the graph arm was repaired
test-first at `3ce1a6352` with a [RED/GREEN receipt](results/2026-10-07-graph-arm-row-repair/README.md).
The focused graph regression and adjacent graph suites passed (10 tests). The
current source has not passed a new full gate; earlier candidate timing and
installed-artifact results are historical for their exact SHAs and need a
current-source refresh before the checkpoint.
An [E01–E12 paired engine feasibility run](results/2026-10-07-e01-e12-feasibility/README.md)
now confirms that all twelve inherited engine paths execute on the exact
0.8.26 baseline and current candidate. Its seven samples per cell and weaker
query assertions do not qualify the broader protocol or a latency verdict.
A fourth confirmed error-suppression defect was repaired at `55f8120f5`:
[missing edge FTS index evidence](results/2026-10-07-edge-fts-missing-index-repair/README.md)
shows that a current-schema search previously returned an empty success after
its edge-index table disappeared. This changes candidate engine bytes again;
the E01–E12 feasibility receipt remains evidence for its own source SHA only.
An [expanded four-case real-database robustness result](results/2026-10-07-robustness-expanded/README.md)
on candidate `7595b31eb` adds a bounded SQLite-full write/refusal/reopen
oracle to the earlier concurrency, process-kill and injected-refusal probes.
The focused suite and its deliberately failing missing-row control behaved as
expected; the raw logs, resource observation and independently recomputed
summary are retained. The remaining fault and schedule rows are still open.
A [current-source projection matrix extension](results/2026-10-07-projection-matrix-current/README.md)
re-ran ten focused real-database capacity, provider-delay, worker-commit,
panic-containment and close/reopen cases on the repaired candidate. All ten
passed with retained test-process resources and exact binary hashes. Only the
named reopen cases prove recovered product state; persistent provider failure,
sustained cancellation and interrupted erasure still need dedicated probes.
The [E01–E12 source-bound adapter and baseline pilot](results/2026-10-07-e12-adapter-pilot/README.md)
now bind exact source, binary, runner, corpus, model, environment and
per-attempt output/state checks. Five baseline-only text and close blocks and
single full-path functional runs on both versions passed independent audits.
The [E01–E12 paired subset](e12-comparison-protocol.json) is frozen before
current-candidate paired timing. The broader Phase 1 protocol and installed
SDK exercise remain open.

### Current Phase 1 checkpoint status (2026-10-07)

- **Candidate and baseline:** the post-Slice-132 candidate and exact 0.8.26
  baseline source are identified. Four search/graph error-suppression defects
  and a Python frozen-error-field defect were repaired test-first. Earlier
  installed S01 measurements remain historical for their exact source SHAs.
  The [integrated Python wheel](results/2026-10-07-python-integrated-candidate/README.md)
  includes both off-ladder landings. A cached-bytecode packaging defect was
  reproduced, rejected by the installed checker and repaired test-first.
- **Pareto and coverage:** the first-results cost proxy ranked four of eleven
  measured operations at 84.12% of elapsed cost. A selected workload/test
  line-and-branch overlay and a defect-catching negative control exist.
  The [exact-candidate E01–E12 overlay](results/2026-10-08-e12-current-coverage/README.md)
  now compares all twelve workload cells with 14 passing targeted test
  binaries and two isolated passing rank cases: the tests hit all 888
  workload-hit engine branch IDs and 8,119 of 8,132 workload-hit lines.
  This is a selected-route overlap, not production-traffic coverage.
  Expand to the declared operation mix, CPU/queue cost, search branches,
  missed-branch dispositions and rare severe paths.
- **Latency:** the E01–E12 paired engine campaign has audited raw blocks and
  descriptive deltas; the paired Python and TypeScript S01 subsets also have
  audited receipts. The [integrated E01–E12 source smoke](results/2026-10-07-e12-integrated-smoke/README.md)
  passed all twelve paths and an [integrated paired subset](e12-integrated-comparison-protocol.json)
  refroze the changed candidate Rust tree before timing. The
  [integrated E01–E12 paired diagnostic](results/2026-10-07-e12-paired-integrated/README.md)
  passed 20 blocks and independent audit across all twelve cells. Vector,
  hybrid and populated-open p50 increased in the five-pair diagnostic;
  investigate and disposition these leads before a release performance claim.
  The subsequent [vector-repaired E01–E12 refresh](results/2026-10-07-e12-vector-repaired-paired/README.md)
  completed all 20 blocks at product source `3f29d649d`; the same three
  p50 leads persisted (+8.79%, +8.04%, +11.16%). Four blocks have host-only
  paging warnings, with zero measured-child swaps. Attribute these leads
  across engine and installed boundaries before a release claim.
  The repaired-source [installed Python S01 refresh](results/2026-10-07-python-s01-vector-repaired-paired/README.md)
  passed 20 blocks and independent audit over 60,120 materialized calls. Its
  32-row vector-bearing p50 decreased in all five pairs (median −3.34%);
  256-row vector and hybrid deltas changed sign. Four blocks have host-only
  paging warnings. The differing engine and Python boundaries need
  attribution, not cancellation by one another.
  The [integrated installed Python S01 diagnostic](results/2026-10-07-python-s01-paired-integrated/README.md)
  also passed 20 blocks and independent audit over 60,120 materialized
  calls. Its vector-bearing p50 decreased in all five pairs at each size;
  host-only paging warnings and differing engine/SDK workload boundaries
  prevent a broad speed claim. The
  [integrated installed TypeScript S01 diagnostic](results/2026-10-07-ts-s01-paired-integrated/README.md)
  passed another 20 blocks and 60,120 calls. At 256 rows, vector-bearing
  p50 increased in all five pairs (median +2.89%), with two warning-bearing
  blocks. Disposition the differing engine, Python and TypeScript results
  at their actual call boundaries.
  The repaired-source [TypeScript S01 refresh](results/2026-10-07-ts-s01-vector-repaired-paired/README.md)
  passed 20 blocks and independent audit over 60,120 calls. Median pair p50
  changes were +2.78% for 32-row vector-bearing queries and +0.57% at 256
  rows. Seven blocks had host-only paging warnings; the result remains a
  diagnostic lead, not an equivalence verdict.
  A [controlled baseline-only Python S02 pilot](results/2026-10-07-python-s02-controlled-noise-pilot/README.md)
  now isolates the product timer and measures five environment-checked
  blocks. Its [Python subset protocol](s02-python-comparison-protocol.json)
  froze before the [paired Python S02 run](results/2026-10-07-python-s02-paired-current/README.md),
  which is historical after release integration. The
  [integrated-candidate Python S02 run](results/2026-10-07-python-s02-paired-integrated/README.md)
  then measured 100 valid sequences per version with independent raw and
  execution-order audits: pooled p50 +0.495% and p95 +0.924%, with host
  paging warnings in every pair. This is an observed slowdown, not an
  equivalence or release verdict. The repaired-source
  [Python S02 refresh](results/2026-10-07-python-s02-vector-repaired-paired/README.md)
  passed 100 whole sequences per version and both independent audits: pooled
  p50 +0.010%, p95 +0.155%, four warning-free pairs. Close and reopened-close
  stage medians still increased by 7.627 ms and 7.209 ms. This near-neutral
  diagnostic does not establish equivalence or resolve the lifecycle tradeoff. A
  [controlled TypeScript S02 baseline pilot](results/2026-10-07-ts-s02-baseline-controlled/README.md)
  preceded its [frozen paired subset](s02-ts-comparison-protocol.json) and
  [integrated-candidate TypeScript S02 diagnostic](results/2026-10-07-ts-s02-paired-integrated/README.md).
  The independent audit accepted 100 whole sequences per version: pooled
  p50 +2.870%, p95 -0.256%, with all five pair-p50 deltas positive and every
  pair carrying host-only paging warnings. This is a latency lead, not an
  equivalence or release verdict. The repaired-source
  [TypeScript S02 refresh](results/2026-10-07-ts-s02-vector-repaired-paired/README.md)
  accepted 100 whole sequences per version and independent raw and order
  audits: pooled p50 +3.308%, p95 +1.370%. All five pair-p50 deltas were
  positive and every pair had a host-only paging warning. The 147.193 ms
  reopened-stage median lead needs attribution against Python S02's near-
  neutral whole sequence. The
  [Rust SDK S02 candidate-only timing](results/2026-10-07-rust-s02-candidate-timing/README.md)
  then passed 100 fresh-process sequences and independent raw, resource and
  reopened-state audits on repaired source `3f29d649d`. Whole p50/p95 were
  5,754.390/5,815.786 ms; reopened open was the largest named stage at
  4,767.522 ms p50. No same-SDK 0.8.26 comparison exists. Bounded contention
  and the cross-boundary attribution remain. The [installed Python S02
  contention pair](results/2026-10-07-python-s02-contention-paired/README.md)
  subsequently audited 100 whole sequences per version on the same repaired
  source. Pooled p50/p95 changes were +0.373%/−0.143%; all five paired block
  p50 changes were positive, and each pair had host-only paging warnings.
  Actual writer/reader overlap, evidence, erasure and reopened SQLite state
  passed independent checks. This is a bounded Python diagnostic, not an
  equivalence claim. The subsequent [TypeScript contention feasibility
  pair](results/2026-10-07-ts-s02-contention-feasibility/README.md) exercised
  the installed packages with shared-handle overlap and independent reopened
  state checks, but its one-off times are unqualified. A subsequent
  [baseline-only TypeScript pilot](results/2026-10-08-ts-s02-contention-baseline-pilot/README.md)
  passed five independently audited blocks with 15 measured sequences and an
  81.814 ms block-median spread. The
  [frozen TypeScript contention subset](s02-ts-contention-comparison-protocol.json)
  binds the runner, installed archives, alternating order and 100 samples
  per version before candidate timing. Its
  [paired result](results/2026-10-08-ts-s02-contention-paired/README.md)
  passed all ten blocks and an independent audit over 200 measured
  sequences. Pooled whole-sequence p50 increased 3.489%, with all five
  pair-p50 deltas positive; p95 changed −0.164%. Four pairs had host-only
  paging warnings, and the one warning-free pair also had a positive p50
  change. This is a TypeScript latency lead, not an equivalence verdict.
  The [candidate-only Rust SDK contention result](results/2026-10-08-rust-s02-contention/README.md)
  passed ten fresh-process shared-engine sequences, each with seven or eight
  overlapping reader cycles and an independently checked reopened database.
  It has no 0.8.26 Rust SDK peer or paired latency claim. Cross-boundary
  attribution remains. The
  [S02-L baseline pilot](results/2026-10-07-python-s02-lifecycle-baseline-pilot/README.md)
  has 100 independently audited fresh-process open/close cycles; its
  [paired subset protocol](s02-python-lifecycle-comparison-protocol.json)
  froze before the
  [integrated-candidate paired campaign](results/2026-10-07-python-s02-lifecycle-paired-current/README.md).
  Independent recomputation found close p50 +141.2% and median PSS release
  45,719 KiB at close, versus -33 KiB on baseline. Disposition this
  lifecycle tradeoff against system-level latency and ownership/robustness;
  run contention, then
  execute S03. The [C01 qualification check](results/2026-10-07-c01-qualification/README.md)
  found the pinned LOCOMO corpus and nonempty Mem0/Qdrant volumes, but the
  exact external harness, configuration and output root are missing. No
  matched comparator timing is claimable. Keep engine, installed SDK and
  competitor boundaries distinct.
  The [S03 Python paired subset](results/2026-10-08-python-s03-paired/README.md)
  subsequently passed its precommitted protocol, 20 alternating blocks,
  14,000 independently audited fixture observations, reopened-state checks
  and three tamper controls. At 256 rows, the evidence p50 was +3.863% with
  five positive within-pair deltas; this needs call-boundary attribution.
  The same fixed mix put 93.49% of candidate elapsed query cost in evidence
  and filtered search at 256 rows. It is a workload proxy, not a production
  traffic profile or whole-release latency verdict.
- **Robustness:** real-database concurrency, kill/reopen, SQLite-full,
  projection recovery, provider error/timeout and close cases have inspected
  results. An [interrupted-erasure reopen case](results/2026-10-07-erasure-reopen-current/README.md)
  now proves that pending telemetry redaction survives a fresh engine open,
  refuses a sinkless retry and completes after the original sink is restored.
  An [exact-candidate projection-commit replay](results/2026-10-08-projection-commit-current/README.md)
  adds ten independently checked runs of six debug-hook fault and recovery
  tests, including stop/reopen, plus tampered-log negative controls.
  Finish the declared fault/schedule matrix with persistent and one-shot
  faults, further interrupted-erasure positions, state and resource oracles.
- **Logic and exceptions:** focused static checks and regression tests caught
  confirmed defects; a four-site `rows.flatten()` source audit classified
  further leads. The active vector site was subsequently confirmed by a
  real-database RED/GREEN test and repaired at `3f29d649d`. Complete the
  remaining site probes, ranked-path and binding-boundary panic/error
  review, current-candidate line/branch coverage, targeted negative probes
  and dispositions for findings and survivors. The exact-candidate coverage
  run also exposed a failing older `slice20_fts_rank_stream` malformed-row
  fallback test; its `Storage` result reproduced without instrumentation.
  Reconcile the old fallback oracle with the newer fail-closed row-error
  contract before claiming the test target or full gate is green.
- **Functional exercise:** the [capability register](phase1-capability-register.md)
  accounts for all 44 canonical operations: Rust 42 selected cases executed
  and two provider/model cases unavailable; Python 40 executed, one
  committed-closure positive-path gap and three unavailable; TypeScript 39
  executed, two positive-path gaps and three unavailable. The repaired-source
  [TypeScript consumer](results/2026-10-07-ts-vector-repaired-candidate/README.md)
  rechecked those 39 operations against rebuilt npm packages. Candidate S02
  functional refreshes pass, including the
  [vector-repaired external Rust consumer](results/2026-10-07-rust-sdk-s02-current-refresh/README.md)
  with independently checked reopened state. Its subsequent
  [candidate-only timing campaign](results/2026-10-07-rust-s02-candidate-timing/README.md)
  passed 100 measured sequences and five independent block audits. The
  integrated Python wheel repeated 40 selected
  positive/negative/reopen routes and one S02 sequence with independent
  audits; its validation-inclusive timer does not qualify S02 latency or
  contention. Close supported gaps and contract
  conditions or state residual risk explicitly.
- **Evidence and gates:** independently audit the exact-candidate raw
  campaigns and publish one four-area checkpoint note with invalid and
  omitted cells. The most recent full agent gate passed shell lint after a
  fix, then stopped at the pinned-override guard because six tracked result
  snapshots were named `Cargo.toml`. Their bytes are now retained under
  `Cargo.toml.snapshot`; the focused guard passes without a policy change.
  Rerun the needed full gate at the Phase 1 checkpoint. Earlier full-gate failures
  also have recorded steward-orient and Python environment dispositions to
  recheck. Do not claim a full green gate meanwhile.

Freeze the broader executable protocol after the remaining baseline pilots
and negative fixtures, run the missing exact-candidate cells, complete the
four matrices, independently recompute the report and record the Phase 1
checkpoint. Only then begin dedicated Phase 2 correct-results scoring.
Leave Gitleaks and final raw-archive retention until the end of this Phase 1
work. The E01–E12 paired archive remains local and untracked; preserve it for
analysis meanwhile. Automatic approval review rejected the proposed
path-scoped Gitleaks policy change, so final retention needs explicit
authorization or another policy-compliant resolution. This retention issue
does not delay the remaining measurements, fault cases, coverage analysis or
checkpoint drafting.
