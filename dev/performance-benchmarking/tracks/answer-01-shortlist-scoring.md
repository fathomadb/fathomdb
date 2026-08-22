# ANSWER-01 — LOCOMO shortlist answer scoring

**Status:** complete; retain A0 for downstream work.

## Decision

Does `hybrid_ce_alpha_10_pool_20` improve answer accuracy and temporal
correctness over A0 while keeping answers grounded and attributable to the
retrieved sources?

## Draft plan

1. Fix 32 questions and two arms: A0 and
   `hybrid_ce_alpha_10_pool_20`. Dry-run both with stubbed answerer and judge;
   make no model calls.
2. After live-run approval, score the same arms through Airlock with `gpt-5.4`
   as answerer and `gemini-3.1-flash-lite` as judge. Use one worker, one retry,
   and checkpoint every question. The cumulative cap is $3 at 32 questions and
   $8 at 100.
3. Report paired answer quality, groundedness, and source attribution overall
   and separately for factoid, temporal, and multi-session questions.
4. Advance the retrieval winner only if paired overall answer quality improves,
   temporal quality does not regress, and no material grounding or attribution
   regression is hidden by the aggregate. Otherwise retain A0 or report
   inconclusive evidence.

## Stop

Stop at the cap, on an incomplete pair, or after repeated route failure. Do not
start MEMORY-01 without a declared answer-scored profile.

## Result

- [$0 dry-run receipt](../../../experiments/runs/answer-01-shortlist-dry-run-20260822T1222Z-8a050808/record.json)
- [Live scoring receipt](../../../experiments/runs/answer-01-shortlist-live-20260822T1234Z-8a050808/record.json)
- The 32-question live run did not demonstrate the required answer-quality or
  temporal improvement for `hybrid_ce_alpha_10_pool_20`; retain A0.
