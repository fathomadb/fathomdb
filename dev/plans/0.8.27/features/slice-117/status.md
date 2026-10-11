---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon status
status: COMPLETE_ON_RELEASE_BRANCH
target_release: 0.8.27
planning_baseline: 80246a567
---

# Slice 117 status

**COMPLETE on `release/0.8.27` as a qualified, unpublished Jetson Node
delivery route (owner direction, 2026-10-10).** The earlier x86_64 hold was
met by Slice 110, and the owner accepted the distribution ADR on 2026-10-09.
The later `slice-160-owns-publication` ruling moved AC27-117F and every
publication action and post-publication proof to Slice 160. Slice 117 therefore
closes at its installed local candidate; this status does not claim that the
0.8.27 Tegra tarball is published or smoke-tested from Pages.

## Candidate and retained evidence

- Product source and exact workflow SHA:
  `663ed88cdfedcf741504285ea1890663603115ad`. The later
  `9d53635e77beafd6293d2b958136289b0cba2c8a` changed release planning
  records only. [Jetson Tegra CUDA evidence run
  38075698749](https://github.com/fathomadb/fathomdb/actions/runs/38075698749)
  passed its candidate guard, Jetson build/witness job and hosted driverless
  ARM64 job. Its Pages preparation, deployment and live-verification jobs were
  skipped because publication was not requested.
- The installed Node packages passed `cpu`, `auto` and `cuda:0` embedding and
  reranking. CPU resolved to CPU; auto and forced CUDA resolved to CUDA. The
  forced-CUDA embedding witness passed `verify-tegra-gpu-witness.py
  --nvidia-smi`; the heap-growth regression passed 3/3 runs with one million
  JavaScript heap objects each. The selected native addon was the Tegra
  package both alone and with a generic-package stand-in co-installed.
- The same run's hosted driverless ARM64 job installed the retained tarballs.
  Direct addon import/open/close passed with early initialization on and off,
  with no CUDA driver mapped. On that non-Tegra host the loader selected the
  generic-package stand-in rather than the Tegra addon. The generic package's
  build features, hosted route and glibc floor were unchanged by this slice;
  the stand-in checks selection, not generic addon functionality.
- SHA-256: main candidate tarball
  `90ff5b96c936c6e647877b01578db87571bdb4968029b0aec34c037e6084b863`;
  Tegra tarball
  `3ede0e23de8c39b26a4a23c960774605ed360ebb2bd941ef9c1d8fa56097aa70`;
  packed Tegra `.node`
  `525c34f4a13991bcf3d4719a73572b8e4c313ccf8fa29155d9a359a09891756e`.
  The build log records the Tegra glibc 2.35 floor, no dynamic CUDA/NVIDIA
  dependency, and an installed import without CUDA library search paths.

The only product-source changes since Slice 135's pinned measurements are the
TypeScript loader and binding selection path; this slice did not change query
or retrieval behavior. Slice 135's paired query measurements remain bound to
its own source. Slice 150 owns integrated final-candidate qualification.

## Closeout boundary and verification limit

The distribution ADR, public install and compatibility documentation, Pages
retention machinery, package build, loader, operator path and contract tests
are on the release branch. The exact Pages URL is intentionally unpublished;
public guidance labels the 0.8.27 command as future and its published digest
as pending. Slice 140 consumes this qualified candidate and reviews the final
documentation surface.

AC27-117H called for a full `./scripts/agent-verify.sh` receipt at the Slice
117 code candidate. The owner reports that build and code checks were already
done and directed closure without another run. No exact-candidate full-gate
log was found in the tracked status or run artifacts, so this closeout does
**not** claim a full-workspace green result for Slice 117. The Jetson workflow
receipt above is the hardware/artifact proof, not a substitute for that gate.
Slice 140 and Slice 150 retain their own required code verification gates.

AC27-117F remains required before the release is complete. After publication
is authorized, Slice 160 installs the main package from npm and the Tegra
tarball from Pages on the Jetson, checks the downloaded SHA-256, and repeats
forced-CUDA embedding and reranking, the GPU witness and heap-growth checks.
Slice 160 also owns the Pages live-retention proof and the pre-release
CUDA redistribution, licence and SBOM confirmation for the `.node`.

## Tracking

- Release state: Slice 117 `COMPLETE_ON_RELEASE_BRANCH` at product source
  `663ed88cd`, with Slice 140 next. The 2026-10-10
  `slice-160-owns-publication` ruling supersedes the accepted ADR's original
  post-publication closure condition.
- Todos ledger: `TC-ffef2129-e5a6-4f8f-8cae-bdfe64792197` (opened at seq
  274 and updated at seq 276 on `release/0.8.27`).
- Decision index: row 61, accepted. The generic Linux AArch64 npm package
  remains CPU-only; the separate Tegra Pages tarball is the Jetson GPU route.
