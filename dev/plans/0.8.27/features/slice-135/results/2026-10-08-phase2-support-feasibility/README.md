---
title: Slice 135 Phase 2 evidence and memory evaluator qualification
status: QUALIFIED_BEFORE_SCORED_SUPPORT_RUNS
target_release: 0.8.27
---

# Phase 2 evidence and memory evaluator qualification — 2026-10-08

The committed [input mapper](../../../../../../../scripts/slice135_phase2_support_inputs.py),
[runner](../../../../../../../scripts/slice135_phase2_support_runner.py),
[scorer](../../../../../../../scripts/slice135_phase2_support_scorer.py) and
[independent auditor](../../../../../../../scripts/slice135_phase2_support_audit.py)
passed 19 focused tests. The tests were observed failing before their
implementations were added. The [full workspace gate](verification.stdout)
then passed 186/186 suites with none skipped or excluded and zero security
violations, blockers or downgrades after commit `0c2f83963`.

The title-preserving MuSiQue pilot used three answerable two-, three- and
four-hop questions, 60 source passages and separate fresh installed-wheel
databases. Its [paired audit](musique-pilot-audit.json) checked exact persisted
rows, 60 text and vector rows per version, all three query receipts and
recomputed complete supporting-set coverage. The
[negative controls](musique-negative-controls.json) showed that replacing a
required hit with a valid but wrong document lowered complete-set coverage
at 20 from 2/3 to 1/3. Unknown hits, omitted queries, altered gold, changed
source and wheel bytes, changed persisted bodies and an incomplete pair were
rejected. An earlier pilot omitted source titles from passage bodies; it was
discarded before full scoring and the mapper gained a failing-first title
test.

The first LOCOMO scope pilot assigned a different `kind` to each conversation.
It produced zero vector rows and was discarded. The corrected pilot wrote
four `kind=doc` sessions from two conversations, projected a filterable
`conversation` attribute, and queried through the public attribute filter.
Its [paired audit](locomo-pilot-audit.json) found four canonical, text and
vector rows per version and four complete query receipts. The
[scope control](locomo-negative-controls.json) rejected an injected
out-of-conversation hit. The invalid pilot and both corrected raw pairs are
retained in the local `raw-archive/` for diagnosis; no pilot score is promoted
as the full-corpus result.

The pilots bound the installed 0.8.26 and post-Slice-132 0.8.27 Python
wheel/native identities, pinned CPU BGE model, source mappings and evaluator
hashes. The [MuSiQue protocol](../../phase2-musique-support-protocol.json)
and [LOCOMO protocol](../../phase2-locomo-memory-protocol.json) were committed
at `c5b3e77ba` after this qualification and before full scored runs.
Source text, derived inputs, question text and SQLite files remain local
outside Git. [SHA256SUMS](SHA256SUMS) binds the retained aggregate and stable
raw evidence files; preserve this bundle before worktree cleanup.
