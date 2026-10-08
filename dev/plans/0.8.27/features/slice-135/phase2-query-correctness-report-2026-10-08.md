---
title: Slice 135 Phase 2 query correctness report — 2026-10-08
status: COMPLETE_WITH_EXPLICIT_UNSUPPORTED_CELLS
target_release: 0.8.27
---

# Slice 135 Phase 2 query correctness report — 2026-10-08

## Disposition and identity

The Phase 2 inquiry is complete for the pinned installed-Python CPU route.
The independently specified deterministic contracts passed on both versions.
Judged retrieval and required evidence availability have no candidate
top-K loss in the measured corpora. The candidate has one observed required
document rank change and a small vector-stage recall difference, neither
isolated as a product-code defect because the vector index states differed.
No test-first product repair was warranted from this evidence. Generated
answer quality is explicitly unsupported. This report is diagnostic and
does **not** close Slice 135, satisfy a release gate or qualify the later
integrated Slice 150 path.

The exact 0.8.26 source is `f99e002f0d2e4002f3694c9f8d4986b56089edaa`;
its installed Python wheel/native SHA-256 values are
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282` /
`269fafc1c696244d05f2eabea19b23cfba6de79f5041b1123fe10589c46c51dc`.
The candidate product source is
`224e44c593c13d86ece648adabe445723db04070`; its wheel/native values are
`ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a` /
`1f213a700cf0c96b9997700d6e44ed7019b26fb91df1259bc6514fefb4a8d013`.
The current product trees under `src/rust/crates/`, `src/python/fathomdb/`
and `src/ts/` still have no diff from that candidate source. Every judged
run used the same pinned CPU `fathomdb-bge-small-en-v1.5` model at revision
`5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`, fresh equivalent databases,
and exact installed wheel/native checks. Source text, derived verbatim gold,
query text, vectors and SQLite files remain local outside Git.

The [approved plan](plan.md#phase-2-qualification-and-execution-order),
[gold qualifications](phase2-evidence-memory-gold-qualification-2026-10-08.md),
and frozen per-cell protocols govern denominators and omissions. Real-database
pilots and deliberate wrong-result controls preceded scored runs. A baseline
and candidate match was never used as the gold oracle.

## Six dimensions

| Dimension | Result on the pinned route | Claim boundary |
| --- | --- | --- |
| Contract correctness | The [paired query matrix](results/2026-10-08-phase2-python-matrix/README.md) met authored filter, successful-empty, cursor-page and typed-refusal oracles before and after reopen on both versions. The [graph/evidence/erasure contract](results/2026-10-08-phase2-graph-evidence/README.md) met authored graph sets, resolved source bytes, physical erasure and reopened-state oracles on both versions. | These are selected deterministic public-Python cases, not an exhaustive fault, SDK or platform proof. |
| Vector fidelity | In the corrected [1,000-body, 100-query vector-stage test seam](results/2026-10-08-phase2-vector-fidelity-v2/README.md), exact-f32 neighbor Recall@10 was **0.933 baseline / 0.931 candidate**. Fourteen queries favored baseline, eleven favored candidate, and 75 tied; the paired bootstrap 95% mean-delta interval was `[-0.016,+0.011]`. | The first duplicate-body protocol was invalid and retained only as a diagnostic. Document/query vectors matched, while pinned mean vectors and vec0 layouts differed. This is an isolated vector-stage seam, not ordinary hybrid output or an established code regression. |
| Retrieval effectiveness | The [judged relevance pair](results/2026-10-08-phase2-ir-relevance/README.md) scored all 4,472 positive queries with zero search errors. Required-document Recall@10 was **0.8902** for 2,888 exact-fact queries and **0.4249** for 1,584 exploratory queries on both versions. Exactly four top-10 lists differed; one named required document moved from rank 2 to 3 on the candidate. | The exact-fact MRR@10 delta was `-0.00005771`, with paired query-bootstrap interval `[-0.00017313,0]`; no named required document left top 10. One required label per positive query cannot support all-corpus nDCG or precision. The 125 negative QAConv queries were omitted from retrieval scoring. |
| Evidence sufficiency | The [MuSiQue support-set pair](results/2026-10-08-phase2-musique-support/README.md) retrieved every named supporting passage by rank 10 for **22/100** questions and by rank 20 for **36/100**, identically on both versions. All 100 top-20 lists matched. | The 100 fixed-hash answerable questions are 40 two-hop, 35 three-hop and 25 four-hop. Only 1/25 four-hop sets were complete by rank 20. This measures context availability, not answer correctness. Local source bytes differ from the acquisition manifest hash. |
| Memory usefulness | The conversation-scoped [LOCOMO session-evidence pair](results/2026-10-08-phase2-locomo-memory/README.md) retrieved complete required sessions by rank 20 for **1,378/1,443** positive questions on each version, including **215/269** strict multi-session cases. All 1,443 top-20 lists matched and had no out-of-conversation hit. | This is session-evidence availability. Extraction, knowledge-update answer resolution, abstention and generated memory answers remain unsupported. The 543 excluded QA entries receive no implicit score; the local corpus is CC-BY-NC evaluation-only. |
| Generated answer quality | **Unsupported.** The [availability receipt](results/2026-10-08-phase2-query-correctness/answerer-availability.json) found no configured answerer endpoint/model or priced-run setting. No external answerer or judge call was made; this campaign recorded zero paid calls and zero spend. | Reference-answer accuracy, evidence faithfulness and citation quality have no paired score. Checkpoint/resume, 429/5xx backoff, completeness and the separate $20 ceiling must be qualified with a selected provider before any priced run. A retrieval score is not an answer score. |

## Diagnosis and decision

There is no confirmed candidate-specific deterministic wrong result in the
supported contract, judged retrieval, evidence-set or scoped session cases.
The [IR diagnosis](results/2026-10-08-phase2-ir-relevance/diagnosis.json)
preserves the one required rank change and all four list differences. The
[vector diagnosis](results/2026-10-08-phase2-vector-fidelity-v2/diagnosis.json)
preserves every missed exact neighbor and the 14 candidate-worse queries.
The unequal index mean/layout states prevent attributing those differences
to a product change. A controlled mean-state or repeated-pair study is the
next diagnostic if a version-specific ANN claim is needed; a measured rank
change is not silently counted as a clean equality result.

Absolute quality gaps remain visible: QMSum's single required-document
Recall@10 was 0.4249 on both versions; MuSiQue complete support at 20 was
36/100, with only 1/25 four-hop cases complete; LOCOMO strict multi-session
coverage at 20 was 215/269. The [MuSiQue](results/2026-10-08-phase2-musique-support/diagnosis.json)
and [LOCOMO](results/2026-10-08-phase2-locomo-memory/diagnosis.json)
diagnoses enumerate shared incomplete cases. These are evidence-reachability
leads, not proof that all other returned documents are irrelevant and not
candidate regressions. No accepted ADR or public contract was changed.

The supported result is limited to the exact installed Python CPU model,
corpora, selections and default search configuration. The TypeScript and
Rust SDKs, CUDA/Jetson and other providers/models, crash-injected query
recovery, complete turn-level memory evidence and paid generated answers
were not qualified by these cells. Those omissions are explicit rather than
converted into passes or failures. Phase 1 performance and platform
follow-ups remain under the [plan's sequencing disposition](plan.md#phase-2-sequencing-decision-2026-10-08).

A later [candidate CPU–CUDA query sample](phase2-gpu-sample-2026-10-08.md)
used 10% stratified query IDs across these four retrieval corpora. Its
611/611 ordered hit lists and gold scores matched. This narrows the CUDA
omission for the named x86_64 candidate route only; the paired 0.8.26 GPU,
Jetson and other provider/platform cells remain unqualified.

## Verification and raw evidence

The new evidence/memory mapper, runner, scorer and auditor used failing-first
tests. The [pilot receipt](results/2026-10-08-phase2-support-feasibility/README.md)
records a wrong but corpus-valid hit lowering complete-set coverage and
rejection of unknown/out-of-scope hits, changed gold/source/wheel/database
bytes, omitted queries and incomplete pairs. The required full
`./scripts/agent-verify.sh` passed 186/186 suites with none skipped or
excluded and zero security violations, blockers or downgrades after the
executable changes. The scoped Markdown gate passed for the new reports.

Independent auditors re-read frozen source and derived hashes, checked
installed wheel/native bytes, inspected canonical SQLite rows and text/vector
index counts, and recomputed each per-case metric. The local
[MuSiQue](results/2026-10-08-phase2-musique-support/SHA256SUMS),
[LOCOMO](results/2026-10-08-phase2-locomo-memory/SHA256SUMS),
[IR](results/2026-10-08-phase2-ir-relevance/SHA256SUMS) and
[vector](results/2026-10-08-phase2-vector-fidelity-v2/SHA256SUMS)
checksums bind retained raw receipts. Preserve the licensed local raw bundle
before any worktree cleanup; do not commit corpus payloads or derived
verbatim text. The [retention receipt](results/2026-10-08-phase2-query-correctness/raw-retention.json)
records a verified local archive outside the worktree, SHA-256
`1817c5a961e9a4e419b78eaef63a9680e7ba2bbf821600f4322054b6b343bfdb`,
containing all eleven Phase 2 result directories, including invalid attempts.
The licensed source store remains separately pinned by the protocol hashes.
