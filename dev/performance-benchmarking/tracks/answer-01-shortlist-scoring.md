# ANSWER-01 — LOCOMO shortlist answer scoring

**Status:** setup active; direct HITL acceptance makes the directional GPU
survivor `hybrid_ce_alpha_10_pool_20` eligible for a preflight-only dry run.
No answerer or judge invocation is authorized until scorer routing, aliases,
retry/checkpoint behavior, and a spend ceiling are frozen.

## Decision

Do the selected retrieval configurations improve answer accuracy and temporal
correctness, not only retrieval proxy metrics?

## Preparation and contract

1. Freeze the accepted directional GPU survivor
   `hybrid_ce_alpha_10_pool_20` as the only dry-run retrieval treatment. The
   historical grid has no committed safe receipt, so this is a decision basis,
   not a reproducible full-grid measurement claim.
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
