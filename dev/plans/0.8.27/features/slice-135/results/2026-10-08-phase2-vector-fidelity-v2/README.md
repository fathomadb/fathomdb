---
title: Slice 135 Phase 2 paired vector-stage fidelity — 2026-10-08
status: AUDITED_DIAGNOSTIC_MEAN_STATE_DIFFERS
target_release: 0.8.27
---

# Phase 2 paired vector-stage fidelity — 2026-10-08

The [v2 frozen protocol](../../phase2-vector-fidelity-protocol-v2.json),
SHA-256 `59bf631282913052785bc659aee0bb403858125f212efb40363d9dcf47ec1b88`,
was committed at `bd9b1880a` before this corrected scored pair. It selects
1,000 globally unique canonical bodies and 100 title-or-lead queries, 25 from
each of four local real-corpus sources. The baseline and candidate exact
installed Python wheels ran sequentially on separate fresh databases with
the pinned CPU `fathomdb-bge-small-en-v1.5` model. The first protocol's
duplicate-body fusion result remains an [invalid attempt](../2026-10-08-phase2-vector-fidelity/README.md),
not a score in this cell.

The measurement copy deletes only lexical-index rows after projection and
then reopens the installed SDK. The [independent audit](audit.json) confirms
1,000 persisted canonical rows per version, all 1,000 expected pre-isolation
FTS rows in each text index, zero post-isolation lexical rows, unchanged
within-version vector shadow-table bytes, zero text-only hits, and vector
branch attribution for every returned hit. It independently recomputes exact
f32 neighbors from the retained vectors, excludes the query's source body,
and rejects a vector-only rank inversion. This is a **vector-stage test seam**,
not ordinary public hybrid-search behavior.

| Version | Queries | Mean exact-neighbor recall@10 | Queries below 1.0 | Missing exact top-10 positions |
| --- | ---: | ---: | ---: | ---: |
| 0.8.26 baseline | 100 | 0.933 | 35 | 67/1,000 |
| 0.8.27 candidate | 100 | 0.931 | 40 | 69/1,000 |

The candidate is lower by 0.002 in this pair. It scores lower on 14 queries,
higher on 11, and equally on 75; the query-bootstrap 95% interval for the
mean paired difference is `[-0.016, +0.011]`. The [per-query diagnosis](diagnosis.json)
records every exact rank omitted from each 15-hit list and the 14 candidate
losses. Per-source means are baseline/candidate: daily logs 0.980/0.984,
CNN/DailyMail 0.948/0.944, Enron 0.868/0.872, and to-dos 0.936/0.924.
Every list had 15 hits and passed the exact-f32 monotonicity check. Thus the
67 and 69 missing exact neighbors were not rescued by the f32 rerank from
the approximate bit-candidate pool; this is a stage attribution, not a
judged relevance or answer-quality score.

The two receipts have byte-equal selected document vectors, query vectors,
query-text hashes and canonical rows. Their pinned mean vectors differ:
baseline SHA-256 `0a8286515df948119c4ecd38ba7f78fd7a9a348fda967fada9c69c4978a97e21`
and candidate SHA-256 `00af6ca380699a0f19802088839c18dde6bbbaacdd96e4f3b84897d19049f93d`.
Their vec0 row layouts also differ for 400 of 1,000 row IDs. The production
path pins the mean after 256 projection outcomes, so asynchronous projection
order can change the centered bit vectors and the 192-candidate shortlist.
The observed paired difference therefore cannot be assigned to a product
code regression from this one pair. No deterministic wrong-result defect was
confirmed. A controlled mean-state or repeated-pair study is required before
claiming a version-specific ANN difference.

The [full workspace gate](verification.stdout) passed 186/186 registered
suites, with no skipped or excluded suite and zero security findings, after
the v2 runner, scorer, auditor and negative-control tests were committed.
The raw receipts, pre-isolation and measurement databases, query text and
vectors remain local outside Git under `raw-baseline/` and `raw-candidate/`;
their [SHA256SUMS](SHA256SUMS) and the aggregate audit support local
recomputation. Preserve the raw bundle through Phase 2 and transfer it
before worktree cleanup. This vector fidelity result does not score judged
retrieval relevance, evidence sufficiency, memory usefulness or generated
answer quality.
