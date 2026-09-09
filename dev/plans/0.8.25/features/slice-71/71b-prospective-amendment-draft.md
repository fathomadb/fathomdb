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

The same split is already visible in the two completed AC-013 `production`
repetitions: acknowledgement spread was 133.974% while total spread was 5.761%.
The two completed `no_op` repetitions had 4.399% acknowledgement spread and
2.195% total spread. The instability therefore tracks the asynchronous
projection-active arms rather than general host invalidity.

This diagnosis comes from the stopped campaign. It does not retroactively make
that campaign valid, and it does not identify a product correction. The
receipt and all bound artifacts remain immutable.

## Proposed prospective change

For only the v2 Phase-2 AC-013 10k current-source attribution matrix:

1. Keep acknowledgement, post-acknowledgement drain, direct total, and all
   generation-boundary observations mandatory and reported.
2. Make `total_ms` the sole within-arm stability estimand and require
   `(max / min - 1) * 100 <= 25`. Preserve the existing threshold exactly.
3. Treat acknowledgement and drain as finite, positive, arithmetically
   reconciled diagnostic partitions. They cannot support attribution or select
   a correction.
4. Seal the source-derived terminal generation delta at 50000. Every
   `production` and `generation_only` cell must start at the expected initial
   generation, reach exactly that delta after drain, and keep its
   acknowledgement generation between its start and terminal values. The
   terminal delta must be identical across all six cells. Every `no_op` cell
   must retain zero generation and nonce delta.
5. Use only total-time contrasts for component support.

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
The exception does not apply to small-write measurements or the Phase-5
baseline/current/fixed recovery campaign; both retain their approved
acknowledgement and total gates.

## Replacement evidence boundary

If approved, implement separately versioned v2 manifest, receipt, JSON Schema,
custom validator, and output directory. Preserve v1 validation unchanged. The
v2 manifest must bind this exact predecessor object:

```json
{
  "path": "dev/plans/runs/0.8.25-slice-71/71b/attribution/attribution-receipt.json",
  "sha256": "98a7b878306ceaf56d70809fbbb530e6caf84748aa6801cfa685dc2a3ed2b042",
  "state": "spread_invalid",
  "superseded_rule": "ac013_ack_spread"
}
```

Write deterministic failing tests first for v2/v1 dispatch, fixture-specific
spread metrics, predecessor binding, terminal-work equivalence, and rejection
of v1-cell reuse. Then implement GREEN without changing the v1 contract or
artifacts. Before execution, obtain an independent read-only review that
confirms the rule is prospective, only the stated validity interpretation has
changed, and the retained result cannot leak into ordering, thresholds, or
cell selection.

Treat v1 as exploratory and v2 as the prospectively sealed confirmation. Run
all 18 v2 cells exactly once; do not reuse v1 cells or merely complete its two
remaining positions. If the v2 total-time gate, equal-work gate, or any
unchanged environment/correctness gate fails, retain it and stop for a new
owner disposition. If it completes, apply the original cross-fixture
materiality rule and proceed to correction design only for a supported cause.
An `unresolved` result follows the already-approved conditional factorial path;
it does not authorize a speculative product fix.

## Advisory review

An independent read-only reviewer found the total-only AC-013 stability
estimand scientifically defensible for this narrow attribution matrix because
direct total measures the complete first-write-through-drain interval and the
existing causal rule already uses total contrasts only. The reviewer required
the scope, equal-work, and versioned-predecessor protections now stated above.
This was advisory design critique, not approval to implement or execute v2.

## Authorization state

This document is a draft, not an approved execution contract. No code change,
replacement measurement, correction selection, or additional verification is
authorized until the repository owner explicitly approves or adjusts it.
