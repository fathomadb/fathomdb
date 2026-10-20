# Preregistration — position-intervention probe (P2)

Written 2026-10-18, after P1 and its Fable-medium review, before any P2 run.
Not edited after the run; deviations go in `README.md` § Deviations.

## Why P2

P1 compared different questions (B: gold before the cut, P: gold past it).
The review showed that its vector effect, +0.137 pooled, falls to about +0.02
once computed within question type: the strata differ in type mix and in
gold-session count (`results/probe_exploratory.json`). P1's hybrid arm also
came from a `limit=100` call, not the shipped `limit=10` default. P2 removes
both problems by moving the gold, not by comparing questions.

## Intervention

Corpus, record unit and store setup are as in `PREREG.md`. A second store is
built with the same records, written in the same order, except in the gold
sessions of eligible questions, where the turns are reordered:

- **P questions** (all gold before the intervention lies past the cut): in
  each gold session, the `has_answer` turns move to the front, in their
  original order, followed by the other turns in their original order.
- **B questions** (all gold before the cut): the `has_answer` turns move to
  the end.

The multiset of lines in each body is unchanged, so FTS5 `bm25()` scores are
unchanged. Only what falls inside the vector's first 510 wordpieces changes.

**Eligibility** (computed with the pinned tokenizer before the run):

- After the intervention, every gold span of a P question ends at or before
  the cut, and every gold span of a B question starts at or after it.
  Otherwise the question is excluded.
- A question is excluded if any of its gold sessions is also a gold session
  of an eligible question in the other stratum.

## Arms, per question, on both stores

- `text`: `Engine.search_text_only(q, limit=10)`.
- `hybrid`: `Engine.search(q, limit=10)`, the shipped defaults.
- `vector`: `vector_rank` from `Engine.search(q, limit=100, explain=True)`.
  Questions with fewer than 10 vector-ranked hits are reported as
  unmeasured.

## Metric and estimator

`recall_any@10` over session ids. For each question and arm,
`d = recall(gold visible) − recall(gold hidden)`. "Gold visible" is the
rotated store for P and the original store for B; "gold hidden" is the
reverse. `D(arm)` is the mean of `d` over all eligible questions. Its 95 % CI
comes from 10,000 bootstrap resamples of questions, seed 20261018. The
standard error and the minimum detectable effect at 80 % power
(2.8 × SE) are reported for each arm. `D` is also reported by stratum and by
question type, as description only.

## Validity check

The text arm must agree between stores: `|D(text)| ≤ 0.01`, and at least 98 %
of questions must get the same text@10 outcome on both stores. If this fails,
P2 is reported as invalid and no rule below is applied.

## Decision rule

1. **Vector failure shown** if `D(vector) ≥ 0.10` and its CI excludes 0.
2. **Product failure shown** if `D(hybrid) ≥ 0.03` and its CI excludes 0.
3. If (2) does not hold and the MDE for hybrid is above 0.03, the result is
   reported as "not shown, underpowered", never as "no effect".
4. `D(hybrid)` is read as an upper bound for this embedder, record unit and
   fusion weights: it is the gain from putting the gold inside the one
   vector the record has. A chunk vector is a different unit, and a fix may
   retune fusion, so it is not a bound for those.
