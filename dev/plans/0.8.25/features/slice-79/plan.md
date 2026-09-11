---
title: Slice 79 — AC-020 runtime configuration
status: DRAFT
---

# Slice 79 — explicit SQLite runtime configuration

## Purpose and authority

Drafted 2026-09-10 at the owner's request. Slice 79 was an unused reserved slot
when checked. This draft occupies the planning slot only: execution is not
activated, and the release-state ladder and `next_slice` are unchanged.

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
shared-runtime statistics disabling for this proposed work. They do not claim
runtime exclusivity or that file-sharing hazards disappear.

## Placement and prerequisites before READY

The [77 plan](../slice-77/plan.md), [80 plan](../slice-80/plan.md), shared
experiment protocol and release-state ruling still encode statistics-enabled
work. Reconcile those authority records through the prescribed ledger/state
tools before execution; preserve historical experiment conditions and receipts.
This draft does not silently amend their execution mandates.

Proposed allocation: 79 implements the runtime API and obtains focused recovery
evidence; 80 consumes it for the required owner consultation, integration
disposition and any separately approved remaining correction; 85 retains final
verification, CI and non-publishing packaging. Do not duplicate implementation
in 80. Decide whether unfinished 77 work is consumed, completed or stopped before
changing dependencies; drafting 79 does not cancel it.

Before READY: resolve the design's small interface choices, approve the revised
placement, seal source/build identity, enumerate exact focused test selectors
and counts, and obtain independent design review. No broad tests are authorized.

## Requirements and acceptance signals

| ID | Requirement | Focused evidence |
| --- | --- | --- |
| R79-1 | Startup-only two-mode Rust/Python/TypeScript admin operation | Fresh-process functional calls and real Engine opens |
| R79-2 | Consistent initialization | Identical calls succeed; conflicting and late calls fail without shutdown |
| R79-3 | Honest memory behavior | Diagnostics accounting/heap-limit witnesses; statistics-off witness |
| R79-4 | No connection restrictions or query-time overhead | Review and multiple Engine/database open-close tests |
| R79-5 | Matched AC-020 comparison with statement reuse in both arms | Seven MEMSTATUS-on and seven off runs; all seven off runs pass |
| R79-6 | Protected performance | Exact AC-072 candidate campaign and both protected 71B 10k workloads |
| R79-7 | Public contract and durable handoff | ADR/interfaces/changelog, surface checks and independent reviews |

These are local requirements, not new ACs. Do not edit acceptance or loosen gates.

## Execution sequence

1. Read 76/77 results and port the reviewed statement-reuse approach into the
   precise product candidate, including its focused semantic tests and bounded
   cache capacity. Keep it identical in both modes. Its faster sequential
   denominator means the prior MEMSTATUS-off ratio does not establish a pass.
2. Stage or commit RED tests for initialization, both modes and SDK parity.
   Runtime-global tests use separate fresh child processes, never forced resets.
3. Implement one Rust configuration owner and thin SDK wrappers. Audit every
   initialization entry point and legacy experiment hooks. Do not copy the old
   shutdown/configure/initialize experiment sequence into production.
4. Run changed-target lint/check and selected real-database/unit tests. Exercise
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

Use production configuration, not experimental environment variables. If startup
setup must call the new API, review that harness-only delta explicitly; timed
work and oracle stay unchanged. Keep accounting witnesses and profilers out of
verdict timing. Diagnostics mode is both timed and functionally verified; retain
its actual AC-020 pass/fail results, but diagnostics failure does not prevent
acceptance when all seven performance-mode runs pass and other requirements hold.

Amend the existing statistics-on protocol explicitly for this treatment before
running; retain its source identity, executor and noise controls. Allow only one
bounded environment correction, no tuning sweep or repeat-until-pass loop.
Failure returns measured evidence for consultation, not an automatic fork.

Run the exact AC-072 candidate campaign (p50 <= 80 ms, p99 <= 300 ms, unchanged
environment-validity rules) and candidate-only 71B Scale-02 and projection-active
10k workloads under their retained limits. Seal exact commands and receipt refs
from Slice 71 before execution. Do not rerun historical write baselines.

## Completion and exclusions

R79 requirements and independent reviews pass on the selected shipping-feature
candidate. Document unavailable memory controls, restart semantics and startup
behavior. Handoff identifies affected Rust/CLI and Python/Node artifacts and
platform checks still required in 85.

No broad regression, hosted CI matrix, packaging redesign, publishing, tagging,
push, migration/query rewrite, tuning sweep or roadmap implementation here.
Slice 85 owns the final broad round; a second requires explicit owner approval.
