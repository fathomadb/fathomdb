---
title: FathomDB 0.8.28 release status
status: ACTIVE
---

# FathomDB 0.8.28 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.28.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here. Started
2026-10-13 to track Slice 31.

## Current state

Slice 30 (Tegra private CUDA memory pool) is complete on `release/0.8.28`.
Slice 31 (cudarc and Candle forks) is in progress; the fork work is done:

- `cudarc-fathomdb` 0.19.10 is on crates.io, published on 2026-10-13 from
  the qualified fork commit `b777f8d` (`coreyt/cudarc`
  `fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md`).
- `candle-core-fathomdb`, `candle-nn-fathomdb` and
  `candle-transformers-fathomdb` 0.11.0 are on crates.io, published on
  2026-10-13 from Candle fork commit `182ceab0` (K9), re-qualified on the
  RTX 3090 and the Orin (`coreyt/candle-fathomdb`
  `fathomdb/qualification/2026-10-13-FK5-publish-182ceab0.md`). 0.11.0, not
  0.10.4, because the change replaces a re-exported dependency (seq-324).
- Each registry crate equals its dry run and its sources equal its commit.
- Slice 31 `requirements.md` is revised after R-REQ.
- FathomDB still builds against the vendored `third_party/cudarc-0.19.7`.

## Immediate next action

1. Slice 31 W2 design (R-DES) and W3 test plan (R-TEST), each with its own
   Opus review.
2. Slice 31 W6: pin registry `cudarc-fathomdb` `=0.19.10` and the Candle
   crates `=0.11.0`, patch `candle-kernels` by git at K9, delete the
   vendored cudarc, and run the full gate and the GPU regression cells.

## Open decisions

There are <!-- BEGIN GENERATED release-state:0.8.28:status-live-open-count -->FOUR<!-- END GENERATED release-state:0.8.28:status-live-open-count --> live open decisions:

- place D28-09 to D28-12 on the ladder, or move them;
- decide whether any D28-01 to D28-07 candidate enters 0.8.28;
- approve each GCP VM session for the Slice 31 CUDA 13.0 cells; and
- authorize tagging and publication after the ladder and qualification pass.

## Ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 30 | Tegra private CUDA memory pool (D28-08) | Complete on `release/0.8.28` at `8d53418d1493090502501180d9223172ae95eaba`; [status](../0.8.28/features/slice-30/status.md). |
| 31 | cudarc and Candle forks from crates.io (D28-13) | In progress; [plan](../0.8.28/features/slice-31/plan.md). Work in `slice/0.8.28-31-cudarc-fork`. |

Ladder remaining: 31 alone, until the open placement decisions add slices.

## Boundaries

- Schema stays at 34; no migration is proposed.
- Publication of 0.8.28 is not authorized.
- The forks' own crates are published under owner rulings of 2026-10-13;
  that is not a FathomDB release.
