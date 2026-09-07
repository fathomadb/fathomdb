---
title: 0.8.25 Slice 60 implementation review — cycle 1
status: FAIL
review_cycle: 1
candidate: e874fed10f1fcdcc3c3512632892fd4a0cd26201
---

# Slice 60 implementation review — cycle 1

## Verdict

**FAIL.** No P0 finding exists, and the traversal core is coherent for snapshot
pinning, authentication, all-or-nothing bounds, deterministic traversal and
origin selection, top-N-after-completion, edge liveness, and vector-arm
non-entry. Four P1 and three P2 findings require FIX-1. The independent review
was read-only and the exact candidate remained clean.

## Findings

1. **P1 — Rust canonical wire bytes use lexicographic map order.** The encoders
   convert structs through `serde_json::Value` without `preserve_order`, while
   Python and TypeScript preserve declaration order. The existing fixture test
   parses both sides through `Value` and masks the divergence. FIX-1 must
   serialize ordered wire structs directly and add raw canonical-byte parity
   oracles across bindings.
2. **P1 — Test rendezvous hooks ship in the default public engine.** Global,
   arbitrary closures are unconditionally re-exported and can block or panic an
   unrelated reader request without disarm or Drop release. FIX-1 must confine
   hook storage, firing, and exports to the non-shipped test feature and provide
   an owned bounded, cancellation-safe handle scoped to the intended call.
3. **P1 — Invalid inner strings bypass binding validation.** Python and
   TypeScript JSON-escape lone surrogates and embedded NUL before native outer-
   string validation; TypeScript also incompletely validates eligibility member
   types. FIX-1 must recursively validate before transport, return the specified
   binding-local error, and prove native is not invoked.
4. **P1 — Query-plan proof explains substitute SQL.** The test helper uses
   hard-coded indexed statements instead of the actual production traversal
   statement, especially for `both`, so production may scan while tests remain
   green. FIX-1 must share the production SQL/bind builder with EXPLAIN or
   explain the exact executed statements for every direction.
5. **P2 — Nested unknown-field precedence is wrong.** Rust and TypeScript read
   `seed.type` before union-wide unknown-field closure, contrary to schema then
   lexicographically first unknown then required/type precedence. FIX-1 must
   close the union object before discriminant validation and add missing/invalid
   discriminant parity cases with escaped JSON-pointer paths.
6. **P2 — Mandatory real-database and high-bound evidence is absent.** FIX-1
   must add exact 10,000/10,001 work-bound and proportional RSS evidence plus
   real dependency-closure, erasure, and projection-state fixtures rather than
   only a pure degradation helper.
7. **P2 — The public wire schema sentinel is stale.** `dev/interfaces/wire.md`
   still says schema 32 while code and Slice 60 require schema 33. FIX-1 must
   correct the sentinel and document Slice 60's no-migration relationship.

## Required FIX-1 discipline

Add new dedicated FIX-1 test files and commit their expected RED before product
changes. Do not modify the six frozen RED artifacts or their three audited
mechanical-correction commits. Then implement GREEN, update chronology and
governed surfaces, and obtain independent review cycle 2. Release packaging,
registry work, tags, and publication remain prohibited.
