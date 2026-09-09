---
title: Slice 71B — prospective attribution amendment draft
status: AWAITING_OWNER_APPROVAL
date: 2026-09-08
---

# Slice 71B prospective attribution amendment draft

## Why an amendment is needed

The first sealed campaign stopped after the third AC-013 `generation_only`
repetition because acknowledgement spread reached 118.430%. The same arm's
direct write-through-drain total varied only 5.776%. Generation observations
show that projector progress at acknowledgement varied from 31189 to 48437
while every repetition reached 50005 after drain. The measured acknowledgement
and drain intervals therefore redistributed concurrent projector work without
material instability in the completion interval.

This diagnosis comes from the stopped campaign. It does not retroactively make
that campaign valid, and it does not identify a product correction. The
receipt and all bound artifacts remain immutable.

## Proposed prospective change

For the AC-013 fixture only:

1. Keep acknowledgement, post-acknowledgement drain, direct total, and all
   generation-boundary observations mandatory and reported.
2. Use direct total time, rather than acknowledgement time, as the within-arm
   spread validity boundary. Preserve the 25% threshold.
3. Treat acknowledgement and drain as diagnostic partitions. Require their
   arithmetic to reconcile with direct total and require the existing
   treatment-specific generation observations to remain valid.
4. Use only total-time contrasts for component support. Do not use an
   acknowledgement-only change to support a cause or select a correction.

Scale-02 retains both acknowledgement and total spread gates unchanged because
its generation is complete at acknowledgement in all retained cells and both
endpoints were stable. All other sealed conditions remain unchanged: source,
fixtures, arms, trigger inventory, 3 repetitions, exact order, environment
gates, materiality thresholds, no outlier deletion, and immediate stop on a
validity failure.

Synchronizing or pausing the projector before acknowledgement is rejected for
this replacement protocol because it would remove the writer/projector
interaction the AC-013 fixture is intended to measure. Increasing repetitions,
relaxing the 25% limit, or deleting the stopped repetition is also rejected.

## Replacement evidence boundary

If approved, implement this as a new protocol and manifest version with a new
output directory. Bind the stopped receipt as predecessor evidence; never
overwrite it. Before execution, obtain an independent read-only review that
confirms the rule is prospective, only the stated validity interpretation has
changed, and the retained result cannot leak into ordering, thresholds, or
cell selection.

Run at most one replacement 18-cell campaign. If its total-time gate or any
unchanged environment/correctness gate fails, retain it and stop for a new
owner disposition. If it completes, apply the original cross-fixture
materiality rule and proceed to correction design only for a supported cause.
An `unresolved` result follows the already-approved conditional factorial path;
it does not authorize a speculative product fix.

## Authorization state

This document is a draft, not an approved execution contract. No code change,
replacement measurement, correction selection, or additional verification is
authorized until the repository owner explicitly approves or adjusts it.
