---
title: 0.8.25 Slice 60 design review — cycle 3
status: NOT_READY
review_cycle: 3
candidate: 41d351dbdcf6bbe9b6eaa37b56a42a4de4796260
---

# Slice 60 design review — cycle 3

## Verdict

**FAIL.** FIX-2 closes vector starvation, Rust constructability, correlation and
projection-code composition, and the required origin/seed/explanation
relations. One P1 and two P2 implementation-shaping findings remain. The exact
candidate was clean; the independent reviewer changed no files or Git state.

## Findings

1. **P1 — Edge temporal admission contradicts accepted and shipped authority.**
   The draft gates on `t_valid <= instant` and rejects not-yet-valid edges, but
   the accepted graph rule treats `t_valid` as provenance and admits an edge
   when it is nonsuperseded and `t_invalid` is absent or later than the effective
   instant. The same section promises node-only `include_out_of_window`
   relaxation while explicit seed and endpoint rules unconditionally require
   temporal eligibility. FIX-3 must pin the shipped edge rule, state that
   `t_valid` does not gate this operation, qualify seed/intermediate/output node
   validity by the effective `ReadView`, and correct the temporal test matrix.
2. **P2 — Malformed-response refusals lack an exact error mapping.** The
   coherence checks are specified but only called binding decode failures, while
   the error contract supplies no stable exception family/code/reason/path.
   FIX-3 must name the exact cross-language malformed-native-response mapping,
   reusing an existing family or defining a closed reason.
3. **P2 — The insertion-permutation oracle can be impossible.** Shuffling node
   writes changes exposed `write_cursor` values and may alter equal-score query
   seed ordering, so byte-identical complete responses are not a valid oracle.
   FIX-3 must constrain byte equality to edge insertion permutations with fixed
   node rows/cursors and explicit seeds, or define an exact normalized structural
   comparison.

## Required disposition

FIX-3 is limited to these three findings. It must not widen the public scope or
begin source/test implementation. Independent Cycle 4 is the final design
review allowed by the four-cycle cap.
