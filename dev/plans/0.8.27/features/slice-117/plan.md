---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon delivery
status: PLANNED
target_release: 0.8.27
planning_baseline: 80246a567
---

# Slice 117 — Jetson CUDA Node addon delivery

This is a draft plan. It adds a slice so Jetson users can get a CUDA-capable
Node addon without building one locally. It extends the existing Tegra
build, packaging and distribution path from the 0.8.26 work instead of
creating a parallel one. The owner ruled on 2026-10-05 to add the slice and to
supersede the 0.8.23 Node exclusions for it. Implementation is **held** (see
the entry criteria), and publication remains behind the release's unruled
`release-0.8.27-publication` decision. The [design note](design.md) owns
options, evidence and risks. The [status](status.md) records lifecycle. The
proposed record of the recommendation is
`dev/adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md`. The todos ledger
tracks the work as `TC-ffef2129-e5a6-4f8f-8cae-bdfe64792197`.

## Owner rulings (2026-10-05)

Recorded in `dev/plans/release-state-0.8.27.json` `decisions.ruled`:

- `slice-117-jetson-node-cuda-direction`: add Slice 117 after Slice 110 and
  authorize the work it needs. Do not begin implementation until Linux
  x86_64 has been tested with the Slice 110 fixes.
- `slice-117-supersede-d-80-7-3-and-d-80-6-2`: for Slice 117's scope, D-80.7-3
  and D-80.6-2 are superseded.
- `slice-110-early-cuinit-with-allocator-fallback`: Slice 110 ships early
  `cuInit` at addon load with the aarch64-Linux synchronous allocation
  fallback.
- `slice-117-channel-tegra-pages`: the channel is the existing first-party
  Tegra Pages route (design Option B). The addon is not published to the npm
  registry in 0.8.27, and a later registry publication needs its own ruling.
  npm ignores semver build metadata, so `X.Y.Z+tegra` cannot coexist with
  `X.Y.Z` under one package name. A separately named registry package could
  avoid that, but the owner chose the Pages route, which reuses the tested
  Tegra workflow, candidate guard, linkage check and post-publication proof.
- `tegra-allocator-0.8.27-sync-fallback-pool-study-0.8.28`: the 0.8.27
  allocator approach is the synchronous fallback with early `cuInit`; no
  memory pool ships in 0.8.27. 0.8.28 evaluates an explicit or lazily created
  pool
  ([study plan](../../../0.8.28/prework/tegra-cuda-memory-pool-study.md)).

## Entry criteria

Implementation (execution step 3 onward) starts only when all of these hold:

1. **x86_64 hold (owner, 2026-10-05).** Linux x86_64 (amd64) has been built and
   tested with the Slice 110 fixes on `llm/slice110-tegra-allocator-fix`, and
   the result is recorded. The Slice 110 branch did not compile x86_64; its
   off-target proxy test is not a substitute
   (`dev/plans/0.8.27/features/slice-110/tegra-integration-pending.md`).
   *Met 2026-10-05 on candidate `8b76f6115`; see [status](status.md#hold).*
2. Slice 110 has closed on `release/0.8.27` with the allocation fallback and
   early `cuInit` integrated and qualified.
   *Met: Slice 110 is complete on `release/0.8.27` at `a25d063cd`.*
3. The owner has ruled the remaining `slice-117-delivery-shape` items and
   accepted, revised or replaced the proposed distribution ADR. The channel
   is already ruled.

AC27-117F additionally requires publication authorization.

## Entry evidence

| Finding | Source |
| --- | --- |
| The published `fathomdb-linux-arm64-gnu` npm package is CPU-only: it is built on hosted `ubuntu-24.04-arm` in `manylinux_2_28` with `default-embedder` only. | `.github/workflows/release.yml`; `src/ts/package.json`; `src/ts/scripts/build-native.mjs` |
| The Linux x86_64 npm package and wheel are CUDA-capable (`embed-cuda,rerank-cuda`). | `release.yml` job `build-cuda-linux-x64-gnu`; `scripts/release/cuda-artifact-contract.sh` |
| A governed Tegra route already exists for the `+tegra` wheel. It has a manual, release-branch-only Jetson workflow with exact-SHA and version guards, a host preflight, a host-native contract-pinned build, a fail-closed linkage check, an installed three-policy smoke with witness, a credential-free Jetson with a hosted Pages publish job, and an operator script with a post-publication smoke. Slice 103 carried the 0.8.26 hardening into 0.8.27. | `.github/workflows/jetson-tegra-cuda-evidence.yml`; `scripts/release/build-python-cuda-tegra.sh`; `check-tegra-wheel-linkage.sh`; `publish-tegra-pages.sh`; `docs/operations/tegra-pages-publication.md`; `origin/release/0.8.26` `1a131e780`..`8c4fdfa9b`; `dev/plans/runs/0.8.27-slice-103-tegra/receipt.md` |
| That route publishes exactly one Tegra wheel per Pages deployment. `docs-pages.yml` redeploys one fixed retained wheel on every qualifying `main` push (`0.8.24+tegra` on `main`; `0.8.26+tegra` on `release/0.8.27` since 2026-10-07), so the two deployers overwrite each other. | `scripts/release/build-tegra-pages-index.sh`; `.github/workflows/docs-pages.yml` |
| No checked-in script builds a Tegra `.node`. `build-napi-cuda.sh` is x86_64-only, and the contract test rejects re-pointing its axis at Tegra. | `scripts/release/build-napi-cuda.sh`; `scripts/tests/test_cuda_release_contract.sh` |
| Slice 110 built and qualified a host-native CUDA addon on the Jetson. Its allocation fallback and early `cuInit` stay in 0.8.27. | `dev/plans/0.8.27/features/slice-110/tegra-integration-pending.md`; branch `llm/slice110-tegra-allocator-fix` |

## Open HITL decision

The channel is ruled (`slice-117-channel-tegra-pages`): the Tegra Pages
route. The npm registry options are not chosen for 0.8.27. The acceptance
below is planned on that channel.

`slice-117-delivery-shape` (unruled) now covers the package name and whether
to reserve it on the npm registry, loader policy, Pages retention, the
aarch64 import-time `cuInit` contract, the driverless evidence bar, licence
coverage, platform-capabilities representation, install form and ladder
dependencies. The full list is in [design](design.md) § 10.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-117A | Tegra Node distribution is governed before it is built. | AC27-117A: the supersession of D-80.7-3/D-80.6-2 is a recorded ruling with in-place notes in the 0.8.23 design and the platform reference (**done at planning**). The channel is ruled (**done at planning**, `slice-117-channel-tegra-pages`). The remaining `slice-117-delivery-shape` items are ruled. The distribution ADR is accepted and states the aarch64 driver-probe contract relative to ADR-0.8.25 and R80-11. No build, workflow or loader change lands first. |
| R27-117B | The artifact is built reproducibly from the release contract on the Jetson. | AC27-117B: `build-napi-cuda-tegra.sh` asserts every Tegra pin in `cuda-artifact-contract.sh` and has an `--assert-only` arm. It builds with `embed-cuda,rerank-cuda`, passes the `tegra` glibc floor and `check-tegra-wheel-linkage.sh` on the `.node`, contains no publication command, and records digests. RED contract tests (assertion arm, publication rejection, x86_64-axis non-interference) precede it. |
| R27-117C | The installed artifact behaves per the dual-runtime contract on the Jetson. | AC27-117C: inside `jetson-tegra-cuda-evidence.yml`, a fresh external consumer of the locally packed packages passes `cpu`, `auto` and `cuda:0` for embedding and reranking. Forced CUDA yields a witness accepted by `verify-tegra-gpu-witness.py --nvidia-smi`, and Slice 110's heap-growth regression passes. Driverless loadability meets the bar ruled under `slice-117-delivery-shape`. Inherited Slice 110 receipts are not a pass. |
| R27-117D | Generic Linux AArch64 users are unaffected, and the loader is safe. | AC27-117D: the generic package's features, floor and hosted build are unchanged. A contract test proves `npm-inject-optional-deps.sh` never injects the Tegra package. `LoaderSeams` unit tests cover Tegra with and without the package, version mismatch, a non-Tegra host with the package, and a Thor-like signal. The loader change is in the main package before the main 0.8.27 npm publication. |
| R27-117E | The existing Tegra route builds, retains and publishes the artifact without widening trust. | AC27-117E: `jetson-tegra-cuda-evidence.yml` builds and retains the wheel and the tarball in one run. The Jetson job still holds only `contents: read`. The version guard checks `src/ts/package.json` as well as `pyproject.toml`. The site builder publishes both artifacts. One retained-artifact list feeds both `docs-pages.yml` and the Jetson route, and a test proves neither drops a listed artifact. `test_tegra_publication_operator_path.sh` gains negative cases for the tarball. `actionlint` passes. Nothing publishes to npm. |
| R27-117F | The published artifact is verified on the Jetson before the release is called done. | AC27-117F: after authorized publication, `publish-tegra-pages.sh` runs a Node smoke on the Jetson. It installs `fathomdb@<version>` from npm and the tarball from the Pages URL, checks the tarball SHA-256, and repeats AC27-117C's forced-CUDA, witness and heap-growth rows. Skipping the smoke reports INCOMPLETE. Green CI alone does not satisfy it. |
| R27-117G | Public documentation states Jetson Node GPU support truthfully. | AC27-117G: `README.md`, `docs/install/typescript.md`, `docs/compatibility/index.md` (with `check-glibc-floor-doc-truth.sh` green), the Jetson section of `docs/embedder.md`, `src/ts/npm/README.md`, `docs/operations/tegra-pages-publication.md` and the release's Tegra publication handoff give the exact install command. They give it only for a confirmed classic Jetson Orin, replacing the README's "CPU-only; build from source" statement. |

These IDs are release-local; `dev/acceptance.md` remains locked. Until
publication is authorized, the slice can reach a qualified, unpublished
candidate.

## Execution order

1. Hold until entry criteria 1 and 2 are met: the x86_64 test of the Slice 110
   fixes, then Slice 110 closure on `release/0.8.27`.
2. Present the remaining `slice-117-delivery-shape` items and the proposed
   ADR for ruling. Record the result in release state and the ADR.
3. RED/GREEN the build wrapper's assertion arm, the publication rejection and
   the `.node` linkage check. Then do the loader change and its `LoaderSeams`
   tests, which must land before the main npm publication.
4. RED/GREEN the Pages retention list, the version guard and the site-builder
   changes. Then extend the Jetson workflow and the operator script, with
   `actionlint` and the operator-path negative tests.
5. Build and qualify on the Jetson through the workflow without publishing
   (AC27-117B–E), with independent review.
6. After publication authorization only: run the extended
   `publish-tegra-pages.sh` (AC27-117F).
7. Update public documentation (AC27-117G) and close status with receipts.

## Boundaries

- Do not change the generic `fathomdb-linux-arm64-gnu` package's features,
  floor or build location.
- Do not add npm registry publication to the Tegra route or to
  `release.yml`. The 0.8.27 channel is ruled as Tegra Pages; a registry
  publication would need its own ruling.
- Do not change device-policy semantics, error kinds or the no-CPU-fallback
  contract for forced CUDA.
- Do not edit Slice 110's documents or evidence; consume its closed result.
- Do not add a postinstall fetch or source build (R80-10).
- Async-allocation recovery, a Tegra wheel reranker, other Jetson modules and
  JetPack 7/CUDA 13 are out of scope. Memory-pool recovery of
  stream-ordered allocation is evaluated in 0.8.28, not here.

**0.8.28 note (Slice 30).** The Tegra Node addon build must enable the
`tegra-pool` Cargo feature, so its features are `embed-cuda,rerank-cuda,tegra-pool`
(R27-117B). Without it the addon keeps 0.8.27's allocator and reports
`not_built`. The x86_64 sets refuse the feature
(`scripts/check-cuda-release-contract.py`); see
`dev/adr/ADR-0.8.28-tegra-private-cuda-pool.md`.

## Ladder placement

Slice 117 depends on Slice 110. Slice 120 kept its dependency on Slice 115,
because it decomposes the TypeScript facade and does not need the Tegra route.
Slice 120 is now complete, so the loader change in this slice to
`src/ts/src/platform.ts` builds on its final facade. In the remaining ladder
(2026-10-07) Slice 117 follows Slice 135 and precedes Slice 140, and Slice 135
stays next. At this plan's baseline, neither Slice 140 nor Slice 150 depends on
Slice 117; whether they should is an open item ([design](design.md) § 10).
