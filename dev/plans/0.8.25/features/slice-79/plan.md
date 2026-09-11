---
title: Slice 79 — AC-020 runtime configuration
status: COMPLETE
---

# Slice 79 — explicit SQLite runtime configuration

## Purpose and authority

Drafted 2026-09-10 and activated by the owner after Slice 77. The explicit
`MEMSTATUS=0` authorization is recorded at `seq-276`; the owner states that
FathomDB is the application owner of the SQLite runtime. Slice 79 is allocated
between 77 and 80 in release state.

The request named AC-070; the discussion concerns **AC-020**, concurrent reads.
AC-070 is failed-migration `user_version` preservation, not this work. See
[acceptance](../../../../acceptance.md) and the [draft design](design.md).

Deliver minimal `admin.configure_runtime()` startup selection of SQLite memory
statistics and verify unchanged AC-020 on the actual candidate. No private
SQLite or fork redesign is included.

## Owner decisions captured from the conversation

- Pursue `MEMSTATUS=0` without requiring runtime isolation. Accept runtime-wide
  loss of the affected SQLite memory-accounting and heap-limit interfaces in
  performance mode. This is not computer-wide configuration.
- Exactly two modes: performance and diagnostics (`MEMSTATUS=1`). No
  shared-runtime compatibility mode or silent fallback. Restart to change modes.
- Add engine-independent `admin.configure_runtime()` to the existing admin
  surface, not a separate top-level initializer.
- Identical repeated configuration succeeds; conflicting settings or requests
  too late to apply configuration return clear errors. Keep handles internal.
- No opening permits, connection authentication, ownership markers, file-access
  restrictions or elaborate sharing safeguards. Another application opening
  the database directly is accepted, not prevented.
- Verify the performance tradeoff; the earlier MEMSTATUS-off pass is not proof
  for the current candidate, particularly when combined with statement reuse.
- Retain the reviewed statement-reuse approach in both measured arms. Compare
  MEMSTATUS on and off on the same candidate, not against historical timings.
- Other runtime options and connection/cache controls remain deferred in
  [ROADMAP.md](../../../../../ROADMAP.md).

These record the new direction, replacing the earlier blanket rejection of
shared-runtime statistics disabling for this work. The configuration contract
requires application-owned startup order; separate accidental-connection
controls are not duplicated here.

## Reconciliation since the draft

1. Slice 77 closed inconclusive at `5484cd17`; it selected no treatment, but
   retained statement reuse as an eligible anchor.
2. The owner allocated reserved Slice 79 and superseded the shared-runtime
   prohibition through `seq-276`; the scope adjustment, protocol, release state
   and Slice 80 dependency are updated prospectively without rewriting history.
3. Code review found the existing experimental runtime hook calls
   `sqlite3_shutdown`; production configuration must replace only that global
   hook and never shut SQLite down.
4. The reviewed statement-reuse behavior is bound to `9e913517`/`b432d24d`:
   ten cached search statements per reader and the exact search/dependency
   preparation sites, without profiler/census features.
5. The public surface currently pins only `admin.configure`. The new runtime
   operation is an approved runtime-control member, not an application command;
   Python and TypeScript surface tests must both enumerate it explicitly.
6. Rust, Python and Node each configure the SQLite image linked into that
   artifact. Slice 79 proves local Rust and installed bindings; Slice 85 owns
   Windows, macOS, Jetson and the final package matrix.

Design review blocked the draft on these six points. This revision resolves
them; the exact selectors, counts, build identity, order and protected receipt
bindings are sealed in [execution-manifest.json](execution-manifest.json).
No broad tests are authorized.

## Requirements and acceptance signals

| ID | Requirement | Focused evidence |
| --- | --- | --- |
| R79-1 | Startup-only two-mode Rust/Python/TypeScript admin operation | Fresh-process functional calls and real Engine opens; CLI remains unchanged |
| R79-2 | Consistent initialization | Identical calls succeed; conflicting and late calls fail without shutdown |
| R79-3 | Honest memory behavior | Diagnostics accounting/heap-limit witnesses; statistics-off witness |
| R79-4 | No connection restrictions or query-time overhead | Review and multiple Engine/database open-close tests |
| R79-5 | Matched AC-020 comparison with statement reuse in both arms | Seven MEMSTATUS-on and seven off runs; all seven off runs pass |
| R79-6 | Protected performance | Exact AC-072 candidate campaign and both protected 71B 10k workloads |
| R79-7 | Public contract and durable handoff | ADR/interfaces/changelog, approved surface pin and independent reviews |

These are local requirements, not new ACs. Do not edit acceptance or loosen gates.

## Execution sequence

1. Read 76/77 results and port the reviewed statement-reuse approach into the
   precise product candidate, including its focused semantic tests and bounded
   cache capacity. Keep it identical in both modes. Its faster sequential
   denominator means the prior MEMSTATUS-off ratio does not establish a pass.
2. Commit RED tests for initialization, both modes and SDK parity.
   Runtime-global tests use separate fresh child processes, never forced resets.
3. Implement one Rust configuration owner and thin SDK wrappers. Audit every
   initialization entry point and legacy experiment hooks. Do not copy the old
   shutdown/configure/initialize experiment sequence into production.
4. Run `cargo test -p fathomdb-engine --test runtime_configuration -- --test-threads=1`,
   the three statement-reuse unit selectors, affected engine search/dependency
   tests, facade checks, and installed Python/Node runtime-control tests. Exercise
   installed Python/Node calls and Rust behavior, not merely exported symbols.
   Update ADR/interfaces/changelog and obtain the required governed-surface pin
   approval; never regenerate test oracles autonomously.
5. Run the bounded performance campaign and protected workloads below. Obtain
   independent code review and separate retained-evidence review. Reviewers
   inspect artifacts, not duplicate campaigns.
6. Hand off exact source/binary hashes, flags, mode, logs, outcomes and invalidated
   verification cells to 80/85. No release acceptance claim beyond the evidence.

## Bounded measurement and stop rules

Keep the registered fixture, eight workers, 1,600 searches per arm and
`concurrent <= sequential * 1.5 / 8` (at least 5.33x speedup). Run fourteen fresh
candidate processes on the registered executor: seven per mode. Retain each assertion/exit
status, absolute times, bound, speedup, median and dispersion. Expected assertion
failures are results, not retries. Do not slow sequential execution to pass.

Arm A is statement reuse plus MEMSTATUS=1; arm B is the identical statement-
reuse candidate plus MEMSTATUS=0. Use the same binary digest, build flags,
fixture and executor, differing only in the startup admin mode. Seal the order
as `A B B A A B B A A B B A A B` (seven adjacent counterbalanced pairs).
Report per-arm summaries and paired absolute/percentage differences for both
sequential and concurrent time, plus each run's own gate bound and outcome.
Do not use one arm's sequential time to judge the other arm. Retain historical
76/77 results as context only. No additional baseline campaign is required.

Keep the canonical performance/default gate name
`ac_020_reads_do_not_serialize_on_a_single_reader_connection` and add only the
diagnostics sibling
`ac_020_reads_do_not_serialize_on_a_single_reader_connection_diagnostics`.
Both share one fixture/timing/oracle helper; the diagnostics sibling invokes
the production API before open while the canonical entry proves first-open
performance default. Fixture, timed work, worker count and oracle remain
unchanged. Keep accounting witnesses and profilers out
of verdict timing. Diagnostics mode is both timed and functionally verified; retain
its actual AC-020 pass/fail results, but diagnostics failure does not prevent
acceptance when all seven performance-mode runs pass and other requirements hold.

Amend the existing statistics-on protocol explicitly for this treatment before
running; retain its source identity, executor and noise controls. Allow only one
bounded environment correction, no tuning sweep or repeat-until-pass loop.
Failure returns measured evidence for consultation, not an automatic fork.

Run the exact commands/fixtures retained by Slice 71's AC-072 receipt and 71B
recovery record: AC-072 10k/384d/1,000-query candidate campaign (p50 <= 80 ms,
p99 <= 300 ms), plus candidate-only Scale-02 and projection-active 10k
workloads under their recorded limits. Do not rerun historical write baselines.

## Completion and exclusions

Passing release acceptance requires all R79 requirements and independent
reviews on the selected shipping-feature candidate. A bounded campaign failure
may close Slice 79 only as a completed experiment/implementation result with
the unmet gate and any invalid protected receipt explicitly carried forward;
it is not release acceptance. Document unavailable memory controls, restart
semantics and startup behavior. Handoff identifies affected Rust/CLI and
Python/Node artifacts and platform checks still required in 85.

No broad regression, hosted CI matrix, packaging redesign, publishing, tagging,
push, migration/query rewrite, tuning sweep or roadmap implementation here.
Slice 85 owns the final broad round; a second requires explicit owner approval.
