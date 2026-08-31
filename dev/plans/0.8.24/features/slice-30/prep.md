---
title: 0.8.24 Slice 30 — preparation review
status: COMPLETE
target_release: 0.8.24
---

# Slice 30 preparation review

## Goal and reviewed allocation

Slice 30 must expose the existing host-native Jetson Orin CUDA Python wheel as
an explicit public target without weakening generic CPU publication. It owns
Tegra package selection, compatibility, artifact provenance, and the publisher
handoff. Slice 60 owns final installed-target smoke aggregation and publisher
preservation; Slice 70 owns release integration.

Reviewed inputs were the Slice 30 draft, Slices 0–6 allocations, completed
Slice 10 CI interface, changes through Slice 20, current release/build scripts,
Python co-install diagnostics, Tegra platform reference, and the authoritative
0.8.23 AArch64/Tegra design.

## Changes since the draft was written

1. Slice 10 completed with no workflow change and assigned target-specific
   routing to Slice 30.
2. Slice 20 completed independently and does not alter Tegra packaging.
3. Review found the draft's distinct-distribution premise stale against
   D-80.6-3 already present in this repository.
4. The owner clarified that “Fathom” means this same
   `fathomadb/fathomdb` repository; there is no second repository to inspect.
5. Existing 0.8.23 evidence confirms the dedicated Jetson route, host matrix,
   wheel shape, installed policy smokes, and GPU witness.

## Exists versus net-new

| Area | Exists | Net-new after prerequisite |
| --- | --- | --- |
| Identity | `fathomdb` plus staged `+tegra` local version | No rename or import split |
| Build | Host-native `build-python-cuda-tegra.sh` | Candidate-version integration only if tests show a gap |
| Wheel truth | Bare `linux_aarch64`, abi3, glibc 2.35 contract | Public index transport |
| Detection | Classic-Tegra/SBSA/Thor classification and generic-build warning | Endpoint-bearing exact repair command |
| Runner evidence | Dedicated `jetson-fathomdb` route and successful run 32296395639 | Both jobs now accept only `release/0.8.24`; execute the exact candidate after the normal version bump |
| Publisher | Credentialless evidence artifact retention | Interim GitHub Pages endpoint and hosted deployment route; durable distribution review before a later Tegra release |
| CPU path | Generic `fathomdb` on PyPI | Preservation proof only |

## Supported matrix

- Jetson Orin / classic Tegra iGPU (`sm_87`), AArch64.
- L4T R36 / JetPack 6, measured R36.5.2.
- CUDA 12.6, measured nvcc 12.6.68.
- Ubuntu 22.04 / glibc 2.35.
- Python abi3 with the existing `cp310-abi3-linux_aarch64` wheel contract.

Generic ARM64/SBSA, Jetson Thor, Xavier, other JetPack/CUDA combinations, npm,
and PyPI Tegra uploads are not claimed.

## Prerequisite disposition

Resolved: repository, distribution/import name, local-version convention,
target matrix, wheel tag, build host, detection rule, exact-pin rule, and
candidate evidence route.

Historical source gap resolved in Slice 30:
`.github/workflows/jetson-tegra-cuda-evidence.yml` now bounds both jobs to
`github.ref == 'refs/heads/release/0.8.24'`. The Jetson job checks
`project.version == 0.8.24` before every candidate build, so release execution
must perform the normal version bump before dispatch. The runner route itself
remains proven.

Resolved after this preparation review: GitHub Pages is enabled in Actions mode
for `fathomadb/fathomdb`; the interim base is
`https://fathomadb.github.io/fathomdb/tegra/simple/`. The hosted publisher uses
the `github-pages` environment with `pages: write` and `id-token: write`, and
no registry secret. The 0.8.24 implementation remains responsible for the
branch-ref correction, hard version check, artifact validation, and explicit
publication input.
