# COMP-03 paired 250-question answer run, and the Slice 40 Unit I gate (2026-10-17)

## Answer run

Run `comp-03-hermes-answer-20261017T1216Z-3bd3546c`, config
`experiments/configs/comp-03/hermes-answer.250.json` (owner approval
2026-10-16). The attempt is `~/.cache/fathomdb-comp/comp03-run-250`.

- **Questions:** 250 LongMemEval-S questions in the full-500 type mix: the
  60 pinned answer-set questions plus 190 drawn per type with seed 250. 18
  of the 250 are abstention questions. The dataset reproduces the pinned
  selection.
- **Arms:** each arm keeps its own renderer, so this compares shipped paths,
  not stores.
  - FathomDB FTS: `pack_context` T5/A0/k50 over caller items, opened with the
    query stop words `had`, `long`, `suggest`.
  - LanceDB FTS: k = 10, the Slice 40 mix renderer (`render_evidence`,
    4000/30000, session expansion, chronological).
- **Reader and judge:** `openai/gpt-5.4` through Airlock, seed 0,
  temperature 0, as in the mix and M3 runs.
- **Reuse:** the pinned 60 reuse the M3 (`pa-m3`) and mix (`comp03-run-mix`)
  answers on each arm. Each reused question reproduced its evidence and both
  prompt digests. Those calls ($1.28 FathomDB, $1.11 LanceDB) are not part of
  this run's spend.
- **Cost:** $7.28 for the 190 new questions, against a ceiling of $11.50.

### Result

| Arm | Correct | Accuracy |
| --- | ---: | ---: |
| FathomDB FTS (pack T5/A0/k50) | 214 / 250 | 0.856 |
| LanceDB FTS (Slice 40 mix) | 196 / 250 | 0.784 |

FathomDB minus LanceDB is +0.072 (paired bootstrap 95% CI +0.028 to
+0.116, seed 0, 10,000 resamples). There are 34 discordant questions: 26
right only on FathomDB, 8 right only on LanceDB.

| Type | n | FathomDB | LanceDB | Difference (questions) |
| --- | ---: | ---: | ---: | ---: |
| multi-session | 67 | 55 (0.821) | 43 (0.642) | +12 |
| temporal-reasoning | 66 | 52 (0.788) | 48 (0.727) | +4 |
| knowledge-update | 39 | 32 (0.821) | 31 (0.795) | +1 |
| single-session-user | 35 | 34 (0.971) | 33 (0.943) | +1 |
| single-session-assistant | 28 | 28 (1.000) | 28 (1.000) | 0 |
| single-session-preference | 15 | 13 (0.867) | 13 (0.867) | 0 |

The gain is concentrated in multi-session and temporal-reasoning, where the
k = 50 retrieval and the full-tier user turns put more of the answer
sessions in the prompt. The two arms differ in retrieval depth, renderer and
store at once, so this run does not attribute the gain to any one of them.

## Slice 40 Unit I gate: PASS

Measured by the orchestrator at the Unit I head `d10ad65d5` against H3
`417eac27d`, both with the v2 fresh-open harness, on a quiet host. All four
clauses pass; the numbers are in Slice 40 test-plan § 3h.6.

1. Timing report: the max after/before ratio is 1.00 on the Defect C route
   and 1.02 on the cohort route (limit 1.15).
2. tc5-10k-latency: c01 p50 falls from 11.98 to 6.80 ms and c10 from 6.22
   to 4.05 ms; the other cells are unchanged within noise. Short results are
   equal.
3. COMP-02b (`filtered-10000.v2`): FathomDB c01 p50 falls from 12.12 to
   6.55 ms and c10 from 5.13 to 3.77 ms, with 0 violations and equal short
   results.
4. COMP-01 nDCG@10 and every COMP-03 sweep retrieval metric are identical at
   both commits. COMP-02a p95 roughly halves at 10k and 17k.

The Slice 40 baseline manifest takes the Unit I values for tc5-10k-latency
(receipt `comp-02b-latency-20261017T1509Z-2412d988`) and comp02b-10000
(receipt `comp-02b-comp02b-10000-20261017T1510Z-19d8c470`), with
tolerances unchanged (AC40-104).
