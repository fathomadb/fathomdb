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

<!-- BEGIN GENERATED release-state:0.8.28:status-current-state -->**Next is Slice 31 (CUDARC-FORK), IN_PROGRESS.** Completed on local `release/0.8.28` per release state: 30 (`8d53418d1493090502501180d9223172ae95eaba`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.28:status-current-state -->

Slice 30 (Tegra private CUDA memory pool) is complete on `release/0.8.28`.
Slice 31 (cudarc and Candle forks) is in progress:

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
- W1 to W3 (requirements, design, test plan) are revised after their Opus
  reviews.
- W6 on windchill3 is done on `slice/0.8.28-31-cudarc-fork` at `48c2aa9bf`:
  FathomDB pins the registry fork crates (pin commit `aa898677f`), the
  vendored cudarc is deleted, and the feature-complete gate and the RTX 3090
  CUDA cells pass. The lock-set guard waits on the cudarc fork's records-only
  run.

## Immediate next action

| Step | Action |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.28:status-next-action -->**Continue Slice 31 (CUDARC-FORK)** — consume cudarc-fathomdb and the Candle fork from crates.io; delete the vendored cudarc (D28-13). **Remaining ladder:** 31.<!-- END GENERATED release-state:0.8.28:status-next-action --> |

Within Slice 31: the cudarc fork records the W6 lock set (FT-3); then the
Orin windows P, A, B and C; then the two owner-approved GCP sessions; then
R-CODE-3 and R-VERIFY.

## Open decisions

There are <!-- BEGIN GENERATED release-state:0.8.28:status-live-open-count -->FOUR<!-- END GENERATED release-state:0.8.28:status-live-open-count --> live open decisions:

- place D28-09 to D28-12 on the ladder, or move them;
- decide whether any D28-01 to D28-07 candidate enters 0.8.28;
- give D28-14 (filtered direct body FTS, placed in 0.8.28 at seq-333) a
  slice and authorize its implementation; and
- authorize tagging and publication after the ladder and qualification pass.

The owner approved both Slice 31 GCP sessions (S-13 and S-C3) at seq-332.

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
