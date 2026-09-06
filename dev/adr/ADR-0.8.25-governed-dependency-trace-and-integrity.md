---
title: ADR-0.8.25-governed-dependency-trace-and-integrity
date: 2026-09-06
target_release: 0.8.25
status: accepted by approved 0.8.25 scope and Slice 55 execution authorization
supersedes: dev/design/0.8.8-explain-and-telemetry-adr.md field-set evolution paragraph only
---

# ADR-0.8.25 — governed dependency trace and bounded integrity

## Decision

FathomDB adds one default-surface, one-page `trace_dependency` read over the
existing normalized direct-dependency chain. It authenticates a frozen read
context before disclosing root state, applies the same eligibility and
lifecycle authority to both endpoints, traverses exactly one hop, uses the
existing source-link index in either direction, and returns no continuation or
partial page. It adds no reverse table, reverse cache, migration, or durable
state.

The operator feature separately adds `check_data_plane_integrity` and
`doctor data-plane-integrity`. The operation classifies a caller-selected
closed set of dependency, searchable-projection, projection-generation, and
mutation-readiness authority under one deferred reader snapshot. Work and
findings are bounded; a cap-plus-one observation returns no partial report.
This surface is absent from Python and TypeScript.

## Explanation compatibility

Explained search additively returns one content-free structural classification
per visible hit and one Engine-minted correlation ID. Under the telemetry mutex,
an enabled sink remains the sole ID source and preserves `q{nonce}-{seq}`;
otherwise an independent Engine-open nonce and sequence mint `x...` without
writing telemetry. Ordinary search stays on its existing allocation-free
explanation-off path.

This supersedes only the older explanation field-set evolution paragraph.
Python retains old dataclass construction through defaulted tail fields;
TypeScript retains old object literals through optional tail properties. New
native responses always populate both fields.

## Boundary

Trace and explanation return no canonical body, query text, predicate,
source/dependency/operation natural key beyond the explicitly authorized trace
identities, locator, or hash. Integrity findings remain content-free. Existing
operator methods, default search, schema, projection generation, receipts, and
SDK doctor absence are unchanged.

The executable contract is
`dev/plans/0.8.25/features/slice-55/design.md`.
