---
title: ADR-0.8.27-jetson-tegra-node-addon-distribution
date: 2026-10-05
target_release: 0.8.27
desc: Distribute a CUDA-capable Node addon for classic Jetson Orin through the existing Tegra Pages route
blast_radius: TypeScript loader; Jetson Tegra CUDA evidence workflow; Tegra Pages site builders and publisher; CUDA artifact and glibc-floor contracts; Tegra platform reference; public install and compatibility docs
status: proposed
---

# ADR — Jetson (classic Tegra) CUDA Node addon distribution

**Status: PROPOSED (draft for HITL review; not accepted).** Nothing in this
record authorizes implementation or publication. It records the
recommendation of the Slice 117 design
(`dev/plans/0.8.27/features/slice-117/design.md`) for the open release-state
decision `slice-117-delivery-shape`.

## Context

The published `fathomdb-linux-arm64-gnu` npm package is CPU-only and built on
a hosted runner. Jetson users get GPU support under Node only by building the
addon from source. The repository owner ruled on 2026-10-05
(`dev/plans/release-state-0.8.27.json`):

- `slice-117-jetson-node-cuda-direction`: deliver a CUDA-capable Linux AArch64
  Node addon for Jetson through the release process; do not begin
  implementation until Linux x86_64 has been tested with the Slice 110 fixes;
- `slice-117-supersede-d-80-7-3-and-d-80-6-2`: for that scope, supersede
  0.8.23 D-80.7-3 (npm permanently out of scope for Tegra) and D-80.6-2 (Tegra
  artifact scope is the Python wheel only);
- `slice-110-early-cuinit-with-allocator-fallback`: Slice 110 ships early
  `cuInit` at Node addon load with the aarch64-Linux synchronous allocation
  fallback.

The structural constraint is unchanged: npm selects platform packages by
`(platform, arch, libc)` only, so it cannot distinguish a Jetson from any
other Linux AArch64 glibc host (`dev/tegra-platform-reference.md` § 2).

FathomDB already has a governed Tegra build, packaging and distribution path
for the `+tegra` Python wheel. It was hardened after the 0.8.26 release and
carried into 0.8.27 by Slice 103. The path consists of the manual,
release-branch-only `.github/workflows/jetson-tegra-cuda-evidence.yml`, the
host-native `scripts/release/build-python-cuda-tegra.sh`, the fail-closed
`scripts/release/check-tegra-wheel-linkage.sh`, a hosted Pages publish job,
and the exact-SHA operator script `scripts/release/publish-tegra-pages.sh`
with its post-publication smoke.

## Decision (proposed)

1. **Package shape.** The Tegra addon is a separately named, unscoped
   platform package (name open; for example
   `fathomdb-linux-arm64-gnu-tegra`, per ADR-0.8.20) carrying a dual-runtime
   `embed-cuda,rerank-cuda` `.node`. The main package never lists it in
   `optionalDependencies`, so npm never selects it automatically. This is the
   shape D-80.7-3 named as the only safe one.
2. **Channel.** In 0.8.27 the package's exact-version tarball is distributed
   only through the existing Tegra Pages route, beside the `+tegra` wheel:
   built and smoked on the self-hosted Jetson, then published by the hosted
   Pages job. It is not published to the npm registry in 0.8.27. A later npm
   registry publication of the same package identity requires its own ruling.
3. **Loader.** The loader considers the Tegra package only on the
   `linux-arm64-gnu` triple, only when it is installed, and only on a host
   that passes the classic-Tegra detection the release workflow uses. It
   requires the package version to equal the main package version. Otherwise
   it loads the generic package. The detection tier and the mismatch
   behavior remain open in `slice-117-delivery-shape`.
4. **Linkage.** ADR-0.8.25's linkage rules apply to this artifact although that
   ADR is scoped to x86_64: no CUDA or NVIDIA `NEEDED` entry, no shipped CUDA
   or NVIDIA shared library, static CUDA runtime, and lazy driver loading
   through cudarc. The existing Tegra linkage checker enforces this.
5. **Driver probe at load (open).** On aarch64 Linux the addon calls `cuInit`
   during module registration (Slice 110). This record must state, before
   acceptance, whether R80-11 ("`cpu` never initializes CUDA") holds only for
   a `cpu` policy visible at load, or is amended for this artifact. As of the
   Slice 110 branch, the call is skipped only when every CUDA-built
   component's open-time policy is exactly `cpu`, or when
   `FATHOMDB_CUDA_EARLY_INIT=off`.

## Consequences

- No new registry identity, publication credential or trusted-publisher
  configuration in 0.8.27. The Jetson stays credential-free, and only the
  hosted Pages job publishes.
- Jetson Node installs use an exact tarball URL. Lockfiles record that URL.
  The Pages site therefore must retain every published Tegra artifact across
  deployments; today each deployment carries exactly one Tegra wheel.
- No npm provenance is attached. The published SHA-256 and the retained
  Jetson evidence are the integrity record.
- The package name is not held on the npm registry under this decision, so a
  third party could publish that name there. Users must install by URL, and
  reserving the name is a separate decision.
- Generic Linux AArch64 users are unaffected. The generic package, its 2.28
  glibc floor and its hosted build do not change.

## Alternatives considered

- **npm registry package now (draft Option A).** This gives a standard
  registry install, provenance and durable versions. It is deferred, not
  rejected: it adds a permanent identity, npm publication from or after a
  self-hosted build, and trusted-publisher setup for a new name, none of which
  the existing Tegra route has.
- **CUDA-capable generic `fathomdb-linux-arm64-gnu`.** Rejected. It raises the
  generic glibc floor to 2.35, ships `sm_87` kernels to SBSA CUDA hosts, and
  runs early `cuInit` in every AArch64 Node process.
- **Install both binaries and probe.** Rejected by D-80.7-3. A Tegra-linked
  binary is not a valid choice on a generic host.
- **Source build only.** Does not meet the owner direction.
