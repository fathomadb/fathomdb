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
decision `slice-117-delivery-shape`. One part is already ruled: the channel
(decision 2, `slice-117-channel-tegra-pages`, 2026-10-05). The record as a
whole stays proposed until the remaining items are ruled and it is accepted.

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
  fallback;
- `slice-117-channel-tegra-pages`: the addon is delivered through the
  existing first-party Tegra Pages route, not the npm registry, in 0.8.27
  (decision 2 below);
- `tegra-allocator-0.8.27-sync-fallback-pool-study-0.8.28`: no explicit or
  lazily created memory pool ships in 0.8.27; 0.8.28 evaluates one.

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

1. **Package shape.** The Tegra addon is the separately named, unscoped
   `fathomdb-linux-arm64-gnu-tegra` platform package carrying a dual-runtime
   `embed-cuda,rerank-cuda` `.node`. The main package never lists it in
   `optionalDependencies`, so npm never selects it automatically. Do not
   reserve the name through an npm publication in 0.8.27; installation
   guidance always uses the exact Pages URL and published SHA-256. This is
   the shape D-80.7-3 named as the only safe one.
2. **Channel (ruled 2026-10-05, `slice-117-channel-tegra-pages`).** In
   0.8.27 the package's exact-version tarball is distributed only through the
   existing first-party Tegra Pages route, beside the `+tegra` wheel: built
   and smoked on the self-hosted Jetson, then published by the hosted Pages
   job. It is not published to the npm registry in 0.8.27. A later npm
   registry publication of the same package identity requires its own ruling.
   Rationale: npm ignores semver build metadata, so `X.Y.Z+tegra` cannot
   coexist with `X.Y.Z` under one package name, which is how the wheel is
   distinguished on its index. A separately named registry package could
   sidestep that. The owner chose the Pages route, which reuses the tested
   Tegra workflow, candidate guard, linkage check and post-publication proof.
3. **Loader.** The loader considers the Tegra package only on the
   `linux-arm64-gnu` triple when installed and when both classic-Tegra tiers
   are confirmed: the device-tree or `/etc/nv_tegra_release` family signal,
   then the `nvgpu` GPU-name signal. The second probe runs only for this
   installed package on a Tier-1 host, so generic imports do not pay for it.
   A Thor-like or indeterminate result selects the generic package. Without
   the Tegra package, the loader selects the generic package without an
   import-time warning; the exact URL is in Jetson install guidance. A Tegra
   package version mismatch fails with a clear install error before loading
   either addon; it never silently selects mismatched bytes.
4. **Linkage.** ADR-0.8.25's linkage rules apply to this artifact although that
   ADR is scoped to x86_64: no CUDA or NVIDIA `NEEDED` entry, no shipped CUDA
   or NVIDIA shared library, static CUDA runtime, and lazy driver loading
   through cudarc. The existing Tegra linkage checker enforces this.
5. **Driver probe at load.** On aarch64 Linux the addon calls `cuInit` during
   module registration (Slice 110). For this artifact, R80-11's "`cpu` never
   initializes CUDA" promise applies when every CUDA-built component's
   `cpu` policy is visible at module registration, or when
   `FATHOMDB_CUDA_EARLY_INIT=off`. A `cpu` option supplied only at a later
   open cannot prevent the earlier probe. A missing driver never fails module
   registration. This is an explicit aarch64 exception to ADR-0.8.25's
   x86_64 import-time no-probe contract and must be stated in public guidance.
6. **Qualification and retention.** Require an installed, driverless AArch64
   import in addition to the linkage check. Retain every published Tegra
   wheel and tarball with a digest-pinned manifest consumed by both Pages
   deployers; removing a published URL requires a separate decision. Publish
   matching wheel and tarball in one Jetson dispatch. Confirm CUDA
   redistribution, licence and SBOM coverage for the `.node` before release.
7. **Capability and install representation.** Represent the Tegra Pages
   package as a separate variant of `linux-arm64-gnu` in
   `dev/platform-capabilities.json`, leaving the generic package's record
   intact. Document the exact-version Pages tarball as an optional dependency
   for lockfiles shared with non-Jetson hosts, after a cross-host install
   fixture confirms that form.
8. **Release handoff.** Slice 140 may consume the reviewed, qualified,
   unpublished candidate and its exact install documentation. The later
   Pages-installed forced-CUDA, witness and heap-growth smoke is required
   before Slice 117 is COMPLETE. Slice 150 owns integrated final-candidate
   qualification. Slice 135's measurements remain bound to its pinned
   source; repeat affected paired cells only if Slice 117 changes query or
   retrieval behavior beyond loader and packaging work.

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

- **npm registry package now (draft Option A), or Pages first with npm
  later.** A registry package gives a standard registry install, provenance
  and durable versions. Not chosen for 0.8.27 by the channel ruling: it adds
  a permanent identity, npm publication from or after a self-hosted build, and
  trusted-publisher setup for a new name, none of which the existing Tegra
  route has. A later release may rule on it separately.
- **CUDA-capable generic `fathomdb-linux-arm64-gnu`.** Rejected. It raises the
  generic glibc floor to 2.35, ships `sm_87` kernels to SBSA CUDA hosts, and
  runs early `cuInit` in every AArch64 Node process.
- **Install both binaries and probe.** Rejected by D-80.7-3. A Tegra-linked
  binary is not a valid choice on a generic host.
- **Source build only.** Does not meet the owner direction.
