---
title: 0.8.25 Slice 20 — dependency registration and liveness
status: DRAFT
depends_on: 15
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 20 plan

## Outcome and carried obligations

Implement R25/AC25-20; Memex needs 4 and 14; and A25-04 and A25-05. Add
queryable canonical-to-derived, derived-to-derived, and source-set dependency
identity with a bounded caller-declared liveness grammar. The Engine enforces
structure and lifecycle mechanics, never semantic truth.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and registry-installed public-contract smokes. Operator, GPU/CUDA, and
live-model are N/A unless the design changes an operator surface.

## Draft-to-ready and delivery

Define reference, cycle, lookup, source-removal, liveness, codec, parity, and
Windows criteria; design dependency persistence and bounded evaluation; review;
implement RED/GREEN property and real-database tests; review; verify selected
routes; and record status. Stop on unbounded liveness, shadow-index ownership,
or ambiguous source-removal consequences.
