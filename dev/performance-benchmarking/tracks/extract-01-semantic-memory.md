# EXTRACT-01 — FathomDB-native extracted semantic memory

**Status:** complete, limited; retain raw knowledge-update memory because
value-changing extracted facts are not consolidated.

## Decision

Do FathomDB-native, provenance-linked extracted facts improve knowledge-update
quality enough to justify their extraction, conflict, and lifecycle costs?

## Plan

1. Compare A0 FTS over canonical turns with the same turns plus question-blind
   native ELPS entities and fact edges on all 78 LongMemEval-S
   `knowledge-update` cases.
2. Use the reference answer and answer-session labels only for answer and
   evidence scoring. Report extraction precision as unscored because no human
   atomic-fact gold exists.
3. Report paired answer accuracy, evidence recall, source-link completeness,
   conflict and supersession behavior, post-erasure absence, storage
   amplification, ingest time, extraction failures, and LLM cost.
4. Accept only if quality improves within the fixed $20 cap with complete
   source attribution and no lifecycle violation.

## Result

The [78-case receipt](../../../experiments/runs/extract-01-knowledge-update-20260823T2236Z-59e805cb/record.json)
shows a descriptive +1/78 answer-accuracy delta and complete source links, but
the lifecycle cell leaves competing value-changing facts active. The decision
is `do_not_adopt_unconsolidated_extraction`. See the
[implementation and result note](../2026-08-23-extract-01-implementation.md).
Preferences, episodes, general long-term memory, and confidence calibration
remain outside this fixed claim.

## Stop

Stop after the fixed treatment or on inadequate gold. Do not tune prompts or
thresholds against the evaluation set, and do not treat ELPS as retrieval gold.
