# COMP-01 result — NFCorpus FTS, FathomDB 0.8.27 vs LanceDB 0.40.0

Run 2026-10-15 under the [side plan](plans/2026-10-15-comp-side-plan.md).
Host: AMD Ryzen Threadripper PRO 5945WX, 62 GiB, local NVMe, CPU only.
Python 3.12.3; FathomDB is the repo 0.8.27 build, and LanceDB is 0.40.0
with pyarrow 26.0.0.

## Method

- Inputs: BEIR NFCorpus, 3,633 documents (`title\n\ntext`), the 323
  `qrels/test.tsv` queries, and the official graded qrels. Converted SHA-256
  values: corpus `1895269a…`, queries `a352f9ad…`, gold `6b226d88…`.
- Arms:
  - FathomDB `Engine.search_text_only(limit=10)`, no embedder.
  - LanceDB native FTS index with library defaults (stemming, stop-word
    removal, ASCII folding) and `search(query_type="fts").limit(10)`.
  - Query translation is native to each product and is a named treatment
    difference.
- Four fresh databases per arm, serial, in `AB/BA/BA/AB` order. Each cell
  builds, closes, reopens, times the first query, runs one untimed warm pass,
  then times one steady pass in a seeded shuffled order. Timing runs from the
  API call to materialized IDs.
- Scoring: nDCG@10 with linear gain (pytrec_eval/BEIR convention),
  Recall@10 and MRR@10 with grade > 0. The primary score uses the unchanged
  official IDs. 121 queries have a relevant document whose body is
  duplicated (40 duplicate pairs) and are also reported separately.

## Results

Medians across the four repetitions. Ordered results were byte-identical
across all four repetitions on both arms.

| Metric | FathomDB | LanceDB |
| --- | ---: | ---: |
| nDCG@10 | **0.3087** | **0.3265** |
| nDCG@10, excluding duplicate-affected queries | 0.2803 | 0.2988 |
| Recall@10 | 0.1456 | 0.1529 |
| MRR@10 | 0.4943 | 0.5283 |
| Queries with no hits | 16 | 15 |
| Steady p50 (ms) | 1.19 | 1.54 |
| Steady p95 (ms) | 5.26 | 1.85 |
| Steady p99 (ms) | 9.0 | 2.0 |
| First query after reopen (ms) | 0.71 | 4.06 |
| Build to ready (s) | 0.44 | 0.31 |
| Disk (MiB) | 28.9 | 4.7 |
| Peak RSS of cell process (MiB) | 86.6 | 264.9 |

Per-repetition p50 ranges: FathomDB 1.16–1.35 ms, LanceDB 1.38–1.66 ms.
FathomDB p95 ranged 4.0–5.7 ms, LanceDB 1.6–2.1 ms.

## Interpretation

- **Quality: LanceDB is ahead by 0.018 nDCG@10.** The paired per-query
  difference is −0.0178, with a bootstrap 95% CI of −0.026 to −0.010 over
  323 queries. FathomDB wins 43 queries, loses 88, and ties 192. The gap
  holds with duplicate-affected queries excluded, so body deduplication is
  not the cause. Both arms stem (FathomDB FTS5 uses `porter unicode61
  remove_diacritics 2`), so the remaining candidates are query translation
  (FathomDB's short-token and stop-word removal) and BM25 scoring
  differences between FTS5 and Lance. The mechanism is untested.
- **Latency depends on query length.** FathomDB is faster at the median and
  on short queries: p50 is 0.68 ms on queries of ≤3 words (n=192), against
  LanceDB's 1.49 ms. It is slower on long queries: p50 is 2.93 ms on queries
  of ≥8 words (n=26), against LanceDB's 1.57 ms. That fits FathomDB's
  OR-of-terms query translation and explains its p95/p99 tail. LanceDB's
  latency is nearly flat across query lengths.
- **Resources.** FathomDB uses about 6× the disk (SQLite canonical store plus
  FTS5 and projection tables, against Lance's columnar file) and about a third
  of the RSS.
- Neither product's numbers here are vendor-published claims. Both arms ran
  locally on identical inputs.

## Follow-ups (not run)

- Quality-gap mechanism: compare the 88 FathomDB-loss queries' translated
  terms, and BM25 parameters, against LanceDB's.
- The long-query tail: measure latency against term count after stop-word
  removal.

## Artifacts

External, under `~/.cache/fathomdb-comp/`: `nfcorpus-input/` (converted
inputs and manifest), `comp01-run1/` (`manifest.json`, eight `cells/*.json`
with ordered IDs and per-query timings, `report.json`). Rerun with
`python -m experiments.comp_fts run --campaign comp01 --inputs … --out …`,
then `score`.
