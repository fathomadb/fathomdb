---
title: Slice 135 Phase 2 paired MuSiQue evidence sufficiency
status: AUDITED_PAIRED_SUPPORT_AVAILABILITY
target_release: 0.8.27
---

# Phase 2 paired MuSiQue evidence sufficiency — 2026-10-08

The [frozen protocol](../../phase2-musique-support-protocol.json), SHA-256
`20080a0db5cf52bf0a2af388e4f6aaaae18d6883334640726935a26b09200aa5`,
was committed at `c5b3e77ba` before either [full run](run-commands.json).
The qualified local MuSiQue development bytes, SHA-256
`3cff37fd7221506a343a125cf7ca20aab7cd09877e376122da9627e1b935b26f`,
supplied 40 two-hop, 35 three-hop and 25 four-hop answerable questions by a
fixed hash selection. Their 2,000 paragraph instances became 1,669 distinct
title-plus-passage documents by exact body hash. Each query required its
entire source-labeled supporting set. The 4,734 unselected source rows,
including all unanswerable variants, were outside this denominator.

Both exact installed Python wheels wrote the same derived corpus to separate
fresh SQLite databases using the pinned CPU BGE model, closed, reopened and
ran ordinary public hybrid search to rank 20. The
[independent audit](audit.json) rebuilt the source mapping, checked installed
wheel/native and evaluator hashes, verified every canonical body, found
1,669 rows in each text index and vector partition per version, and
recomputed all 100 support-set outcomes. SQLite integrity was `ok`; neither
version had a search error or missing query.

| Required support set | Questions | Complete by rank 10, both versions | Complete by rank 20, both versions | Mean required-support recall at 20, both versions |
| --- | ---: | ---: | ---: | ---: |
| Two hop | 40 | 15/40 (37.5%) | 23/40 (57.5%) | 77.5% |
| Three hop | 35 | 7/35 (20.0%) | 12/35 (34.3%) | 72.4% |
| Four hop | 25 | 0/25 (0%) | 1/25 (4.0%) | 56.0% |
| All selected | 100 | 22/100 (22.0%) | 36/100 (36.0%) | 70.3% |

All 100 top-20 hit lists were identical between baseline and candidate, so
there was no paired coverage loss. The [diagnosis](diagnosis.json) lists every
shared incomplete case without question text: 64 lacked at least one
required support document by rank 20, including 24/25 four-hop questions.
This is a substantial shared evidence-availability gap for this selected
corpus, not a confirmed candidate regression. The paired case-bootstrap
delta interval is exactly zero because every observed list is identical;
the deterministic sample does not establish a population confidence bound.

The [pilot and negative controls](../2026-10-08-phase2-support-feasibility/README.md)
preceded scoring. A valid wrong hit lowered complete-set coverage, and
malformed identities, omissions, changed persisted content and incomplete
pairs were rejected. The executable evaluator and tests passed the full
workspace gate there. [SHA256SUMS](SHA256SUMS) binds the retained aggregate
results and stable local raw receipts, databases and derived input under
`raw-baseline/`, `raw-candidate/` and `raw-archive/`; corpus text remains
outside Git.

This cell measures retrieval of **all named supporting passages**, not
generated-answer correctness, faithfulness or citations. The acquisition
manifest's historical MuSiQue hash differs from the pinned local source
bytes, and the corpus is kept local under its cache-only CC-BY-4.0 boundary.
Only the installed Python CPU route and selected questions are qualified.
