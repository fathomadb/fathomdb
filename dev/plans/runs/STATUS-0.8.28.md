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

**Slices 40, 41 and 42 are open.** Completed on `release/0.8.28`: 30 (`8d53418d1493090502501180d9223172ae95eaba`) · 31 (`1ceefddd894c041b0a8622856dc2677be3e49c88`).

Slice 30 (Tegra private CUDA memory pool) is complete on `release/0.8.28`.
Slice 31 (cudarc and Candle forks) is complete on `release/0.8.28`
(qualification: `dev/plans/0.8.28/features/slice-31/qualification.md`):

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
| **Immediate next action** | **Continue Slice 40; then land Slice 41, then Slice 42.** Then the open decisions below, then publication of 0.8.28 (Slice 31 plan W8, then R-CODE-4). |

Slice 31's qualification is complete: the Orin windows P, A, B and C, the
RTX 3090 cells and the two GCP sessions pass, with G8 accepted as measured
(Q39); R-CODE-3 and R-VERIFY findings are fixed.

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
| 31 | cudarc and Candle forks from crates.io (D28-13) | Complete on `release/0.8.28` at `1ceefddd894c041b0a8622856dc2677be3e49c88`; [status](../0.8.28/features/slice-31/status.md). |
| 40 | FTS ranking, latency and page-cache defects (COMP benchmarks) | In progress on `slice/0.8.28-40-fts-ranking-defects`, not landed; [status](../0.8.28/features/slice-40/status.md). |
| 41 | SDK context pack, full-render override, configurable query stop words | Planned on `slice/0.8.28-41-sdk-evidence-helper` (docs there; reserved here). Depends on Slice 40; lands before Slice 42. |
| 42 | slice135 descriptor check (todos ledger seq 312, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`); published-Rust page-cache investigation (todos ledger seq 313, `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`) | Planned; plan review R1 APPROVE-WITH-FIXES applied; [plan](../0.8.28/features/slice-42/plan.md). Depends on Slice 40; lands after Slice 41. |

Ladder remaining: 40, 41, 42. Slices 41 and 42 were added by owner rulings on 2026-10-16; 41 lands before 42.

## Boundaries

- Schema stays at 34; no migration is proposed.
- Publication of 0.8.28 is not authorized.
- The forks' own crates are published under owner rulings of 2026-10-13;
  that is not a FathomDB release.
