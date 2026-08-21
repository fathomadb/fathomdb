---
title: FathomDB 0.8.24 — public CUDA artifacts for Tegra and Windows
status: PROPOSED
target_release: 0.8.24
---

# FathomDB 0.8.24 — Draft Plan · public CUDA artifacts for Tegra and Windows

> **Status: scope commissioned; design not yet ratified.** This is a planning
> record only. It does not authorize a tag, registry upload, runner change, or
> a local Windows CUDA build. A live release board and state file are created
> only when Slice 0 accepts this plan.

**Theme.** Make CUDA support distributable where it is presently either a
local-only Jetson wheel or absent from the Windows release artifacts. The
library remains CPU-safe by default; CUDA stays an opt-in runtime capability.

**Footprint.** Tegra builds and its installed-package smoke run on the Jetson
runner. Windows CUDA is built and proved only on an approved remote Windows
CUDA executor; no local Windows compiler, toolkit, VM, or GPU is a prerequisite
for this release.

---

## 1. Goal and scope

- Publish a clearly identified, installable Tegra CUDA Python artifact from the
  Jetson runner, with its JetPack/glibc 2.35 contract and a real
  install/open/write/search/close smoke.
- Add a supported Windows x64 CUDA artifact route, including the public Python
  and npm binding surfaces if the Slice 0 packaging decision keeps both SDKs in
  scope.
- Make artifact selection explicit: a generic Linux ARM64 or Windows CPU wheel
  must never be silently mistaken for a CUDA-capable binary.
- Keep the existing CPU artifacts and their compatibility floors intact.

**Out of scope:** local Windows CUDA compilation; Windows ARM64; macOS CUDA;
Tegra SBSA support; CUDA performance claims; a CUDA default flip; and changing
the existing 0.8.23 registry artifacts.

## 2. Provisional requirements and acceptance signals

These become frozen requirements only at Slice 0.

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R24-1 | A public Tegra distribution identity is valid for its registry and cannot be confused with the generic ARM64 CPU wheel. | A packaging test rejects the previous `+tegra` public-upload shape; a registry-query fixture proves the selected name/version and installation command. |
| R24-2 | The Jetson runner builds the selected Tegra artifact with the declared CUDA, `sm_87`, native Linux, and glibc 2.35 contract. | Target-host build witness plus `check-glibc-floor.sh --floor 2.35` against the published bytes. |
| R24-3 | The Tegra artifact is published through a configured trusted publisher and works when installed from its public registry. | Registry version/file query and a clean Jetson install/open/write/search/close/process-exit smoke. |
| R24-4 | Windows x64 CUDA has a named artifact and proof route that does not depend on local Windows compilation. | Remote executor provenance plus a CUDA-enabled artifact build and installed-artifact smoke on Windows. |
| R24-5 | Python and npm CUDA support have one explicit per-platform support matrix; unsupported combinations fail clearly. | Cross-SDK matrix test and documentation truth check covering Windows x64 and Tegra versus generic ARM64/SBSA. |
| R24-6 | The release workflow publishes only the approved new artifacts, idempotently, and preserves existing CPU publication behavior. | Focused workflow-contract tests, actionlint, one release dry run, and registry-installed smokes before a tag is cut. |

## 3. Slice ladder

```text
0 → 5 → 10 → 15 → 20 → 40
```

| Slice | Title | Work type | Depends on |
| ---: | --- | --- | --- |
| **0** | Artifact identity and publication ADR — choose the Tegra public distribution name/installer contract; decide whether Windows CUDA ships Python, npm, or both; create live board/state. | design-adr | — |
| **5** | Tegra public package contract — refactor the current local-only `+tegra` build into the approved public artifact form and test its metadata/selection boundaries. | implementation | 0 |
| **10** | Tegra build, trusted publisher, and registry smoke — wire the Jetson runner, publication artifact, and installed-package proof. | implementation | 5 |
| **15** | Windows CUDA artifact contract — define the remote Windows executor, toolchain, artifact names, CPU fallback, and Py/TS surface matrix. | design-implementation | 0 |
| **20** | Windows CUDA remote build and installed-artifact smoke — add the approved remote route; no local Windows compilation task is permitted. | implementation | 15 |
| **40** | Verification and release readiness — preserve CPU lanes, run scoped artifact proofs and one dry run, then perform the registry/tag release only with HITL direction. | verification-release | 10, 20 |

**Hard gates.** Slice 0 is HITL-gated because the Tegra package identity and
Windows SDK surface determine immutable public names. Slice 10 cannot publish
until the new PyPI trusted-publisher entry exists. Slice 20 cannot claim
Windows CUDA support without a real remote Windows CUDA executor and an
installed-package smoke. Slice 40 is the sole release-publication gate.

**Parallel tracks.** Tegra (Slices 5–10) and Windows (Slices 15–20) may run in
separate worktrees after Slice 0. They serialize changes to
`.github/workflows/release.yml`, `docs/compatibility/index.md`, release
contracts, and version/package metadata.

## 4. Packaging decisions to settle at Slice 0

1. **Tegra public identity (blocking).** The current artifact is
   `0.8.23+tegra`; PyPI does not permit local-version labels on public uploads.
   The leading candidate is a separately named distribution such as
   `fathomdb-tegra==0.8.24`, installed explicitly on Jetson. Do not upload a
   second `fathomdb==0.8.24` `linux_aarch64` wheel beside the generic manylinux
   ARM64 CPU wheel: pip selection would not establish the requested CUDA
   capability deterministically.
2. **Windows CUDA surfaces (blocking).** Decide whether 0.8.24 ships both the
   Python wheel and npm platform package, or one SDK only. The default planning
   assumption is both, because the public bindings must not report divergent
   CUDA support without an explicit ruling.
3. **Remote Windows executor (blocking Slice 20).** Name the Windows CUDA
   runner/host, its CUDA toolkit, GPU capability, artifact retention boundary,
   and trusted execution route. “No local compile required” does not relax the
   requirement for a real Windows artifact and installed-package evidence.

## 5. Verified override and duplication register

| # | Concept | Sources | Consequence if they drift |
| ---: | --- | --- | --- |
| 1 | Tegra version and upload boundary | `scripts/release/build-python-cuda-tegra.sh:165-212`; `docs/compatibility/index.md:48-62` | An unauditable local wheel could be advertised or uploaded as public. |
| 2 | PyPI artifact aggregation | `.github/workflows/release.yml:1159-1184`; `src/python/pyproject.toml:16-19` | A new wheel could be omitted, collide with the main project, or bypass trusted publishing. |
| 3 | Native-platform name resolution | `src/ts/src/platform.ts:25-93`; `src/ts/package.json:20-21` | npm could install a CPU or wrong-platform binding without a clear failure. |
| 4 | Existing Windows CPU build lanes | `.github/workflows/release.yml:404-464`; `.github/workflows/release.yml:466-495` | A Windows CUDA route could accidentally replace or weaken the CPU artifact. |

## 6. Behavior-change register

| # | Change | Who notices | Required changelog/documentation outcome |
| ---: | --- | --- | --- |
| 1 | Tegra CUDA becomes a public, explicitly installed artifact. | Jetson Python users. | Exact package/install command, JetPack/glibc support floor, and CPU fallback behavior. |
| 2 | Windows x64 gains CUDA-capable artifact(s), if Slice 20 closes. | Windows Python and/or Node users. | Supported SDK/platform matrix, prerequisite driver/toolkit contract, and unsupported-route error. |
| 3 | Public package topology expands. | Release operators and dependency resolvers. | Trusted-publisher and registry identity documentation. |

## 7. Prerequisites

1. 0.8.23 remains closed on `main`; its published registries are immutable and
   are not re-cut for this work.
2. The Jetson runner is online, has the already-declared JetPack/CUDA contract,
   and can access its model/cache prerequisites.
3. Before Slice 10, the owner configures a trusted publisher for the selected
   new PyPI project, including its exact workflow filename and environment.
4. Before Slice 20, the owner supplies or approves a Windows CUDA executor.
   No local Windows setup is requested.

## 8. Definition of done

- TDD for package identity, artifact selection, failure messages, and release
  workflow wiring; no generated-oracle tests.
- Existing CPU Python/npm packages remain installable and their declared ABI
  floors stay enforced.
- Every new public artifact is installed from its registry in a clean target
  environment and completes open/write/search/close with a clean process exit.
- `actionlint`, release-contract checks, compatibility documentation truth, and
  the normal release dry run pass before tag authorization.
- The final tag is only pushed after the owner approves the evidence; no
  speculative `v*` tag push.

## 9. Decisions recorded

- 2026-08-21 — 0.8.24 scope includes public Tegra publication and Windows CUDA.
  Local Windows CUDA compilation is expressly out of scope. Source: repository
  owner.
- 2026-08-21 — 0.8.23 remains Tegra-unpublished; its local `+tegra` wheel is a
  build-and-prove artifact, not a public PyPI release. Source: current release
  contract and registry check.

## 10. Immediate next slice

**Slice 0 — Artifact identity and publication ADR.** Resolve the three blocking
decisions in §4, then create the 0.8.24 release-state file and live board. No
CUDA artifact implementation or registry configuration is authorized before
those choices are recorded.
