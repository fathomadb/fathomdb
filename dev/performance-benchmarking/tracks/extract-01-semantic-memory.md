# EXTRACT-01 — FathomDB-native extracted semantic memory

**Status:** planned; requires TRACE-01 lifecycle coverage and qualified update gold.

## Decision

Do FathomDB-native, provenance-linked extracted facts, preferences, and
episodes improve update and recall quality enough to justify their extraction,
conflict, and lifecycle costs?

## Draft plan

1. Use ELPS only for extraction conformance and a fixed human-reviewed set of
   facts, preferences, episodes, conflicts, and updates for quality scoring.
2. Compare the selected canonical-record baseline with one bounded FathomDB
   extraction treatment carrying confidence and canonical source provenance.
3. Report extraction precision/coverage, evidence or answer quality, source-link
   completeness, conflict/merge outcomes, supersession, post-erasure absence,
   storage amplification, and ingest cost.
4. Accept only if quality improves within the declared cost and confidence
   boundary with no attribution or lifecycle violation.

## Stop

Stop after the fixed treatment or on inadequate gold. Do not tune prompts or
thresholds against the evaluation set, and do not treat ELPS as retrieval gold.
