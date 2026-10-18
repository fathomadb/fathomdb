# Preregistration — truncation probe (P1)

Written 2026-10-18, before any probe run. Not edited after the run; any
deviation is recorded in `REPORT.md` § Deviations.

## Question

Does the 512-wordpiece cut measurably hide gold evidence from the vector arm,
and does that leak through to hybrid (the shipped default) retrieval?

## Corpus and records

- LongMemEval-S cleaned (`longmemeval_s_cleaned.json`), all 500 questions.
- One record per unique haystack session (19,195), body built exactly as
  `src/python/eval/r2_parity_eval.py:980` `_format_lme_session`.
- One FathomDB store holding all sessions, default embedder (bge-small,
  `FATHOMDB_EMBED_DEVICE=cuda:0`), vectors drained to coverage 1.0.
- Retrieval is global over the store (a single memory store, as a consumer
  would run it), not restricted to the question's own haystack.

## Strata (fixed before the run, from `size_corpora.py` positions)

A gold span is a `has_answer` turn's line in its gold session body. The cut is
the end of content wordpiece 510.

- **B (before):** every gold span of the question ends at or before the cut.
- **P (past):** every gold span of the question starts at or after the cut.
- Questions with straddling or mixed spans, or with no `has_answer` turn, are
  excluded from the A/B comparison and reported separately.

## Arms

- `text`: `Engine.search_text_only`.
- `hybrid`: `Engine.search` defaults (`rerank_depth=0`, no graph arm).
- `vector`: the vector arm's own ranking, read from `explain=True`
  `vector_rank` on a `limit=100` hybrid call. Verified per query that at least
  10 hits carry a `vector_rank`; otherwise the query is reported as unmeasured.

## Metric

Session-level `recall_any@k`, k ∈ {5, 10}: 1 if any gold session is in the
top k. Gold sessions = `answer_session_ids`. Primary metric: R@10.

## Decision rule

Let `gap(arm) = R@10(B) − R@10(P)` for that arm. Text search sees the whole
body, so `gap(text)` measures stratum difficulty unrelated to the cut. The
truncation effect for an arm is `E(arm) = gap(arm) − gap(text)`. 95 % CIs by
10,000 paired-question bootstrap resamples within each stratum, seed 20261018.

1. **Vector failure shown** if `E(vector) ≥ 0.10` and its CI excludes 0.
2. **Product failure shown** if `E(hybrid) ≥ 0.03` and its CI excludes 0.
3. If (1) holds and (2) does not, the cut is a vector-arm defect that hybrid
   absorbs today. The recommendation then leans to a guard (hint or warning)
   plus EP-17 anchoring, unless a held-out answer-level loss is shown.
4. If neither holds, the probe does not show the failure, and the cut is
   reported as not material on this corpus.

## Not decided by this probe

Which fix to build. The probe sizes the harm; the option experiment in
`dev/design/long-record-embedding.md` § 7 decides promotion.
