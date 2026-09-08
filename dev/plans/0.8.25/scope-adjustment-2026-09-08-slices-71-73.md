---
title: 0.8.25 owner scope adjustment — Slices 71–73
status: APPROVED
date: 2026-09-08
authority: repository owner
---

# 0.8.25 owner scope adjustment — Slices 71–73

The active closing ladder is `60 -> 71 -> 72 -> 73 -> 75`. Slices 65 and 70
remain reallocated by the earlier owner adjustment and are not revived.

## Allocation

- **Slice 71:** AC-013 vector-latency investigation and the Slice 35 bulk-
  ingest visibility-trigger investigation.
- **Slice 72:** installed cross-encoder CPU/CUDA profile and the generic
  release-state-aware preflight correction.
- **Slice 73:** Windows Node/N-API CI coverage. The post-Slice-60 handoff at
  `88d21040` is an input to this split, not a second implementation scope.
- **Slice 75:** consumes the preceding receipts and performs integrated
  release closure, including the final full local matrix and exact-head hosted
  CI. It must not repeat the focused investigations or profiles above.

Slices 71–73 use only focused verification proportionate to their changes.
The full regression and heavy verification routes remain owned by Slice 75.

This adjustment does not authorize publication, registry mutation, tagging,
merging to `main`, or any other external release side effect.
