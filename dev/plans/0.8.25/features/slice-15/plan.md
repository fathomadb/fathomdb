---
title: 0.8.25 Slice 15 — identity and canonical provenance
status: DRAFT
depends_on: 10
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 15 plan

## Outcome and carried obligations

Implement R25/AC25-15; Memex needs 1, 2, and the first part of 23; and A25-02
and A25-05. Add immutable record revisions, caller source-version identity,
UTF-8 byte locators, canonical hashes, Rust identity exports, versioned wire
evolution, and typed unknown-field behavior while preserving `IdSpace`.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and packaged SDK/CLI smokes. Operator, GPU/CUDA, live-model, and
pre-publication registry-installed are N/A. Name exact jobs, fixtures, and
receipt paths at readiness.

## Draft-to-ready and delivery

Write identity/provenance requirements and restart/reindex, corrupt locator,
hash mismatch, codec, migration, parity, and Windows criteria; design storage
and wire compatibility; review; implement with preserved RED/GREEN tests;
review; verify all selected routes; and record status. Stop on mutable revision
identity, ambiguous byte semantics, or incompatible wire evolution.
