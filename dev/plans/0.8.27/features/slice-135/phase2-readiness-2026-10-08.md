---
title: Slice 135 Phase 2 query correctness evidence index — 2026-10-08
status: PHASE2_REPORTED_WITH_UNSUPPORTED_CELLS
target_release: 0.8.27
---

# Phase 2 query correctness evidence index — 2026-10-08

The [approved plan](plan.md#phase-2-qualification-and-execution-order) and
[Phase 1 checkpoint](phase1-checkpoint-2026-10-08.md) authorized this work.
The [six-dimension Phase 2 report](phase2-query-correctness-report-2026-10-08.md)
is the current disposition. This page indexes its independent evidence;
Phase 2 reporting does not close Slice 135 or a release gate.

| Dimension | Current paired evidence | Boundary |
| --- | --- | --- |
| Contract correctness | [Query matrix](results/2026-10-08-phase2-python-matrix/README.md) and [graph/evidence/erasure](results/2026-10-08-phase2-graph-evidence/README.md) passed authored real-database oracles on both installed Python identities. | Selected public-Python contracts. |
| Vector fidelity | [Corrected exact-f32 vector-stage pair](results/2026-10-08-phase2-vector-fidelity-v2/README.md): 0.933/0.931 Recall@10 on 100 queries; 95% paired interval includes zero. | [First protocol](results/2026-10-08-phase2-vector-fidelity/README.md) invalidated by duplicate-body fusion. The corrected measurement isolates a vector-stage seam; index means differ. |
| Retrieval relevance | [Full judged pair](results/2026-10-08-phase2-ir-relevance/README.md): 4,472 positive queries, exact-fact Recall@10 0.8902 and exploratory Recall@10 0.4249 on both versions. | One named required document per query; no all-corpus nDCG or precision claim. |
| Evidence sufficiency | [MuSiQue complete-support pair](results/2026-10-08-phase2-musique-support/README.md): 36/100 complete by rank 20 on both versions. | Fixed 100-question answerable selection; no answer-quality inference. |
| Memory usefulness | [Conversation-scoped LOCOMO pair](results/2026-10-08-phase2-locomo-memory/README.md): 1,378/1,443 positive questions and 215/269 strict multi-session cases have complete required sessions by rank 20 on both versions. | Session-evidence availability only; extraction, knowledge update and abstention remain unsupported. |
| Generated answer quality | [Configuration receipt](results/2026-10-08-phase2-query-correctness/answerer-availability.json): no answerer endpoint/model or priced-run setting; zero paid campaign calls. | Correctness, faithfulness and citations unsupported until a provider and spending/resume controls are qualified. |

The [IR gold qualification](phase2-ir-gold-qualification-2026-10-08.md)
records the limited required-document labels and 125 negative retrieval
omissions. The [evidence and memory gold qualification](phase2-evidence-memory-gold-qualification-2026-10-08.md)
records MuSiQue, LOCOMO and LongMemEval source identities, label fitness and
licensing limits. The [support evaluator pilot](results/2026-10-08-phase2-support-feasibility/README.md)
proved wrong-result sensitivity and rejected malformed or incomplete pairs
before the MuSiQue and LOCOMO protocols were committed. All scored cells
bind exact installed wheel/native bytes and the pinned CPU BGE model.

The changed executable evaluator and tests passed the full
`./scripts/agent-verify.sh`: 186/186 suites, none skipped or excluded and
zero security violations, blockers or downgrades. Scoped Markdown lint
passed for the new reports. Raw per-case text, vectors, SQLite files and
derived gold stay local outside Git. The [Phase 2 retention receipt](results/2026-10-08-phase2-query-correctness/raw-retention.json)
binds a verified 96 MiB archive outside the worktree; source payloads remain
in the separately pinned local corpus store.

The next qualifications are conditional on the claim sought: controlled
mean-state or repeated vector pairs for a version-specific ANN verdict;
provider/model, paired answerer/judge, resume/backoff/completeness and $20
budget controls for generated answers; and separate SDK, platform or
fault-injection cells where a release claim depends on them. Phase 1
performance and platform follow-ups remain outside the query-correctness
entry gate under the [plan's disposition](plan.md#phase-2-sequencing-decision-2026-10-08).
