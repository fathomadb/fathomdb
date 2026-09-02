---
title: 0.8.25 Slice 45 — minimal pagination and operational state
status: DRAFT
depends_on: 40
design: design.md
design_status: REVIEWED_MAX_ENVELOPE_SCOPE_RECONCILIATION_REQUIRED
---

# Slice 45 plan

## Outcome and carried obligations

Implement the minimal subset of R25/AC25-45; Memex need 9 and the current-state
integration need; and A25-03/A25-05 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add bounded stable
continuation for canonical and governed `operational_state` reads plus point
reads. `latest_state` remains a consumer concept and ranked top-K remains
distinct. General graph pagination and full cursor-lease semantics are
allocated to 0.8.27.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and packaged pagination smokes. Operator, GPU/CUDA, live-model, and
pre-publication registry-installed are N/A.

## Draft-to-ready and delivery

Define request/order mismatch, duplicate/omission race, point/page,
replacement, codec, parity, and Windows criteria; design a compact opaque
continuation that can bind an optional Slice 35 read context; review; implement
RED/GREEN property and real-database tests; review; verify; and record status. Stop on unstable pages,
cursor leakage, or a new `latest_state` storage authority.
