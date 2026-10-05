---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon delivery
status: PLANNED
target_release: 0.8.27
planning_baseline: 6735fd9f1
---

# Slice 117 — Jetson CUDA Node addon delivery

This is a draft plan. It adds a release-pipeline slice so Jetson users can
obtain a CUDA-capable Node addon from a registry install instead of building
one locally. It is **not authorization to implement or publish**: it depends
on Slice 110 and on owner decisions listed below, and every publication step
remains behind the release's unruled `release-0.8.27-publication` decision.
The [design note](design.md) owns options, evidence and risks; the
[status](status.md) records lifecycle. The todos ledger tracks the work as
`TC-ffef2129-e5a6-4f8f-8cae-bdfe64792197`.

## Entry evidence

| Finding | Source |
| --- | --- |
| The published `fathomdb-linux-arm64-gnu` npm package is CPU-only: `build-napi` builds it on hosted `ubuntu-24.04-arm` in the `manylinux_2_28` container with `npm run build:native`, which enables only `default-embedder`. | `.github/workflows/release.yml`; `src/ts/package.json`; `src/ts/scripts/build-native.mjs` |
| The Linux x86_64 npm package and wheel are CUDA-capable (`embed-cuda,rerank-cuda`), built on the self-hosted x86_64 GPU runner. | `release.yml` job `build-cuda-linux-x64-gnu`; `scripts/release/cuda-preflight.sh`; `scripts/release/cuda-artifact-contract.sh` |
| Python on Jetson has a host-native `+tegra` wheel built and smoked on the self-hosted Jetson runner and optionally published to a first-party Pages index; the public docs serve exact `0.8.24+tegra`. | `scripts/release/build-python-cuda-tegra.sh`; `.github/workflows/jetson-tegra-cuda-evidence.yml`; `.github/workflows/docs-pages.yml`; `docs/install/python.md` |
| `scripts/release/build-napi-cuda.sh` builds only the x86_64 axis. No checked-in script builds a Tegra `.node`. | `scripts/release/build-napi-cuda.sh`; `cuda-artifact-contract.sh` comments |
| Slice 110 built a host-native `embed-cuda,rerank-cuda` addon on the Jetson AGX Orin for qualification and found intermittent forced-CUDA failures that its allocator fallback and early `cuInit` address. Per the owner's 2026-10-05 ruling, both stay in 0.8.27 as part of Slice 110. | `dev/plans/runs/0.8.27-slice-110-tegra/receipt.md`; branch `llm/slice110-tegra-allocator-fix` |
| D-80.7-3 rules npm permanently out of scope for Tegra and names the only safe shape if that is ever reversed. | `dev/design/0.8.23-aarch64-tegra.md`; `dev/tegra-platform-reference.md` § 3.7 |

## HITL decisions required before implementation

Implementation starts only after the owner records:

1. a ruling that supersedes D-80.7-3 and D-80.6-2 for Node on Tegra, in the
   form the owner chooses (ADR or release-state decision);
2. the distribution channel and package name ([design](design.md) § 3;
   the design recommends a separately named, opt-in npm package);
3. whether the Jetson build and post-publish smoke gate the whole release or
   run as a separate route; and
4. the licence determination for statically linked CUDA runtime code in an
   npm artifact.

The full list is in [design](design.md) § 9. Slice 117 does not decide them.
Async-allocation recovery on Jetson (an explicit memory pool, or early
`cuInit` with a lazily created pool) is outside Slice 117's scope and remains
an open item for a later slice.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-117A | Tegra Node distribution is governed before it is built. | AC27-117A: a recorded owner ruling supersedes D-80.7-3/D-80.6-2 for Node, names the channel, package identity and loader policy, and states the aarch64 driver-probe contract relative to ADR-0.8.25 and R80-11. `dev/tegra-platform-reference.md` and the 0.8.23 design cross-reference it. No build or publish change lands first. |
| R27-117B | The artifact is built reproducibly from the release contract on the Jetson. | AC27-117B: a checked host-native wrapper asserts every Tegra toolchain pin in `cuda-artifact-contract.sh` (toolkit root, `nvcc` V12.6.68, GCC 11.4.0, `CUDA_COMPUTE_CAP=87`, CUDA `bin` on `PATH`, runtime library path, driver library present), builds with `embed-cuda,rerank-cuda`, passes the declared glibc floor and a `.node` linkage check (no unresolved `cuda*` symbols, no CUDA/NVIDIA `NEEDED` entry, no `RPATH`/`RUNPATH`), and records artifact digests. RED tests for the wrapper's assertion arm precede it. |
| R27-117C | The installed artifact behaves per the dual-runtime contract on the Jetson. | AC27-117C: from locally packed tarballs in a fresh external consumer, `cpu`, `auto` and `cuda:0` pass for embedding and reranking; forced CUDA records an allocation witness accepted by `verify-tegra-gpu-witness.py`; Slice 110's heap-growth regression passes; a driverless environment completes `auto` and `cpu` on CPU and refuses forced CUDA with the typed error. Inherited Slice 110 receipts are not a pass. |
| R27-117D | Generic Linux AArch64 users are unaffected. | AC27-117D: on hosted `ubuntu-24.04-arm`, a plain install still selects only `fathomdb-linux-arm64-gnu` with its 2.28 floor and passes `smoke-npm-package.sh`; the Tegra package is never injected by `npm-inject-optional-deps.sh`; the loader on a non-Tegra host ignores an installed Tegra package with a warning. Loader behavior has unit tests through the existing `LoaderSeams`. |
| R27-117E | The release pipeline builds, publishes and attests the artifact without widening runner trust. | AC27-117E: the Jetson job holds no publication credential; a hosted job publishes only the uploaded artifact; the runner route is restricted like the x86_64 CUDA route; publication is idempotent and carries provenance consistent with the other platform packages; `actionlint` passes; a non-publishing dry-run route exercises the build and pre-publication smoke. |
| R27-117F | A registry install is verified on the Jetson before the release is called done. | AC27-117F: after an authorized publication, a post-publish smoke on the Jetson installs from the registry (or Pages URL) and repeats AC27-117C's forced-CUDA, witness and heap-growth rows, bound to the published version and digests. Green CI alone does not satisfy it. |
| R27-117G | Public documentation states Jetson Node GPU support truthfully. | AC27-117G: when the slice completes, `README.md`, `docs/install/typescript.md`, `docs/compatibility/index.md` (with `check-glibc-floor-doc-truth.sh` green), the Jetson section of `docs/embedder.md` and `src/ts/npm/README.md` describe Jetson Node GPU support and the exact install command, replacing the README's current "CPU-only; build from source" statement. |

These IDs are release-local; `dev/acceptance.md` remains locked. AC27-117F is
satisfiable only after publication is separately authorized; until then the
slice can reach a qualified, unpublished candidate.

## Execution order

1. Wait for Slice 110 to close with the allocator fallback and early `cuInit`
   integrated; reconcile the exact release-branch source.
2. Obtain the HITL rulings above and record them; update the 0.8.23 design
   decision and the Tegra platform reference.
3. RED/GREEN the build wrapper's assertion arm and linkage check, then the
   loader change and its unit tests.
4. Build and qualify on the Jetson (AC27-117B–D), with independent review.
5. Wire the release workflow and dry-run route (AC27-117E); run `actionlint`
   and the scoped validators.
6. After publication authorization only: publish and run the Jetson
   registry smoke (AC27-117F).
7. Update public documentation (AC27-117G) and close status with receipts.

## Boundaries

- Do not change the generic `fathomdb-linux-arm64-gnu` package's features,
  floor or build location.
- Do not change device-policy semantics, error kinds or the no-CPU-fallback
  contract for forced CUDA.
- Do not edit Slice 110's documents or evidence; consume its closed result.
- Do not add a postinstall fetch or source build (R80-10).
- Async-allocation recovery, a Tegra wheel reranker, other Jetson modules and
  JetPack 7/CUDA 13 are out of scope.

## Ladder placement

Slice 117 depends on Slice 110. Slice 120 keeps its existing dependency on
Slice 115: it decomposes the TypeScript facade and consumes Slice 110's native
substrate, not the release pipeline, so it does not need Slice 117. At this
plan's baseline, neither Slice 140 nor Slice 150 depends on Slice 117; whether
integrated qualification should is an owner decision
([design](design.md) § 9).
