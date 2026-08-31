# REASON-01 — Native HippoRAG-2 comparison

**Status:** complete; treatment rejected before native comparison.

## Decision

Can an explicit FathomDB relationship profile improve the diagnosed multi-hop
gap, and how does an accepted treatment compare with native HippoRAG-2 on
supporting-evidence recall and answer quality?

## Draft plan

1. Register `protected_multiquery_v1`: use the preserved development evidence
   only to bind its protected A0 prefix, bounded model-free queries, hybrid
   retrieval, and deterministic 20-item merge, not as an acceptance result.
2. Freeze the untouched held-out input and dated contract. Compare the profile
   with A0 on primary fractional gold-session recall, answer quality, latency,
   and context size. It is eligible only if both primary evidence recall and
   answer correctness pass their paired boundaries, groundedness and attribution
   do not regress, and the registered resource boundary passes. Preserve deep
   compact as a rejected development offshoot.
3. Only if eligible, pin the official HippoRAG-2 build and reconciled MuSiQue
   set; run one smoke, then one matched comparison with the same top-k,
   answerer, judge, and cap.
4. Decide the native comparison from supporting-evidence recall and answer F1,
   with paired uncertainty and failures counted equally for both systems.

## Current gate

The [held-out result](../2026-08-30-reason-01-result.md) rejects
`protected_multiquery_v1`. It improved fractional gold-session recall by 7.81
points and met its latency/context limits, but answer accuracy fell 2.75 points;
groundedness and attribution also regressed. Stop before native HippoRAG-2 and
do not refresh MEMORY-01 from this treatment. Unrelated npm-recovery and expired
CUDA release-manifest failures remain known repository debt.

## Stop

Stop if the FathomDB treatment is ineligible, on an environment or corpus
mismatch, on incomplete arms, or at the cost cap. Do not tune against held-out
outcomes or replace the native system with a local approximation.
