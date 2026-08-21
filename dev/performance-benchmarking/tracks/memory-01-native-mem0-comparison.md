# MEMORY-01 — Native Mem0 comparison

**Status:** blocked on ANSWER-01 and external comparator prerequisites.

## Decision

Is the selected FathomDB profile near-parity or better than Mem0 under the
official harness and an identical answer-scoring contract?

## Preparation and contract

1. Pin official harness/container commits, Python interpreter, Compose overlays,
   corpus digest, services, credentials route, and spend ceiling.
2. Run the native and FathomDB arms under matched question set, top-k, timeout,
   answerer, and judge controls; refuse mismatched or incomplete arm receipts.
3. Report raw-evidence retrieval and extracted-semantic-memory regimes separately.
4. Use paired confidence intervals and preserve all unavailable prerequisites as
   typed evidence rather than replacing them with local reimplementations.

## Exit evidence

A comparison receipt names its matched inputs, uncertainty, cost, and limitations.
Any loss is characterized into mechanism, fairness fix, and capability options for
human decision; it never autonomously selects a fork or product default.
