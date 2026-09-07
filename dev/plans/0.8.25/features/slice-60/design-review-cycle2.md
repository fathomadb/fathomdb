---
title: 0.8.25 Slice 60 design review — cycle 2
status: NOT_READY
review_cycle: 2
candidate: 6e03dcdf09cf3fd7e6a1caa5af928a9ada9b2c09
---

# Slice 60 design review — cycle 2

## Verdict

**FAIL.** FIX-1 closes the broad Cycle 1 gaps, but two P1 and three P2
implementation-shaping findings remain. The exact candidate was clean and the
independent reviewer changed no files or Git state.

## P1 findings

1. **Logical-node eligibility remains post-KNN for vector query seeds.** The
   draft rejects edge and anonymous candidates only during hydration after the
   existing fixed-overfetch KNN cap. More than `TOP_K_BIT_CANDIDATES` nearer
   nonlogical rows can therefore starve a valid logical seed, contrary to the
   accepted constraint-before-cap rule. FIX-2 must define a pre-KNN proof of
   logical-node identity or decline the entire vector arm before KNN and use the
   specified text fallback. It must add a fixture with more than the fixed cap
   of nearer nonlogical rows.
2. **Temporal relaxation is incorrectly applied to edges.** The draft permits
   `include_out_of_window` to traverse out-of-window edges, but the accepted and
   shipped contract relaxes node validity only and always enforces edge recency.
   FIX-2 must retain one effective instant while excluding expired edges in all
   contexts and update the liveness matrix accordingly.

## P2 findings

1. **Rust constructability is not exact.** The draft conditionally mentions
   `#[non_exhaustive]` without assigning it per public type, and provides no
   constructor or builder for a potentially non-exhaustive request. FIX-2 must
   enumerate derives and exhaustiveness for every public type and leave request
   construction exact for external callers.
2. **Explanation bytes and projection-degradation composition are ambiguous.**
   FIX-2 must specify the existing correlation grammar/allocation and how
   deterministic response tests normalize it. It must also give an exhaustive
   projection-origin/readiness/fallback-to-code mapping, including ordered
   multi-code composition for `legacy_unverified + degraded`.
3. **Response-coherence validation is incomplete.** FIX-2 must require each
   origin `seed_ordinal` to reference an existing seed, its `seed_logical_id` to
   equal that seed's ID, and each explanation origin to equal the corresponding
   target origin. Cross-language malformed-response fixtures must pin these
   failures.

## Accepted areas

Once the five findings above close, depth zero, direction and `both`, loops,
cycles, parallel edges, origin ordering, `W`/`W+1`, exhaustive top-N, endpoint
query plans, schema 33 with no migration, cancellation-safe races, and the
Linux/Windows ephemeral verification boundary are sufficiently specified.

## Required disposition

FIX-2 is limited to the five findings above and must not widen scope or begin
source/test implementation. A third independent design-review cycle is
required.
