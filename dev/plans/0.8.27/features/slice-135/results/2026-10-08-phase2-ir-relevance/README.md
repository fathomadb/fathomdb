---
title: Slice 135 Phase 2 paired judged retrieval relevance — 2026-10-08
status: AUDITED_PAIRED_REQUIRED_DOCUMENT_RETRIEVAL
target_release: 0.8.27
---

# Phase 2 paired judged retrieval relevance — 2026-10-08

The [frozen protocol](../../phase2-ir-relevance-protocol.json), SHA-256
`9ea9d3718f7906c5495ff238284158053d4696f5c5b97ce8682dfe22d9788d99`,
was committed at `09dcf491b` before this full score. The committed runner,
scorer and independent auditor passed a [real-database pilot and nine tamper
controls](../2026-10-08-phase2-ir-feasibility/README.md). Both exact
installed Python wheels then wrote the same 2,050 qualified EnronQA, QAConv
and QMSum documents to separate fresh databases with the pinned CPU BGE
model, closed, reopened and ran normal public hybrid search for all 4,472
positive gold queries. The 125 negative QAConv questions were reserved for
answer-abstention evaluation; no search-empty assertion was assigned to
them.

The [independent audit](audit.json) re-read the pinned source and gold files,
verified both wheel/native identities and every query ID, checked the exact
canonical body and source mapping in both SQLite files, and found 2,050 rows
in each text index and vector partition with integrity `ok`. All 4,472
queries completed without a search error on each version. The auditor
recomputed the named required-document rank for every query. A failed search
would have scored zero, and an omitted query would have invalidated the run.

| Gold class | Queries | Baseline Recall@1 / @5 / @10 | Candidate Recall@1 / @5 / @10 | Baseline / candidate MRR@10 |
| --- | ---: | --- | --- | --- |
| Exact fact (EnronQA + QAConv) | 2,888 | 0.6080 / 0.8525 / 0.8902 | 0.6080 / 0.8525 / 0.8902 | 0.709657 / 0.709599 |
| Exploratory (QMSum) | 1,584 | 0.0593 / 0.3144 / 0.4249 | 0.0593 / 0.3144 / 0.4249 | 0.159915 / 0.159915 |

The [diagnosis](diagnosis.json) includes the per-source metrics: EnronQA
Recall@10 is 0.9859, QAConv 0.8590 and QMSum 0.4249 on both versions.
Exactly four of 4,472 top-10 hit lists differ. Only one named required
document changes rank, from 2 on baseline to 3 on candidate for
`enronqa:39d3d5c75dfa883dd0bddd4c`; its candidate list gains a vector
hit above the required text hit. All other required ranks are equal.
The exact-fact paired MRR@10 difference is `-0.00005771`; its query-bootstrap
95% interval is `[-0.00017313, 0]`. This is a measured rank difference,
not a Recall@10 loss.

The two pinned mean vectors have the same hash, while the vec0 row layout
differs for 600 of 2,050 IDs. Physical insertion order can affect tied
approximate bit-candidate selection, but this receipt does not prove the
cause of the four list differences. No deterministic version-specific
wrong-result defect is confirmed. The low absolute QMSum reachability is a
shared diagnostic lead, not a candidate regression or proof that all other
retrieved documents are irrelevant.

Each positive query names **one** required document. These labels support
required-document Recall@K and reciprocal rank, not all-corpus nDCG,
precision, complete evidence sufficiency or answer correctness. QMSum's
single label is especially incomplete for broader relevance judgments.
Scores are for this CPU model, installed Python route, corpus and exact
artifact pair; they do not qualify CUDA or other provider/model variants.

The [full workspace gate](../2026-10-08-phase2-ir-feasibility/verification.stdout)
passed 186/186 suites, none skipped or excluded, with zero security
findings after the executable evaluator and tests were committed. The raw
per-query text and hits, source-derived bodies, and separate SQLite files
remain local under `raw-baseline/` and `raw-candidate/`, outside Git.
[SHA256SUMS](SHA256SUMS) binds the local bundle. Preserve it through Phase 2
and transfer it before worktree cleanup.
