---
title: Slice 135 Phase 2 paired LOCOMO memory evidence availability
status: AUDITED_PAIRED_SCOPED_MEMORY_EVIDENCE
target_release: 0.8.27
---

# Phase 2 paired LOCOMO memory evidence availability — 2026-10-08

The [frozen protocol](../../phase2-locomo-memory-protocol.json), SHA-256
`30a01243e2055e0c756b45ee43ba8b87cbdbb99601e2cfec33c7f847d3169b62`,
was committed at `c5b3e77ba` before either [full run](run-commands.json).
The pinned local LOCOMO source, SHA-256
`79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4`,
and existing loader resolved 272 session documents and 1,443 positive
questions: 841 factoid, 321 temporal and 281 labeled multi-session. Twelve
of the latter cite only one distinct session, leaving 269 strict
multi-session cases. The remaining 543 QA entries were excluded by the
positive loader; this cell assigns them no abstention or answer score.

Both exact installed Python wheels wrote the same sessions to separate fresh
SQLite databases with `kind=doc`, a vector-searchable summary and a
filterable conversation field. Every query used the public exact conversation
attribute filter, then closed/reopened normal hybrid search to rank 20.
The [independent audit](audit.json) rebuilt the source mapping, checked
installed wheel/native and evaluator hashes, verified every canonical body
and conversation scope, and found 272 rows in each text index and vector
partition per version. SQLite integrity was `ok`. It recomputed all 1,443
outcomes with zero search errors, missing queries or out-of-conversation hits.

| Source class | Questions | Complete required sessions by rank 10, both versions | Complete by rank 20, both versions |
| --- | ---: | ---: | ---: |
| Factoid | 841 | 793/841 (94.3%) | 835/841 (99.3%) |
| Temporal | 321 | 293/321 (91.3%) | 316/321 (98.4%) |
| Labeled multi-session | 281 | 133/281 (47.3%) | 227/281 (80.8%) |
| Strict multi-session subset | 269 | 123/269 (45.7%) | 215/269 (79.9%) |
| All positive questions | 1,443 | 1,219/1,443 (84.5%) | 1,378/1,443 (95.5%) |

All 1,443 top-20 hit lists were identical between baseline and candidate;
there was no paired coverage loss. The [diagnosis](diagnosis.json) lists the
65 shared cases without a complete required set by rank 20, including 54
labeled multi-session cases. The strict subset is a subset of the labeled
multi-session row and is not added to the overall denominator. The paired
case-bootstrap delta interval is exactly zero because every observed list
is identical; these ten conversations do not establish a population bound.

The [real-database pilots and negative controls](../2026-10-08-phase2-support-feasibility/README.md)
preceded scoring. An injected out-of-conversation hit was rejected; the
initial `kind`-scoped pilot was discarded because it had no vector rows.
The executable evaluator and tests passed the full workspace gate there.
[SHA256SUMS](SHA256SUMS) binds the retained aggregate results and stable
local raw receipts, databases and derived input under `raw-baseline/`,
`raw-candidate/` and `raw-archive/`; licensed source text remains outside
Git under its CC-BY-NC-4.0 evaluation-only boundary.

This is **required-session evidence availability**, not memory extraction,
knowledge-update resolution, abstention, answer correctness or faithful
citations. Those claims remain unsupported by this cell. It qualifies this
installed Python CPU model and conversation filter, not other SDKs, models
or providers.
