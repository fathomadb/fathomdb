# Slice 31 qualification

FathomDB product qualification for Slice 31 (test-plan § 4). Each row links
its receipt in [qualification/](qualification/). Orin artifacts were built
from `32295692f`; later commits do not change runtime pool behaviour.

| Run | Host | Result | Receipt |
| --- | --- | --- | --- |
| T-CUDA (§ 4.1) | `windchill3`, RTX 3090 by UUID | pass | [2026-10-13-T-CUDA-3090-c82b187505a9.md](qualification/2026-10-13-T-CUDA-3090-c82b187505a9.md) |
| § 15.8 RTX 3090 row | `windchill3`, RTX 3090 by UUID | pass at `392832c72` (`slice31_cuda_pool_x86.rs`: embed dim 384, mode conflict refused with the documented message) | `status.md` |
| Orin window P and A (§ 4.2, § 4.5) | Jetson AGX Orin 64 GB | every cell pass at `32295692f`; two-model close test pass again at `fc60245a4` | [2026-10-15-ORIN-PA-32295692f56b.md](qualification/2026-10-15-ORIN-PA-32295692f56b.md) |
| Orin window B, G1–G8 (§ 4.2) | Orin | G1, G2, G4–G7 pass; G3 pass under the ruled 0.400 bound (Python median 0.380, Node 0.210); G5 Slice 30 equality (report-only, Q28): equal; G8 accepted as measured (Q39) | [2026-10-15-ORIN-BC-32295692f56b.log](qualification/2026-10-15-ORIN-BC-32295692f56b.log), [G8 soak](qualification/2026-10-15-ORIN-G8-SOAK-32295692f56b.md) |
| Orin window C, G9 (§ 4.3, PERF-01/02) | Orin | pass: Node new/base 0.927–0.998 (embed CI upper 1.070); speed-up over pool off 2.01–4.75; import +2.2 ms; RSS +21 MiB; Python P/S ≤ 1.067 (embed CI upper 1.198, noted); Python import within S + cuInit + 5 ms | [2026-10-15-ORIN-BC-32295692f56b.log](qualification/2026-10-15-ORIN-BC-32295692f56b.log) |
| PERF-03 (§ 4.4) | `windchill3`, RTX 3090 by UUID | pending | — |
| S-C3 (§ 4.6) | GCP T4 | pending | — |
| S-13 (§ 4.7) | GCP | pending | — |

The driver and CUDA guard lines were unchanged on every host and run: Orin
R36 5.2 with `/usr/local/cuda-12.6` (570 window B lines, 328 window C lines,
one distinct value), `windchill3` driver 580.178.04 with
`/usr/local/cuda-12.6`.

Not measured: the pool's reserved bytes at the SDK level (no binding
exposes pool counters; 0.8.29 todo TC-8278f17d). The embedder-level Orin
tests prove the close release.
