---
title: 0.8.25 Slice 71 — status
status: IN_PROGRESS_71B_COMPLETE
slice: 71
updated: 2026-09-09
---

# Slice 71 status

Slice 71B is complete at `eda95b07`. The fix coalesces visibility invalidation
per canonical/projection transaction, caches repeated write and projection
checks, reuses the writer connection for the final drain check, and increases
the bounded projection commit batch from 16 to 64. Persistent triggers and
external-SQL invalidation remain unchanged; custom main/TEMP triggers force the
original row-trigger path.

The performance limits were not changed. Corrected Scale-02 10k median
acknowledgement/total are 1,403.217/1,407.768 ms against historical
1,833.777/1,834.571 ms. Corrected projection-active AC-013 median total is
1,311.089 ms against historical 2,337.097 ms. Sizes 1, 10, 100, and 1,000 pass
their conjunctive small-write bounds. Exact repetitions and focused checks are
in [71b-performance-recovery.md](71b-performance-recovery.md).

Focused correctness tests and affected-crate check/clippy pass. Independent
code review passes on the exact candidate; the separate evidence audit passes
the retained prospective run. No broad verification round or AC-072 rerun
occurred. Slice 71 remains open solely for the separate AC-072 disposition;
Slice 72 remains dependency-blocked.
