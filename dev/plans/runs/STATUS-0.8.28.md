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
Slice 31 (cudarc and Candle forks) is in progress, fork work first:

- `cudarc-fathomdb` 0.19.10 is on crates.io, published on 2026-10-13 from
  the qualified fork commit `b777f8d`. The registry `.crate` matches the
  dry run byte for byte, and its sources match `b777f8d`
  (`coreyt/cudarc` `fathomdb/qualification/2026-10-13-FC10-publish-b777f8d.md`).
- The Candle fork is qualified at `853324fc` (K-G1 RTX 3090, K-G2 GCP T4
  with CUDA 13.0, K-G3 Orin).
- FathomDB still builds against the vendored `third_party/cudarc-0.19.7`.

## Immediate next action

1. Candle FK5: K9 moves the three Candle crates to 0.11.0 on registry
   `cudarc-fathomdb` 0.19.10 (re-qualified on the RTX 3090 and the Orin
   on 2026-10-13); R-FK-PUB re-check; publish the three crates in order.
2. Slice 31 W1 to W3: requirements, design and test plan, each with its own
   Opus review.
3. Slice 31 W6: pin the registry crates, delete the vendored cudarc, and run
   the full gate and the GPU regression cells.

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
