---
title: CUDA version support proposal for FathomDB GPU artifacts
observed_on: 2026-10-11
status: proposal (for owner decision)
---

# CUDA version support proposal

Built from [cuda-version-adoption.md](cuda-version-adoption.md) and from
FathomDB's own artifact contract. The research measures PyPI, GitHub and
Docker downloads, which are proxies; nobody publishes installed-driver data.

## What decides whether a FathomDB GPU build runs on a host

FathomDB's artifacts link no CUDA library at load time
(`ADR-0.8.25-driverless-cuda-runtime-linkage.md`). cudarc loads the driver
and cuBLAS lazily, once CUDA is selected. Candle's kernels ship as PTX for
compute capability 7.5 on x86 (`scripts/release/cuda-artifact-contract.sh:71`)
and 8.7 on the Orin. So a build made with toolkit 1X.Y needs:

1. **A driver new enough to JIT 1X.Y PTX.** A 12.6 build needs driver 560 or
   later; a 13.0 build needs R580 or later; a 13.4 build would need R615.
2. **A cuBLAS of the same major** on the library path (`libcublas.so.12` or
   `.so.13`). Torch 2.11 and later install only CUDA 13 libraries, so an
   environment shared with current PyTorch has no `libcublas.so.12`.
   Whether cudarc's library-name candidates can fall back across majors is
   unverified and must not be assumed.
3. **A Turing or newer GPU on x86.** The compute 7.5 PTX already excludes
   Maxwell, Pascal and Volta, so CUDA 13 dropping those architectures costs
   FathomDB nothing extra.

The installed toolkit's version doesn't matter. What matters is the driver
branch and which CUDA library major is present.

## Proposed importance per version

| Version | Evidence (Sep–Oct 2026) | Importance | Proposal |
| --- | --- | --- | --- |
| 11.x | ~3 % of PyPI CUDA runtime downloads, falling; JetPack 5 is ~27 % of Ollama's Jetson downloads | **None** | Do not support. JetPack 5 Orin users can move to JetPack 6; Xavier stays CPU. Leave it to cudarc's own compile matrix |
| 12.6 | the JetPack 6 fleet (Orin, most of Jetson through 2027); drivers 560–575; environments with CUDA 12 libraries | **Tier 1 — the shipping floor** | Keep as the 0.8.x baseline for x86 and Jetson. Hold it until JetPack 6 Orin and pre-580 drivers fade, at least through 2027 |
| 12.4, 12.8, 12.9 | 8.5 %, 14–15 %, 3–4 % of downloads | **Covered, no separate build** | A 12.6 build runs on these hosts' drivers. If their cuBLAS 12 is newer than 12.6, it serves the 12.6 build. A 12.4 cuBLAS may lack symbols that only fail when called; record that as a known gap, not a build. Compile coverage comes from the cudarc fork matrix |
| 13.0 | ~58 % of downloads and 96 % of 13.x; the default for PyTorch 2.11–2.14 and ONNX Runtime 1.27+; PyTorch 2.15 is 13.x-only; R580 is the LTS driver branch (EOL mid-2028) | **Tier 1 next — the second artifact** | Qualify in 0.8.28 (planned toolkit cells). Ship a 13.0 artifact beside 12.6 in the release after 0.8.28 (0.8.29 or 0.9). It runs on every 13.x driver, Thor, and Orin on JetPack 7.2 |
| 13.2 | ~0.6 %; JetPack 7.2 ships 13.2.1; AWS 24.04 images use 595 + 13.2 | **Low — runs the 13.0 build** | No separate build. The Q5 Tegra follow-up measures the 13.0 build on JetPack 7.2. Compile coverage from the fork matrix |
| 13.4 | ~1 %, rising; needs R615 for its new features | **Compile-check only** | Never build artifacts with 13.4: its PTX would need R615 and lock out drivers 580–610. `windchill3` must never run R615 (Q4) |

## Consequences for Slice 31 and after

- **Slice 31 is unchanged.** It ships 12.6, proves 13.0 builds, and
  compile-checks through 13.4 in the fork.
- **Two things to verify in Slice 31** (proposed additions to T-CUDA):
  - the driver floor: a 12.6 build's PTX needs driver 560 or later. The
    docs should state that, not CUDA 12's 525.
  - cudarc's cuBLAS library-name candidates: whether a 12.6 build can load
    in an environment that has only CUDA 13 libraries. If it can't, the
    error must say so plainly.
- **Follow-up todo:** a 13.0 artifact alongside 12.6 in the next release,
  including the Orin on JetPack 7.2 once Q5 is measured. Artifact naming and
  `auto` selection between the two are design work for that release.
- **Re-check this proposal** when PyTorch 2.15 ships (13.x only) and when
  NVIDIA publishes the R580 end of life.
