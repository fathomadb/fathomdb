---
title: ADR-0.8.25-eligibility-and-frozen-reads
date: 2026-09-04
target_release: 0.8.25
status: accepted by approved 0.8.25 scope and Slice 35 execution authorization
supersedes: execution placement in ADR-0.8.11-filter-grammar-unification; vocabulary retained
---

# ADR-0.8.25 — eligibility before truncation and optional frozen reads

## Decision

FathomDB retains `SearchFilter` as its single ranked-search eligibility
vocabulary in 0.8.25. It supports the existing metadata fields and exact values
from declared `filterable` projections. It does not add range, set-membership,
existence, owner, scope, or arbitrary-JSON search terms without a native indexed
representation.

Every accepted predicate is applied before FTS ranking limits, vector KNN, and
search-graph seed/frontier caps. A backend that cannot honor canonical
lifecycle, validity, or dependency eligibility before a bounded stage declines
that whole arm with a typed soft fallback. Post-cap filtering is forbidden.

FathomDB also provides an optional, stateless frozen read context for the search
family. The Engine authenticates a content-free token that binds one resolved
validity instant, the complete read view and eligibility, the canonical write
boundary, a monotonic read-visibility generation, dependency generation, and
projection state digests. Consumption reproduces the exact current state or
fails typed; it does not retain or reconstruct historical serving state.

## Rationale

The shipped filter grammar already provides the high-value personal-agent
constraints, and declared exact projections let applications map owner/scope
without FathomDB owning their semantics. Widening the grammar before indexed
storage exists would create misleading APIs or client/post-KNN filtering.

A stateless reproduce-or-fail token provides deterministic multi-operation
coordination without holding SQLite readers, retaining snapshots, or adding a
lease service. A monotonic generation is necessary because write cursors alone
cannot detect in-place lifecycle and projection mutations.

## Consequences

- Existing `SearchFilter`, `Filter`, `Predicate`, and `ReadView` remain source
  and wire compatible.
- Ordinary reads keep the legacy dispatch and cost.
- Frozen contexts can become unavailable after any relevant mutation; this is a
  documented conservative outcome, not transparent advancement.
- Slice 40 may introduce projection generation identity, but must map existing
  frozen bindings exactly or return drift.
- Slice 50 and Slice 60 may add evidence and standalone-graph consumers without
  changing the token codec.
- Retained snapshot leases, generalized membership/existence, and semantic
  access-control policy remain outside 0.8.25.

The executable contract is the Slice 35 design at
`dev/plans/0.8.25/features/slice-35/design.md`.
