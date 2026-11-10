---
title: 0.8.29 CUDA usage and importance review
observed_on: 2026-11-10
status: PROPOSED
review_scope: bounded; quantitative gaps
---

# CUDA version review — November 10, 2026

This is a fresh primary-source review against the October 11 baseline and accepted CUDA policy. No installs, host/GPU changes, qualification, policy ratification or product support claims occurred. Source/importance analysis is complete for this bounded pass; the requested comparable-month download refresh is NOT complete. Slice 9 must explicitly accept that limit or commission a bounded remaining collection route.

MEASURED means computed/inspected from saved raw data; REPORTED means a primary source states it; INFERRED means a recommendation. Downloads never establish installed drivers, machines, or FathomDB demand.

## Fresh evidence

| Label | Observation | Source and scope |
| --- | --- | --- |
| MEASURED | torch2.14.1 metadata pins Linux cuda-toolkit13.0.3; torch2.15.0 pins13.2.2. Upload timestamps2026-09-30T17:43:48.426734Z and2026-10-28T15:16:25.498612Z. | [primary source](https://pypi.org/pypi/torch/2.14.1/json) and [primary source](https://pypi.org/pypi/torch/2.15.0/json) ; exact requires_dist inspected, raw JSON saved. This is packaging/default evidence, not adoption. |
| REPORTED | PyTorch2.15 stops publishing CUDA12.6 binaries;2.14 is its last CUDA12 wheel release. One12.6 CI compile configuration is retained. | [primary source](https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432) |
| REPORTED | CUDA13.2.2 fixes a compiler reconvergence issue present since12.8 that can silently corrupt register values in kernels with at least two nested divergence levels. | [primary source](https://docs.nvidia.com/cuda/archive/13.2.2/cuda-toolkit-release-notes/index.html) §2.4.1 [6111901]. No FathomDB exposure has been established. |
| REPORTED | PyTorch removed13.0 from its nightly matrix from the week ofSept28, citing that compiler issue;13.2 default and13.4 prototype. | [primary source](https://dev-discuss.pytorch.org/t/removing-cuda-13-0-builds-from-pytorch-ci-cd-starting-the-week-of-sept-28/3447) .2.14 and earlier unaffected by packaging change. |
| REPORTED | Current NVIDIA release notes cover13.5 and list corresponding branches13.0/R580,13.1/R590,13.2/R595,13.3/R610,13.4/R615,13.5/R620. | [primary source](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html) .Corresponding branch is distinct from minor-compatibility floor. |
| REPORTED | CUDA13 family minor compatibility starts at580,12 at525. NVIDIA explicitly warns newer PTX will not work on older drivers. | [primary source](https://docs.nvidia.com/deploy/cuda-compatibility/minor-version-compatibility.html) .Cannot infer that FathomDB PTX built by13.2/13.5 runs onR580. |
| REPORTED | NVIDIA matrix, updatedNov4, listsR580 LTS EOLJune2028, R595 production EOLMarch2027, R620 LTS EOLSeptember2029. | [primary source](https://docs.nvidia.com/datacenter/tesla/drivers/supported-drivers-and-cuda-toolkit-versions.html) .Resolves baseline June/August R580 ambiguity to currently reportedJune. |
| REPORTED | NVIDIA architecture matrix identifiesR580 as last driver forMaxwell/Pascal/Volta; their last toolkit support isCUDA12.x. | [primary source](https://docs.nvidia.com/datacenter/tesla/drivers/cuda-toolkit-driver-and-architecture-matrix.html) .FathomDB's x86PTX floor remainsTuring75; these older GPUs do not become supported FathomDB devices. |
| REPORTED | JetPack7.2.1 includesCUDA13.2.2/Linux39.2.1/Ubuntu24.04 and supportsThor+Orin.6.2.3 includesCUDA12.6.10/Linux36.5.2/Ubuntu22.04. | [primary source](https://developer.nvidia.com/embedded/jetpack/downloads/archive-7.2.1) ; [primary source](https://developer.nvidia.com/embedded/jetpack-sdk-623) .Available software, not fleet/adoption or measured hardware compatibility. |
| REPORTED | ONNX Runtime docs still listCUDA13.0 asPyPI/NuGet default from1.27, with separateCUDA12.8+ packages. | [primary source](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html) .This source's version table extends through1.30; do not infer unlisted1.31 matrix. |

## Quantitative collection attempt and exact gaps

Target comparable complete windows: September and October2026, date>=2026-09-01 anddate<2026-11-01. Raw SQL is clickpy-query.sql. Packages: nvidia-cuda-runtime-cu11, nvidia-cuda-runtime-cu12, nvidia-cuda-runtime, nvidia-cuda-runtime-cu13. Intended denominator: total per-version CUDA-runtime downloads for each complete month, classifying actual runtime version major/minor; deprecated0.0.1 cu13 package excluded from real-runtime major/minor denominators and reported separately.

- SQL endpoint [primary source](https://sql-clickhouse.clickhouse.com/) with public demo user/query: web tool inaccessible.
- Dashboard [primary source](https://clickpy.clickhouse.com/dashboard/nvidia-cuda-runtime) :403.
- Cross-check [primary source](https://pypistats.org/api/packages/nvidia-cuda-runtime-cu12/overall?mirrors=false) :web tool inaccessible.
- [primary source](https://pepy.tech/api/v2/projects/nvidia-cuda-runtime) and [primary source](https://pepy.tech/projects/nvidia-cuda-runtime) :web tool inaccessible.
- Public pypistats pages returned reported last-month totals, but sources have inconsistent crawl ages:cu11 four months old,cu12/unsuffixed13 last month,cu13 two weeks old; search excerpts also disagree with opened page values. These are NOT comparable current windows. Do not derive a fresh percentage from them.
- [primary source](https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=5) and [primary source](https://api.github.com/repos/ollama/ollama/releases?per_page=5) :web tool inaccessible. Public latest-release pages load narrative but asset sections return loading errors; no fresh asset-download counts/share.
- Therefore fresh September/October major/minor shares, cross-source count agreement, current desktop12/13 split, and JetPack5/6/7 fleet split remain UNKNOWN. Prior October11 shares remain historical only.
- No installed-driver distribution was measured. No fresh cloud/distro default claims are made. No retry loop or shell egress to these nonregistry sources was attempted.

## Importance and action comparison — proposals, not ratification

| Version | Accepted importance/action | Fresh assessment and proposed consequence |
| --- | --- | --- |
| 11.x | Unsupported | Retain. No new share estimate; no scope to reinstate. |
| 12.6 | Tier1 shipping floor | Retain. JetPack6 remains current vendor software; isolated FathomDB artifact is not required to trackPyTorch's dropped build. Never suggestK620 or other preTuring product support. |
| Other12.x | Covered library cells; no separate artifact | Retain selected current compatibility tests. Do not replace component version numbers with toolkit minors or extend claims without loaded-library proof. |
| 13.0 | Tier1 next artifact | Proposed delivery HOLD: assess exact consumed kernel/compiler exposure to6111901 and review mitigation before delivering a13.0-built artifact. Owner ratification is required for any policy/toolkit/artifact change. No automatic switch to13.2PTX or claim that current FathomDB is affected. |
| 13.2 | Low; no separate build | Currenttorch2.15 andJetPack7.2.1 make13.2.2 an important compatibility/consumer test input. Propose prioritizing that runtime/library cell while retaining existing artifact policy pending owner ruling. This is not measured adoption and does not prove need for a separate build. |
| 13.4+ | Compile-check only |13.5 now exists; retain no-new-product-artifact policy. Any added fork compile cell requires selected scope/support. NoR620 host change. |

Decision package for120/140: preserveR580 and primary12.6, K620 ban and existing artifactPTX floors; review compiler6111901 against exact kernels/compilation pipeline, compare permissible mitigation/build choices and driver costs, then seek owner ratification if artifact/toolkit/importance policy must change.130 separately qualifies selected final pins/platforms;Q5Tegra remains unmeasured. Data-source unavailability does not imply policy changes or permission to weaken protected runner routes.

## Saved evidence and hygiene

- pypi-torch-2.14.1.json SHA256 a4370e74210b70a7834937a59272f2ed4fc4df61f383d441183e15316330b532
- pypi-torch-2.15.0.json SHA256 d4cc526b712833ee86cab592fa629f0d2b48e39b13ba0f8f58d7d8fe5f010452
- web-extract-1.txt through web-extract-10.txt preserve tool-returned extracts/access failures, not complete raw HTML or API payloads.
- clickpy-query.sql preserves the exact failed comparable-window query.
- Known live-token patterns (GitHub token prefixes, sk-keys and Bearer values) did not match saved files before import proposal. Documentation contains literal placeholders; no credentials were read.
- Registry-only urllib requests initially failed sandboxDNS, then succeeded with explicit tool approval for publicPyPI metadata. No broader shell network permission was requested or used.

## Local evidence paths

[Torch2.14.1 raw metadata](data/pypi-torch-2.14.1.json),
[Torch2.15 raw metadata](data/pypi-torch-2.15.0.json) and
[attempted SQL](data/clickpy-query.sql) are preserved alongside tool-returned
extracts/access logs `data/web-extract-1.txt` through `data/web-extract-10.txt`.
Extracts are not complete raw HTML/API responses. Parent independently opened
the NVIDIA13.2.2 release notes and PyTorch13.0 removal notice before import.
Known-token scans were repeated and metadata hashes verified at import.
