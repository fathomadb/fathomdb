---
title: AC-020 statistics-enabled experiment protocol
status: APPROVED_FOR_SLICES_76_77
target_release: 0.8.25
---

# AC-020 statistics-enabled experiment protocol

## Purpose and authority

Generate decision-quality evidence with a bounded experiment sequence, not a
benchmark leaderboard. The owner requires real AC-020 recovery, rejects
shared-runtime MEMSTATUS disabling, prefers statistics-enabled optimization,
and wants to avoid fork maintenance. This protocol binds Slices 76 and 77.
Independent review must pass before either slice executes its experiments.
Experiment success means a question is answered; it need not mean a gate passes.

The baseline is the safe release checkpoint at `5056db9e`, with subsequent
planning-only commits allowed by relevant-tree identity. Never use the
rejected `a318f916` prototype as the release baseline. Do not edit the
registered test, workloads, worker count, oracle, or timing boundary.

## Source and executor preflight

1. Read release AGENTS/memory, the research, Slice 75 carry-forward, and Slice
   71 protected receipts. Verify branch, clean state and one writer per checkout.
2. Seal source SHA, relevant source-tree digest, Cargo.lock digest, Rust
   toolchain, SQLite/rusqlite/sqlite-vec versions, features, compiler flags,
   binary SHA-256, exact commands and fixture identity.
3. Confirm eight reader workers and registered four-document/8-dimensional
   fixture, 8 × 50 × 4 = 1,600 searches in each arm. Record any source mismatch.
4. Establish the registered executor from current acceptance/runner records;
   record CPU model, physical/logical CPUs, affinity/cpuset, CPU quota,
   frequency policy, load, memory, swap, thermal/throttling observations and
   competing work. Do not change gate-hosting authority to obtain a pass.
   A four-core history is a warning, not proof of current environmental failure.
5. Unset `FATHOMDB_PERF_EXPERIMENTS` and every discovered
   `FATHOMDB_PERF_SQLITE_*` override; enumerate the actual names from source
   in the manifest. Remove `LIBSQLITE3_FLAGS` for build and inspect effective
   SQLite flags. Verify statistics remain enabled in the diagnostic process.
   Do not emit unrelated environment variables or secrets into receipts.
6. Build each frozen source/configuration once. Separate build time from
   execution time; no benchmarks concurrent with builds, other benchmarks,
   model jobs or broad tests. Use release builds for timing.

## Unchanged performance command

Run from the candidate checkout; preserve return code and full raw log:

```bash
timeout 600 env AGENT_LONG=1 cargo test --release -p fathomdb-engine --test perf_gates ac_020_reads_do_not_serialize_on_a_single_reader_connection -- --exact --nocapture --test-threads=1
```

The sealed wrapper removes the experiment variables listed above. Require
exactly one executed test, no skip/ignore, and `AC020_NUMBERS`. Capture
sequential, concurrent and bound values from every run, including failures.
Use full-precision durations when already available; do not change the test
to improve reporting or recompute its verdict from truncated milliseconds.

## Timing design and classification

- Seven fresh-process repetitions per configuration. Seven baseline runs
  establish current behavior; matched control arms are interleaved with each
  treatment in a sealed alternating order, e.g. B,C,C,B,B,C,C,B,B,C,C,B,B,C.
  Every pair/block uses the same machine and build profile. Do not alter the
  internal sequential-before-concurrent order of the registered test.
- Preserve all runs. Predeclare environment-invalid criteria before timing:
  competing benchmark/build, CPU affinity/quota change, thermal throttling,
  swap activity, missing markers/counts or relevant source mismatch.
  Run failures remain failures; they are not environment-invalid by convenience.
- Report each observation, median, min/max, IQR and individual speedups;
  distinguish ratio-of-medians from median-of-ratios. Predeclare percentile
  calculation in the collector. IQR/median >10% on either timing arm is a
  noise stop, not a passing or failing product verdict.
- At most one documented environment correction/restart per slice, within
  its run cap. Retain the invalid campaign. If still noisy, stop.
- Screening: retain a candidate as performance-promising only if concurrent
  median improves > max(3% of matched baseline median, twice baseline IQR),
  with no >3% sequential median degradation. A smaller effect may be reported
  as inconclusive, not a proven win. These are selection rules, not new ACs.
- A consistently faster candidate that misses the ratio is useful evidence,
  not recovery. A ratio improved by slowing sequential execution is rejected.
- A strong experiment recovery requires all seven registered assertions to
  pass. Report margin; a 5% margin above 5.33x is desirable, not mandatory.
  Any failed repetition precludes a stable-recovery claim for that series.
  Slice 85 still owns formal release evidence; no 6-of-7 replacement oracle.

## Diagnostic design

For Slice 76, its reviewed four-slot design and execution manifest control this
section. Mandatory signals are the positive MEMSTATUS witness, combined current
SQL/preparation/reuse-distance census, one delimited on-CPU profile pair, and one
post-treatment census. Allocation-call/byte totals, page-cache detail,
lookaside detail and queue-delay telemetry are optional only when existing or
small already-authorized hooks expose them; Slice 76 must not build a general
instrumentation subsystem to obtain them. Missing off-CPU measurement prevents
a Slice 77 dispatch/lock-wait recommendation. Slice 76's missing
allocation/page counters prevent it from selecting BLOB, lookaside or
page-cache treatments. Slice 77 may select BLOB transport from its direct
post-cache CPU profile when a named parsing/conversion stack meets its sealed
threshold; allocation effects remain unknown. Lookaside and page-cache still
require quantitative counters. Unavailable signals remain unresolved rather
than inferred from CPU samples.

Diagnostics are non-shipping, process-owned and never timed as verdicts.
Use the available profiler first; if symbols are inlined/stripped, explicitly
record visibility and use one diagnostic-symbol build. A counting allocator
wrapper may be installed only in a disposable process that owns SQLite
initialization, with statistics enabled and identical allocation semantics.
No such configuration enters library startup or the shipping default.

Record calls, bytes and peak/live memory separately. Net
`SQLITE_STATUS_MALLOC_COUNT` or `PAGECACHE_OVERFLOW` deltas do not measure
total allocation/free churn. Count allocator entry calls and page-cache
allocation/free calls, including internal paths; label sampled/partial counts.
FTS5/sqlite-vec may bypass lookaside.

Classify visible allocation stacks and on-CPU contention separately: preparation/finalization,
ephemeral pager/page-cache, FTS5, sqlite-vec parsing/scratch, WAL/VFS, glibc
allocator, and application dispatch. Futex entry counts are neither total
mutex contention nor elapsed wait time; use CPU stacks and waits where
available, and report unknown/unattributed share. Never sum overlapping
percentages as if they were disjoint elapsed-time fractions.

When available under a slice manifest, dispatch diagnostics measure enqueue-to-start delay, service time, queue
depth, worker busy/idle periods and response waiting. A request queued behind
a busy worker while another is idle counts even if the bounded channel is
not full. Do not instrument verdict timing.

Measure per-reader lookaside hits, size misses, full misses, used/high-water
after each arm, reporting deltas and known reset semantics. Census distinct
SQL strings for both search shapes; record actual prepare/finalize/reprepare,
cache hits/evictions and retained statement bytes where observable.
Use EXPLAIN/EQP to investigate materialization, not as proof of allocations
removed. Verify ephemeral lifecycle across reset/re-execution directly if
claiming reuse; static presence of OpenEphemeral is insufficient.

## Correctness and prototype discipline

- Separate worktree per concurrent writer, based on the sealed release
  checkpoint rather than unrelated main. Do not install editable bindings
  from worktrees. No production default change on the release branch in 76/77.
- TDD for instrumentation/collectors and prototype correctness. RED covers
  missing/zero-test/skip/wrong-identity results, counter accounting, and the
  selected semantic boundary. Existing AC-020 is the unchanged performance RED.
- Caching tests cover alternating bindings, statement/row release before
  commit, error recovery, same-connection reuse, schema invalidation/reprepare,
  frozen/current visibility, dependencies and lifecycle. Do not insist that
  reprepare is zero during legitimate schema changes.
- Vector tests preserve accepted input/error semantics, f32 encoding,
  dimension checks, signed zero and finite edge values, binary quantization
  and bit order. Cover non-finite values only according to existing contracts.
  Use property tests and the current SQLite implementation as a differential
  mechanism reference, plus existing human-authored ranking/recall oracles.
- Dispatch changes preserve bounded queues/backpressure, worker affinity,
  shutdown/cancellation, error delivery, fairness and snapshot boundaries.
- No transaction merging across independent requests, result memoization,
  reduced candidate sets, weakened eligibility/frozen checks or hidden fixture
  special cases. Keep schema and public API unchanged.
- Capture before/after query plans as evidence. Require semantic invariants,
  not universally identical opcode listings for binding/query changes.

## Protected evidence and scope

Slices 76/77 do focused static checks and selected real-DB tests only.
For a candidate advanced from Slice 77, run the exact retained AC-072
10k/384d campaign and both 71B 10k candidate workloads; reuse command/fixture
definitions from Slice 71, with its existing environment and recovery guards.
Resolve and freeze those commands before candidate execution. Missing retained
artifacts block the guard claim; do not invent substitutes or historical runs.
No broad regressions, hosted matrices, package rebuilding, paid models or
100k/1M experiments in 76/77.

## Receipts and resource limits

Each slice writes under `dev/plans/runs/0.8.25-slice-N/`:
`manifest.json`, immutable raw logs, `observations.jsonl`,
`summary.json`, `result.md`, and `review.md`. A manifest binds every
observation to SHA/configuration/binary, host, order, timestamps, duration,
exit code, counts and log digest. Large profiles may live in an approved
artifact location with SHA-256 and retrieval instructions; /tmp alone is not
durable evidence. JSONL here is experiment data, not the program ledgers.
Retain negative results and all deviations; never regenerate old receipts.

Collector validation, builds and focused correctness tests are reported
separately from timing counts. Each slice caps profiling at nine instrumented
executions and timing at its declared count; individual diagnostic timeout
600 s, AC-020 timeout 600 s. Stop on any timeout rather than spending the
remaining cap blindly. At four hours of active experiment work in a slice,
report progress and remaining cost before continuing; this is a checkpoint,
not permission for broad tests or extra candidates. Two same-mode unsuccessful
correction attempts or tooling/correctness failures require rethinking and
escalation under repository retry rules. Predeclared AC-020 performance-assertion
failures in a sealed series are observations, not correction retries: retain
them and complete the series. The immediate stop on timeout, identity mismatch
or invalid instrumentation still applies; never retry until a measurement turns green.
