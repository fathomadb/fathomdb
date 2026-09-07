---
title: 0.8.25 Slice 60 design review — cycle 4
status: READY
review_cycle: 4
candidate: 601dfb568c55a26cc458b8626ff3a7989667f667
---

# Slice 60 design review — cycle 4

## Verdict

**PASS — READY.** No P0, P1, P2, or optional P3 findings remain. The exact
candidate was clean, and the independent reviewer changed no files or Git
state.

## Closure confirmed

- Edge admission matches shipped authority: it requires a nonsuperseded edge
  whose `t_invalid` is absent or later than the effective instant. `t_valid` is
  provenance and does not gate this operation.
- `include_out_of_window` consistently relaxes node validity only for explicit
  and query seeds, intermediate nodes, and returned targets; the temporal test
  matrix pins current and frozen behavior in every direction.
- Malformed native responses have exact Python and TypeScript exception class,
  code, reason, field path, message, and shared fixtures.
- The permutation oracle fixes node rows and cursors, uses explicit seeds and a
  fixed current instant, disables explanation, and permutes edge insertion only.
- The complete API and wire surface, transaction and authentication precedence,
  starvation-safe query fallback, traversal/liveness/origin ordering,
  `W`/`W+1`, bounded top-N, degradation composition, schema-33 index plan, and
  Linux/Windows TDD routes are implementable without unresolved judgment.

## Landing obligations

Implementation must update the governed public-interface surfaces in the same
change and retain the handoff's hard boundary: disposable local verification is
allowed, while release packaging, registries, tags, and publication are not.

The design is READY for RED implementation.
