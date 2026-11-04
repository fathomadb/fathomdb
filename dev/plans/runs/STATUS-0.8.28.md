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

<!-- BEGIN GENERATED release-state:0.8.28:status-current-state -->**Next is Slice 90 (FILTERED-BODY-FTS), NOT_STARTED.** Completed on local `release/0.8.28` per release state: 30 (`8d53418d1493090502501180d9223172ae95eaba`) · 31 (`1ceefddd894c041b0a8622856dc2677be3e49c88`) · 40 (`869d6a9e100d40841efc5cb56688040f4b645e42`) · 41 (`0ec01bc22197c641566830ef7b5ee6f446994f21`) · 42 (`fdbb74e109f02cbda7b8b8ff575be90e011b2803`) · 50 (`ecea4a1e79afe4cf7f5583914d405806767474ff`) · 60 (`e240efd4b91b8e3ba4ea53dc3edea73639f44747`) · 70 (`398d5c1edaf58d0d2dd68ac13136f3939130059a`) · 80 (`d641f43f73d0576fc24af28c80d5be199fc5332f`) · 81 (`7ca1d182bef03c69199f538904ce8332842d80a2`) · 82 (`ab046b2167c64b5edc3109c0d9cd087ec4ef8487`) · 83 (`af84f12f82d092a00a8d14f3fe657eff79bf0b4f`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.28:status-current-state -->

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

Slice 60 is complete at qualified candidate
`e240efd4b91b8e3ba4ea53dc3edea73639f44747`; see its
[status](../0.8.28/features/slice-60/status.md) for accepted format 35,
Astra reviews, independent checks, scale and GPU recall qualification.

## Immediate next action

| Step | Action |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.28:status-next-action -->**Commission Slice 90 (FILTERED-BODY-FTS)** — D28-14: SearchFilter on direct node and edge body FTS-only search (search_text_only / searchTextOnly), applied before truncation, ranking and limit. **Remaining ladder:** 90.<!-- END GENERATED release-state:0.8.28:status-next-action --> |

Slice 31's qualification is complete: the Orin windows P, A, B and C, the
RTX 3090 cells and the two GCP sessions pass, with G8 accepted as measured
(Q39); R-CODE-3 and R-VERIFY findings are fixed.

## Open decisions

There are <!-- BEGIN GENERATED release-state:0.8.28:status-live-open-count -->TWO<!-- END GENERATED release-state:0.8.28:status-live-open-count --> live open decisions:

- authorize the `v*` tag push and publication after the ladder and
  release qualification pass.

Ruled on 2026-10-30 and 2026-10-31 (release state, `decisions.ruled`):

- **The AC-013 fix lands here.** It runs as Slices 42 to 80; Slice 42
  Unit B becomes the `rusqlite` 0.41.1 bump.
- **D28-14** is Slice 90.
- **D28-09** moves to a 0.8.29 slice.
- **D28-10 and D28-01 to D28-07** go to the 0.8.29 candidate-selection
  review checkpoint.
- **D28-11** is not run for 0.8.28; the owner accepts the risk and a check
  goes in 0.8.30.
- **D28-12** is met for vector and hybrid latency; populated-open latency
  moves to 0.8.29.
- **Execution.** 0.8.28 is a planned release. Slices 42 to 90 run
  serially; 0.8.29 starts when 0.8.28 enters final verification, and
  `dev/0.8.28` is merged forward then.
- **B2-8 storage** goes in Slice 60's format 35: a per-vector span table
  and an `Embedder` truncation method. The reporting is 0.8.29 Block 2.
- **Overflow chunking** stays parked, but format 35 and Slice 70 are built
  so it can be enabled without another format break.

The owner approved both Slice 31 GCP sessions (S-13 and S-C3) at seq-332.

## Ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 30 | Tegra private CUDA memory pool (D28-08) | Complete on `release/0.8.28` at `8d53418d1493090502501180d9223172ae95eaba`; [status](../0.8.28/features/slice-30/status.md). |
| 31 | cudarc and Candle forks from crates.io (D28-13) | Complete on `release/0.8.28` at `1ceefddd894c041b0a8622856dc2677be3e49c88`; R-CODE-4 fixes on `slice/0.8.28-31-post`; [status](../0.8.28/features/slice-31/status.md). |
| 40 | FTS ranking, latency and page-cache defects (COMP benchmarks) | Complete on `release/0.8.28` at `869d6a9e100d40841efc5cb56688040f4b645e42`; Unit I gate passed and ADR-0.8.28-sqlite-per-connection-page-cache accepted; [status](../0.8.28/features/slice-40/status.md). |
| 41 | SDK context pack, full-render override, configurable query stop words | Complete on `release/0.8.28` at `0ec01bc22197c641566830ef7b5ee6f446994f21`; [status](../0.8.28/features/slice-41/status.md). |
| 42 | slice135 descriptor check (Unit A); adopt `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1 and retire the vendored crate (Unit B, re-scoped 2026-10-31) | Complete on `dev/0.8.28` and `release/0.8.28` at `fdbb74e109f02cbda7b8b8ff575be90e011b2803`; SQLite 3.54.1 from the registry, Windows verbatim-path tests pass on the local VM; AC-013 still fails as at the base (Slices 50 to 90); [status](../0.8.28/features/slice-42/status.md). |
| 50 | Bounded pending-work scans; narrowed provider contract | Complete on `dev/0.8.28` at `ecea4a1e79afe4cf7f5583914d405806767474ff`; Astra reviews, paired scaling and independent verification pass with documented scoped repairs. [Status](../0.8.28/features/slice-50/status.md). |
| 60 | Store format 35: 64-row vec0 chunks, `search_index` rowid = `write_cursor`, refuse version-34 stores | Complete on `dev/0.8.28` and `release/0.8.28` at `e240efd4b91b8e3ba4ea53dc3edea73639f44747`; Astra reviews and independent verification pass; [status](../0.8.28/features/slice-60/status.md). |
| 70 | Vector eligibility inside vec0 (lifecycle and validity columns) | Complete on `dev/0.8.28` and `release/0.8.28` at `398d5c1edaf58d0d2dd68ac13136f3939130059a`; Astra reviews, independent verification and AC-013 pass; [status](../0.8.28/features/slice-70/status.md). |
| 80 | `include_superseded` and `include_inactive` on search | Complete on `dev/0.8.28` and `release/0.8.28` at `d641f43f73d0576fc24af28c80d5be199fc5332f`; Astra reviews, independent qualification and lifecycle/cost tests; [status](../0.8.28/features/slice-80/status.md). |
| 81 | Erasure availability POC; durable lazy deletion/SLA feasibility | COMPLETE at `7ca1d182bef03c69199f538904ce8332842d80a2`; bounded POC, production erasure/SLA unqualified. [Status](../0.8.28/features/slice-81/status.md). |
| 82 | CH1 Phase 1: physical correctness and native cost | Complete on `dev/0.8.28` and `release/0.8.28` at `ab046b2167c64b5edc3109c0d9cd087ec4ef8487`; Astra reviews, independent verification and paired cost qualification. Property baseline is an accepted incomplete supplemental measurement. [Status](../0.8.28/features/slice-82/status.md). |
| 83 | CH1 Phase 2: resumable cleanup and availability | Engineering complete at `af84f12f82d092a00a8d14f3fe657eff79bf0b4f`; Astra reviews and independent qualification. Loaded throughput regresses; no new API/SLA. End-of-phase HITL guarantee decision remains open. [Status](../0.8.28/features/slice-83/status.md). |
| 90 | D28-14 filtered direct body FTS | Not started; follows 83. |

Ladder remaining: 90. Slices 41 and 42 were added by owner rulings on
2026-10-16, Slices 50 to 90 on 2026-10-31, and the explicit Slice 81 POC
on 2026-11-03. CH1 Slices 82/83 were subsequently approved and allocated on
2026-11-03. Slice 81 remains complete; physical erasure remediation is outside
the POC and now belongs to CH1. The guarantee/API decision stays at the end of 83.

## Boundaries

- Schema is 35 after Slice 60 with no upgrade path: a version-34 store
  is refused at open (owner 2026-10-31).
- The `v*` tag push for 0.8.28 is not authorized.
- The forks' own crates are published under owner rulings of 2026-10-13;
  that is not a FathomDB release.
