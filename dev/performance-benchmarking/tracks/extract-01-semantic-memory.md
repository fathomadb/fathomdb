# EXTRACT-01 — FathomDB-native extracted semantic memory

**Status:** input acquisition complete; requires a factual preflight and fixed
extraction treatment contract.

## Decision

Do FathomDB-native, provenance-linked extracted facts, preferences, and
episodes improve update and recall quality enough to justify their extraction,
conflict, and lifecycle costs?

## Draft plan

1. Use ELPS only for extraction conformance. Bind the registered LongMemEval-S
   knowledge-update slice to the answer/evidence portion of the quality
   contract; it does not substitute for extraction-precision gold.
2. Fix the facts, preferences, episodes, conflicts, and updates to score, with
   the corresponding source, answer, and lifecycle boundaries declared before
   execution.
3. Compare the selected canonical-record baseline with one bounded FathomDB
   extraction treatment carrying confidence and canonical source provenance.
4. Report extraction precision/coverage, evidence or answer quality, source-link
   completeness, conflict/merge outcomes, supersession, post-erasure absence,
   storage amplification, and ingest cost.
5. Accept only if quality improves within the declared cost and confidence
   boundary with no attribution or lifecycle violation.

## Stop

Stop after the fixed treatment or on inadequate gold. Do not tune prompts or
thresholds against the evaluation set, and do not treat ELPS as retrieval gold.
