---
title: Slice 71B — retained attribution result
status: STOPPED_SPREAD_INVALID
date: 2026-09-08
---

# Slice 71B retained attribution result

## Disposition

The single sealed attribution campaign stopped as required after 16 of 18
cells. Its classification is `spread_invalid`; the next unrun cells are
`ac013/production-3` and `ac013/no_op-3`. This result is retained without a
retry, discarded cell, outlier deletion, or threshold adjustment.

The campaign is incomplete, so it does not support a causal attribution or a
product correction. Slice 71B and parent Slice 71 remain open. Slice 72 remains
blocked pending the separate AC-072 disposition; this work neither reran nor
waived that gate.

## Bound execution

- The reviewed protocol, manifest, runner, probe, schemas, and validator are
  fixed at `76e42b90`. Independent protocol/code review passed 34 focused
  contract tests and the manifest, schema, and digest checks at that commit.
- The campaign used clean source `c80e6909e0860f6e57ed423fdf47fbafce2479c9`
  and the sole timed entry point:
  `python3 scripts/perf-experiments/run_slice71b.py run-attribution --source-root /tmp/fathomdb-s71b-current`.
- The retained receipt is
  `dev/plans/runs/0.8.25-slice-71/71b/attribution/attribution-receipt.json`,
  SHA-256
  `98a7b878306ceaf56d70809fbbb530e6caf84748aa6801cfa685dc2a3ed2b042`.
- The exact 9,532,344-byte executable has SHA-256
  `92604c16a0ad376ceeb291f9445222b2a8e3db307f99ac6cd7619c7f6713160f`.
  It is retained beside the build log and described by
  `attribution/executable-preservation.json`; the sealed receipt is unchanged.
- After both evidence audits, cleanup removed the 16 fresh disposable
  `cell.fathom` databases and their lock files. They were neither named nor
  hash-bound by the receipt. Every receipt-bound JSON/log artifact, the build
  log, and the copied executable remain. The clean detached source worktree was
  also removed; no implementation branch or integration worktree was created.

## Stop evidence

All 16 completed cells passed the registered load, memory, swap, temperature,
and competing-process gates. Independent audit recomputed a maximum load of
3.59 against a 12.0 limit across 24 online CPUs, at least 86.0% available
memory, a maximum temperature of 85.125 C, no swap delta, and no competing
process.

The three completed AC-013 `generation_only` repetitions were:

| Repetition | Acknowledgement (ms) | Drain (ms) | Total (ms) | Generation at acknowledgement | Generation after drain |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 1 | 2477.388 | 2909.088 | 5386.475 | 32501 | 50005 |
| 2 | 2266.078 | 3169.801 | 5435.880 | 31189 | 50005 |
| 3 | 4949.798 | 189.233 | 5139.031 | 48437 | 50005 |

Acknowledgement spread was 118.430%, above the registered 25% limit. Total
spread was 5.776%. The third repetition moved much more asynchronous projector
work before acknowledgement and correspondingly less into the drain interval;
all three reached generation 50005 after drain. This is evidence of unstable
acknowledgement/drain allocation for this fixture and timing boundary. It is
not evidence that any trigger component caused the product regression.

The completed Scale-02 matrix was stable. Its median total times were
3857.799 ms for `no_op`, 4208.207 ms for `generation_only`, and 4262.412 ms for
`production`. The generation-only/no-op contrast was approximately 9.08%, and
the production/generation-only contrast was approximately 1.29%. Neither can
satisfy the sealed rule requiring greater than 10% and greater than 0.25 ms in
both fixtures, and the incomplete AC-013 matrix cannot supply the second
fixture result.

## Independent review

Two read-only evidence reviews passed the retained receipt and artifacts. They
independently confirmed the exact 16-cell order prefix, immediate stop,
arithmetic, environment validity, source/build/runtime identities, artifact
hashes, treatment signatures, timing arithmetic, and lack of an admissible
causal or correction conclusion. Neither reviewer launched another campaign.
The durable verdict is [71b-attribution-review.md](71b-attribution-review.md).

## Verification accounting

This work used focused checks only: 34 contract tests; targeted Python lint and
syntax checks; JSON, JSON-Schema, manifest, and digest validation; Rust format
checking; a locked release probe build; two one-record harness smokes; the one
sealed campaign; and read-only protocol and evidence reviews. Two mistakenly
expanded Markdown-lint invocations terminated without a retained completion
result and are not evidence. Zero broad-verification rounds were completed, no
broad result is claimed, and AC-072 was not executed.

## Required next decision

The registered stop rule has done its job. Existing artifacts support a narrow
diagnosis that the AC-013 acknowledgement endpoint is not a stable completion
boundary while the projector runs concurrently. A prospective amendment is
drafted in [71b-prospective-amendment-draft.md](71b-prospective-amendment-draft.md).
Because that draft changes a primary validity rule after observing a result,
it requires explicit owner approval and independent prospective review before
any replacement measurement. No replacement campaign is authorized yet.
