---
title: 0.8.25 Slice 35 — frozen reads and eligibility
status: DRAFT
depends_on: 30
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 35 plan

## Outcome and carried obligations

Implement R25/AC25-35; Memex needs 7, 8, and the contract half of 21; and
A25-01, A25-05, and A25-06. Add Engine-minted frozen snapshots with typed
unavailable/drift/expiry outcomes and indexed allowlisted eligibility before
lexical, vector, or graph truncation.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
GPU/CUDA because dense eligibility is affected, and registry-installed search
smokes. Operator and live-model are N/A.

## Draft-to-ready and delivery

Define observable snapshot, mutation/validity race, unsupported predicate,
native query-plan, all-arm equivalence, parity, Windows, and CUDA criteria;
design without assuming a permanently held SQLite transaction; review;
implement RED/GREEN real-database/property tests; review; verify; and record
status. Stop on post-truncation filtering or ambiguous snapshot boundaries.
