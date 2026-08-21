# REASON-01 — Native HippoRAG-2 comparison

**Status:** parked; multi-hop calibration after GRAPH-01 establishes relevance.

## Decision

How does the selected FathomDB multi-hop treatment compare with native
HippoRAG-2 on supporting-evidence recall and answer quality?

## Preparation and contract

1. Pin the official repository, Python 3.10 environment, credential route,
   corpus reconciliation, model identities, and a cost ceiling.
2. Freeze MuSiQue question/evidence filtering, top-k, all-bridges metric,
   answer F1, timeout, and failure accounting for both systems.
3. Run a small external-only smoke before a complete matched comparison.

## Exit evidence

Matched complete receipts support a bounded multi-hop conclusion. A blocked
environment or corpus mismatch remains a `blocked_prerequisite` result.
