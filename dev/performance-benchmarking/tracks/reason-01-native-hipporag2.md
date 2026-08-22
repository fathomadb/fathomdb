# REASON-01 — Native HippoRAG-2 comparison

**Status:** parked; multi-hop calibration after GRAPH-01 establishes relevance.

## Decision

How does the selected FathomDB multi-hop treatment compare with native
HippoRAG-2 on supporting-evidence recall and answer quality?

## Draft plan

1. Start only if GRAPH-01 shows a useful multi-hop treatment.
2. Pin the official HippoRAG-2 build and reconciled MuSiQue set; run one smoke,
   then one matched comparison with the same top-k, answerer, judge, and cap.
3. Decide from supporting-evidence recall and answer F1, with failures counted
   equally for both systems.

## Stop

Stop on environment or corpus mismatch, incomplete arms, or the cost cap. Do
not replace the native system with a local approximation.
