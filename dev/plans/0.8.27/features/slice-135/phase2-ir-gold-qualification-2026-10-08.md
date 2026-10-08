---
title: Slice 135 Phase 2 IR gold qualification — 2026-10-08
status: INPUT_MAPPING_CHECKED_UNSCORED
target_release: 0.8.27
---

# Phase 2 IR gold qualification — 2026-10-08

This read-only check qualifies the locally available retrieval inputs for
protocol design. It does not score either product version or assert that the
installed SDK has ingested the corpus with the required document identities.

## Frozen input and mapping check

| Input | SHA-256 or pin | Local boundary |
| --- | --- | --- |
| Frozen snapshot `tests/corpus/snapshot.json` | `1f8eba4e908019a4b43eef867e1514a2e2d29163bea09de47c34f4d6766507ba`; declared corpus hash `fe973fcd49fbbda083158f69fe720f17858ab8528e171fa2188eec84131c7d4e` | Ten sources, 10,506 documents. |
| Acquisition manifest `tests/corpus/scripts/manifest.json` | `18a8715c48fdaf479196f0796506b1b61874ec6962318cf81bf88989aea36217` | Source file and license pins. |
| Local resolved gold `data/corpus-data/eval/ir_gold/all.gold.json` | `4caabddf7ce55f417e639e3c169fe2035b09c231f36d2f39d293a596373de2bb`; `ir-c-reused-v2` | Eval-only; no corpus payload is committed here. |
| EnronQA raw | `bc30eb0674eb72e6c762e094a514983f767e5122a6dd3a711325dd0fde5248ab` | 200 documents; cache-only under the manifest's undeclared-license note. |
| QAConv raw | `8c416c765134d971870df451eebb3246bbb79617014484e409e7d383b2bc5d1d` | 1,250 documents; manifest records BSD-3-Clause. |
| QMSum raw | `19a2e5b45174380b3bf46b2926b66500e121d2ac7e0679009957440b8f9f5e2e` | 600 documents; cache-only while the underlying transcript rights chain remains unverified. |

The three local raw file hashes match the manifest and snapshot. Every
positive gold `expected_top_k_doc_ids` and `required_evidence[].doc_id`
exists in its declared source file. Those two gold ID sets agree for every
query. All 4,597 query IDs are unique and all query texts are nonempty.

## Class denominators and claim limits

| Class | Queries | Source | Gold and permissible interpretation |
| --- | ---: | --- | --- |
| Exact fact | 2,888 | EnronQA 710; QAConv 2,178 | One named required document per query. Score required-document Recall@K and reciprocal rank if the installed write/read mapping preserves document IDs. |
| Exploratory | 1,584 | QMSum | One named required document per query. Score reachability of that required document separately; a single label is not a complete relevance set for general nDCG or precision. |
| Negative | 125 | QAConv | Empty required-evidence set and `answer_type=abstain`. Reserve for an independently qualified answer-abstention cell; an empty qrel set does not require the database search to return no hits. |

The 4,472 positive queries each name exactly one required document; the 125
negative queries name none. These are source-specific judgments, not a
complete relevance assessment over all 10,506 documents. Do not report
unqualified all-corpus nDCG@10 or treat one source's unlabeled documents as
judged irrelevant. Keep raw EnronQA and QMSum content and verbatim derived
gold outside Git under the manifest's distribution boundaries.

Before a scored paired retrieval run, freeze the exact per-source document
selection, canonical write mapping, model/configuration, source and installed
artifact hashes, query denominator, K values, handling of missing or failed
queries, and scoring formula. Verify a real-database pilot preserves every
gold document ID through write, search and reopen; prove the scorer rejects a
wrong ID, dropped query, altered input hash and incomplete paired receipt.
Run same-model exact-f32 vector fidelity as a separate dimension.
