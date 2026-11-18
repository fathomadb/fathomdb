---
title: 0.8.30 CUDA usage and importance review
date: 2026-11-17
status: PROPOSED
target_release: 0.8.30
---

# 0.8.30 CUDA policy refresh

Read-only primary-source review for the [standing policy](../../../design/cuda-version-support-review.md),
retrieved November 17 America/Chicago (November 18 UTC). It changes no policy,
artifact target, host, driver or toolkit. The structured observations, source
URLs and uncertainty labels are in [the data summary](data/cuda-policy-observations-2026-11-17.json).
This is one planning input, not a new runtime campaign.

| Signal | Fresh observation and limit |
| --- | --- |
| Download shares/trend | UNKNOWN: comparable monthly SQL retrieval inaccessible, [ClickPy](https://clickpy.clickhouse.com/dashboard/nvidia-cuda-runtime) 403, Pepy inaccessible; rolling Pypistats totals lack the required denominator. No fresh installed-major or fleet share is asserted. |
| PyTorch | REPORTED: [2.15 announcement](https://pytorch.org/blog/pytorch-2-15-release-blog/) identifies CUDA 13.2.2 as default and 13.4 GA wheels with experimental Rubin support. This reinforces the prior qualitative direction, not an adoption percentage. |
| Desktop distribution | REPORTED: [llama.cpp](https://github.com/ggml-org/llama.cpp/releases) lists both CUDA 12 and 13 assets, including 13.5 libraries. Counts are unavailable; download split remains UNKNOWN. |
| Tegra | REPORTED: [JetPack 6.2.3](https://developer.nvidia.com/embedded/jetpack-sdk-623) lists CUDA 12.6.10; [7.2.1](https://developer.nvidia.com/embedded/jetpack/downloads) lists CUDA 13.2.2 with Thor/Orin. This does not qualify FathomDB or justify a reflash. |
| Drivers | REPORTED: [family floors](https://docs.nvidia.com/deploy/cuda-compatibility/latest/minor-version-compatibility.html) are R525 for 12/R580 for 13, with separate PTX restrictions; [R580 lifecycle](https://docs.nvidia.com/datacenter/tesla/drivers/620/supported-drivers-and-cuda-toolkit-versions.html) remains June 2028. A family floor does not prove newer-toolkit PTX works on the fixed host. |
| Compiler safety | REPORTED: [CUDA 13.2.2 notes](https://docs.nvidia.com/cuda/archive/13.2.2/cuda-toolkit-release-notes/index.html#cuda-compiler) describe 6111901, introduced since 12.8. Exact consumed-pipeline exposure remains UNKNOWN. |
| Peers | REPORTED documentation continues to span 12/13 with differing defaults. Exact links and qualified observations are in the data summary; do not infer downloads, a justified fork upgrade or another test matrix from them. |

**Recommendation — INFERRED:** retain every accepted importance rating:
11 unsupported; 12.6 shipping floor; other 12.x existing compatibility scope;
13.0 tier1-next; 13.2 low/no separate artifact; 13.4+ compile-only policy. No new
compile cells or artifacts are commissioned by this note. Keep R580/primary 12.6
and existing support boundaries. New CUDA 13 delivery/compiler/quantitative work
remains .31; selected .30 fork work must preserve these limits.

No importance/action change is proposed for owner ratification. Unknown monthly
shares, installed drivers, desktop/JetPack fleet split and compiler exposure stay
unknown. Fresh qualitative evidence is not relabeled as measured adoption.
The owner's required .29 Tegra CUDA 12.6 Pages completion is separate immediate
release work; it is not a new .30/13-family allocation.
