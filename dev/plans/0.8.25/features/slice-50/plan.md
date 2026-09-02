---
title: 0.8.25 Slice 50 — source-complete evidence
status: DRAFT
depends_on: 45
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 50 plan

## Outcome and carried obligations

Implement the compact subset of R25/AC25-50; Memex needs 10/11 and its share of
12; and A25-02/A25-05/A25-07 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add opt-in compact
`EvidenceRef` creation and exact resolution under the original or equivalently
constrained eligibility envelope without bloating default `SearchHit`.
Persisted evidence leases and replayable receipt retention are allocated to
0.8.27.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and registry-installed evidence smokes. Operator, GPU/CUDA, and live-model are
N/A because resolution is storage/visibility work.

## Draft-to-ready and delivery

Define exact-byte/hash, current/superseded/inactive/invisible/erased/stale/
mismatched/unavailable, non-disclosure, compact-hit, codec, parity, and Windows
criteria; design compact resolver authorization without mandatory persisted
leases; review; implement preserved RED/GREEN
tests; review; verify; and record status. Stop on stale-reference disclosure or
mandatory per-hit expansion.
