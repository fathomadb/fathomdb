# LATENT-01 — Long-context late-chunking feasibility

**Status:** parked until LOCOMO-01/PARENT-01 identify cross-window discourse loss.

## Decision

Can a long-context, token-output embedder improve the diagnosed subset enough to
justify its ingest, storage, and query costs?

## Preparation and contract

1. Document the failure signature and a labelled cross-window subset; do not use
   a generic stride sweep as its proxy.
2. Preflight model context length, token-vector access, licensing, hardware,
   identity pinning, and external artifact capacity.
3. Freeze naïve chunking, whole-document, and late-chunking controls with equal
   parent aggregation and answer-context treatment.
4. Measure retrieval, answer quality, duplicate rate, ingest/index cost, storage,
   and query tails separately.

## Exit evidence

The result identifies a query shape for which late chunking earns use or records
that it does not. It does not alter defaults without a separate product decision.
