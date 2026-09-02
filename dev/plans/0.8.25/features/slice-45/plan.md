---
title: 0.8.25 Slice 45 — governed pagination and operational state
status: DRAFT
depends_on: 40
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 45 plan

## Outcome and carried obligations

Implement R25/AC25-45; Memex need 9 and the current-state integration need; and
A25-03 and A25-05. Add opaque ordered cursors bound to request, snapshot,
projection generation, and ordering for canonical, graph, and governed
`operational_state` point/page reads. `latest_state` remains a consumer concept;
ranked top-K remains distinct.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
and registry-installed pagination smokes. Operator, GPU/CUDA, and live-model
are N/A.

## Draft-to-ready and delivery

Define cursor mismatch/expiry/drift, duplicate/omission race, point/page,
replacement, codec, parity, and Windows criteria; design opaque continuation
and snapshot/generation binding; review; implement RED/GREEN property and
real-database tests; review; verify; and record status. Stop on unstable pages,
cursor leakage, or a new `latest_state` storage authority.
