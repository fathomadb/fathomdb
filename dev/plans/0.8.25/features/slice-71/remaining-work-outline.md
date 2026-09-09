---
title: 0.8.25 Slice 71 — remaining work outline
status: IN_PROGRESS
date: 2026-09-09
---

# Slice 71 remaining work outline

## Direction

The owner approved the remaining Slice 71 completion path on 2026-09-09. The
[71B write recovery](71b-performance-recovery.md) is complete and protected.
The remaining work is the independent AC-072 read-latency discrepancy described
by plan/design v5. The full regression matrix remains owned by Slice 75.

This outline retains historical evidence and indexes remaining parent work;
do not duplicate or execute a competing write protocol here.

## What the retained data establish

The write slowdown is not known to be exclusive to the dedicated bulk-ingest
runner:

- Six retained Slice 35 paired campaigns wrote the same 10,000-record fixture.
  The Slice 30 parent `b2bfb1f3` took median `ingest_ack_ms` values between
  2,086.61 and 2,128.08 ms. Successive Slice 35 candidates took between
  3,816.61 and 4,283.91 ms. The six median regressions were 88.04%, 85.34%,
  83.85%, 80.71%, 103.56%, and 101.57%. Bulk ingest therefore has run
  materially faster, repeatedly, on the same host and fixture.
- The admissible Slice 71 AC-013 campaign uses a different 10,000-node seeder
  in batches of 1,024. Its earlier baseline `4fc1b890` recorded separated seed
  writes of 1,834, 1,846, and 1,548 ms; the current candidate `5546585d`
  recorded 4,829, 5,104, and 5,096 ms. This is corroborating evidence that the
  signal reaches the ordinary `Engine.write` path used by another fixture,
  not proof of a distinct root cause.
- Slice 60's single `seed_ms=10264` observation is not directly comparable to
  the separated Slice 71 write/drain fields or its exact runner. It remains a
  symptom, not a baseline.

71B now retains matched 1, 10, 100, 1,000, and 10,000-row write measurements.
Every small-write criterion passes, and both 10k workloads recover beyond their
historical total-time limits. This closes the general-write question; it does
not establish or change full-search AC-072 latency.

The underlying Slice 35 run artifacts are retained outside the checkout under
`data/performance-benchmarking/scale-02/slice35-runs/`. The admissible AC-013
receipt and raw logs are under `dev/plans/runs/0.8.25-slice-71/`.

## Narrow change windows and hypotheses

The highest-value comparison is narrow and already available in Git:

1. `b2bfb1f3` is the common pre-Slice-35 baseline. Commit `c3de8c59` adds
   schema step 31 and the read-visibility trigger family. The Slice 35
   candidates descend from that change. This is the first window in which the
   repeatable 80.71%-103.56% 10k-ingest regression appears.
2. Step 31 adds three trigger definitions per authoritative table. The simple
   foreground node path causes three generation updates per node (30,000/10k);
   this is not a write-plus-drain count. 71B requires measured phase/connection
   attribution and treats the reviewer's roughly five-fire aggregate as
   preliminary until its starting state and artifacts are checked.
3. Commit `2a65a38a` (Slice 40) adds two projection-generation tables and six
   visibility triggers. Those triggers need not fire for every normal node
   write, so they are a secondary path-specific hypothesis rather than an
   explanation for the already-present Slice 35 regression.
4. Commit `76a61470` (Slice 45) replaces generation-only trigger bodies with
   generation plus `lower(hex(randomblob(32)))` nonce rotation. This cannot
   explain the earlier Slice 35 result, but it is the narrowest plausible
   contributor to additional current/Slice-60 write cost.
5. The full `4fc1b890..5546585d` interval contains many unrelated release
   changes and is unsuitable for direct attribution. Use the boundaries above
   rather than a broad historical bisect unless the narrow cells falsify all
   trigger hypotheses.

## Remaining parent obligations

### General writes — complete as 71B

Product candidate `eda95b07` passes the unchanged large/small-write criteria,
focused correctness checks, independent code review, and separate evidence
audit. Do not reopen this track or spend its recovered margin on AC-072.

### Read latency — separate unresolved disposition

The admissible AC-013 campaign remains `environment_invalid`. All six p50
values miss AC-072's 80 ms boundary (baseline 163–165 ms, candidate 200–201 ms),
and candidate p99 spread is 40%. Preserve this result and its original receipt.

The approved completion path first establishes equivalence with the historical
10k/384 synthetic runner and runtime, then decomposes one representative slow
full `Engine.search` call. Inspect vector fallback eligibility and the hybrid
FTS arm first, select a correction only from measured dominant work, implement
it with deterministic RED/GREEN proof, and run the exact prospective AC-072
campaign. A vector-only probe cannot close the gate.

### Verification and parent closeout

Use only the direct AC-072 blast radius: selected mechanism tests, affected
search/dependency/lifecycle/ranking tests, affected-crate check/clippy, and the
registered AC-072 campaign. If product code changes, rerun only the two 71B
10k candidate workloads; do not rerun their historical baselines. Full release
verification stays in Slice 75.

Close Slice 71 only after write recovery and the separate read-latency
obligation have their required dispositions. Update the release-state JSON
and regenerate its owned regions. Slice 72 remains dependency-blocked.

## Current durable boundary

The targeted search correction and its TDD chronology are retained through
`5546585d`. The admissible campaign, receipt, and investigation are retained
through `2863f7f8`; independent code review found no P1/P2 issue at that exact
commit. The separate verification pass was interrupted when the owner changed
the task to outline-only, so it is not a completion claim. The bulk-ingest
campaign was not rerun because the approved AC-013 stop condition fired.

Slice 71 is in progress and incomplete. 71B is complete at `eda95b07` with
focused tests, bounded performance evidence, and independent reviews. Immediate
next action is focused review of plan/design v5 followed by the measurement-
equivalence audit. Slice 72 remains blocked until AC-072 and Slice 71 close.
