---
title: 0.8.25 Slice 35 — eligibility and optional frozen reads
status: DRAFT
depends_on: 30
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 35 plan

## Outcome and carried obligations

Implement the eligibility-first core of R25/AC25-35; Memex needs 7/8 and the
contract half of 21; and A25-01/A25-05/A25-06 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Apply indexed,
allowlisted eligibility before lexical, vector, or graph truncation and offer a
compact Engine-minted frozen read context when requested. Ordinary reads do
not require it; full lease/retention machinery is allocated to 0.8.27.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
GPU/CUDA because dense eligibility is affected, and registry-installed search
smokes. Operator and live-model are N/A.

## Draft-to-ready and delivery

Define optional snapshot, mutation/validity race, unsupported predicate,
native query-plan, all-arm equivalence, parity, Windows, and CUDA criteria;
design without mandatory snapshot overhead or a permanently held SQLite
transaction; review;
implement RED/GREEN real-database/property tests; review; verify; and record
status. Stop on post-truncation filtering or ambiguous snapshot boundaries.
