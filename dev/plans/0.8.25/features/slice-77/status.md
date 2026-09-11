---
title: Slice 77 status
status: COMPLETE
date: 2026-09-10
cleanup_candidate: b3ccba83
---

# Slice 77 status

Slice 77 is complete as a bounded candidate-selection experiment. It does not
ship a product correction and does not close AC-020.

## Result

The statement-reuse anchor reconfirmed 174.216944 ms sequential and 86.531221 ms
concurrent medians, a 2.013342x scaling ratio, and zero AC-020 passes in seven fresh
processes. At unchanged sequential performance, concurrent time needs roughly
another 62% reduction to reach the median registered bound.

A direct post-cache gperftools profile reproduced the registered sequential
warm state and covered all eight reader workers. It captured 52 samples, but
five cover teardown inside the temporary boundary. Of 47 search-phase concurrent
CPU samples, 23 included SQLite locking paths, 16 allocation paths, 11
page-cache paths, one vector conversion, and two parser/preparation paths.
Categories overlap. This is CPU-only evidence and does not measure blocked
time.

The 47 search samples miss the sealed minimum of 50, so V and S remain
unresolved rather than excluded. Q was therefore ineligible, D lacked
queue-wait plus idle-worker evidence, and L lacked quantitative lookaside miss
counters. No treatment was authorized and no speculative correction was
implemented.

## Acceptance

| Requirement | Status | Evidence |
| --- | --- | --- |
| R77-1 selection informed by Slice 76 | Pass | Exact reviewed statement-reuse anchor and direct post-cache profile pair |
| R77-2 attributable effects | Complete with qualification | Seven fresh anchor processes confirm failure, but missing contemporaneous executor preflight prevents treatment attribution |
| R77-3 preserve semantics and prior recovery | Pass | No shipping product/test change; restored prototype matches reviewed blobs and final product matches protected checkpoint |
| R77-4 supportable implementation decision | Pass | `decision.md` ranks and excludes candidates without converting unknowns to findings |
| R77-5 no forced isolation | Pass | One bounded counter-census option is proposed for owner consultation; neither isolation nor a reserved slice is authorized here |
| R77-6 direct post-cache residual measurement | Inconclusive | 47 search-only samples miss the minimum of 50; teardown and off-CPU limitations are explicit |

Planning/design review and temporary prototype code review pass. The focused
runner contract passes 8 tests. The separate evidence audit verifies retained
results without launching a duplicate campaign.

## TDD and verification

The unchanged registered AC-020 failure remains the performance RED. A focused
evidence-runner RED/GREEN correction preserves full-precision durations already
present in the raw logs. Because no
treatment qualified, no semantic RED/GREEN implementation or protected
candidate guard was applicable. No broad regression, package matrix, hosted CI,
AC-072 rerun, or Slice 71B timing rerun was performed.

## Cleanup and handoff

The statement-reuse/profile prototype is preserved only in identifiable Git
history and durable evidence. Cleanup commit `b3ccba83` restores the four engine
product/test files to the pre-slice release tree byte-for-byte. Temporary build
and profile working artifacts are removed after the evidence audit; the
committed raw profiles, reports, logs, summaries, hashes, and commands remain.

Slice 80 receives the decision dossier for mandatory owner consultation. Its
smallest proposed next diagnostic is an explicitly allocated reserved Slice 78
with a corrected search-only profile, one per-reader lookaside
hit/miss/high-water census, and at most one evidence-directed setting. Slice 77 itself allocates no reserved slice and
authorizes no isolation or shared-runtime `MEMSTATUS` change.
