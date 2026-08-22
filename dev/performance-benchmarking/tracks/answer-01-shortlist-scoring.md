# ANSWER-01 — LOCOMO shortlist answer scoring

**Status:** ready to implement and run the $0 dry run; live scoring is separate.

## Decision

Does `hybrid_ce_alpha_10_pool_20` improve answer accuracy and temporal
correctness over A0?

## Draft plan

1. Fix 32 questions and two arms: A0 and
   `hybrid_ce_alpha_10_pool_20`. Dry-run both with stubbed answerer and judge;
   make no model calls.
2. After live-run approval, score the same arms through Airlock with `gpt-5.4`
   as answerer and `gemini-3.1-flash-lite` as judge. Use one worker, one retry,
   and checkpoint every question. The cumulative cap is $3 at 32 questions and
   $8 at 100.
3. Advance the retrieval winner only if paired overall answer quality improves
   and temporal quality does not regress. Otherwise retain A0 or report
   inconclusive evidence.

## Stop

Stop at the cap, on an incomplete pair, or after repeated route failure. Do not
start MEMORY-01 without a declared answer-scored profile.
