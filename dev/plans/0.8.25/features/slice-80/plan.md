---
title: Slice 80 — remaining AC-020 disposition
status: DRAFT
depends_on: 79
---

# Slice 80 — remaining AC-020 disposition

## Boundary after the Slice 79 allocation

The `seq-276` owner ruling allocates Slice 79 to the explicit runtime
configuration and statement-reuse candidate. Slice 80 no longer duplicates
that implementation. It consumes Slice 79's reviewed result and decides only
the remaining AC-020 disposition, if any, before final verification in 85.

Slice 79's product candidate `a6650c81` retains statement reuse and makes
performance (`MEMSTATUS=0`) the ordinary first-open mode. Its seven-run median
was 174.632177 ms sequential / 53.616722 ms concurrent, with a 3.135542x
median per-run speedup and 0/7 AC-020 passes against a 32.743532 ms median
bound. The ratio of medians was 3.257047x. Diagnostics was
170.293082/76.731105 ms (2.219349x), also 0/7. Both protected 71B write
workloads passed. AC-072 met its numerical limits in all three runs, but all
three had nonzero swap activity and are environment-invalid under the retained
protocol. Treat that product and evidence as the current anchor; do not repeat
the resolved runtime-mode decision or assume it closes the remaining post-cache
concurrency cost. Obtain one environment-valid AC-072 guard on the eventual
Slice 80 candidate.

## Not yet an implementation-ready plan

Additional implementation cannot responsibly be selected before Slice 79
finishes. This file specifies how to produce any remaining plan and the
required consultation. It does not authorize isolation, a rusqlite fork,
additional runtime-global settings or packaging redesign. AC-020 cannot be
deferred.

## Required planning inputs

Read the closed Slice 76 result, Slice 77 decision dossier and raw summaries,
prototype diffs/SHAs, research corrections, protected AC-072/71B receipts,
Slice 75 carry-forward inventory and same-file usage census. Verify each
against code. A prototype performance pass is not a shipping implementation
pass; unreviewed or missing evidence blocks READY.

## Owner consultation before design approval

Present a short recommendation comparing:

1. Minimal statistics-enabled candidate, including individual and combined
   effects, absolute times, uncertainty, memory retention and semantic risks.
2. Any remaining focused experiment proposed for reserved Slices 78–79.
3. Only if needed, private-runtime alternatives: generated adaptation versus
   maintained fork, extension linkage, migration API compatibility, supported
   platforms, update/security maintenance, and selective statistics policy.

Ask for decisions on the selected scope, acceptable resource tradeoffs,
same-process same-file support, and any isolation/API/dependency implications.
Do not ask again whether shared-runtime MEMSTATUS disabling or AC-020 deferral
is acceptable: both are rejected. If no justified solution is ready, keep this
slice in planning and request a bounded additional experiment.

## What the completed design must contain

- Numbered local requirements mapped to unchanged AC-020, host noninterference,
  retrieval correctness, protected performance and existing SDK contracts.
- Exact chosen code changes, rejected alternatives, connection/statement
  ownership and cleanup, allocation lifetime and bounded cache/queue memory.
- Current/frozen snapshot, eligibility-before-ranking/cap, error precedence,
  cancellation, concurrent DDL/reprepare, lifecycle/erasure and projection
  invariants. No per-fixture branches or cross-request result caching.
- Explicit treatment of private runtime and memory controls if approved;
  proof that all SQLite/sqlite-vec references bind as intended, and that the
  public migration contract and same-file access are safe. A C ABI boundary
  need not inherently preclude source builds; compare actual implementations.
- Full RED/GREEN sequence, exact focused selectors/counts, diff-scoped static
  checks, artifact implications and acceptance/rollback criteria.
- Slice 85 invalidation map by source, fixture, feature, model, workflow and
  package inputs. Changing engine code generally invalidates installed engine
  artifacts even if SDK wrapper code is unchanged.
- Any required ADR/interface-doc/changelog updates. Do not amend public
  behavior through an internal implementation note alone.

Obtain independent design review and explicit owner approval after consultation
before marking READY. The normal review/retry caps apply; no extra correction
cycles are implicitly authorized.

## Implementation and focused verification

Preserve a committed/staged RED before product fixes. Implement the selected
minimal correction; retain or port justified prototype tests, remove diagnostic
startup hooks from shipping paths, and verify the shipping feature set matches
the tested one. Experiments are not merged wholesale.

On the actual implementation candidate, run the unchanged seven-process
AC-020 recovery series under the established protocol, focused functional and
property tests, and affected-target check/clippy. Reuse Slice 77 performance
only if relevant source/build identity is demonstrably identical; otherwise
measure the actual final implementation.

Run the exact AC-072 candidate campaign and both 71B 10k candidate workloads
if relevant code changed from the guard-tested prototype; no historical
write-baseline reruns. Preserve the existing limits and environment rules.
Also select direct-text/real-vector parity checks if SQL/vector semantics
changed; small targeted correctness evidence comes before the final expensive
real-model campaign in Slice 85.

Independent implementation review and separate evidence audit must pass.
If a correctness defect appears during verification, return to focused TDD;
do not weaken an oracle. Isolation or a broader rewrite discovered necessary
mid-implementation requires a new decision, potentially reserved Slices 81–84.

## Completion criteria

A reviewed shipping candidate meets unchanged AC-020, focused semantics,
host compatibility, and protected read/write criteria; exact evidence and a
complete Slice 85 invalidation/handoff manifest are durable. No broad suite
or package/CI matrix runs in Slice 80 by default. Slice 85 remains responsible
for final release-level validation.

A legitimate no-code outcome is possible only if experiments prove the
unchanged release candidate passes on the registered executor and reviews
resolve the discrepancy without altering the gate. It still requires owner
consultation, exact evidence and the Slice 85 handoff.

No publishing, version cut, registry mutation, tags, push or merge to main.
