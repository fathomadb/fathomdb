# LATENT-01 — Long-context late-chunking feasibility

**Status:** parked until LOCOMO-01/PARENT-01 identify cross-window discourse loss.

## Decision

Can a long-context, token-output embedder improve the diagnosed subset enough to
justify its ingest, storage, and query costs?

## Draft plan

1. Start only with a labelled set of real cross-window failures.
2. Compare the selected baseline with one late-chunking treatment under the
   same parent and answer-context rules.
3. Accept only if answer/retrieval quality improves enough to justify measured
   ingest, storage, and query cost.

## Stop

Do not run a stride grid. Stop if no diagnosed subset exists or the model cannot
provide token-level vectors.
