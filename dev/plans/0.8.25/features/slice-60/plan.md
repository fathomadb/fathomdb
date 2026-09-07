---
title: 0.8.25 Slice 60 — minimal constrained graph parity
status: READY
depends_on: 55
design: design.md
design_status: READY_REVIEW_PASS_CYCLE_4
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

Cycle 3 found one P1 and two P2s after FIX-2. Design v5/FIX-3 restores the
shipped edge rule (`t_invalid` gates while `t_valid` is provenance), consistently
limits `include_out_of_window` to node validity, maps every malformed native
response to the exact graph-expansion exception/code/reason/path, and confines
the byte-identity oracle to edge-only insertion permutations over fixed node
rows/cursors and explicit seeds. The fourth independent review—the final review
under the four-cycle cap—returned READY with no findings.
Then commit RED, freeze tests through GREEN, obtain independent implementation
review and verification, and record closure. Stop on an ignored constraint,
client-side/post-cap filtering, partial success at the work bound, unbounded
expansion, a new migration without reviewed evidence, or reintroduction of the
rejected exact-anchor treatment.
