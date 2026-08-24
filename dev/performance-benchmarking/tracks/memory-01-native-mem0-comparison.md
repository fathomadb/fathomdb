# MEMORY-01 — Native Mem0 comparison

**Status:** complete; registered near-parity rule passed.

## Decision

Is the selected FathomDB profile near-parity or better than Mem0 under the
official harness and an identical answer-scoring contract?

## Plan

1. Run A0 and native Mem0 OSS on all 1,540 LOCOMO category 1–4 questions with
   the same corpus, order, user IDs, questions, and top-10 cutoff.
2. Score both completed retrieval arms with the official evidence-aware
   LOCOMO prompt and identical `gpt-4o-mini` answerer and judge settings.
3. Report paired overall and category 1–4 accuracy. Pass only when the
   one-sided 95% paired confidence lower bound for FathomDB minus Mem0 is at
   least zero.

The [execution controls](../2026-08-24-memory-01-execution-controls.md) pin the
harness, rate limits, retry behavior, cost ceiling, and current receipt.

## Result

FathomDB scored 75.19% versus Mem0 OSS at 67.21%. The overall paired delta was
+7.99 percentage points and its one-sided 95% lower bound was +5.78 points.
The [result note](../2026-08-24-memory-01-result.md) preserves category results,
the multi-hop loss, spend, and evidence links.

## Stop

Stop on an input mismatch, incomplete arm, or the $20 cost cap. Preserve
checkpoints and resume after provider backoff; characterize a loss without a
tuning loop.
