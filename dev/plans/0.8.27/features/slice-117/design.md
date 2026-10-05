---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon delivery design
status: DRAFT
target_release: 0.8.27
planning_baseline: 6735fd9f1
---

# Slice 117 — Jetson CUDA Node addon delivery design

This is a draft design note for owner review. Statements about the current
pipeline cite the file that establishes them. Everything under a
**Proposal** or **Recommendation** label is a proposal, not a ruling. The
[plan](plan.md) owns requirements, acceptance and execution order; the
[status](status.md) records lifecycle.

## 1. Problem

The published npm platform package `fathomdb-linux-arm64-gnu` is CPU-only:

- `.github/workflows/release.yml` job `build-napi` builds the
  `linux-arm64-gnu` row on the hosted `ubuntu-24.04-arm` runner, inside the
  digest-pinned `manylinux_2_28` container
  (`scripts/release/napi-artifact-contract.sh`,
  `scripts/release/provision-napi-manylinux.sh`), with `npm run build:native`.
- `src/ts/package.json` maps `build:native` to `src/ts/scripts/build-native.mjs`,
  which passes `--features default-embedder` only. No CUDA feature is enabled.
- `publish-npm-platform-linux-arm64-gnu` publishes that artifact, and
  `post-publish-smoke-aarch64` smokes it on `ubuntu-24.04-arm`
  (`scripts/release/smoke/smoke-npm-package.sh`), which has no GPU.

By contrast, the Linux x86_64 npm package and Python wheel are CUDA-capable:
`build-cuda-linux-x64-gnu` runs `scripts/release/cuda-preflight.sh ...
--rerank-cuda` on the self-hosted `[self-hosted, Linux, X64, gpu, cuda-12]`
runner and stages the result under the normal artifact names consumed by the
existing publishers. `--rerank-cuda` selects `CUDA_RERANK_NAPI_FEATURES`
(`embed-cuda,rerank-cuda`) from `scripts/release/cuda-artifact-contract.sh`.

Python on Jetson already has a CUDA route: the host-native `+tegra` wheel
(`scripts/release/build-python-cuda-tegra.sh`), built and smoked on the
self-hosted Jetson runner by `.github/workflows/jetson-tegra-cuda-evidence.yml`
and optionally published to the first-party PEP 503 index on GitHub Pages
(`scripts/release/publish-tegra-pages.sh`,
`scripts/release/build-tegra-pages-index.sh`,
`docs/operations/tegra-pages-publication.md`). The public docs currently serve
only the exact `fathomdb==0.8.24+tegra` wheel from that index
(`.github/workflows/docs-pages.yml`, `docs/install/python.md`,
`docs/compatibility/index.md`).

Node on a Jetson therefore has GPU support only from a locally built addon.
Slice 110 built one host-natively on the Jetson AGX Orin with
`embed-cuda,rerank-cuda`, `CUDA_COMPUTE_CAP=87` and CUDA 12.6
(`dev/plans/runs/0.8.27-slice-110-tegra/receipt.md`). That build was a
qualification exercise, not a pipeline artifact; no checked-in script
produces it.

## 2. Governing decisions this slice must respect or supersede

| Record | What it says | Consequence for Slice 117 |
| --- | --- | --- |
| `dev/design/0.8.23-aarch64-tegra.md` D-80.7-3 | npm is **permanently** out of scope for Tegra: npm's matching key `(platform, arch, libc)` cannot distinguish Tegra from generic AArch64/SBSA. If a future slice must ship Tegra on npm, the only safe shape is a distinctly named package that the main package does **not** list in `optionalDependencies`, with `doctor gpu` printing the install command after detection and the loader loading it if present. Install-both-and-probe is rejected. | Slice 117 contradicts a ruled decision. It needs an owner ruling that supersedes D-80.7-3, recorded before implementation (see § 9). D-80.7-3's own fallback shape is the starting point for the options below. |
| D-80.6-2 (same design) | Tegra artifact scope is the Python wheel only. | Superseded in scope by the same ruling. |
| `dev/tegra-platform-reference.md` § 2, § 3.7 | Tegra vs SBSA is not expressible in wheel tags, npm fields or Rust triples; npm restated as permanently out of scope. | Same. The reference must be updated with the successor. |
| `dev/adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md` | For supported Linux x86_64 CUDA artifacts: no CUDA/NVIDIA `NEEDED` entries; driver discovery is lazy and occurs only after runtime policy selects CUDA; a static runtime may be linked but is "not permission to probe a CUDA driver during module import". It rejected "a separate CUDA-only npm package" because it changes normal package identity and loader semantics. | Scoped to x86_64, but its linkage rules are the right bar for the aarch64 artifact. Two tensions must be resolved in the successor: Slice 110's early `cuInit` runs at addon load on aarch64 Linux (§ 5), and the recommended shape (§ 3) is a separately named package. |
| `dev/adr/ADR-0.8.23-dual-runtime-device-policy.md` | One CUDA-capable artifact supports CPU and CUDA; `auto` default on CUDA-capable artifacts; `cpu` never initializes CUDA; forced CUDA fails typed. | Preserved unchanged. The Tegra addon is a dual-runtime artifact, not a CUDA-only one. |
| `dev/adr/ADR-0.8.20-unscoped-npm-platform-packages.md` | Platform packages are unscoped `fathomdb-<triple>`; there is no `@fathomdb` npm organization. | Any new package name must be unscoped. |
| `dev/adr/ADR-0.8.20-linux-aarch64-native-artifacts.md` | The generic `fathomdb-linux-arm64-gnu` package and its native-runner smoke. | Unchanged: the generic package stays CPU-only and manylinux 2.28. |

The repository owner ruled on 2026-10-05 that Slice 110 keeps both the
aarch64-Linux synchronous-allocation fallback and the early `cuInit` at
addon load in 0.8.27. That ruling was relayed to this planning session and is
not yet recorded in `dev/plans/release-state-0.8.27.json` at this plan's
baseline.

## 3. How Jetson users obtain the addon

### Constraints that hold for every option

- npm selects optional platform packages only by `os`, `cpu` and `libc`. A
  Jetson Orin and an AWS Graviton both match `linux`/`arm64`/`glibc`
  (`dev/tegra-platform-reference.md` § 2).
- The loader (`src/ts/src/platform.ts`, `loadPlatformBinding`) resolves the
  host triple, prefers a local `fathomdb.<triple>.node` next to the loader
  (`src/ts/src/binding.ts`, `loadLocal`), then `require`s
  `platformPackageName(triple)`, and otherwise throws
  `UnsupportedPlatformError`. It has no notion of an alternative package for
  the same triple.
- `scripts/release/npm-inject-optional-deps.sh` injects **every** directory
  under `src/ts/npm/` as an exact-version optional dependency of the main
  package. A Tegra package directory placed there would be auto-installed on
  every Linux AArch64 host. Any option that adds a package must keep it out of
  that directory or teach the script to exclude it.
- `docs/install/python.md` and `docs/compatibility/index.md` print the Tegra
  index command only for a confirmed classic Jetson Orin, and pin an exact
  version, because a floating or merged-index install can select the wrong
  build (`dev/tegra-platform-reference.md` § 3.6).

### Option A — separately named npm package, opt-in install (D-80.7-3 shape)

**Proposal.** Publish a second unscoped package for the same triple, for
example `fathomdb-linux-arm64-gnu-tegra` (name is an open decision), with
`os: ["linux"]`, `cpu: ["arm64"]`, `libc: ["glibc"]`, and the CUDA-capable
`.node`. It is **not** injected into the main package's
`optionalDependencies`. Users install it explicitly at the exact main-package
version:

```bash
npm install fathomdb@0.8.27 fathomdb-linux-arm64-gnu-tegra@0.8.27
```

The CPU `fathomdb-linux-arm64-gnu` package is still installed through
`optionalDependencies`, so a Jetson install carries both binaries.

Loader change (proposal): on the `linux-arm64-gnu` triple, before the generic
package, attempt the Tegra package only if it is installed **and** the host
carries a Tegra signal (the same signals `jetson-tegra-cuda-evidence.yml`
checks: `nvidia,tegra` in `/proc/device-tree/compatible` or
`/etc/nv_tegra_release`). Require its `package.json` version to equal the main
package version; on mismatch, or on a non-Tegra host, fall back to the generic
package with a one-time warning rather than loading a mismatched or wrong-family
binary. `fathomdb doctor platform` / `doctor gpu` (Rust CLI) and the docs
print the install command only after Tegra detection, as for Python.

- For: a registry install with the same tooling, lockfile semantics and
  publish/smoke machinery as the other five packages; npm provenance from a
  hosted publish job (verify at implementation; § 7); generic AArch64 users
  are unaffected.
- Against: a new permanent package identity; the loader gains a second
  candidate for one triple, which ADR-0.8.25 called out as a loader-semantics
  change; npm trusted publishing must be configured manually for each new
  package (comment above `publish-npm-platform-darwin-x64` in `release.yml`);
  two binaries on disk on Jetsons.

### Option B — tarball on the first-party Tegra Pages index

**Proposal.** Build the same package, but publish its `.tgz` next to the
`+tegra` wheel on GitHub Pages and document
`npm install fathomdb@0.8.27 https://fathomadb.github.io/fathomdb/tegra/npm/<file>.tgz`.
The loader change is the same as Option A.

- For: no new npm registry identity; matches the Python Tegra route and its
  "interim" status (`docs/operations/tegra-pages-publication.md`).
- Against: URL-tarball dependencies are pinned by URL in lockfiles, carry no
  npm provenance, and depend on the Pages transport that the docs call
  interim and require re-review before a later Tegra release. Every Pages
  deployment replaces the whole site, so `docs-pages.yml` and
  `jetson-tegra-cuda-evidence.yml` would both need to retain the tarball (today
  `docs-pages.yml` retains only the 0.8.24 wheel by exact SHA-256).

### Option C — documented build-from-source only

**Proposal.** Add a checked build wrapper (§ 4) and document it; publish
nothing. This is the current state plus a supported script.

- For: no distribution or identity change; no D-80.7-3 supersession beyond
  documentation.
- Against: does not meet the todo
  (`TC-e5496fb8-cad8-49a1-a2fe-8570eb9728b4`) of GPU support from a registry
  install.

### Option D — make `fathomdb-linux-arm64-gnu` itself CUDA-capable (rejected)

Rejected on existing evidence: the Tegra build has a 2.35 glibc floor versus
the generic package's 2.28 (`scripts/release/glibc-floor-contract.sh`), so it
would drop older generic AArch64 hosts; `sbsa-linux` and Tegra CUDA are
mutually incompatible (`dev/tegra-platform-reference.md` § 2), so CUDA hosts
such as Grace Hopper would receive `sm_87` kernels; and it would make every
generic AArch64 Node process run Slice 110's early `cuInit` attempt at load.

### Recommendation

Option A, with Option C's build wrapper as its first deliverable. It is the
only option that satisfies the todo through a registry, and it is the shape
D-80.7-3 itself named as the only safe one. Option B is the fallback if the
owner prefers not to create an npm identity for a single-host-family build.

## 4. Where and how it is built

### Host-native on the self-hosted Jetson runner

**Proposal.** Build on the existing self-hosted Jetson runner, labels
`[self-hosted, Linux, ARM64, jetson, aarch64]`
(`.github/workflows/jetson-tegra-cuda-evidence.yml`), not by
cross-compilation. The repository already records why for the wheel
(`scripts/release/build-python-cuda-tegra.sh` header;
`dev/tegra-platform-reference.md` § 4.2): the Tegra CUDA runtime is
host-bound, `sbsa-linux` ships no Tegra SASS/PTX, and no manylinux image
carries a Tegra toolkit. The same reasons apply to the `.node` artifact.

`scripts/release/build-napi-cuda.sh` is x86_64-only today: it uses the
`CUDA_NAPI_HOST_*` x86_64 axis, `LIBRARY_PATH` under
`targets/x86_64-linux/lib`, and `CUDA_COMPUTE_CAP` resolved to the x86_64
value. **Proposal:** add a host-native Tegra N-API wrapper (for example
`scripts/release/build-napi-cuda-tegra.sh`) that mirrors
`build-python-cuda-tegra.sh`: an `--assert-only` arm, toolchain identity
assertions, then `napi build --platform --release` with
`CUDA_RERANK_NAPI_FEATURES`, followed by the glibc-floor gate and the linkage
check. The alternative is to parameterize `build-napi-cuda.sh` by target axis;
either way the contract comment in `cuda-artifact-contract.sh` that says the
N-API names "select the x86_64 axis ... D-80.6-2 scopes Tegra to the Python
wheel" must change with it.

### Toolchain pins (all from `scripts/release/cuda-artifact-contract.sh` unless noted)

| Input | Value |
| --- | --- |
| CUDA toolkit root | `CUDA_TEGRA_HOST_TOOLKIT_ROOT` = `/usr/local/cuda-12.6` |
| `nvcc` identity | `CUDA_TEGRA_HOST_NVCC_VERSION` = `release 12.6, V12.6.68` |
| Host compiler | `CUDA_HOST_GCC_VERSION_TEGRA_ORIN` = 11.4.0, `/usr/bin/gcc`, `/usr/bin/g++` |
| Compute capability | `CUDA_COMPUTE_CAP_TEGRA_ORIN` = `87` (single pin; Tegra is not forward-compatible) |
| CUDA runtime import library | `CUDA_TEGRA_HOST_CUDART_LIB` = `/usr/local/cuda-12.6/targets/aarch64-linux/lib` (must be on `LIBRARY_PATH`) |
| Driver library (asserted present, never linked) | `CUDA_TEGRA_HOST_DRIVER_LIB` = `/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1` |
| `PATH` | Must include `$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin`; the CUDA build invokes `nvcc` by name (both existing wrappers export it) |
| Rust / Node | 1.95.0 / 25.9.0 (`scripts/release/napi-artifact-contract.sh`; `release.yml`) |
| Features | `CUDA_RERANK_NAPI_FEATURES` = `embed-cuda,rerank-cuda` |

The measured reference host is one Jetson AGX Orin 64 GB, L4T R36
REVISION 5.2, Ubuntu 22.04 glibc 2.35, driver 540.5.0
(`dev/tegra-platform-reference.md` § 1). The CUDA runtime is linked
statically through the Candle fork's `cudart_static` path and the driver is
loaded dynamically by cudarc (`dev/design/0.8.25-driverless-cuda-runtime.md`;
`check-tegra-wheel-linkage.sh` enforces this for the wheel). Slice 110's
Jetson CUDA `.node` recorded no `RPATH`/`RUNPATH` and only `libstdc++`,
`libgcc_s`, `libm`, `libc` and the loader as `NEEDED`
(`dev/plans/runs/0.8.27-slice-110-tegra/receipt.md`). **Proposal:** apply the
same linkage check to the `.node` file and fail the build on any unresolved
`cuda*` symbol or CUDA/NVIDIA `NEEDED` entry.

### Glibc floor and compatibility declaration

The `tegra` glibc family (2.35) is declared for the Tegra Python wheel only
(`scripts/release/glibc-floor-contract.sh`). **Proposal:** extend that family
to the Tegra `.node` (or add a sibling family), update
`docs/compatibility/index.md` so `scripts/check-glibc-floor-doc-truth.sh`
stays green, and extend the `.node` basename case in
`scripts/release/smoke/smoke-npm-package.sh`, which currently fails closed for
any Linux artifact other than `linux-x64-gnu` and `linux-arm64-gnu`.

Declared support (proposal, owner decision in § 9): classic Jetson Orin
(`sm_87`) on JetPack 6 / L4T R36 with CUDA 12.6, matching the Python Tegra
route's wording in `docs/install/python.md`. Only the AGX Orin 64 GB is
measured. Declare it in the package `description`, the compatibility page and
the loader's Tegra check; do not encode an L4T revision range that has not
been measured.

## 5. What the artifact contains and how it behaves

- **Features:** `embed-cuda,rerank-cuda`, the same tuple as the published
  Linux x86_64 npm package. Note the Tegra Python wheel builds with
  `CUDA_PYTHON_FEATURES` (`pyo3/extension-module,embed-cuda`) and therefore has
  no cross-encoder; aligning the two is out of Slice 117's scope and should be
  a named follow-up if wanted.
- **Slice 110 runtime fixes:** the vendored aarch64-Linux-only cudarc
  synchronous-allocation fallback and the early `cuInit` at addon load. Per
  the owner's 2026-10-05 ruling both remain in 0.8.27 as part of Slice 110, so
  the Slice 117 artifact is expected to carry both, subject to Slice 110
  closing. Their measured behavior is recorded on the Slice 110 branch
  (`dev/tegra-platform-reference.md` §§ 7.7–7.8 there; `docs/embedder.md`
  "Node.js on Jetson: load fathomdb first" there). Slice 117 must re-verify
  them in the installed registry artifact, not inherit Slice 110's receipts.
- **Without a usable GPU:** per Slice 110's documentation, the early `cuInit`
  checks for the driver library first, never fails the load, and is skipped
  when both `FATHOMDB_EMBED_DEVICE` and `FATHOMDB_RERANK_DEVICE` are exactly
  `cpu` in the starting environment. Under `auto` the engine records a typed
  CPU resolution; forced `cuda:N` fails with `EmbedDevicePolicyError` /
  `RerankerDevicePolicyError` and never runs on CPU
  (`ADR-0.8.23-dual-runtime-device-policy.md`).
- **Contract reconciliation (not an inclusion decision):** ADR-0.8.25 forbids
  probing the driver during module import for x86_64 artifacts and requires
  static runtime code to stay dormant until CUDA is selected. The early
  `cuInit` deliberately initializes the driver at import on aarch64 Linux. The
  successor record that admits the Tegra npm artifact must state the
  aarch64 contract explicitly, including whether a process that selects CPU
  by a means other than both environment variables may still run `cuInit` at
  load, so that R80-11 ("`cpu` never initializes CUDA") remains true as
  written or is amended deliberately.
- **Out of Slice 117's scope:** recovering stream-ordered (async) allocation
  speed on Jetson, for example with an explicit memory pool or early `cuInit`
  with a lazily created pool. Slice 110 documents the synchronous path as about
  1.8–2.4 times slower per steady embed. That remains an open item for a later
  slice, not a Slice 117 decision.

## 6. Qualification

All rows bind to the exact candidate SHA and artifact SHA-256.

1. **Build witness on the Jetson:** toolchain assertion log, feature list,
   glibc floor, linkage check, `.node` and `.tgz` digests.
2. **Pre-publication installed package on the Jetson:** a fresh external
   consumer installs the main package plus the Tegra package from local
   tarballs, outside the source tree, with lifecycle scripts disabled (the
   Slice 110 receipt method). Run `cpu`, `auto` and `cuda:0` for embedding and
   for reranking (`FATHOMDB_RERANK_DEVICE`), and capture
   `openReport().embedderGpuAllocationWitness` under
   `FATHOMDB_GPU_ALLOCATION_WITNESS=1`, validated by
   `scripts/release/verify-tegra-gpu-witness.py`. Run the Slice 110 heap-growth
   regression (`scripts/tests/test_tegra_node_early_cuinit.sh` on the Slice 110
   branch) against the installed package.
3. **Generic AArch64 unaffected:** on hosted `ubuntu-24.04-arm`, a plain
   `npm install fathomdb@<version>` resolves only `fathomdb-linux-arm64-gnu`,
   and if the Tegra package is also installed there, the loader falls back to
   the generic package with its warning.
4. **Driverless CPU fallback:** the Tegra `.node` loads and completes `auto`
   and `cpu` on a Jetson-compatible image or host with no GPU device, no CUDA
   library mounts and no CUDA search paths, as ADR-0.8.25 requires for x86_64.
   The x86_64 route uses `CUDA_DRIVERLESS_NODE_IMAGE`; an aarch64 equivalent
   must provide glibc ≥ 2.35.
5. **Post-publish registry smoke on the Jetson:** after publication, install
   from the registry (or the Pages URL for Option B) on the Jetson, rerun row 2
   including forced CUDA, the allocation witness and the heap-growth check.
   `AGENTS.md` and `dev/design/release.md` § "Post-publish smoke" make this a
   completion condition: green CI is not done.
6. **Provenance and receipts:** npm `--provenance` and the same
   `npm-publish-if-new.sh` idempotency as the other platform packages; a
   receipt consistent with the existing CUDA evidence (`cuda-preflight`
   witness, `native-artifact-receipts.py` schema
   `fathomdb.native-artifact-receipt/v1`, whose
   `EXPECTED_TARGET_LABELS` currently has one label per target and would need a
   distinct label for the Tegra variant).

## 7. Risks

| Risk | Detail | Mitigation (proposal) |
| --- | --- | --- |
| Runner availability and contention | One self-hosted Jetson serves this repository's evidence workflow, has a non-cancelling concurrency group because one integrated GPU cannot give attributable evidence to concurrent runs (`jetson-tegra-cuda-evidence.yml`), and the same host also runs another repository's runner (`dev/design/0.8.23-aarch64-tegra.md`, § 7, 80.6). Slice 110 repeated its Jetson forced-CUDA runs after that other repository's CI finished (`dev/plans/runs/STATUS-0.8.27.md`). | Decide whether the Jetson build/smoke gates the whole release (§ 9). Reuse the existing concurrency group and record host load in receipts. |
| Self-hosted runner security on release jobs | Labels are routing hints, not access control; a repository-scoped runner on a public repository can be selected by a hostile workflow (`dev/design/0.8.23-gpu-artifacts.md`, "Runner and trust boundary"). The x86_64 CUDA lane is gated by `verify-cuda-trusted-route` and the `cuda-unmerged-preflight` environment. The Jetson runner holds no publication credential; a hosted job publishes Pages (`jetson-tegra-cuda-evidence.yml`). | Keep the Jetson credential-free; build there, publish from a hosted job using only the uploaded artifact; restrict the runner group and require an environment approval, mirroring the x86_64 route. |
| npm provenance from a self-hosted build | Provenance attests the publishing workflow run. Whether npm provenance accepts or meaningfully covers bytes built on a self-hosted runner is unverified here. | Verify against npm documentation at implementation; record the attestation's actual subject. |
| Artifact size | The CUDA build carries kernels for one `sm_87` target plus CPU paths; its size is unmeasured in this repository. The 0.8.27 Tegra wheel was 9,063,350 bytes (`dev/plans/0.8.27/features/slice-103/status.md`). | Measure and record the `.node` and `.tgz` sizes in the build witness. |
| Licence and redistribution of CUDA components | The artifact statically links CUDA runtime code and must ship no CUDA/NVIDIA shared libraries. `dev/tegra-platform-reference.md` § 3.3 states the CUDA EULA forbids redistributing `libcuda.so.1` and the CUDA runtime libraries. The repository has no recorded licence review of statically linked runtime code in npm artifacts. | Obtain and record an owner/legal determination before first publication; the linkage check enforces the "no shared library payload" half. |
| Measured hardware is narrow | Only the AGX Orin 64 GB on L4T R36.5.2 is measured. Slice 110's allocator regression test skips on smaller Jetsons, and the cfg also reaches unmeasured non-Tegra aarch64 CUDA hosts. | Declare only the measured row as qualified; label other Orin modules unmeasured. |
| Wrong-build selection | A user on Graviton or SBSA installs the Tegra package by mistake. | Loader Tegra-signal check and exact version check (§ 3, Option A). |

## 8. Documentation obligations at completion

When Slice 117 completes, `README.md` and the `docs/` install pages
(`docs/install/typescript.md`, `docs/compatibility/index.md`, and the Jetson
section of `docs/embedder.md`), together with `src/ts/npm/README.md`, must
describe Jetson Node GPU support and how to install it, replacing the
"CPU-only; build from source" statement the README carries at this plan's
baseline. If the slice ships nothing (Option C), the same pages must name the
supported build wrapper instead.

## 9. Open decisions for the owner

These are listed, not decided.

1. **Supersede D-80.7-3** (npm permanently out of scope for Tegra) and
   D-80.6-2 with a recorded ruling, and decide whether that record is an ADR
   (for example a successor touching ADR-0.8.25's x86_64-only scope and its
   rejected "separate CUDA-only npm package" alternative) or a release-local
   decision in `release-state-0.8.27.json`.
2. **Distribution channel:** Option A (npm package), B (Pages tarball) or C
   (build-from-source only).
3. **Package name** for Option A or B, unscoped per ADR-0.8.20.
4. **Loader policy:** whether the loader prefers the Tegra package only on a
   detected Tegra host, and whether a version mismatch is a warning with CPU
   fallback or a hard error.
5. **Release coupling:** whether the Jetson build is part of
   `all-builds-passed` and the Jetson post-publish smoke is a prerequisite of
   `promote-npm-latest`, or whether the Tegra package follows its own
   dispatch route (as the Tegra wheel does) after the main release.
6. **Compatibility declaration:** the JetPack/L4T/CUDA range to declare, and
   whether the loader checks L4T R36.
7. **aarch64 driver-probe contract:** how the successor states the import-time
   `cuInit` relative to ADR-0.8.25 and R80-11 (§ 5). This concerns the
   contract wording, not early `cuInit`'s inclusion, which the owner has ruled.
8. **Licence determination** for statically linked CUDA runtime code in an npm
   artifact.
9. **Ordering in the ladder:** whether Slice 140 or 150 should depend on
   Slice 117 so integrated qualification covers it (at this plan's baseline
   they do not; see the [plan](plan.md)).
