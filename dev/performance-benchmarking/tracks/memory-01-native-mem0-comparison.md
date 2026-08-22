# MEMORY-01 — Native Mem0 comparison

**Status:** blocked on ANSWER-01 and external comparator prerequisites.

## Decision

Is the selected FathomDB profile near-parity or better than Mem0 under the
official harness and an identical answer-scoring contract?

## Draft plan

1. Wait for ANSWER-01 to select the FathomDB profile and for the official Mem0
   harness, corpus, credentials, and cost cap to be available.
2. Run one matched comparison with the same questions, answerer, judge, top-k,
   and timeout for both systems.
3. Decide near-parity or loss from paired results; report raw-evidence and
   extracted-memory regimes separately.

## Stop

Stop on an input mismatch, incomplete arm, or cost cap. Characterize a loss;
do not start an unbounded tuning loop.
