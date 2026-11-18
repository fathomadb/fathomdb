---
title: 0.8.30 CUDA usage and importance review
date: 2026-11-17
status: PROPOSED
target_release: 0.8.30
---

# 0.8.30 CUDA policy refresh

Read-only primary-source review for the [standing policy](../../../design/cuda-version-support-review.md),
retrieved November17 America/Chicago (November18 UTC). It changes no policy,
artifact target, host, driver or toolkit. The structured observations, source
URLs and uncertainty labels are in [the data summary](data/cuda-policy-observations-2026-11-17.json).
This is one planning input, not a new runtime campaign.

| Signal | Fresh observation and limit |
| --- | --- |
| Download shares/trend | UNKNOWN: comparable monthly SQL retrieval inaccessible, [ClickPy](https://clickpy.clickhouse.com/dashboard/nvidia-cuda-runtime)403, Pepy inaccessible; rolling Pypistats totals lack the required denominator. No fresh installed-major or fleet share is asserted. |
| PyTorch | REPORTED: [2.15 announcement](https://pytorch.org/blog/pytorch-2-15-release-blog/) identifiesCUDA13.2.2 as default and13.4 GA wheels with experimental Rubin support. This reinforces the prior qualitative direction, not an adoption percentage. |
| Desktop distribution | REPORTED: [llama.cpp](https://github.com/ggml-org/llama.cpp/releases) lists bothCUDA12 and13 assets, including13.5 libraries. Counts are unavailable; download split remains UNKNOWN. |
| Tegra | REPORTED: [JetPack6.2.3](https://developer.nvidia.com/embedded/jetpack-sdk-623) listsCUDA12.6.10; [7.2.1](https://developer.nvidia.com/embedded/jetpack/downloads) listsCUDA13.2.2 withThor/Orin. This does not qualify FathomDB or justify a reflash. |
| Drivers | REPORTED: [family floors](https://docs.nvidia.com/deploy/cuda-compatibility/latest/minor-version-compatibility.html) areR525 for12/R580 for13, with separate PTX restrictions; [R580 lifecycle](https://docs.nvidia.com/datacenter/tesla/drivers/620/supported-drivers-and-cuda-toolkit-versions.html) remains June2028. A family floor does not prove newer-toolkit PTX works on the fixed host. |
| Compiler safety | REPORTED: [CUDA13.2.2 notes](https://docs.nvidia.com/cuda/archive/13.2.2/cuda-toolkit-release-notes/index.html#cuda-compiler) describe6111901, introduced since12.8. Exact consumed-pipeline exposure remains UNKNOWN. |
| Peers | REPORTED documentation continues to span12/13 with differing defaults. Exact links and qualified observations are in the data summary; do not infer downloads, a justified fork upgrade or another test matrix from them. |

**Recommendation — INFERRED:** retain every accepted importance rating:
11unsupported;12.6shippingfloor; other12.x existing compatibility scope;
13.0tier1-next;13.2low/no separate artifact;13.4+ compile-only policy. No new
compile cells or artifacts are commissioned by this note. Keep R580/primary12.6
and existing support boundaries. NewCUDA13 delivery/compiler/quantitative work
remains.31; selected.30 fork work must preserve these limits.

No importance/action change is proposed for owner ratification. Unknown monthly
shares, installed drivers, desktop/JetPack fleet split and compiler exposure stay
unknown. Fresh qualitative evidence is not relabeled as measured adoption.
The owner's required.29 TegraCUDA12.6 Pages completion is separate immediate
release work; it is not a new.30/13-family allocation.
