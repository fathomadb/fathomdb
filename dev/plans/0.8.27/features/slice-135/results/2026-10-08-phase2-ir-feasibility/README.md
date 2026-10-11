---
title: Slice 135 Phase 2 judged retrieval feasibility — 2026-10-08
status: INPUT_AND_AUDITOR_QUALIFIED_UNSCORED
target_release: 0.8.27
---

# Phase 2 judged retrieval feasibility — 2026-10-08

The committed [runner](../../../../../../../scripts/slice135_phase2_ir_runner.py),
[scorer](../../../../../../../scripts/slice135_phase2_ir_scorer.py) and
[independent auditor](../../../../../../../scripts/slice135_phase2_ir_campaign_audit.py)
were exercised on a three-document, three-positive-query pilot: one document
and its named required query from each of EnronQA, QAConv and QMSum. The
pilot's source rows, gold text, protocol and raw databases are retained
locally under `raw-inputs/`, `raw-baseline/` and `raw-candidate/`, outside
Git. They are qualification evidence only; the pilot's perfect ranks are
not a judged-relevance estimate.

Both exact installed Python wheels wrote three stable logical IDs, closed,
reopened and returned the required ID for each query. The [independent
audit](audit.json) verified all three canonical bodies, three rows in each
text index, three vector rows and SQLite integrity for each database. It
recomputed the required-document rank rather than trusting the runner.

Nine [tamper controls](negative-controls.json) used copies of the raw
receipts. Unknown hits, omitted queries, changed query/source/gold/wheel
hashes, a changed persisted body and an incomplete pair were rejected. A
known but wrong hit lowered the recomputed exact-fact recall instead of
being rejected. The original raw JSON and database hashes remained unchanged.

The evaluator code and ten focused tests were committed at `da609adab`.
The required [full workspace gate](verification.stdout) passed from a clean
tracked checkout: 186/186 suites, none skipped or excluded, and zero
security violations, blockers or downgrades. An earlier dirty-checkout gate
attempt failed only the Python native-receipt guard and was preserved in the
local gate logs; it is not a green claim.

The [full frozen protocol](../../phase2-ir-relevance-protocol.json) uses all
2,050 qualified source documents and all 4,472 positive queries, while
reserving 125 negatives for a separate answer-abstention rule. It scores
required-document Recall@1/5/10 and MRR@10 by class. The raw pilot and
later scored data remain subject to the source license and cache-only
boundaries; preserve them with the Slice 135 local raw bundle.
