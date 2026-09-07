---
title: 0.8.25 Slice 60 design review — cycle 1
status: NOT_READY
review_cycle: 1
candidate: a823e2b7e9208951ac239422d72172f32080fcf1
---

# Slice 60 design review — cycle 1

## Verdict

**FAIL.** The scope is appropriately narrow, but the draft is not executable as
a cross-language public contract. Six P1 findings and one acceptance-blocking P2
finding require FIX-1 before implementation begins. No files were changed by the
independent reviewer.

## P1 findings

1. **The additive public and wire contract is unspecified.** The pseudocode does
   not define exact Rust, Python, or TypeScript APIs and types; target and
   explanation shapes; origin cardinality; JSON casing and integer encoding;
   closed-request/open-response decoding; or the exact error envelope,
   validation precedence, and FFI mappings.
2. **Current and frozen read contexts are contradictory.** The request carries
   only `ReadContextV1`, but execution promises both an unfrozen one-call
   transaction and authenticated `FrozenReadContextV1` reproduction. FIX-1 must
   define an exact closed context union or separate exact methods, including
   authentication and validation precedence, existence relaxation, and the
   transaction boundary.
3. **Query-seed and result identity semantics are undefined.** FIX-1 must define
   native logical-node seeding before `ranked_limit`, fixed search options and
   seed ordinals, projection fallback, duplicate/nonlogical/empty handling,
   all-or-nothing explicit visibility, seed inclusion or exclusion, and the
   exact `max_depth = 0` result.
4. **Work accounting and complete top-N are not executable.** FIX-1 must define
   what consumes work, the exact `W`/`W+1` failure rule and metadata, exhaustive
   frontier completion, result-limit behavior, total SQL/order keys, complexity
   and memory limits, and whether a compound persisted index is required.
5. **Kind, direction, cycle, and origin semantics are incomplete.** FIX-1 must
   decide open-string versus registered kinds; define empty/duplicate grammar,
   intermediate versus return-only filters, node and edge liveness, visited
   scope, seed/self-loop policy, actual terminal traversal direction, parallel
   edges, and a total origin tie-break.
6. **Explanation and projection behavior is not a contract.** FIX-1 must define
   an exact versioned graph-explanation sidecar and association, explicit- and
   query-seed contents, correlation/origin/lifecycle/dependency and projection-
   generation fields, plus the hard-failure versus soft-degradation matrix.

## Acceptance-blocking P2 finding

The verification plan lacks codec/property and malformed-input matrices,
deterministic race rendezvous, high-degree `W`/`W+1` fixtures, exact query-plan
assertions, coherent feature-specific CPU/Metal/CUDA routes, fresh local
wheel/packed-Node/native-module proof on Linux and Windows, and an explicit
schema-33 no-migration decision or additive migration/index proof.

## P3 finding

The design header still says `BLOCKED_ON_SLICE_7`, although Slice 7 and Slice 55
are complete. Readiness metadata must be corrected only after the substantive
findings close and a later independent review returns READY.

## Required disposition

FIX-1 may refine the design and plan only. It must not begin source or test
implementation. A second independent design-review cycle is required.
