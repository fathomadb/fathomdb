---
title: 0.8.25 Slice 71 — remaining work outline
status: PAUSED_BY_OWNER
date: 2026-09-08
---

# Slice 71 remaining work outline

## Direction

Execution stopped on 2026-09-08 at the owner's direction. Do not run another
performance campaign, implement another treatment, or perform generic
verification from this outline. Resume only after the owner approves the next
measurement boundary. The full regression matrix remains owned by Slice 75.

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

There is no retained paired timing series for 1, 10, 100, or 1,000 records
across the relevant commits. Therefore the repository cannot presently say
how much user-visible latency a small write or small batch gained. The schema
mechanism fires per affected row, so some per-row cost is structurally
expected at every size, but its magnitude and fixed-cost crossover are
unmeasured. Do not report "bulk only" or a numeric small-write regression.

The underlying Slice 35 run artifacts are retained outside the checkout under
`data/performance-benchmarking/scale-02/slice35-runs/`. The admissible AC-013
receipt and raw logs are under `dev/plans/runs/0.8.25-slice-71/`.

## Narrow change windows and hypotheses

The highest-value comparison is narrow and already available in Git:

1. `b2bfb1f3` is the common pre-Slice-35 baseline. Commit `c3de8c59` adds
   schema step 31 and the read-visibility trigger family. The Slice 35
   candidates descend from that change. This is the first window in which the
   repeatable 80.71%-103.56% 10k-ingest regression appears.
2. Step 31 adds three triggers for each of 14 authoritative tables. A normal
   node fixture causes three visibility-generation updates per node, or
   30,000 additional singleton-row updates for 10,000 nodes. This is the
   leading explanation for the initial Slice 35 regression, but it has not
   been isolated by a controlled ablation.
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

## Work to perform after owner authorization

### 1. Set the performance decision boundary

- Decide whether AC-072 remains a release blocker on this host. All six
  admissible p50 values miss the 80 ms boundary: baseline 163-165 ms and
  candidate 200-201 ms. Candidate p99 spread is 40%, so the preregistered
  classifier correctly returns `environment_invalid`; no threshold change or
  repeat is authorized by the present manifest.
- If AC-013 is to resume, approve a revised preregistered environment/protocol
  before collecting data. Do not relabel the exploratory or admissible
  campaigns.
- Decide whether the write investigation must quantify interactive small
  writes or only prove and correct the already-repeated 10k regression. The
  recommended scope includes 1, 10, 100, 1,000, and 10,000 rows so a fix does
  not hide a fixed-cost penalty behind bulk throughput.

### 2. Attribute the write regression before designing a fix

- Reuse the exact 10k fixture and host controls from the retained Slice 35
  campaigns. Compare, in counterbalanced order, the pre-Slice-35 baseline,
  step-31 generation-only state, post-Slice-40 state, post-Slice-45
  generation-plus-nonce state, and current release state.
- Capture ingest acknowledgement separately from projection drain. Record row
  counts, fired trigger count, generation delta, nonce change, database/WAL
  bytes, CPU, RSS, errors, and host pressure.
- If the current candidate remains more than 20% slower, run only the planned
  diagnostic ablations: production triggers, generation-only/no-nonce, and a
  no-op trigger body. These are non-shipping counterfactuals.
- Repeat the minimum cells at the approved small sizes. Report absolute
  latency as well as relative regression; percentages are misleading near the
  fixed-cost floor.
- Stop if results are unstable, if the trigger cells do not explain the
  regression, or if isolation requires changing a public/schema/ADR contract.

### 3. Design and implement only a proved correction

- Preserve cross-process invalidation, raw-SQL mutation visibility,
  commit/rollback distinction, monotonic generation exhaustion, branch nonce
  behavior, authoritative-table coverage, and virtual-table owner coupling.
- Write deterministic RED tests for the selected mechanism and invariants
  before production changes. The performance measurement is supporting
  evidence, not the TDD oracle.
- Make the smallest correction supported by the ablation. Do not move
  invalidation solely into `Engine.write` unless external SQLite mutations and
  cross-process readers remain covered.
- Re-run only the preregistered affected-size cells and classify against the
  approved boundary. Stop for an owner decision if the invariant-preserving
  correction still misses it.

### 4. Apply proportional verification

There is no value in a full verifier while the performance disposition is
blocked. The following is the complete Slice 71 verification ladder:

- With evidence-only changes: validate receipt schema, hashes, derived
  classification, commit ancestry, and retained raw-log binding. No workspace
  regression run is required.
- With a harness-only change: run its shell/Python contract tests and syntax or
  type checks for the changed runner only.
- With an Engine/schema correction: run the new RED/GREEN tests, existing
  visibility trigger/rollback/frozen-read/cross-process tests touched by the
  mechanism, and focused `cargo check`/clippy for the affected crates.
- Obtain independent code review on the exact candidate and a separate
  read-only audit of the focused results. Do not run `agent-verify`,
  `scripts/check.sh`, long stress, CUDA, Windows, packaged cross-SDK, or hosted
  CI in Slice 71.
- Slice 75 performs the full release-wide regressions after Slices 71-73 have
  a disposition.

## Current durable boundary

The targeted search correction and its TDD chronology are retained through
`5546585d`. The admissible campaign, receipt, and investigation are retained
through `2863f7f8`; independent code review found no P1/P2 issue at that exact
commit. The separate verification pass was interrupted when the owner changed
the task to outline-only, so it is not a completion claim. The bulk-ingest
campaign was not rerun because the approved AC-013 stop condition fired.

Slice 71 is paused and incomplete. Slice 72 remains dependency-blocked until
Slice 71 receives either a measured disposition or an explicit owner-approved
deferral.
