# LOCOMO-01 — LOCOMO retrieval self-characterization

**Status:** directional decision accepted; no additional retrieval grid planned.

## Decision

Which FathomDB retrieval configuration is eligible for answer scoring on
conversational personal memory?

## Draft plan

1. Treat the reported 26-cell GPU grid as the directional decision basis, with
   the missing safe receipt stated as a limitation.
2. Carry `hybrid_ce_alpha_10_pool_20` forward as the single retrieval winner.
3. Validate that winner in ANSWER-01 rather than rerunning the grid.

## Stop

Do not run a CPU grid or another confirming sweep. Reopen only if answer scoring
finds a concrete retrieval-related regression.
