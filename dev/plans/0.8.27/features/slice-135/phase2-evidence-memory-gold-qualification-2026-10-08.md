---
title: Slice 135 Phase 2 evidence and memory gold qualification — 2026-10-08
status: INPUT_MAPPING_CHECKED_UNSCORED
target_release: 0.8.27
---

# Phase 2 evidence and memory gold qualification — 2026-10-08

This read-only check identifies candidate evidence and memory denominators.
It does not report a retrieval, evidence-sufficiency, memory-usefulness or
answer-quality score for either installed product version. All corpus payloads
remain in the eval-only local data store, outside Git.

## LongMemEval

The local `longmemeval_oracle.json` SHA-256 is
`821a2034d219ab45846873dd14c14f12cfe7776e73527a483f9dac095d38620c`;
`longmemeval_s_cleaned.json` is
`d6f21ea9d60a0d56f34a05b609c79c88a451d2ae03597821ea3d5a9678c3a442`.
Both match `tests/corpus/scripts/manifest.json`. The local MIT license file
matches the manifest's SHA-256
`d3c4b9aa54759df6ded337978a6f3b55b75615e5e4525c3b82d7e2627d4b9732`;
the manifest still marks the dataset cache-only.

| Native question type | Questions |
| --- | ---: |
| Temporal reasoning | 133 |
| Multi-session | 133 |
| Knowledge update | 78 |
| Single-session user | 70 |
| Single-session assistant | 56 |
| Single-session preference | 30 |

All 500 question IDs are unique. Each gold `answer_session_ids` entry resolves
inside its own `haystack_session_ids`, and no case repeats a haystack session
ID. Session-level required-set coverage therefore has a 500-case candidate
population, subject to a frozen installed-write mapping. The `has_answer`
turn annotations cover at least one turn in 479 cases, but only 438 cases
have marked turns covering **every** named answer session. A claim requiring
complete turn-level evidence must use the 438-case subset unless the other
62 cases receive separately qualified gold. These are different denominators.

## LOCOMO

The local `locomo10.json` SHA-256 is
`79fa87e90f04081343b8c8debecb80a9a6842b76a7aa537dc9fdf651ea698ff4`.
Its local license file is CC-BY-NC 4.0, SHA-256
`41003d4a74749c0220e33dd415042164b5a1093ed401f36277234f772d22d3d0`.
Keep corpus rows and verbatim derived gold out of Git. The ten conversations
contain 1,986 QA entries. The existing `src/python/eval/locomo_loader.py`
resolves 272 session documents and 1,443 positive questions with nonempty
answers and valid session evidence: 841 factoid, 321 temporal and 281 labeled
multi-session. Twelve of the latter cite only one distinct session, leaving
269 candidates for a strict multi-session claim. Category 3 open-domain and
category 5 adversarial questions are excluded by that positive loader;
neither is silently counted as memory success or abstention.

## MuSiQue evidence sets

The local `musique_dev.jsonl` SHA-256 is
`3cff37fd7221506a343a125cf7ca20aab7cd09877e376122da9627e1b935b26f`.
It is the same pin used by the existing `src/python/eval/m1_baseline.py` and
prior local evaluations. The acquisition manifest instead records
`83e0e49856b21d9b8259efb39497ec03b7b08df15dd987c140c1692c56a1234d`.
That mismatch is unresolved: the manifest is **not** evidence that the local
MuSiQue bytes reproduce its acquisition record. Pin the local file explicitly
for any Phase 2 cell and retain the mismatch in its limitations. The manifest
records CC-BY-4.0 and cache-only distribution.

The local file has 4,834 rows: 2,417 question IDs, each paired once as
answerable and once as unanswerable. The paired rows share question and
answer text but differ in paragraph sets. Every answerable row has distinct
paragraph indices and a complete `is_supporting` set equal to its hop count:
1,252 two-hop, 760 three-hop and 405 four-hop. No unanswerable row has a
complete supporting set. Use the 2,417 answerable rows as the candidate
complete-support denominator; treat the unanswerable variants as separate
absence-of-complete-evidence controls only after their scoring rule is frozen.

## Before scored runs

Freeze exact source file hashes, row selection, canonical document IDs and
write mapping, query class, K, required-set rule, missing-result handling,
paired source/wheel identities and omission rule. Prove the evaluator rejects
missing supporting IDs, a partial multi-session set, an altered corpus hash,
an omitted case and an incomplete pair. A database hit count alone cannot
prove a complete support set or useful memory. Do not infer generated-answer
quality from these source labels; that remains a separately qualified external
consumer outcome under the plan's $20 ceiling and resume/backoff controls.
