---
title: 0.8.25 Slice 60 — constrained combined graph expansion
status: DRAFT
depends_on: 55
---

# Slice 60 plan

## Outcome and carried obligations

Implement R25/AC25-60; Memex need 15 and graph-path portions of 12; and A25-05
and A25-06. Extend combined expansion with query/explicit seeds, direction,
edge kind, target kind, indexed predicates, frozen snapshot, bounded
deterministic continuation, and exact seed/edge/path evidence.

## Verification routes

Selected: fast, heavy, all, all-feature, Windows CPU/native Rust/Python/Node,
GPU/CUDA because combined fused search can use dense/rerank arms, and
registry-installed graph/search smokes. Operator and live-model are N/A.

## Draft-to-ready and delivery

Define constraint-before-truncation, direction/kind matrix, deterministic page,
path lifecycle/evidence, fallback, parity, Windows, and CUDA criteria; design
against existing BFS/graph-arm contracts; review; implement RED/GREEN matrix
tests; review; verify; and record status. Stop on ignored constraints,
unbounded expansion, or reintroduction of rejected exact-anchor treatment.
