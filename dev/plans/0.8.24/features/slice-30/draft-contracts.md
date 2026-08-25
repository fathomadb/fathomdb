---
title: 0.8.24 Slice 30 — reviewed slice-local contracts
status: APPROVED-CONDITIONAL
target_release: 0.8.24
---

# Slice 30 reviewed slice-local contracts

These identifiers are Slice 30 planning contracts, not additions to the locked
canonical `dev/acceptance.md` namespace.

## Need

**N30-1 — explicit prebuilt Jetson CUDA selection.** A supported Jetson Orin
user needs a prebuilt CUDA-enabled FathomDB Python wheel that is selected
explicitly and cannot be confused with generic AArch64 CPU installation.

## Requirements

**R30-1 — same identity, separated source.** Generic CPU is
`fathomdb==0.8.24` on PyPI. Tegra is the same `fathomdb` distribution/import
with exact version `0.8.24+tegra` on one declared first-party PEP 503 index.

**R30-2 — truthful target.** The Tegra artifact remains
`cp310-abi3-linux_aarch64`, built host-natively for Jetson Orin, L4T R36 /
JetPack 6, CUDA 12.6, and glibc 2.35. It is never submitted to PyPI or relabelled
manylinux without a successor design and proof.

**R30-3 — selection safety.** An alternate-index install command is exposed
only after confirmed classic-Tegra detection and pins
`fathomdb==0.8.24+tegra`. Generic AArch64, SBSA, Thor, and indeterminate hosts
receive no Tegra index command.

**R30-4 — source/publisher separation.** The Jetson build job has read-only
repository access and no publication authority. An immutable artifact and
digest cross to a separately trusted publisher whose endpoint, environment,
and authentication are explicitly declared.

**R30-5 — CPU preservation.** Slice 30 does not rename, replace, or alter the
PyPI CPU package, generic ARM64 artifacts, or npm packages.

**R30-6 — evidence honesty.** Structural tests are not target evidence. A real
Jetson candidate proof retains source identity, host/toolchain facts, wheel
digest, `cpu`/`auto`/`cuda:0` installed smokes, and the validated in-process
GPU witness.

## Acceptance criteria

**AC30-1 — metadata and routing.** Contract fixtures prove the Tegra wheel is
`fathomdb-0.8.24+tegra-*-linux_aarch64.whl`, the generic package remains
`fathomdb==0.8.24` on PyPI, and no route submits the Tegra wheel to PyPI.

**AC30-2 — detection-gated exact install.** Positive classic-Orin and negative
SBSA/Thor/generic/indeterminate fixtures prove that only the positive case
receives the concrete endpoint and exact `==0.8.24+tegra` command.

**AC30-3 — publisher boundary.** Workflow mutation fixtures prove the builder
cannot publish and the hosted publisher consumes the exact named artifact and
digest under the declared environment.

**AC30-4 — candidate-installed target proof.** On the dedicated Jetson runner,
a clean candidate install proves open/write/search/close/exit for CPU, auto,
and forced CUDA policy, with auto/forced CUDA selecting the expected device and
forced CUDA producing a valid witness.

**AC30-5 — handoff.** Slice 60 receives the exact package identity, index URL,
artifact digest, candidate proof, and clean registry-installed smoke command.

## Approval condition

N30-1 and R30-1 through R30-6 are approved as the corrected design direction.
AC30-1 through AC30-5 are executable through the authorized interim GitHub
Pages route. Publication and the registry-installed smoke remain explicit
release actions; this record does not claim either has occurred.
