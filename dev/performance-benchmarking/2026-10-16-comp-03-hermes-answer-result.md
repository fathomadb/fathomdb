# COMP-03 result — Hermes-style LongMemEval-S answers, FathomDB 0.8.27 vs LanceDB 0.40.0

Run 2026-10-16 under the [side plan](plans/2026-10-15-comp-side-plan.md).
Config: [`hermes-answer.v1.json`](../../experiments/configs/comp-03/hermes-answer.v1.json).
Receipt: `experiments/runs/comp-03-hermes-answer-20261016T1239Z-2e410a4f/`.

## Method

- **Questions:** 60 LongMemEval-S questions, ten per type. They were picked
  with seed 303, sorted within each type, and the IDs are pinned in the
  config. The run checks the dataset digest and confirms that the seed
  regenerates the same IDs. Four of the 60 are abstention questions.
- **Stores:** each question gets its own fresh store per arm, built from the
  same history.
  - The document unit is a round: one user turn plus the assistant turns
    that follow it. The median store has 250 rounds.
  - Round IDs come from session and round position. Session IDs, answers and
    `has_answer` flags are never read when indexing. LongMemEval names gold
    sessions `answer_*`, so using its session IDs would leak the gold.
  - A test changes every gold field and checks that the indexed documents,
    the rendered evidence and the answer prompt stay byte-identical.
- **Arms:**
  - FathomDB: `Engine.search_text_only`, no embedder. Each database is
    created through `fathomdb_test_setup`, and `doctor check-integrity`
    reported clean every time.
  - LanceDB: native FTS index with library defaults.
  - Both retrieve the top 5 for the verbatim question. Arm order alternates
    by question.
- **Evidence:** both arms go through the same renderer. Each round is
  prefixed with its session date and capped at 4,000 characters, with a
  20,000-character cap per question.
- **Models:** both run through Airlock at temperature 0, seed 0.
  - Answerer: `openai/gpt-5.4`, with the LongMemEval reading prompt plus the
    question date.
  - Judge: `openai/gpt-5.4`, with LongMemEval's per-type answer-check
    prompts. It sees only the question, the gold and the response, never
    the product.
  - The Hermes reference used gpt-5.4 for answers and gpt-5.4-mini as the
    judge. Using the stronger judge was the owner's decision.
- **Passes:** one deterministic pass. The answers and the judge call are
  network time and are recorded separately from retrieval time.

## Results

All 60 questions completed for both arms. Total cost was $1.01.

| Metric | FathomDB | LanceDB |
| --- | ---: | ---: |
| Answer accuracy (60 questions) | **0.700** (42) | **0.700** (42) |
| Session Recall@5, any (56 non-abstention) | 0.893 | 0.893 |
| Session MRR@5 | 0.848 | 0.815 |
| Turn Recall@5, any (`has_answer` round) | 0.839 | 0.839 |
| Turn MRR@5 | 0.627 | 0.601 |
| Retrieval, first query p50 (ms) | 1.47 | 4.96 |
| Retrieval, warm p50 / p95 (ms) | 1.45 / 2.66 | 1.77 / 3.35 |
| Store build p50 (s) | 0.038 | 0.039 |
| Answer call p50 (s) | 1.23 | 1.20 |
| Judge call p50 (s) | 0.71 | 0.73 |

- **Paired answer difference:** 0.000, bootstrap 95% −0.050 to +0.050. There
  are only two discordant questions, one won by each arm.
- **Per type:** correct answers out of ten:

  | Type | FathomDB | LanceDB |
  | --- | ---: | ---: |
  | single-session-assistant | 10 | 10 |
  | single-session-user | 9 | 9 |
  | temporal-reasoning | 9 | 9 |
  | knowledge-update | 6 | 6 |
  | multi-session | 5 | 4 |
  | single-session-preference | 3 | 4 |

  The Wilson intervals at n = 10 are wide; see the receipt.

## Interpretation

- **Answer accuracy: parity.** The two arms retrieved the same five rounds
  in 37 of 60 questions and in the same order in 25. They fail on the same
  questions.
- **Failures:** each arm has 18 wrong answers.
  - 6 are retrieval misses: no round from an answer session in the top 5.
    Five of the six, on both arms, are preference questions. Their wording
    shares few terms with the round that holds the preference.
  - 10 had answer-session evidence, but the answer was still wrong. These
    are mostly multi-session counts (4 for FathomDB, 5 for LanceDB) and
    knowledge updates (3 each).
  - 2 are missed abstentions.
  - Better FTS ranking would not move most of these. Retrieval depth,
    multi-hop aggregation, or vector and hybrid evidence would.
- **The two discordant questions:**
  - `09ba9854`: FathomDB answered from a dollar estimate, while LanceDB's
    evidence led to a yen range.
  - `1da05512`: LanceDB retrieved the round in which the user describes
    limited storage, which the preference rubric asks for.

  Both verdicts were checked by hand and hold.
- **Retrieval:** FathomDB ranks the gold round slightly higher (MRR +0.03 at
  session level, +0.03 at turn level) with identical recall. On a fresh
  store, FathomDB's first query is about 3× faster. Warm latency is similar.
- **Judge audit:** a fixed sample of 12 answers (seed 303) was read by hand,
  and all 12 verdicts agree with the rubric.
- **Context only:** LanceDB's published Hermes table reports 0.66 accuracy
  for `lancedb-vector` and 0.53 for `hermes-session-search`. It uses a
  different memory unit, prompt and judge, and its latency includes network
  embedding. Our 0.70 for both FTS arms is not a paired comparison with
  those rows.
- **Not covered:** vector, hybrid and reranked variants. FathomDB 0.8.27 has
  no public vector-only query.

## Reproduce

```bash
python -m experiments.comp03_answer select experiments/configs/comp-03/hermes-answer.v1.json --dataset <longmemeval_s_cleaned.json>
python -m experiments.comp03_answer run experiments/configs/comp-03/hermes-answer.v1.json \
  --dataset <longmemeval_s_cleaned.json> --attempt-root <new dir> --fathomdb-bin <fathomdb 0.8.27 CLI>
```

The Airlock key comes from `AIRLOCK_VIRTUAL_KEY` or `R2_JUDGE_API_KEY`. The run
resumes from its attempt directory; finished stages make no new calls.

The raw attempt is `~/.cache/fathomdb-comp/comp03-run1/`. It holds the resolved
config, and for every question and arm the retrieval IDs, timings, evidence
digest, answer call and judge call, plus `summary.json`. The receipt binds the
summary by digest.

The receipt has `dirty: true`. It ran from the uncommitted tree that the
commit adding this note records. The only later code change in that tree is
the `--config` option in `comp_fts`'s CLI, which COMP-03 does not use.
