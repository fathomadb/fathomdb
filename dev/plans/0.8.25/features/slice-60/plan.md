---
title: 0.8.25 Slice 60 — minimal constrained graph parity
status: FIX2_AWAITING_REVIEW
depends_on: 55
design: design.md
design_status: FIX2_AWAITING_REVIEW
---

# Slice 60 plan

## Outcome and carried obligations

Implement the minimal subset of R25/AC25-60; Memex need 15 and graph-origin
portions of 12; and A25-05/A25-06 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Make combined
expansion honor query or explicit seeds, direction, edge kind, target kind,
indexed eligibility, bounds, and one read context with deterministic one-page
results. The additive `graph.expand`/`Engine::graph_expand` operation accepts a
closed current-or-frozen context union, returns resolved seeds and one compact
origin per target, and fails without a partial result at work row W+1. Rich
continuation and replayable full path evidence are allocated to 0.8.28.

## Verification routes

Selected: focused real-database Rust/Python/TypeScript and canonical-wire
property/malformed matrices; seed/direction/kind/liveness/race/W-and-W+1
fixtures; exact endpoint-index query plans; schema-33 no-migration proof; fast,
heavy, all, all-feature, and full-workspace gates; and fresh local
source-independent Rust/Python/Node smokes on Linux and Windows. GPU/CUDA/Metal
is N/A because query seeding must decline the vector arm before embedding/KNN
and cannot enter dense device dispatch.
Operator, live-model, registry, packaging, tag, publication, and
post-publication routes are N/A.

## Draft-to-ready and delivery

Cycle 2 found two P1s and three P2s after the broader FIX-1 closure. Design
v4/FIX-2 now declines vector query seeding before embedding/KNN because schema
33 cannot prove logical identity pre-KNN, makes temporal relaxation node-only,
pins every public Rust derive/exhaustiveness and external request construction,
fully specifies correlation and ordered projection-code composition, and closes
response origin/seed/explanation coherence. A third independent review must
return READY before implementation.
Then commit RED, freeze tests through GREEN, obtain independent implementation
review and verification, and record closure. Stop on an ignored constraint,
client-side/post-cap filtering, partial success at the work bound, unbounded
expansion, a new migration without reviewed evidence, or reintroduction of the
rejected exact-anchor treatment.
