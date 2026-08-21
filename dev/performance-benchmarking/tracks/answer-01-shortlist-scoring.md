# ANSWER-01 — LOCOMO shortlist answer scoring

**Status:** blocked on LOCOMO-01/PARENT-01 survivors and scorer readiness.

## Decision

Do the selected retrieval configurations improve answer accuracy and temporal
correctness, not only retrieval proxy metrics?

## Preparation and contract

1. Freeze at most canonical A0, best Fast, and best Quality-GPU survivors with
   their complete retrieval receipts and class breakdowns.
2. Verify A0 fingerprint parity before reusing historical scoring; otherwise
   score all candidates under one pinned invocation.
3. Preflight the authenticated loopback route, model aliases, one-worker retry
   behavior, checkpoint/resume, cumulative spend ceiling, and no-direct-egress rule.
4. Pre-register answerer, judge, prompts, temporal metric, uncertainty method,
   and abstention treatment.

## Exit evidence

Each candidate has a complete scored receipt and safe external artifact manifest.
The result selects a profile or reports inconclusive evidence; it does not start
MEMORY-01 without a declared winner and cost approval.
