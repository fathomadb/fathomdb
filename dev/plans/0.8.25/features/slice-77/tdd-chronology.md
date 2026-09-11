# Slice 77 TDD chronology

No performance treatment qualified, so treatment-specific semantic RED/GREEN
was not applicable. The registered AC-020 assertion remained RED in all seven
anchor processes.

The evidence audit found one focused tooling defect: the collector summarized
integer millisecond markers even though failing raw logs already contained
full-precision durations.

1. **RED — `c8cd3873`:** added a focused test requiring the parser to prefer the
   existing full-precision failure diagnostic and preserve floats in summary
   statistics. The exact test failed because `540` was returned instead of
   `540.625`.
2. **GREEN — `fbfc6686`:** parse exactly one full-precision failure diagnostic
   when present, retain the integer marker fallback, reject duplicates, and
   remove integer coercion from summaries.
3. **Focused verification:** 8 runner tests pass. The seven observations and
   summary were regenerated from retained raw logs; no timing was rerun.

The registered performance oracle, fixture, and test source did not change.
