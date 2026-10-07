---
title: FathomDB 0.8.27 Slice 135 — Phase 1 measurement protocol draft
status: DRAFT_NOT_FROZEN
target_release: 0.8.27
---

# Phase 1 measurement protocol draft

The [approved Slice 135 plan](plan.md) controls this draft. It covers Pareto
paths, system latency, system robustness, and logic/exception handling. Use
existing benchmark datasets as workload inputs where qualified. Dedicated
gold-based quality scoring begins only after the four-result checkpoint. This
protocol is **not frozen**: the workload adapter, mapping the completed Slice 132
SDK surface to benchmark operations, baseline-only noise pilot, corpus/model
hashes and independent review remain.

## Source-grounded starting point

Slice 115's [runner](../../../../../scripts/slice115_runner.py),
[workload](../../../../../scripts/slice115_workload.rs), and
[protocol](../slice-115/protocol.json) cover twelve engine paths: fresh/open
populated, close, canonical write, projection, small CPU model probe, text,
vector stage, hybrid, graph expand, graph evidence and erasure. The runner is
locked to its own source SHA, Cargo lock, model and seven-sample count. It
cannot be run unchanged as a 0.8.26 versus final-0.8.27 comparison. Reuse
the cell semantics and receipt validation pattern; build a separate Slice 135
adapter with red tests for wrong source, state, sample count and output.

Historical 0.8.26 gauntlet receipts are context, not matched system timing:
the [result report](../../../../performance-benchmarking/gauntlet-v1.2/RESULTS-0.8.26.md)
separates retrieval scores from latency and records different boundaries.
Readiness snapshot on 2026-10-06: this host's
`/proc/sys/kernel/perf_event_paranoid` is `4`, so the Slice 115
GDB sampling fallback is available if unconfined ptrace is permitted. A
different host may qualify `perf`; record its setting. This checkout lacked a
checkout-owned `.venv` and LLVM coverage tools at that snapshot. Resolve
these before claiming binding tests or measured source coverage. The
worktree filesystem had about 53 GB free; estimate corpus, index and
temporary build footprint before any heavy comparator re-ingest.

## Workload cells to freeze

| ID | Boundary and scenario | Samples and validity |
| --- | --- | --- |
| E01–E12 | Port the twelve Slice 115 engine paths to separately labeled 0.8.26 and final-candidate runs; preserve pre/post semantic checks and fresh mutation state. | Query cells need at least 1,000 valid observations for p99; lifecycle cells at least 100 for p50/p95. A path that cannot meet this count reports only supported statistics. |
| S01 | Installed SDK call through materialized text/hybrid/vector result, with cold and warm read states and two corpus sizes. Map the completed Slice 132 Rust SDK and the Python/TypeScript methods to matching capabilities. | Pair the same semantic workload and model on both versions; keep Rust, Python and TypeScript boundaries labeled separately. |
| S02 | Mixed sequence: open, governed write, projection-to-ready, text/vector/hybrid retrieval, graph/evidence retrieval, erasure, close/reopen. Include single caller and bounded contention. | Time whole sequence plus declared stages; verify state at each transition. A stage timer cannot substitute for the end-to-end timer. |
| S03 | Qualified LOCOMO, LongMemEval, MuSiQue or query-correctness cases selected for distinct filter, temporal, graph, evidence and memory load shapes. | Record subset selection, adaptation, answer-bearing fixtures used for basic validity, and denominators. Phase 1 measures workload cost and exercised paths, not the full gold-quality score. |
| C01 | Matched native Mem0 warmed external client-to-materialized-top-10 LOCOMO cell. | Validate the pinned index before reuse; paid re-ingest has a $20 ceiling and requires checkpoint/resume/backoff. Report separately from 0.8.26 comparison. |

All paired cells use fresh databases from a byte-identical seed, a verified
release artifact or exact source build, the same eligible operation, model,
settings, cache state, host, storage and concurrency. Record intentional
capability differences rather than masking them. Timer start is client
invocation; timer end is materialized output. Setup and verification are
outside the timer except named lifecycle/ingest cells. Alternate version
blocks to reduce drift. Record full raw samples, failures and run order.

Run a baseline-only noise pilot before freezing the sample count and
uncertainty method. Percentiles use a declared nearest-rank rule. Report
paired deltas, throughput, CPU, memory, I/O and backlog where measured.
Unprofiled latency is the primary timing; instrumented/coverage/profiler
runs are attribution and test-quality evidence with overhead controls.
Slower or inconclusive results are diagnostic; accepted release gates are
reported separately.

## Pareto and logic coverage overlay

Use the fixed query-correctness/benchmark workload mix as a **proxy**, with
operation frequencies and measured total cost by operation and material
condition. Rank by aggregate elapsed time and separately by CPU/queue cost.
Report the smallest set accounting for 80% of observed cost and its fraction
of paths; do not presuppose a 20% code share. Keep severe rare cases visible.
Capture test and workload line/branch coverage in separate compiler-
instrumented runs and map hot branches to tests. A targeted plausible mutant
must be detected for at least one high-use assertion gap. Run existing static
checks, then audit `unwrap`/`expect`, error conversions, cleanup and FFI panic
containment on the ranked and boundary paths. Classify every finding.

## Robustness matrix

| Fault or schedule | State oracle after real-database reopen |
| --- | --- |
| Concurrent read/write and projection completion | Snapshot consistency, no missing committed record, truthful readiness and bounded worker shutdown |
| Process crash before and after a FathomDB-owned commit/queue transition | Canonical state and projection work either committed or absent as promised; replay/idempotence and typed status are truthful |
| Provider timeout/error before and after queued work | No false ready state, retained or explicitly failed work, bounded close and recoverability |
| SQLite busy and full-disk/permission failure at a named write boundary | No partial governed mutation; usable reopened database and exact error precedence |
| Close/cancellation under load | No stranded worker, outstanding handle or silent late commit |
| Interrupted erasure | No visible erased source after promised completion; interrupted work resumes or reports incomplete state truthfully |

At selected FathomDB-owned side-effect points, exercise one-shot and
persistent failures. Retain seed, fault point, schedule, before/after state
and resource counts. `PRAGMA integrity_check` is supplementary to FathomDB
state assertions. A process-kill case does not simulate torn or reordered
power-loss writes. Bounded resource pressure is separate from promising a
typed error for every Rust allocator exhaustion.

## Receipt and freeze criteria

Each attempt retains artifact/source SHA, runner/protocol SHA, corpus/model
digests, resolved settings, host/kernel/CPU/storage, toolchain and profiler
versions, call boundary, raw observations, semantic checks, environment
invalidators and invalid-attempt reasons. The independent summarizer must
reject mismatched hashes, altered settings, insufficient samples, missing
states and a deliberately wrong result. Disk pressure, competing heavy jobs,
swap, governor/storage drift and profiler substitution are explicit
invalidators. Do not silently discard slow valid samples.

Freeze this document as an executable, hashed protocol only after the
adapter's negative fixtures pass, the baseline-only pilot fixes counts and
uncertainty, exact assets and installed SDK methods are known, and a review
confirms feasible disk/time budgets. The first four actual final-candidate
result sections and their independent recomputation constitute the Phase 1
checkpoint in the approved plan. Neither this draft nor baseline-only data
crosses that checkpoint.
