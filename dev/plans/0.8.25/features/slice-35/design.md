---
title: 0.8.25 Slice 35 — eligibility and optional frozen-read design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 3
target_release: 0.8.25
depends_on: 30
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 35 design

## Authority and boundary

Implements the retained core of R25/AC25-35, Memex needs 7/8 and the contract
portion of 21, N25-02/N25-03, and A25-01/A25-05/A25-06. Eligibility is
mandatory before truncation. A frozen read context is opt-in and compact;
ordinary reads keep their existing behavior and cost.

Persisted leases, renewal/retention policy, and guaranteed long-lived replay
are allocated to the post-0.8.25 design notes.

## Eligibility contract

```text
EligibilityEnvelopeV1 { schema_version: 1, all: [EligibilityPredicateV1] }
EligibilityPredicateV1 =
  Eq { field, value }
  | Range { field, lower?, upper?, lower_inclusive, upper_inclusive }
  | AnyOf { field, values[1..256] }
  | Exists { field, exists }
ReadContextV1 {
  schema_version: 1, view: ReadView,
  eligibility: EligibilityEnvelopeV1,
  frozen: FrozenReadContextV1?
}
FrozenReadContextV1 {
  schema_version: 1, database_id, effective_valid_at,
  canonical_write_boundary, eligibility_sha256,
  projection_generation_ids, token
}
```

The closed field registry includes native owner, scope, artifact kind,
lifecycle status, and validity-time fields plus fields declared by an indexed
exact/range projection. Membership and existence are accepted only where the
declared index supports them. Predicates are parameterized implicit-AND,
canonicalized deterministically, and reject duplicate or contradictory forms.
No raw SQL or SDK-side filtering exists.

The Engine compiles the same envelope into canonical, FTS seed, vector
pre-KNN, and graph seed/frontier eligibility. Eligibility runs before every
candidate, seed, frontier, ranking, or result limit. Unsupported fields,
operators, types, or unindexed execution paths fail closed rather than
post-filtering.

## Optional frozen-read semantics

`freeze_read(view, eligibility)` reads one UTC instant, canonical boundary,
and serving projection generations in one transaction and returns an
authenticated self-contained context. The token contains no content and
requires no lease row. Subsequent context-bearing operations either reproduce
that boundary using retained immutable revisions/generations or return a typed
failure. They never silently advance to newer state.

The context does not promise retained history. A projection rebuild, pruned
revision, database mismatch, changed eligibility digest, or unavailable
boundary returns `frozen_read_unavailable`, `frozen_read_mismatch`, or
`frozen_read_drifted`. There is no TTL, renewal, pruning job, or permanently
held SQLite reader transaction in 0.8.25. Current access, erasure, and Slice 30
barriers are always rechecked and can only reduce visibility.

The context names at most 64 projection generations and its encoded token is at
most 4 KiB. A database with more applicable generations must use an ordinary
read or a narrower operation-specific context; overflow returns
`frozen_read_bound_exceeded`.

Existing methods synthesize an unfrozen `ReadContextV1`; new context-bearing
overloads are additive. Empty eligibility preserves previous SQL, ordering,
hit shape, and allocation behavior.

## Compatibility, failures, and verification

All types inherit Slice 15 wire/SDK rules. Failures are
`unsupported_eligibility`, `invalid_eligibility`,
`frozen_read_unavailable`, `frozen_read_mismatch`, and
`frozen_read_drifted`, plus `frozen_read_bound_exceeded`.

RED/GREEN tests place the only eligible row below every lexical/vector/graph
pre-filter cap, exercise all built-in and declared predicate forms, inspect
native query plans, and reject unsupported/post-filter-only execution. Race
tests prove a frozen operation reproduces its boundary or fails typed; current
erasure/access barriers still win. Default-cost tests prove unfrozen empty-
eligibility search is byte- and allocation-compatible.

Run fast, heavy, all/all-feature, Windows Rust/Python/Node, locally packed
search, and CUDA pre-KNN routes. Operator/live-model/pre-publication registry
routes are N/A. A formal independent READY review remains required after Slice
7 and Slice 30 complete.
