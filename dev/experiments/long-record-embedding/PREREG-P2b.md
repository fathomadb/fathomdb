# Preregistration amendment — P2b

Written 2026-10-18, after P2 failed its own validity check and before any P2b
run. `PREREG-P2.md` stands except where this file amends it.

## What failed in P2, and why

P2's text arm agreed between stores on 85.4 % of questions, below the 98 %
bar (`results/probe2_summary.json`), so P2 was invalid and no rule was
applied. The cause is not positional:

- LongMemEval-S holds 640 groups of byte-identical sessions under different
  ids, 1,371 sessions in all. 241 of P2's 554 gold sessions have a twin, in
  168 of its 315 questions (`results/diag_twins.json`).
- Fusion sums the RRF terms of hits with equal bodies
  (`fathomdb-engine/src/fusion.rs:99-111`). So twins score about double,
  and the merged hit carries the first twin's `source_id`.
- P2 rotated a gold session but not its twins. That broke the byte equality,
  and the pair stopped merging. Raw `bm25()` was identical on both stores
  (`diag_bm25.py`).

## Amendments

1. **Twins move together.** Every session whose original body equals that of
   a rotated gold session gets the same reordering, so twin groups stay
   byte-identical in the rotated store. A twin group spanning both strata
   excludes its questions, as in P2's cross-stratum rule.
2. **Twins count as gold.** A hit counts as a gold session if its id is a
   gold session id or the id of a body-identical twin of one. This applies to
   both stores and all arms.

Strata, arms, estimator, validity check and decision rule are unchanged.
