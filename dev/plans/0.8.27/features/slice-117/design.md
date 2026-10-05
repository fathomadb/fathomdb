---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon delivery design
status: DRAFT
target_release: 0.8.27
planning_baseline: 80246a567
---

# Slice 117 — Jetson CUDA Node addon delivery design

This is a draft design note for owner review. Statements about the current
pipeline cite the file or commit that establishes them. Everything labelled
**Proposal** or **Recommendation** is a proposal, not a ruling. The
[plan](plan.md) owns requirements, acceptance and execution order. The
[status](status.md) records lifecycle and the implementation hold. The
proposed record of the recommendation is
`dev/adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md`.

**Revision, 2026-10-05.** The owner directed that Slice 117 build on the
existing Tegra build, packaging and distribution path from the 0.8.26 work,
instead of a parallel one. This revision studies that path (§ 3), adopts it
wherever it already answers a question, and changes the recommendation from
an npm registry package to the Tegra Pages route (§ 4). The owner's rulings of
the same day are recorded in § 2.

## 1. Problem

The published npm platform package `fathomdb-linux-arm64-gnu` is CPU-only:

- `.github/workflows/release.yml` job `build-napi` builds the
  `linux-arm64-gnu` row on the hosted `ubuntu-24.04-arm` runner, inside the
  digest-pinned `manylinux_2_28` container
  (`scripts/release/napi-artifact-contract.sh`,
  `scripts/release/provision-napi-manylinux.sh`), with `npm run build:native`.
- `src/ts/package.json` maps `build:native` to `src/ts/scripts/build-native.mjs`,
  which passes `--features default-embedder` only.
- `publish-npm-platform-linux-arm64-gnu` publishes that artifact, and
  `post-publish-smoke-aarch64` smokes it on `ubuntu-24.04-arm`
  (`scripts/release/smoke/smoke-npm-package.sh`), which has no GPU.

The Linux x86_64 npm package and wheel are CUDA-capable:
`build-cuda-linux-x64-gnu` runs `scripts/release/cuda-preflight.sh ...
--rerank-cuda` on the self-hosted x86_64 GPU runner and selects
`CUDA_RERANK_NAPI_FEATURES` (`embed-cuda,rerank-cuda`) from
`scripts/release/cuda-artifact-contract.sh`.

Node on a Jetson therefore has GPU support only from a locally built addon.
Slice 110 built one host-natively on the Jetson AGX Orin with
`embed-cuda,rerank-cuda`, `CUDA_COMPUTE_CAP=87` and CUDA 12.6
(`dev/plans/runs/0.8.27-slice-110-tegra/receipt.md` on
`llm/slice110-tegra-allocator-fix`). That build was qualification work, not a
pipeline artifact. No checked-in script produces it.

## 2. Governing decisions

### Owner rulings, 2026-10-05

Recorded in `dev/plans/release-state-0.8.27.json` `decisions.ruled`:

| Ruled id | Effect on Slice 117 |
| --- | --- |
| `slice-117-jetson-node-cuda-direction` | Slice 117 is on the 0.8.27 ladder after Slice 110, and its planning and governance work is authorized. **Implementation is held** until Linux x86_64 has been tested with the Slice 110 fixes on `llm/slice110-tegra-allocator-fix`. Publication remains behind `release-0.8.27-publication`. |
| `slice-117-supersede-d-80-7-3-and-d-80-6-2` | D-80.7-3 and D-80.6-2 no longer bar a Jetson Node addon. Other 0.8.23 Tegra rulings are unchanged. In-place notes mark the supersession in `dev/design/0.8.23-aarch64-tegra.md` and `dev/tegra-platform-reference.md` § 3.7. |
| `slice-110-early-cuinit-with-allocator-fallback` | The Slice 117 artifact carries Slice 110's early `cuInit` at addon load and the aarch64-Linux synchronous allocation fallback, as integrated and qualified by Slice 110. Memory-pool speed recovery is undecided and outside both slices. Evidence: `dev/plans/0.8.27/features/slice-110/tegra-integration-pending.md`. |

The remaining choice is recorded as unruled `slice-117-delivery-shape` (§ 10).

### Records this slice builds on or must respect

| Record | What it says | Consequence for Slice 117 |
| --- | --- | --- |
| `dev/design/0.8.23-aarch64-tegra.md` D-80.7-3 (superseded for Slice 117) | npm cannot distinguish Tegra. If Tegra ever ships on npm, the only safe shape is a distinctly named package that the main package does **not** list in `optionalDependencies`, with `doctor gpu` printing the install command after detection and the loader loading it if present. Install-both-and-probe is rejected. | The structural finding and the safe shape still hold. They define the package shape in § 4. |
| D-80.6-1 (same design) | 80.6 builds and proves; it does not publish to a registry. | Unchanged. The Tegra build wrapper stays publication-free (`scripts/tests/test_cuda_release_contract.sh` rejects a publishing wrapper). Pages publication is a separate hosted job (§ 3). |
| D-80.6-3, D-80.7-4 (same design) | The Tegra wheel keeps the `fathomdb` name and is distinguished by the `+tegra` local version on a first-party index. | This does not carry over to npm (§ 4, same-name variant). |
| `dev/tegra-platform-reference.md` § 3.6, § 6 | Print a Tegra install command only after Tegra detection, and pin it exactly. Detection is two-tier. Thor reports Tegra but is not a classic Tegra CUDA target. | The loader and docs follow the same rule (§ 4, loader). |
| `dev/adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md` | For x86_64 CUDA artifacts: no CUDA/NVIDIA `NEEDED` entries; lazy driver discovery only after policy selects CUDA; static runtime "not permission to probe a CUDA driver during module import". It rejected "a separate CUDA-only npm package". | The linkage rules apply to the Tegra `.node` (§ 5). The Tegra package is dual-runtime, not CUDA-only. Early `cuInit` at import needs an explicit aarch64 contract (§ 6, § 10). |
| `dev/adr/ADR-0.8.23-dual-runtime-device-policy.md` (proposed) | One artifact supports CPU and CUDA; `auto` default; `cpu` never initializes CUDA; forced CUDA fails typed. | Preserved, subject to the § 6 contract question. |
| `dev/adr/ADR-0.8.20-unscoped-npm-platform-packages.md`, `ADR-0.8.22-windows-native-npm-package.md` | Platform packages are unscoped. A new platform package identity has been recorded by ADR. | The package name is unscoped. The proposed ADR follows this precedent. |
| `dev/adr/ADR-0.8.20-linux-aarch64-native-artifacts.md` | The generic `fathomdb-linux-arm64-gnu` package and its hosted native smoke. | Unchanged. |

## 3. The existing Tegra path (0.8.26, carried into 0.8.27)

After the 0.8.26 release, five commits on `origin/release/0.8.26`
(`1a131e780`, `600c598ce`, `9c7c3da71`, `91b28587c`, `8c4fdfa9b`) made the
`+tegra` wheel publishable through a guarded first-party route. Slice 103
carried them into 0.8.27 (`f3af72a6a`, `0569baa09`;
`dev/plans/runs/0.8.27-slice-103-tegra/receipt.md`), rebuilt the wheel on the
Jetson, and dispatched no Pages publication. The path has these parts.

1. **A dedicated workflow, independent of `release.yml`.**
   `.github/workflows/jetson-tegra-cuda-evidence.yml` is `workflow_dispatch`
   only and runs only on `refs/heads/release/*`. It is not part of the release
   train's `all-builds-passed` or promotion graph.
2. **Candidate guard.** `validate-candidate` requires `candidate_sha` to be
   40 lowercase hex and equal to the dispatched commit, and requires
   `candidate_version` to match the release branch suffix. The build job then
   checks out exactly that SHA, rejects a dirty checkout and retains a
   source-identity record.
3. **Host preflight.** The job fails closed unless the host is Linux aarch64
   with a Tegra signal (`nvidia,tegra` in `/proc/device-tree/compatible` or
   `/etc/nv_tegra_release`). It also requires an approved absolute
   `nvidia-smi` that reports an `nvgpu` device, which excludes non-classic
   Tegra.
4. **Serialization.** A non-cancelling concurrency group keeps runs from
   sharing the integrated GPU, because shared runs could not give
   attributable allocation evidence.
5. **Host-native build from the contract.**
   `scripts/release/build-python-cuda-tegra.sh` runs an `--assert-only` arm,
   then builds. It asserts every Tegra pin in `cuda-artifact-contract.sh`,
   stamps `+tegra`, and asserts the `tegra` glibc floor (2.35). It runs
   `scripts/release/check-tegra-wheel-linkage.sh`, which fails closed on any
   unresolved `cuda*`/`__cuda*` symbol or CUDA/NVIDIA `NEEDED` entry
   (`0569baa09`). It then imports the installed wheel from a fresh venv
   outside the source tree with CUDA search paths removed (`9c7c3da71`). The
   static CUDA runtime comes from the pinned Candle fork (`1a131e780`).
6. **Installed smoke and witness.** The job installs the wheel into a fresh
   venv with `--no-index --no-deps` and runs `cpu`, `auto` and `cuda:0`
   against an offline model cache, checking the expected device resolution
   for each. It validates the in-process allocation witness with
   `scripts/release/verify-tegra-gpu-witness.py --nvidia-smi`, then uploads
   all evidence as one run-scoped artifact (30-day retention).
7. **Credential-free Jetson, hosted publication.** The Jetson job holds only
   `contents: read`. The opt-in `prepare-tegra-pages` job runs on
   `ubuntu-latest` and downloads only this run's evidence artifact. It checks
   the project version with `scripts/release/require-tegra-pages-release-version.sh`
   and builds one combined site with `scripts/release/build-pages-site.sh`:
   `mkdocs build --strict` plus the PEP 503 tree from
   `scripts/release/build-tegra-pages-index.sh`, which accepts exactly one
   wheel, checks its name and version metadata, and links it with a SHA-256
   fragment. `deploy-tegra-pages` deploys with `pages: write` in the
   `github-pages` environment and shares the Pages concurrency group with
   `docs-pages.yml`.
8. **Exact-SHA operator route and post-publication smoke.**
   `scripts/release/publish-tegra-pages.sh` refuses a SHA that is not the
   current remote release-branch head. It dispatches the workflow with
   `publish_to_pages=true`, identifies and watches the run, and verifies
   success at that SHA. It then downloads the evidence and runs
   `scripts/release/smoke/smoke-tegra-pages-wheel.sh` on the Jetson. That
   smoke accepts only the authorized index URL, uses a sole `--index-url`,
   and checks the wheel SHA-256. `--skip-pages-smoke` exits 3 as INCOMPLETE.
   The runbook is `docs/operations/tegra-pages-publication.md`.
9. **Identity and user guidance.** The artifact is `fathomdb==X.Y.Z+tegra`,
   never on PyPI. Docs print the exact pin only for a confirmed classic Jetson
   Orin (`docs/install/python.md`, `docs/compatibility/index.md`).
10. **Governance and review.** Publication needs separate release
    authorization. Each release has a handoff prompt
    (`dev/plans/prompts/0.8.27-TEGRA-PAGES-PUBLICATION.md`) and a retained
    review transcript (`600c598ce`). The tests are
    `scripts/tests/test_tegra_publication_operator_path.sh`,
    `test_cuda_release_contract.sh` and `test_tegra_wheel_linkage.sh`.

### Gaps in that path that matter for Node

- **G1 — Pages keeps one Tegra artifact, and the two deployers disagree.**
  Every Pages deployment replaces the whole site. `docs-pages.yml` runs on
  each qualifying push to `main` and rebuilds the site with exactly one
  retained wheel, pinned as `0.8.24+tegra` by URL and SHA-256 on `main`,
  `release/0.8.26` and `release/0.8.27`. The Jetson workflow deploys only the
  wheel it just built. Whichever ran last decides what the index serves. A
  pinned pip install fails once its file is gone. A Node install that records
  a tarball URL in a lockfile fails the same way, more visibly. Slice 117
  cannot use Pages without fixing retention. The same fix protects the wheel.
- **G2 — the route knows only the wheel.** The workflow, version guard
  (`pyproject.toml` only), index builder, site builder, operator script and
  post-publication smoke all handle a single wheel.
- **G3 — no measured driverless Jetson run.** The wheel route checks
  driverless loadability structurally (linkage check plus clean import). The
  Slice 103 receipt states that this is not a measured import on a driverless
  Jetson. ADR-0.8.25 requires a measured driverless run for x86_64.
- **G4 — embedding only.** The wheel builds with `CUDA_PYTHON_FEATURES`
  (`pyo3/extension-module,embed-cuda`), and the workflow smoke covers
  embedding only. The Node addon adds reranking.
- **G5 — no platform-capabilities representation.**
  `dev/platform-capabilities.json` keys rows by npm loader triple and has no
  Tegra family (`dev/tegra-platform-reference.md` § 8.4; todos seq 228).

### What the existing path answers for Slice 117 (adopted)

| Question in the first draft | Existing answer, adopted |
| --- | --- |
| Where and how is it built? | Host-natively on the self-hosted Jetson, from the same contract pins, by a wrapper with an `--assert-only` arm (§ 5). |
| Does the Jetson gate the whole release? | No. The Tegra route is a separate, manual, release-branch dispatch after the main release, as for the wheel. The first draft left this open. |
| Runner trust | The Jetson holds no publication credential. A hosted job publishes only this run's retained artifact. |
| Candidate binding | The same exact-SHA, branch-version and clean-checkout guards, and the same operator check against the remote head. |
| Linkage | `check-tegra-wheel-linkage.sh` takes any extracted ELF file. Reuse it on the `.node`, plus the `tegra` glibc floor. |
| Install guidance | Exact version only, printed only after classic-Tegra detection. |
| Post-publication proof | Operator-run smoke from the public URL on the Jetson, with an SHA-256 check and INCOMPLETE on skip. |
| Review and receipts | Retained run evidence, a per-release handoff prompt, and a retained independent review. |
| Compatibility declaration | Classic Jetson Orin, JetPack 6 / L4T R36, CUDA 12.6, as the wheel declares. Only the AGX Orin 64 GB is measured. |

## 4. Delivery options, re-evaluated

### Constraints that hold for every option

- npm selects optional platform packages only by `os`, `cpu` and `libc`. A
  Jetson Orin and an AWS Graviton both match `linux`/`arm64`/`glibc`
  (`dev/tegra-platform-reference.md` § 2).
- The loader (`src/ts/src/platform.ts` `loadPlatformBinding`, with
  `LoaderSeams`) resolves the triple and prefers a local
  `fathomdb.<triple>.node` (`src/ts/src/binding.ts` `loadLocal`). It then
  `require`s `platformPackageName(triple)`, or throws
  `UnsupportedPlatformError`. It has no alternative package per triple.
- `scripts/release/npm-inject-optional-deps.sh` injects every directory under
  `src/ts/npm/` as an exact-version optional dependency. A Tegra package must
  stay out of that directory or be excluded explicitly.
- **The loader change ships in the main package.** Whatever the channel,
  `fathomdb@0.8.27` itself must know how to find the Tegra package. The
  loader change and its tests must therefore land on `release/0.8.27` before
  the main npm publication, even though the Tegra artifact is produced after
  it. If they miss it, the Tegra artifact cannot serve 0.8.27.

### Package shape (common to A and B)

**Proposal.** A distinctly named, unscoped package for the same triple, for
example `fathomdb-linux-arm64-gnu-tegra` (name open), with
`os: ["linux"]`, `cpu: ["arm64"]`, `libc: ["glibc"]` and a dual-runtime
`embed-cuda,rerank-cuda` `.node`. The main package never lists it in
`optionalDependencies`. A Jetson install carries both binaries.

*Same-name variant, not proposed.* The analogue of the wheel's `+tegra` would
be a tarball named `fathomdb-linux-arm64-gnu` that overrides the generic
optional dependency. npm drops semver build metadata, so `+tegra` cannot
carry the identity. Whether npm keeps a URL-installed package in place of the
exact-version optional dependency on later installs is unverified. The
loader would also need a separate identity marker. Not pursued.

### Option B — Tegra Pages route, extended to a Node tarball (recommended)

**Proposal.** Extend the existing route rather than add one.

- The same dispatch of `jetson-tegra-cuda-evidence.yml` builds and smokes both
  the `+tegra` wheel and the Tegra Node package tarball for the candidate SHA,
  and retains both in its evidence artifact.
- `prepare-tegra-pages` verifies both, and the site builder places the
  tarball beside the wheel, for example at
  `https://fathomadb.github.io/fathomdb/tegra/npm/<name>-<version>.tgz`.
  The route publishes both in one deployment so that the site always holds a
  matching pair.
- `publish-tegra-pages.sh` runs a second post-publication smoke for Node on
  the Jetson. It installs `fathomdb@<version>` from npm and the tarball from
  the Pages URL, and checks the tarball SHA-256.
- Users install with an exact version and URL, printed only for a confirmed
  classic Jetson Orin:

  ```bash
  npm install fathomdb@0.8.27 https://fathomadb.github.io/fathomdb/tegra/npm/fathomdb-linux-arm64-gnu-tegra-0.8.27.tgz
  ```

- For: reuses every control in § 3 unchanged: workflow, guards, preflight,
  serialization, hosted-only publication, operator script and receipt
  practice. It adds no registry identity, credential, trusted-publisher
  configuration or `release.yml` coupling. It keeps Jetson Python and Node
  guidance in one place. Pages artifacts can be withdrawn, which npm versions
  cannot.
- Against: lockfiles pin the URL, so G1 must be fixed first. There is no npm
  provenance; SHA-256 and the retained evidence are the record. The package
  name is not held on npm, so someone else could publish it there; docs must
  always give the URL (§ 7). The docs still call Pages an "interim" index
  that must be re-reviewed before a later Tegra release
  (`docs/install/python.md`).

### Option A — npm registry package (deferred)

**Proposal (not recommended for 0.8.27).** Publish the same package to the
npm registry and install it by name.

- For: standard registry install and lockfile behavior, npm provenance from a
  hosted publish job, and durable immutable versions.
- Against: it adds a permanent identity and needs trusted publishing
  configured for a new package name (comment above
  `publish-npm-platform-darwin-x64` in `release.yml`). It brings npm
  publication into a route that today publishes only to Pages, or it couples
  the release train to the single shared Jetson runner. It is unverified
  whether npm provenance meaningfully covers bytes built on a self-hosted
  runner. A defective Tegra version cannot be withdrawn cleanly. ADR-0.8.25
  named a separate npm package identity as a loader-semantics change.

### Option C — documented source build only

Add the build wrapper (§ 5) and publish nothing. It does not meet the owner
direction of GPU support without a source build. The wrapper is still the
first deliverable of A and B.

### Option D — CUDA-capable generic `fathomdb-linux-arm64-gnu` (rejected)

The Tegra build has a 2.35 glibc floor against the generic 2.28
(`scripts/release/glibc-floor-contract.sh`). `sbsa-linux` and Tegra CUDA are
mutually incompatible, so SBSA CUDA hosts would receive `sm_87` kernels. It
would also run early `cuInit` in every generic AArch64 Node process.

### Recommendation

**Option B for 0.8.27, with Option A kept open as a later, separately ruled
step on the same package identity.** That is a combination in sequence, not
in parallel. A later npm publication would ship the same bytes under the
same name, so the loader would not change.

What changed from the first draft: that draft recommended Option A, with B as
fallback. Its main reasons were registry tooling and npm provenance, and it
left build location, runner trust and release coupling as open questions.
Measured against the 0.8.26 path:

1. Every open control question already has a tested answer on the Pages route
   and none on the npm route. B reuses them, and A would have to invent
   them.
2. B keeps the credential boundary the Tegra route was built around: a
   credential-free Jetson plus a hosted Pages job. A needs npm publication
   authority from, or after, a self-hosted build.
3. B keeps the Tegra route decoupled from the release train and its single,
   shared Jetson runner, as the wheel route already is.
4. The first draft's objections to B were lockfile durability and the
   interim transport. Durability is a concrete, fixable defect (G1) that the
   wheel already has, and B's work fixes it. The interim status applies
   equally to the wheel.
5. Only the AGX Orin 64 GB is measured, and Slice 110 carries known residual
   limits. A withdrawable first channel suits that.

## 5. Build

**Proposal.** Add `scripts/release/build-napi-cuda-tegra.sh`, modelled on
`build-python-cuda-tegra.sh` rather than on the x86_64
`build-napi-cuda.sh`. The x86_64 wrapper uses the `CUDA_NAPI_HOST_*` axis, and
`test_cuda_release_contract.sh` rejects re-pointing that axis at Tegra. The
new wrapper:

- sources the contract and asserts every Tegra pin below, with an
  `--assert-only` arm;
- builds with `napi build --platform --release` and
  `CUDA_RERANK_NAPI_FEATURES`;
- runs `check-glibc-floor.sh --floor "$(glibc_floor_for_family tegra)"` and
  `check-tegra-wheel-linkage.sh` on the produced `.node`;
- loads the packed package from a fresh consumer outside the source tree with
  `LD_LIBRARY_PATH` and `LIBRARY_PATH` removed;
- contains no publication command, so the existing publishing-wrapper
  rejection applies to it.

| Input | Value (from `scripts/release/cuda-artifact-contract.sh` unless noted) |
| --- | --- |
| CUDA toolkit root | `CUDA_TEGRA_HOST_TOOLKIT_ROOT` = `/usr/local/cuda-12.6` |
| `nvcc` identity | `CUDA_TEGRA_HOST_NVCC_VERSION` = `release 12.6, V12.6.68` |
| Host compiler | `CUDA_HOST_GCC_VERSION_TEGRA_ORIN` = 11.4.0, `/usr/bin/gcc`, `/usr/bin/g++` |
| Compute capability | `CUDA_COMPUTE_CAP_TEGRA_ORIN` = `87` |
| CUDA runtime import library | `CUDA_TEGRA_HOST_CUDART_LIB` = `/usr/local/cuda-12.6/targets/aarch64-linux/lib` (on `LIBRARY_PATH`) |
| Driver library (asserted present, never linked) | `CUDA_TEGRA_HOST_DRIVER_LIB` = `/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1` |
| `PATH` | Includes `$CUDA_TEGRA_HOST_TOOLKIT_ROOT/bin` |
| Rust / Node | 1.95.0 / 25.9.0 (`scripts/release/napi-artifact-contract.sh`; `release.yml`) |
| Features | `CUDA_RERANK_NAPI_FEATURES` = `embed-cuda,rerank-cuda` |

**New runner requirement.** The Jetson job today requires only the runner's
Cargo and a Python venv. Node 25.9.0 and the `src/ts` dependencies must be
provisioned and asserted in the same way, through a runner-owned toolchain or
a pinned setup action. The choice is an implementation decision.

Glibc floor and declaration: extend the `tegra` family in
`glibc-floor-contract.sh` to the `.node`. Update `docs/compatibility/index.md`
so `scripts/check-glibc-floor-doc-truth.sh` stays green. Extend the `.node`
basename case in `smoke-npm-package.sh` if that smoke is reused. Update the
comments in `cuda-artifact-contract.sh`, `glibc-floor-contract.sh` and
`build-python-cuda-tegra.sh` that cite D-80.6-2 as limiting Tegra to the
wheel, in the same change that adds the Node build.

## 6. What the artifact contains and how it behaves

- **Features:** `embed-cuda,rerank-cuda`, the same tuple as the Linux x86_64
  npm package. The Tegra wheel has no reranker (G4); aligning it is out of
  scope.
- **Slice 110 runtime changes** (ruled into Slice 110,
  `slice-110-early-cuinit-with-allocator-fallback`): the vendored
  aarch64-Linux cudarc synchronous allocation fallback, and `cuInit` during
  module registration. Per the Slice 110 branch (tip `8b76f6115` when this
  revision was written; `dev/tegra-platform-reference.md` §§ 7.7–7.8 there),
  the call runs from napi-rs module registration, not an ELF constructor. It
  runs once per process, checks for the driver library first, and never fails
  registration. It is skipped when every CUDA-built component's open-time
  policy is exactly `cpu` or when `FATHOMDB_CUDA_EARLY_INIT=off`. Its stated
  limits: a late import into a large heap is still refused (remedy:
  `node --import fathomdb`); the synchronous path is about 1.8–2.4 times
  slower per steady embed; only the AGX Orin 64 GB is measured. Slice 117
  re-verifies these in the installed artifact rather than inheriting Slice
  110 receipts, and rechecks the branch text at integration.
- **Without a usable GPU:** `cuInit` returns an error, `auto` records a typed
  CPU resolution, and forced `cuda:N` fails with `EmbedDevicePolicyError` /
  `RerankerDevicePolicyError` and never runs on CPU.
- **Contract to state (open, § 10 item 5):** ADR-0.8.25 forbids a driver
  probe at module import for x86_64 artifacts. ADR-0.8.23 states that `cpu`
  never initializes CUDA (R80-11). A process that selects `cpu` through open
  options rather than the environment still runs `cuInit` at load. The
  accepted distribution ADR must say whether R80-11 is read as "a `cpu`
  policy visible at load" or is amended for this artifact.

## 7. Qualification

Rows 1–3 run inside the Jetson job of `jetson-tegra-cuda-evidence.yml` and
are retained in its evidence artifact. All rows bind to the exact candidate
SHA and artifact SHA-256 values.

1. **Build witness:** assert-only log, build log, feature list, glibc floor,
   linkage output, and `.node` and `.tgz` digests and sizes.
2. **Installed package on the Jetson, before publication:** a fresh consumer
   outside the source tree installs the locally packed main and Tegra
   packages with lifecycle scripts disabled, and without resolving an
   unpublished generic package from the registry. It runs `cpu`, `auto` and
   `cuda:0` for embedding and for reranking (`FATHOMDB_RERANK_DEVICE`). It
   captures `openReport().embedderGpuAllocationWitness` under
   `FATHOMDB_GPU_ALLOCATION_WITNESS=1`, validated by
   `verify-tegra-gpu-witness.py --nvidia-smi`. It runs Slice 110's heap-growth
   regression against the installed package
   (`scripts/tests/test_tegra_node_early_cuinit.sh` with
   `FATHOMDB_TEGRA_NODE_PACKAGE`, on the Slice 110 branch until it
   integrates).
3. **Loader policy:** unit tests through `LoaderSeams` cover a Tegra host with
   and without the package, a version mismatch, a non-Tegra host with the
   package installed, and a Thor-like signal.
4. **Generic AArch64 unaffected:** `npm-inject-optional-deps.sh` never
   injects the Tegra package (contract test). The hosted `ubuntu-24.04-arm`
   generic smoke is unchanged.
5. **Driverless loadability:** at minimum the wheel route's structural
   standard (linkage check, plus a clean import without CUDA search paths).
   Whether a measured driverless aarch64 run is also required is open (G3,
   § 10 item 6).
6. **After publication:** `publish-tegra-pages.sh` runs the Node smoke on the
   Jetson from npm plus the Pages URL, repeating row 2's forced-CUDA, witness
   and heap-growth checks. `AGENTS.md` and `dev/design/release.md`
   § "Post-publish smoke" make this a completion condition. Green CI is not
   done.

## 8. Risks

| Risk | Detail | Mitigation (proposal) |
| --- | --- | --- |
| Pages retention (G1) | A deployment that drops the tarball breaks lockfiles that recorded its URL. Today the two Pages deployers each publish one, different, Tegra wheel. | One retained-artifact list consumed by both `docs-pages.yml` and the Jetson route, each entry pinned by SHA-256, with a test that both deployers keep every listed artifact. |
| Name not held on npm (B) | Someone else could publish the package name on the registry. A user who installs by name would get that package, and the loader would load it on Tegra hosts. | Docs and `doctor` guidance give only the exact URL form. The owner decides whether to reserve the name (a placeholder publication needs npm authority; ADR-0.8.22 records a bootstrap publication precedent). |
| Wrong-build selection | Installed on Graviton, SBSA or Thor, the package loads, because it has no CUDA `NEEDED` entry, but its `sm_87` kernels are wrong for SBSA CUDA. | Classic-Tegra detection in the loader. Exact version match. |
| Mixed-host lockfiles | A direct dependency with `os`/`cpu` fields may fail installation on non-arm64 developer hosts that share the lockfile (npm `EBADPLATFORM`; verify). | Decide whether docs prescribe an optional save. |
| Runner availability | One self-hosted Jetson, a non-cancelling group, and the same host also serves another repository's runner (`dev/design/0.8.23-aarch64-tegra.md` § 7, 80.6). The Node rows lengthen each run. | Keep the existing group. Record host load in receipts. The route does not gate the main release. |
| Main-package coupling | The loader change must be in the published main package (§ 4). | Land the loader change and its tests before the main 0.8.27 npm publication. |
| Provenance (B) | No npm provenance. | Publish SHA-256 values in the docs and the retained evidence. Revisit under Option A. |
| Licence | The `.node` statically links CUDA runtime code, as the x86_64 npm package and the Tegra wheel already do. 0.8.25's design required a CUDA redistribution/licence/SBOM review before release (`dev/design/0.8.25-driverless-cuda-runtime.md`, plan step 5). This session did not locate its recorded outcome. | Extend that review's conclusion to the Tegra `.node`. Do not start a separate review unless none was recorded. |
| Measured hardware | Only the AGX Orin 64 GB on L4T R36.5.2 is measured. The aarch64-Linux cfg also reaches unmeasured CUDA hosts. | Declare only the measured row as qualified. |
| Artifact size | Unmeasured. The 0.8.27 Tegra wheel was 9,063,350 bytes (`dev/plans/0.8.27/features/slice-103/status.md`). | Record sizes in the build witness. |

## 9. Documentation obligations at completion

`README.md` (its platform-support table and the "CPU-only; build from source"
statement), `docs/install/typescript.md`, `docs/compatibility/index.md`, the
Jetson section of `docs/embedder.md` and `src/ts/npm/README.md` must describe
Jetson Node GPU support and the exact install command. In addition,
`docs/operations/tegra-pages-publication.md` and the release's
`dev/plans/prompts/<version>-TEGRA-PAGES-PUBLICATION.md` handoff must cover the
Node tarball, and `dev/tegra-platform-reference.md` must record the Node
route.

## 10. Open decisions for the owner

Ruled on 2026-10-05 and removed from this list: superseding D-80.7-3 and
D-80.6-2; including Slice 117 in the 0.8.27 ladder; early `cuInit` and the
allocation fallback in Slice 110. Questions adopted from the existing Tegra
path (build location, runner trust, release coupling, candidate guard,
exact-version guidance, compatibility declaration; § 3) are proposals the
owner may still override.

The items below make up unruled `slice-117-delivery-shape`:

1. **Channel:** Option B (recommended), Option A, or B now with A later.
2. **Package name** (unscoped), and under B whether to reserve it on npm.
3. **Loader policy:** which detection tier the loader uses (the workflow uses
   device-tree or `nv_tegra_release`, plus `nvgpu` from `nvidia-smi`, which is
   too slow for every load). How Thor is excluded. Whether a version mismatch
   warns and falls back or fails. Whether the loader warns on a confirmed
   classic Tegra host that runs the generic CPU package (the Node analogue of
   D-80.7-1).
4. **Pages retention (new):** one retained-artifact list for both deployers,
   how many versions to keep, and whether to publish wheel and tarball in one
   dispatch.
5. **aarch64 driver-probe contract:** the R80-11 / ADR-0.8.25 wording for
   import-time `cuInit` (§ 6). This concerns the contract wording only;
   inclusion is ruled.
6. **Driverless evidence bar (new):** a measured driverless aarch64 run, or
   the wheel route's structural standard (G3).
7. **Licence/SBOM:** confirm that 0.8.25's review covers the Tegra `.node`.
8. **Platform capabilities (new):** how `dev/platform-capabilities.json`
   represents a second package on the `linux-arm64-gnu` triple (G5; todos
   seq 228).
9. **Install form (new):** whether docs prescribe an optional save for
   mixed-host teams.
10. **Ladder:** whether Slice 140 or 150 should depend on Slice 117. At this
    revision's baseline they do not.
11. **ADR:** accept, revise or reject
    `dev/adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md` (proposed).
