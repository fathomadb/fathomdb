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
full protocol is **not frozen**: the remaining workload cells, mapping the
completed Slice 132 SDK surface to benchmark operations, corpus/model hashes
for those cells and independent review remain.

The bounded engine-only first-results subset is frozen separately in
[first-results-protocol.json](first-results-protocol.json) after the
[mixed-sequence baseline noise pilot](results/2026-10-07-mixed-noise-pilot/).
Its five alternating pairs, three cells and supported statistics govern the
first comparison. This broader Phase 1 protocol remains a draft until the
installed-SDK and remaining workload cells are qualified.

An [installed 0.8.26 Python S01 functional probe](results/2026-10-07-python-s01-baseline-probe/README.md)
established a real-database 32/256-row text, vector-bearing and hybrid
workload with basic validity assertions. The later
[baseline-only noise pilot](results/2026-10-07-python-s01-noise-pilot/README.md)
retains five valid separate blocks per size, raw attempts, a failed launcher
attempt and independent recomputation. The 100-sample pilot does not qualify
p99 or a candidate speed comparison. Its 32-row text p95 varied by 28.75%
across blocks; the S01 paired rule must use longer blocks and alternating
version order. This is a qualification input, not the full protocol freeze.

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
| S01 | Installed SDK call through materialized text/hybrid/vector result, with cold and warm read states and two corpus sizes. Map the completed Slice 132 Rust SDK and the Python/TypeScript methods to matching capabilities. | Pair Python and TypeScript where the same capability exists in both versions. Pair equivalent Rust engine cells: 0.8.26 has no Rust SDK crate, so the new Rust SDK is candidate-only and cannot support a version-paired installed-SDK comparison. Keep each boundary labeled. |
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

### Baseline-only noise pilot procedure

After the adapter's negative fixtures and exact 0.8.26 build provenance pass,
run at least five **separate** unprofiled baseline blocks, each with a unique
output directory and at least 100 valid text-query and 100 valid close/reopen
observations. Use the same 32-row seed, model, settings, host and artifact in
every block. Keep one warm-up per cell outside the timed samples. Preserve
each block's raw attempts, validator summary, start/end environment snapshots,
command, runner/protocol/artifact hashes and any invalid attempt. Run blocks
serially using the same hash-verified workload binary without rebuilding
between blocks. Let the host return to the recorded idle/thermal condition
after the qualification build; run without competing builds or profilers.
Never replace a slow valid sample with a rerun. A single passing block is an
adapter smoke test, not a noise estimate.

Report the per-block p50/p95, valid-attempt fraction, elapsed block time and
start-order trend, plus spread of block medians and p95 values. Treat drift or
invalid environments as a fixture problem and repeat the baseline pilot after
the cause is addressed. Use the baseline spread to justify final block and
sample counts and a paired uncertainty rule; the current 100-sample pilot
blocks cannot support p99. Freeze those decisions and the workload mix before
examining paired candidate timing. Retain baseline blocks even if the final
comparison protocol needs a larger sample count.

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

The provisional [Slice 135 receipt validator](../../../../../scripts/slice135_receipt.py)
is the first bounded harness increment. Its protocol and raw JSON objects
both use `schema_version: 1`, exact 40-digit Git `source_sha`, 64-digit
`runner_sha256`, relative-path-to-SHA-256 `artifact_sha256` maps,
`features` and resolved `settings` maps. The raw object additionally binds
the exact protocol-file bytes with `protocol_sha256`; the command checks the
measured runner or bundle passed with `--runner` and each artifact's actual
bytes. Invoke it with `--raw`, `--protocol`, `--runner`, `--artifacts-root`,
`--source-sha` and `--output`. The summary records the validator's own hash
separately; it does not substitute for the measured runner hash.
Feature, setting and observed-state comparisons preserve JSON value types.
The source SHA is an explicit input and still needs separate source/build
provenance verification. A package version string is not candidate identity:
the current 0.8.27 source still uses a 0.8.26 pre-release version string.

Each protocol cell declares `kind` (`query` or `lifecycle`), `boundary` and
nonempty `expected_checks` for basic semantic/state validity. The matching
raw cell contains an ordered `attempts` array. A valid attempt has positive
integer `latency_ns`, `semantic_ok: true` and matching `observed_checks`.
An invalid attempt has `valid: false` and a nonempty `reason`; it remains
in the raw file and counts in the summary's `valid_attempt_fraction`.
An attempt that records `semantic_ok: false` or mismatched `observed_checks`
is rejected even if labeled invalid. A genuine no-output failure can omit
these fields while retaining its reason. Every cell
needs at least 100 valid attempts for nearest-rank p50/p95. A query cell
reports p99 only with at least 1,000 valid attempts; otherwise it records
`unsupported_statistics: ["p99"]`. Lifecycle p99 is never reported.
The summary includes counts, maximum, bindings and the exact raw-file hash.
No valid slow sample is trimmed.

The raw `environment` has `start`, `end` and an empty `invalidators` list.
Both snapshots require nonempty host, kernel, CPU, storage, governor,
toolchain and profiler strings; zero named competing jobs; integer swap pages;
and disk space at or above the protocol's `min_disk_free_bytes`. Stable
identity fields must agree. The validator rejects child swap events using a
hashed GNU Time resource report; host-only swap drift remains an exact
warning, as justified by the [mixed pilot](results/2026-10-07-mixed-noise-pilot/).
It rejects a declared invalidator. The producer must record truthful inventory
and semantic observations; this validator does not collect them, prove that
a feature was actually enabled, or produce a quality gold answer. Its
thresholds and shape for the remaining Phase 1 cells are provisional until
their own pilots and protocol review. The same script can validate separate 0.8.26 and 0.8.27
receipts by binding each to its own source, protocol, runner and artifacts.

Freeze the **full** Phase 1 protocol as an executable, hashed protocol only after the
adapter's negative fixtures pass, the baseline-only pilot fixes counts and
uncertainty, exact assets and installed SDK methods are known, and a review
confirms feasible disk/time budgets. The
[dated first-results note](results/2026-10-07-first-results/) fulfills the
bounded first-results milestone only. The full four-area expansion and
independent recomputation required by the approved plan will constitute the
Phase 1 checkpoint; this draft and baseline-only data do not cross it.
