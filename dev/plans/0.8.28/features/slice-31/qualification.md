# Slice 31 qualification

FathomDB product qualification for Slice 31 (test-plan § 4). Each row links
its receipt in [qualification/](qualification/). Orin artifacts were built
from `32295692f`; later commits do not change runtime pool behaviour.

| Run | Host | Result | Receipt |
| --- | --- | --- | --- |
| T-CUDA (§ 4.1) | `windchill3`, RTX 3090 by UUID | pass | [2026-10-13-T-CUDA-3090-c82b187505a9.md](qualification/2026-10-13-T-CUDA-3090-c82b187505a9.md) |
| § 15.8 RTX 3090 row | `windchill3`, RTX 3090 by UUID | pass at `392832c72` (`slice31_cuda_pool_x86.rs`: embed dim 384, mode conflict refused with the documented message) | `status.md` |
| Orin window P and A (§ 4.2, § 4.5) | Jetson AGX Orin 64 GB | every cell pass at `32295692f`; two-model close test pass again at `fc60245a4` ([log](qualification/2026-10-15-ORIN-M1-fc60245a4.log)) | [2026-10-15-ORIN-PA-32295692f56b.md](qualification/2026-10-15-ORIN-PA-32295692f56b.md) |
| Orin window B, G1–G8 (§ 4.2) | Orin | G1, G2, G4–G7 pass; G3 pass under the ruled 0.400 bound (Python median 0.380, Node 0.210); G5 Slice 30 equality (report-only, Q28): equal; G8 accepted as measured (Q39) | [2026-10-15-ORIN-BC-32295692f56b.log](qualification/2026-10-15-ORIN-BC-32295692f56b.log), [G8 soak](qualification/2026-10-15-ORIN-G8-SOAK-32295692f56b.md) |
| Orin window C, G9 (§ 4.3, PERF-01/02) | Orin | pass: Node new/base 0.927–0.998 (embed CI upper 1.070); speed-up over pool off 2.01–4.75; import +2.2 ms; RSS +21 MiB; Python P/S ≤ 1.067 (embed CI upper 1.198, noted); Python import within S + cuInit + 5 ms | [2026-10-15-ORIN-BC-32295692f56b.log](qualification/2026-10-15-ORIN-BC-32295692f56b.log) |
| Orin G10 (§ 4.2) | full gate and fork | POOL-06 in the full gate at `392832c72`; fork FEV-05 receipt | `status.md`; cudarc `fathomdb/qualification/` |
| Orin windows P and A, first run (§ 4.2, § 4.5: D-14 stages, ORIN-13 usable capacity) | Orin | pass at `e9036a912` | [ORIN-P](qualification/2026-10-13-ORIN-P-e9036a912442.md), [ORIN-A](qualification/2026-10-13-ORIN-A-e9036a912442.md) |
| Feature-complete gate (PRD-33) | `windchill3` | pass at `9b43d29c1` | [2026-10-13-FC-9b43d29c13e0.md](qualification/2026-10-13-FC-9b43d29c13e0.md) |
| Tegra on CUDA 13 | — | UNMEASURED (owner ruling Q5; todo `TC-b267d940`, CloudJetson) | — |
| PERF-03 (§ 4.4) | `windchill3`, RTX 3090 by UUID | pass at `14ef6f4d0`: steady embed new/base 0.899 [0.889, 0.912], N = 20 per arm; identical embedding hash. The contract build has no reranker, so each process ends at the rerank step after its embed timings; `harness/g9x_embed_only.py` judges the embed measures (rerank not measured) | [2026-10-15-T-PERF-3090-14ef6f4d04e6.md](qualification/2026-10-15-T-PERF-3090-14ef6f4d04e6.md) |
| S-C3 (§ 4.6; CMP-C3, CUDA-09) | GCP Spot T4, driver 580 only | pass at `14ef6f4d0`: missing CUDA 12 cuBLAS refused (typed, doctor exit 70, CPU under `auto`, no `cuda_incompatible` on the 7.5 floor); after the pip remedy cuBLAS 12.6.4 and cuRAND 10.3.7 load and forced embed and rerank pass on the T4; `libcuda.so.1`-only passes; CUDA 13 cuBLAS alone is still refused | [2026-10-15-C3-14ef6f4d04e6.md](qualification/2026-10-15-C3-14ef6f4d04e6.md) |
| S-13 (§ 4.7; WSB-02, CUDA-08, CMP-C5) | GCP Spot T4, CUDA 13.0 toolkit | pass at `14ef6f4d0`: CUDA 13 build from the registry crates; forced embed and rerank pass; `doctor cuda-libraries` exit 0 with built major 13 and only 13.0 libraries mapped. Deviations in the receipt: `git archive` push, vendored `candle-kernels` | [2026-10-15-S13-14ef6f4d04e6.md](qualification/2026-10-15-S13-14ef6f4d04e6.md) |

The driver and CUDA guard lines were unchanged on every host and run: Orin
R36 5.2 with `/usr/local/cuda-12.6` (570 window B lines, 328 window C lines,
one distinct value), `windchill3` driver 580.178.04 with
`/usr/local/cuda-12.6`.

Not measured: the pool's reserved bytes at the SDK level (no binding
exposes pool counters; 0.8.29 todo TC-8278f17d). The embedder-level Orin
tests prove the close release.
