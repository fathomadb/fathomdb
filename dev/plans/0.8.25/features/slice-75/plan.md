---
title: 0.8.25 Slice 75 — integrated closure
status: DRAFT
depends_on: 70
design: design.md
design_status: REVIEWED_BLOCKED_ON_SLICE_7
---

# Slice 75 plan

## Outcome and carried obligations

Implement R25/AC25-75; the measurement half of Memex need 21, needs 22 and 24,
and the integrated audit portion of need 23. Audit—do not backfill—A25-01
through A25-07, feature-local parity, wire compatibility, Windows CPU/native
evidence, snapshots, lifecycle, performance, resources, installed artifacts,
and retrieval-only evaluation.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, GPU/CUDA, registry-installed Python/npm/native/CLI, native
`Engine.search` global witness, and every live-model route accepted by Slices
65/70. Windows CUDA is N/A and remains deferred.

## Draft-to-ready and delivery

Define receipt-presence and agreement criteria, installed wire fixtures,
cold/steady concurrency, evidence/page/dependency overhead, mutation-to-ready,
erasure propagation, rebuild/storage/resource, and classification gates;
design the workload matrix without semantic answer claims; review; implement
RED/GREEN harness checks; review; execute all selected routes; and record final
release evidence. Stop when an owning slice lacks proof or a data-plane claim
mixes answer-system metrics.
