---
title: CUDA version adoption across the GPU user base (Apr 2025 to Oct 2026)
observed_on: 2026-10-11
status: research
---

# CUDA version adoption across the GPU user base

All retrievals happened on 2026-10-11. Each claim carries one of three labels:
**MEASURED** (computed from fetched data saved under `data/`),
**REPORTED** (stated by a cited source), and **INFERRED** (this author's
reasoning).

## Summary

- **The ecosystem flipped to CUDA 13 in Q2 2026.** In the PyPI
  CUDA-runtime wheel downloads, CUDA 13.x went from 0.3 % (2025 Q4) to
  6.5 % (2026 Q1), 53.5 % (2026 Q2) and 60.7 % (2026 Q3). Over the same
  quarters CUDA 12.x fell from 94.5 % to 36.1 %. CUDA 11.x is about 3 %.
  (MEASURED, ClickPy; table in the 18-month history section.) The cause is
  PyTorch 2.11 (PyPI, 2026-03-23), which made CUDA 13.0 the default `pip
  install torch` build on Linux x86_64 and aarch64 (MEASURED from PyPI
  `requires_dist`, REPORTED by PyTorch).
- **Within CUDA 13, nearly everything is 13.0.** PyTorch 2.11 to 2.14 pin
  `cuda-toolkit==13.0.x`. CUDA 13.1, 13.2, 13.3 and 13.4 runtime wheels are
  each at or below 1.5 % of runtime downloads (MEASURED).
- **Within CUDA 12, the long tail is 12.8 > 12.4 > 12.6 > 12.1 > 12.9.** Those
  minors are exactly the ones that older PyTorch pins (2.8 to 2.10, 2.5 to 2.6, 2.7,
  2.1 to 2.4) and vLLM (cu129) pull in. The distribution reflects lagging
  pins, not toolkit choice (MEASURED plus INFERRED).
- **End-user binaries split more evenly than pip does.** For llama.cpp's
  prebuilt CUDA binaries over the last 17 days, Windows x64 downloads were
  12.4 = 48 % and 13.4 = 52 %; Ubuntu x64 downloads were 12.8 = 60 % and
  13.4 = 40 % (MEASURED, GitHub asset counts). A large share of real desktop
  users still choose, or need, a CUDA 12 build.
- **The driver is the hard bound.** Every CUDA 13.x app needs driver 580 or
  later. CUDA 12.x apps need 525 or later (REPORTED, NVIDIA release notes).
  R580 is the last branch that supports Maxwell, Pascal and Volta, and CUDA
  13.0 toolchains no longer target those GPUs. Owners of those GPUs can only
  use CUDA 12.x builds (REPORTED). On Steam, Maxwell and Pascal GeForce cards
  account for about 4.3 % of all GPUs (Sep 2026, REPORTED).
- **Jetson Orin mostly runs JetPack 6 (CUDA 12.6).** JetPack 7.2 (June 2026,
  CUDA 13.2.1) is the first JetPack 7 release that supports Orin (REPORTED).
  JetPack 7.0 and 7.1 support only Thor. In Ollama's Jetson-specific assets
  for Jan to Sep 2026, JetPack 6 accounts for 73 % and JetPack 5 (CUDA 11.4)
  for 27 % (MEASURED; bot noise caveat below).
- **Implication for FathomDB (INFERRED).** CUDA 12.6 remains the right floor
  for x86_64 and Jetson. It covers driver 560 and later, every Turing-and-later
  GPU, Pascal and Volta, and the whole JetPack 6 fleet. A CUDA 13.0 build would
  serve the majority of the PyTorch-era Linux user base on driver 580 and
  later. Building against 13.1 to 13.4 serves almost no one extra, and it
  risks needing a newer driver for PTX JIT (see the driver-branch section).

### Headline share estimates (as of Sep to Oct 2026)

These estimates describe the population of **active GPU ML environments**.
The proxy is PyPI downloads, cross-checked against end-user binaries. They do
not describe the hardware fleet.

| Version | Share, pip proxy | Share, end-user proxy | Best estimate (range) | Confidence |
| --- | --- | --- | --- | --- |
| 11.x (mostly 11.7/11.8) | 3.0 % | ~0 % (llama.cpp dropped 11.7 in mid 2025) | 1–4 % | medium |
| 12.0–12.3, 12.5 | <1 % combined | n/a | <1 % | medium |
| 12.1 | 4.1–4.4 % | n/a | 3–5 % | medium |
| 12.4 | 8.4–8.6 % | Win 12.4 build: ~48 % of llama.cpp Win DLs | 8–15 % | low–medium |
| 12.6 | 5.2–5.8 % | Jetson JP6: ~73 % of Jetson DLs | 5–10 % (x86); most Orin | medium |
| 12.8 | 14.2–15.0 % | Ubuntu 12.8 build: ~60 % of llama.cpp Linux DLs | 13–20 % | medium |
| 12.9 | 3.1–3.6 % | n/a | 3–5 % | medium |
| 12.x total | 36 % | 48–60 % of llama.cpp | 35–55 % | medium |
| 13.0 | 58.3–58.5 % | n/a | 50–60 % | medium |
| 13.1 | ~0.1 % | (llama.cpp shipped 13.1 Jan–May 2026) | <1 % | medium |
| 13.2 | 0.6 % | n/a | <2 % | medium |
| 13.3 | 0.3–0.5 % | (llama.cpp shipped 13.3 Jun–Aug 2026) | <2 % | medium |
| 13.4 | 0.9–1.2 % (rising) | 40–52 % of llama.cpp (current 13.x build) | 1–3 % (pip) | low |
| 13.x total | 61 % | 40–52 % of llama.cpp | 45–61 % | medium |

How to read the columns:

- **Pip proxy** shares are of `nvidia-cuda-runtime*` downloads in Sep 2026 and
  in Oct 2026 month-to-date (MEASURED, ClickPy).
- **End-user proxy** shares are of llama.cpp and Ollama release-asset
  downloads (MEASURED, GitHub).
- **Best estimate** is INFERRED by combining the two proxies.

The llama.cpp "13.4" builds only show that a user's driver can run *some*
13.x build. Under minor-version compatibility, 13.x apps run on driver 580
and later. The figure therefore measures driver capability at 580 or later,
not uptake of the 13.4 toolkit.

## Method and sources

1. **PyPI per-version downloads, full history.** These come from ClickPy, the
   public ClickHouse mirror of the PyPI BigQuery dataset, via the read-only
   `demo` user at <https://sql-clickhouse.clickhouse.com/>. The table is
   `pypi.pypi_downloads_per_day_by_version` and covers 2022-11-16 to
   2026-10-10. The query text is in `data/clickpy-queries.sql`. Packages:
   - `nvidia-cuda-runtime-cu11` (11.x)
   - `nvidia-cuda-runtime-cu12` (12.x)
   - `nvidia-cuda-runtime` (13.x; NVIDIA dropped the `-cuNN` suffix for 13)

   Versions are bucketed to `major.minor`. Outputs:
   - `data/clickpy-cuda-runtime-monthly-by-version.csv` (raw)
   - `data/clickpy-cuda-runtime-monthly-share-by-minor.csv`
   - `data/clickpy-cuda-runtime-quarterly-share-by-major.csv`
2. **Cross-checks.**
   - pypistats.org `/recent` and `/overall?mirrors=false`, which cover
     2026-04-08 onward (`data/pypistats-*.json`).
   - pepy.tech `/api/v2/projects/<pkg>`, a 91-day per-version window
     (2026-07-12 to 2026-10-10), saved as `data/pepy-v2-*.json` and
     `data/pepy-per-version-monthly.csv`.

   Both agree with ClickPy on direction and on approximate shares.
3. **Related packages, monthly totals.** `torch`, `cuda-toolkit`, the jax CUDA
   plugins, `cupy-cuda{11,12,13}x`, `onnxruntime-gpu`, `vllm`, `numpy` and
   others are in `data/clickpy-monthly-totals-related.csv`.
4. **PyTorch's pinned CUDA per release.** This comes from the PyPI JSON
   `requires_dist` for torch 2.6.0 to 2.14.1
   (`data/pypi-torch-<ver>.json`, <https://pypi.org/pypi/torch/2.11.0/json>
   and so on).
5. **End-user binaries.** GitHub release asset `download_count` values from:
   - llama.cpp: the last 300 releases, 2026-09-24 to 2026-10-11, plus one
     sampled release roughly every 400 builds from b4800 (2025-03) to b11200
     (2026-09).
   - Ollama: 200 releases, 2024-05 to 2026-10.

   Raw release JSON was reduced to CSV: `data/gh-llamacpp-*.csv` and
   `data/gh-ollama-release-assets.csv`. The full JSON (32 MB) was not kept.
6. **NVIDIA primary pages**, saved under `data/pages/`:
   - CUDA Toolkit release notes:
     <https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html>
   - CUDA Toolkit archive: <https://developer.nvidia.com/cuda-toolkit-archive>
   - JetPack archive: <https://developer.nvidia.com/embedded/jetpack-archive>
7. **PyTorch dev-discuss packaging notices**, saved under `data/pages/`.
8. **Steam hardware survey (video cards)**, saved under `data/pages/`.
9. **Web search** for driver-branch lifecycle, Ubuntu packaging, AWS DLAMI,
   vLLM, ONNX Runtime and Jetson details. These are REPORTED, with URLs inline.

**What these proxies measure.**

- `nvidia-cuda-runtime*` downloads are mostly transitive dependencies of
  `torch` on Linux, and to a lesser degree of jax, vLLM, cupy and others. Most
  of them come from CI, Docker builds and ephemeral cloud VMs, not distinct
  humans. They show which CUDA user-space PyTorch-era environments install.
  They do not show which driver the host has: a CI runner without a GPU still
  downloads them.
- GitHub asset counts for llama.cpp and Ollama are closer to end users,
  especially on Windows, but they include bots and mirrors.
- No source measures the installed **driver** distribution (see the gaps
  section).

## Per-version findings

Release dates are REPORTED from the NVIDIA CUDA Toolkit archive (retrieved
2026-10-11). Shares are MEASURED from the ClickPy runtime-wheel downloads.
"Sep-26" is the full month of Sep 2026 and "Oct-26" is Oct 1 to 10.

| Version | GA release | Runtime-wheel share: Apr-25 / Oct-25 / Apr-26 / Sep-26 / Oct-26 | Trend | Main puller (INFERRED from pins) | Confidence |
| --- | --- | --- | --- | --- | --- |
| 11.7 | 2022 | 10.4 / 4.9 / 2.8 / 2.6 / 2.5 % | slow decline, sticky | torch 1.13 (`11.7.99`) | high (as downloads) |
| 11.8 | 2022-10 | 1.4 / 0.8 / 0.7 / 0.5 / 0.5 % | decline | torch cu118 index, TF, older stacks | high |
| 12.0 | 2022-12 | ~0 % | gone | — | high |
| 12.1 | 2023-02 | 22.8 / 17.5 / 7.7 / 4.4 / 4.1 % | steady decline | torch 2.1 to 2.4 (`12.1.105`) | high |
| 12.2 / 12.3 | 2023-06 / 2023-10 | ≤0.1 % each | gone (on x86 pip) | — | high |
| 12.4 | 2024-03 | 55.5 / 19.4 / 7.8 / 8.6 / 8.4 % | fell, then flat ~8 % | torch 2.5 to 2.6 (`12.4.127`); torch 2.6.0 is still 12.7 % of torch DLs | high |
| 12.5 | 2024-05 | 0.4 / 2.2 / 0.2 / 0.2 / 0.1 % | negligible | — | high |
| 12.6 | 2024-08 | 8.2 / 7.2 / 4.8 / 5.2 / 5.8 % | flat 5–7 % | torch 2.7 (`12.6.77`), cu126 legacy wheels | high |
| 12.8 | 2025-01 | 1.1 / 45.8 / 24.1 / 15.0 / 14.2 % | peaked Feb-26 (60.8 %), now falling | torch 2.8 to 2.10 (`12.8.90`) | high |
| 12.9 | 2025-05 | 0 / 1.7 / 2.9 / 3.1 / 3.6 % | small, slowly rising | vLLM default cu129, jax-cuda12 | high |
| 13.0 | 2025-08 | 0 / 0.1 / 48.3 / 58.3 / 58.5 % | dominant since Apr-26 | torch 2.11 to 2.14 (`cuda-toolkit==13.0.x`) | high |
| 13.1 | 2025-12 | 0 / 0 / 0.0 / 0.0 / 0.1 % | never adopted | — | high |
| 13.2 | 2026-03 | — / — / 0.3 / 0.6 / 0.6 % | small | torch cu132 (experimental), JetPack 7.2 | high |
| 13.3 | 2026-05 | — / — / — / 0.5 / 0.3 % | blip (1.5 % Jun to Aug) | — | medium |
| 13.4 | 2026-07 (DP), 2026-09 (GA) | — / — / — / 0.9 / 1.2 % | rising | torch nightly cu134 prototype | medium |

Further notes:

- The Oct-25 13.0 figure is 0.1 % (ClickPy 2025-10). (MEASURED)
- On cuBLAS naming: `nvidia-cublas` 13.1.x ships *with CUDA 13.0* (torch 2.12
  pins `nvidia-cublas>=13.1.0.3,<=13.1.1.3`). Per-library version numbers do
  not equal toolkit versions, so this analysis uses the cudart wheel.
  (MEASURED, PyPI metadata)
- pepy, as a cross-check: the 91-day window (2026-07-12 to 10-10) covers
  `nvidia-cuda-runtime-cu11` 5.0 M, `-cu12` 57.6 M and `nvidia-cuda-runtime`
  (13.x) 98.1 M. That gives 3.1 % / 35.8 % / 61.1 %. Within 13.x, `13.0.96`
  is 95.9 %. (MEASURED, <https://pepy.tech/api/v2/projects/nvidia-cuda-runtime>)

## 18-month history

### Quarterly CUDA major share of runtime-wheel downloads (MEASURED, ClickPy)

| Quarter | Downloads (all three pkgs) | 11.x | 12.x | 13.x | PyTorch PyPI default in effect |
| --- | --- | --- | --- | --- | --- |
| 2025 Q1 | 77.3 M | 19.2 % | 80.8 % | 0.0 % | 2.6 → cu124 |
| 2025 Q2 | 95.0 M | 10.7 % | 89.3 % | 0.0 % | 2.7 (2025-04-23) → cu126 |
| 2025 Q3 | 119.4 M | 7.9 % | 92.1 % | 0.0 % | 2.8 (2025-08-06) → cu128 |
| 2025 Q4 | 124.5 M | 5.1 % | 94.5 % | 0.3 % | 2.9 (2025-10-15) → cu128 |
| 2026 Q1 | 136.3 M | 4.6 % | 88.9 % | 6.5 % | 2.10 (2026-01-21) cu128; 2.11 (2026-03-23) → cu130 |
| 2026 Q2 | 183.6 M | 3.3 % | 43.1 % | 53.5 % | 2.12 (2026-05-13) → cu130 |
| 2026 Q3 | 167.9 M | 3.2 % | 36.1 % | 60.7 % | 2.13 (2026-07-08), 2.14 (2026-09-02) → cu130 |
| 2026 Q4 (to 10-10) | 13.2 M | 3.0 % | 36.2 % | 60.7 % | 2.14.1 (2026-09-30) → cu130 |

### Monthly share by minor version, Apr 2025 to Oct 2026 (MEASURED, ClickPy)

Each cell is the percentage of that month's runtime-wheel downloads. The
source is `data/clickpy-cuda-runtime-monthly-share-by-minor.csv`.

| Month | 11.7 | 11.8 | 12.1 | 12.4 | 12.6 | 12.8 | 12.9 | 13.0 | 13.2 | 13.3 | 13.4 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 2025-04 | 10.4 | 1.4 | 22.8 | 55.5 | 8.2 | 1.1 | 0.0 | 0.0 | 0.0 | 0.0 | 0.0 |
| 2025-05 | 8.0 | 1.4 | 19.6 | 33.2 | 35.7 | 0.7 | 0.8 | 0.0 | 0.0 | 0.0 | 0.0 |
| 2025-06 | 9.4 | 1.4 | 19.7 | 28.4 | 38.9 | 0.8 | 0.8 | 0.0 | 0.0 | 0.0 | 0.0 |
| 2025-07 | 7.8 | 1.4 | 17.7 | 27.2 | 43.2 | 1.1 | 0.9 | 0.0 | 0.0 | 0.0 | 0.0 |
| 2025-08 | 6.2 | 0.9 | 14.7 | 18.1 | 16.1 | 41.6 | 0.8 | 0.0 | 0.0 | 0.0 | 0.0 |
| 2025-09 | 6.4 | 1.0 | 18.4 | 15.9 | 9.6 | 45.2 | 1.8 | 0.0 | 0.0 | 0.0 | 0.0 |
| 2025-10 | 4.9 | 0.8 | 17.5 | 19.4 | 7.2 | 45.8 | 1.7 | 0.1 | 0.0 | 0.0 | 0.0 |
| 2025-11 | 4.2 | 0.6 | 14.5 | 12.5 | 9.3 | 53.3 | 2.1 | 0.4 | 0.0 | 0.0 | 0.0 |
| 2025-12 | 4.2 | 0.7 | 12.8 | 11.4 | 10.0 | 55.0 | 2.2 | 0.3 | 0.0 | 0.0 | 0.0 |
| 2026-01 | 4.6 | 0.6 | 13.5 | 10.1 | 11.4 | 54.4 | 2.3 | 0.3 | 0.0 | 0.0 | 0.0 |
| 2026-02 | 3.9 | 0.8 | 9.2 | 9.3 | 9.5 | 60.8 | 3.3 | 0.5 | 0.0 | 0.0 | 0.0 |
| 2026-03 | 3.2 | 0.9 | 8.3 | 9.8 | 5.6 | 53.8 | 2.4 | 15.0 | 0.2 | 0.0 | 0.0 |
| 2026-04 | 2.8 | 0.7 | 7.7 | 7.8 | 4.8 | 24.1 | 2.9 | 48.3 | 0.3 | 0.0 | 0.0 |
| 2026-05 | 2.8 | 0.4 | 6.9 | 8.2 | 5.3 | 18.8 | 3.0 | 53.8 | 0.2 | 0.1 | 0.0 |
| 2026-06 | 2.8 | 0.4 | 5.9 | 8.7 | 5.7 | 16.4 | 2.7 | 55.5 | 0.1 | 1.5 | 0.0 |
| 2026-07 | 3.0 | 0.3 | 5.5 | 8.3 | 7.1 | 14.6 | 1.9 | 57.3 | 0.2 | 1.4 | 0.0 |
| 2026-08 | 2.6 | 0.4 | 4.8 | 8.5 | 5.6 | 13.0 | 2.0 | 60.8 | 0.4 | 1.4 | 0.2 |
| 2026-09 | 2.6 | 0.5 | 4.4 | 8.6 | 5.2 | 15.0 | 3.1 | 58.3 | 0.6 | 0.5 | 0.9 |
| 2026-10* | 2.5 | 0.5 | 4.1 | 8.4 | 5.8 | 14.2 | 3.6 | 58.5 | 0.6 | 0.3 | 1.2 |

\* Oct 2026 covers the 1st to the 10th. Column 13.1 is omitted because it is
≤0.1 % in every month. Minors 12.0, 12.2, 12.3 and 12.5 are omitted because
each is ≤3 %, and usually <1 %.

**Absolute-volume anomaly (MEASURED).** Sep 2026 totals drop about 30 to 40 %
from Aug 2026 for *every* package checked, including `numpy` (1,110.8 M →
801.5 M) and `torch` (87.4 M → 61.7 M). pypistats shows the same drop. This
looks like an upstream BigQuery/linehaul collection effect, not a real usage
change (INFERRED). Use shares, not absolute counts, for Sep to Oct 2026.

### End-user binary history: llama.cpp Windows x64 (MEASURED)

These are cumulative downloads of one sampled release roughly every 400 builds
(`data/gh-llamacpp-sampled-release-cuda-assets.csv`). Each release is
downloaded for only a few days, so read the shares rather than the levels.

| Release (date) | CUDA builds offered, Windows x64 | 13.x share of CUDA DLs |
| --- | --- | --- |
| b4800 (2025-03-02) | 11.7, 12.4 | 0 % (11.7 = 19 %) |
| b5200 (2025-04-27) | 11.7, 12.4 | 0 % (11.7 = 28 %) |
| b5600 (2025-06-06) | 12.4 only (11.7 dropped) | 0 % |
| b6401 to b7200 (2025-09 to 11) | 12.4 only | 0 % |
| b7600 (2026-01-01) | 12.4, 13.1 | 43 % |
| b8001 (2026-02-11) | 12.4, 13.1 | 53 % |
| b8802 (2026-04-15) | 12.4, 13.1 | 58 % |
| b9200 (2026-05-17) | 12.4, 13.1 | 47 % |
| b9601 (2026-06-11) | 12.4, 13.3 | 59 % |
| b10000 (2026-07-14) | 12.4, 13.3 | 48 % |
| b10400 (2026-08-13) | 12.4, 13.3 (+13.4 arm64) | 41 % |
| last 300 releases (09-24 to 10-11) | 12.4, 13.4; Ubuntu 12.8 and 13.4 added | Win 52 %, Ubuntu x64 40 % |

INFERRED: llama.cpp's Windows audience has split roughly 50/50 between CUDA 12
and CUDA 13 builds for nine months. The split has not trended toward 13.x
since spring 2026. The CUDA 12 build is offered first and is
driver-conservative, so some users with 13-capable drivers pick it anyway. The
50 % figure is therefore an upper bound on users who *require* CUDA 12, not a
measurement.

## Driver-branch constraints

All REPORTED from the NVIDIA CUDA Toolkit release notes, Tables 2 and 3
(<https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html>,
retrieved 2026-10-11; saved copy in `data/pages/`):

| CUDA toolkit | Driver branch | Runs under minor-version compat on |
| --- | --- | --- |
| 13.4 (GA Sep 2026; 13.4.2 latest) | R615 | ≥580 |
| 13.3 | R610 | ≥580 |
| 13.2 | R595 | ≥580 |
| 13.1 | R590 | ≥580 |
| 13.0 | R580 | ≥580 |
| 12.x | 525 to 575 | ≥525 |
| 11.x | 450 to 520 | ≥450 (450.80.02 Linux) |

Selected 12.x Linux x86_64 minimums (toolkit-native): 12.4 GA ≥550.54.14,
12.6 GA ≥560.28.03, 12.8 GA ≥570.26, 12.9 GA ≥575.51.03. From 13.x on, NVIDIA
lists a branch rather than a minimum version. The Linux toolkit stopped
bundling the driver at 13.4.

- **Does 13.1 or later need R615? No (REPORTED).** "Existing CUDA 13.x
  applications run on drivers >=580 under CUDA minor version compatibility."
  Only *new features and newly enabled platforms* of 13.4 need R615. The
  caveat is INFERRED from the NVIDIA minor-version-compat rules: under minor
  compat, a driver older than the toolkit cannot JIT PTX produced by the newer
  toolkit. Candle loads its kernels as PTX through cudarc. A FathomDB build
  whose kernels are compiled as PTX by toolkit 13.N may therefore need the
  matching R-branch driver unless it ships SASS (cubin/fatbin) for the target
  archs. The same logic applies to the current 12.6 build: PTX from 12.6
  needs driver ≥560, not just ≥525. **Verify in Slice 31 how the
  candle-kernels PTX is produced and loaded.**
- **Old GPUs (REPORTED).**
  - NVIDIA says the R580 series is the last to support Maxwell, Pascal and
    Volta; Linux 590 no longer recognises them
    ([Guru3D](https://www.guru3d.com/story/nvidia-ends-linux-driver-support-for-gtx-900-and-10-series-gpus/),
    [Notebookcheck](https://www.notebookcheck.net/Nvidia-confirms-upcoming-end-of-driver-support-for-GTX-700-900-and-10-series-GPUs.1048442.0.html)).
  - The CUDA 13.0 libraries (cuFFT, cuSPARSE) dropped pre-Turing (release
    notes).
  - PyTorch's CUDA 13 builds cover Turing (sm_75) through Blackwell, and its
    cu126 wheels exist solely to keep sm_50, sm_60 and sm_70 working
    ([PyTorch notice, 2026-09-04](https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432)).

  INFERRED: Pascal, Maxwell and Volta owners are capped at driver 580. In
  practice they are also capped at CUDA 12.x builds, because CUDA 13
  toolchains cannot target their architectures. A CUDA 12.6 build is the only
  way FathomDB can serve them.
- **Size of the old-GPU population (REPORTED).** In the Steam Hardware Survey
  for Sep 2026, GTX 9xx and 10xx cards listed individually total about 4.3 %
  of all Steam GPUs: GTX 970 0.16, 1050 0.47, 1050 Ti 1.03, 1060 1.34, 1070
  0.54, 1070 Ti 0.18, 1080 0.34 and 1080 Ti 0.27
  (<https://store.steampowered.com/hwsurvey/videocard/>). Gamers are not
  FathomDB's audience; treat this as a rough floor for consumer machines
  stuck on R580.
- **Lifecycle (REPORTED).**
  - R580 is a Long-Term Support Branch with EOL in mid-2028. NVIDIA sources
    disagree between June and August 2028
    ([NVIDIA datacenter matrix](https://docs.nvidia.com/datacenter/tesla/drivers/supported-drivers-and-cuda-toolkit-versions.html),
    [endoflife.ai](https://endoflife.ai/nvidia/R580-Linux)).
  - R595 is a Production Branch with EOL around Mar 2027.
  - R610 and R615 are New Feature Branches. Driver 615 released around
    Sep 2026, per secondary sources.
- **Distro and cloud defaults (REPORTED).**
  - Ubuntu 24.04 ships `nvidia-driver-580` in `restricted` (580.159.03 in
    updates) and `nvidia-driver-595` in `multiverse` (595.84)
    ([ubuntuupdates 580](https://ubuntuupdates.org/package/core/noble/restricted/updates/nvidia-driver-580),
    [ubuntuupdates 595](https://ubuntuupdates.org/package/core/noble/multiverse/updates/nvidia-driver-595)).
    No 610 or 615 packaging was found.
  - AWS base-GPU DLAMI (June 2026): the Ubuntu 22.04 image has driver
    580.167.08 and CUDA 13.0; the Ubuntu 24.04 image has driver 595.71.05 and
    CUDA 13.2
    ([AWS DLAMI notes, CN mirror](https://docs.amazonaws.cn/dlami/latest/devguide/aws-deep-learning-ami-gpubasesinglecuda-ul2204-2026-05-28.md)).
  - INFERRED: the mainstream managed Linux driver in late 2026 is R580, then
    R595. A CUDA 13.0 build covers both; a build that hard-requires R610 or
    R615 does not.

## Jetson

| JetPack | L4T | CUDA | Released | Modules | Source |
| --- | --- | --- | --- | --- | --- |
| 5.1.x (to 5.1.7) | 35.x | 11.4 | 2023 to 2025 | AGX/NX Xavier, all Orin | REPORTED: JetPack archive |
| 6.0 | 36.3 | 12.2 | 2024 | Orin only | REPORTED |
| 6.1 / 6.2 / 6.2.1 | 36.4.x | 12.6 (6.2.1: 12.6.10) | 2024-12 to 2025 | Orin only | REPORTED: [JetPack 6.2 page](https://developer.nvidia.com/embedded/jetpack-sdk-62), [release notes](https://docs.nvidia.com/jetson/jetpack/release-notes/index.html) |
| 6.2.2 / 6.2.3 | 36.5 / 36.5.2 | 12.6 (same SDK set) | ~2026-02 / 2026 | Orin only | REPORTED: [NVIDIA forum](https://forums.developer.nvidia.com/t/jetpack-6-2-2-jetson-linux-36-5-is-now-live/359623) |
| 7.0 | 38.2 | 13.0.0 | 2025-08 | Thor only | REPORTED: JetPack archive |
| 7.1 | 38.4 | 13.0.0 | 2026 (page shows "1/06/2026", ambiguous) | Thor, T5000, T4000 | REPORTED: [7.1 page](https://developer.nvidia.com/embedded/jetpack/downloads/archive-7.1) |
| 7.2 / 7.2.1 | 39.2 / 39.2.1 | 13.2.1 | 2026-06-02 | Thor + **Orin family** | REPORTED: [7.2 page](https://developer.nvidia.com/embedded/jetpack/downloads/archive-7.2) |

- **Which modules can run CUDA 13 (REPORTED).** Orin can since JetPack 7.2
  (June 2026). Thor has been able to since 7.0. Xavier cannot: it is not
  listed for JetPack 6 or 7.
- **Fleet split (MEASURED; proxy weak).** Ollama ships
  `ollama-linux-arm64-jetpack5` and `-jetpack6` assets. Across releases
  published Jan to Sep 2026, these had 18,577 and 50,893 downloads, so
  JetPack 6 is 73 % of the Jetson-specific downloads. The monthly JetPack 6
  share was 63 to 77 %. The 2024-11 to 2025-02 months were ~50/50, which
  suggests a baseline of mirror bots that download every asset (INFERRED).
  The true JetPack 6 share among Ollama Jetson users is therefore likely
  **above** 73 %. JetPack 7 users, on Thor and on Orin with 7.2, use the
  generic `ollama-linux-arm64` asset, which also serves DGX Spark and Arm
  servers, so their share cannot be isolated.
- INFERRED: through 2027 the Orin fleet will be predominantly JetPack 6 with
  CUDA 12.6. JetPack 7.2 is only four months old, and reflashing production
  Orin units to a new Ubuntu 24.04 base is slow. A CUDA 12.6 aarch64 Jetson
  build remains essential. A CUDA 13.x Jetson build becomes useful for Thor
  now and for Orin on JetPack 7.2 over time. JetPack 7.2 ships CUDA 13.2.1,
  so a 13.0 build relies on minor-version compat with the Tegra driver; verify
  this on hardware.

## Peer-project support matrices

| Project | CUDA builds shipped now | 11.x dropped | 13.x added | Source / label |
| --- | --- | --- | --- | --- |
| PyTorch (PyPI default) | 2.6: 12.4 · 2.7: 12.6 · 2.8–2.10: 12.8 · 2.11–2.14: 13.0 (x86_64 and aarch64) | cu118 not in recent releases; last default 11.x was 1.13 (11.7) | default from 2.11 (2026-03-23) | MEASURED: PyPI `requires_dist` |
| PyTorch (download.pytorch.org) | 2.11: cu126, cu128 (+cu130 default); 2.12: cu130 stable, cu132 experimental, cu126 legacy (cu128 removed Apr 2026); 2.14: last with any 12.x (cu126); 2.15: 13.x only; nightly from 2026-09-28: 13.2 default, 13.4 prototype (13.0 removed) | — | — | REPORTED: [2.11 notice](https://dev-discuss.pytorch.org/t/transitioning-pypi-cuda-wheels-to-cuda-13-0-as-the-stable-release-2-11/3325), [2.12 notice](https://dev-discuss.pytorch.org/t/introducing-cuda-13-2-and-deprecating-cuda-12-8-release-2-12/3337), [2.15 notice](https://dev-discuss.pytorch.org/t/notice-cuda-12-6-wheels-will-no-longer-be-published-from-pytorch-2-15-drops-maxwell-pascal-volta/3432) |
| llama.cpp | Win x64: 12.4 + 13.4; Ubuntu x64: 12.8 + 13.4; arm64: 13.4 | Win 11.7 dropped between b5200 (2025-04-27) and b5600 (2025-06-06) | 13.1 from ~b7600 (2026-01); tracks the newest 13.x | MEASURED: GitHub assets |
| Ollama | linux-amd64, linux-arm64, jetpack5, jetpack6 tarballs; internal CUDA runtime set not verified | jetpack5 (11.4) still shipped | not verified | MEASURED (asset names) |
| vLLM | default cu129; cu128 and cu130 also built | — | cu130 variant | REPORTED: [vLLM GPU install docs](https://docs.vllm.ai/getting_started/installation/gpu.html) (v0.19.1 snapshot; a later default may differ) |
| ONNX Runtime GPU | ≥1.27: CUDA 13.0 default on PyPI and NuGet; CUDA 12.8 "legacy"/deprecated | long ago | 1.27 | REPORTED: [CUDA EP docs](https://onnxruntime.ai/docs/execution-providers/CUDA-ExecutionProvider.html), [1.27 notes](https://newreleases.io/project/github/microsoft/onnxruntime/release/v1.27.0) |
| JAX | `jax-cuda12-plugin` and `jax-cuda13-plugin`; the cu13 plugin is ~33–36 % of CUDA plugin DLs, Jul to Sep 2026 | — | cu13 plugin from ~2026-04 | MEASURED: ClickPy totals |
| CuPy | `cupy-cuda12x` and `cupy-cuda13x`; cu13 is ~17–22 % of the two, Aug to Sep 2026; `cupy-cuda11x` ≈ 0 by 2026-08 | effectively gone in 2026 | 2025-10 | MEASURED: ClickPy totals |
| cudarc (crate) | features cuda-11040 … cuda-13040, plus `cuda-version-from-build-system` | 11.4 to 11.8 still present in 0.19.10 | 13.0: 0.17.1 (2025-08-06); 13.1: 0.18.2 (2025-12-11); 13.2: 0.19.4 (2026-03-23); 13.3: 0.19.8 (2026-06-19); 13.4: 0.19.10 (2026-09-24) | MEASURED: crates.io API |
| candle-core | 0.11.0 (2026-06-26) depends on `cudarc ^0.19.8` with `cuda-version-from-build-system` and `dynamic-linking` | inherits cudarc | inherits cudarc | MEASURED: crates.io API |

PyTorch matrices for 2.7 to 2.10 are not covered here: only their PyPI
defaults were verified, and the other builds of those releases (cu118, cu126,
cu129 and cu130) were not re-checked. The NVIDIA `nvidia/cuda` Docker images
have tags for every minor from 11.8 to 13.4. The 12.8, 12.9 and 13.x tags were
rebuilt in 2026, while ≤12.6 tags were last updated in 2024
(`data/dockerhub-nvidia-cuda-by-minor.csv`). Docker Hub exposes only a
repo-level pull count (126.3 M), not per-tag pulls, so this is not a usage
signal (MEASURED).

## Gaps and caveats

- **No direct measurement of the installed driver or CUDA base.**
  - No public source reports a driver-version distribution. Steam publishes
    GPU models, not driver versions. NVIDIA publishes nothing.
  - Ubuntu popcon was not queried. It covers few NVIDIA hosts and would show
    package installs only.
  - Every share above describes **downloads by ML tooling**, not machines.
    CI, Docker layer builds and ephemeral VMs dominate PyPI counts, and a
    CPU-only CI job still downloads `nvidia-cuda-runtime`.
- **The PyPI proxy is PyTorch-shaped.** Shares follow PyTorch's default pin
  with lags of about one to three months. This shows *what CUDA user-space is
  installed*. It does not show what drivers can run, nor what a Rust or local
  FathomDB user has. A CUDA 13.0 torch wheel installed on a CI box with no GPU
  says nothing about drivers.
- **Windows is barely visible.** The runtime wheels are Linux-only
  dependencies of torch. Windows evidence comes only from llama.cpp assets,
  which carry a user-selection bias: the first-listed or "safe" build gets
  picked.
- **Jetson is under-measured.** Jetson wheels largely come from NVIDIA's own
  index (jetson-ai-lab) rather than PyPI, so the Ollama asset split is the
  only fleet proxy. It contains bot noise, and JetPack 7 users are hidden in
  the generic arm64 asset.
- **Absolute counts for Sep to Oct 2026 are depressed** across all packages
  (see the anomaly note). Shares are unaffected only if the loss is uniform,
  which was not verified.
- **Partial data.** pypistats `/overall` covers only 180 days, starting
  2026-04-08, and pepy's free API covers 91 days. The 18-month history rests
  on ClickPy alone, cross-checked against those two where the windows
  overlap.
- **Release dates are month-granular**, taken from NVIDIA's archive page.
  JetPack 7.1's date string is ambiguous.
- **Several lifecycle facts are REPORTED only via secondary aggregators.**
  These include the R580 EOL month, R610/R615 dates, Ubuntu package versions
  and AWS DLAMI contents. Recheck against NVIDIA and Canonical primary pages
  before relying on them.
- **Not fetched.** Unobtainable or out of scope: Steam driver data (not
  published), GCP/Azure/Colab defaults (no 2026 source found), and an Ollama
  CUDA runtime matrix (not verified).

## Data files

All files are under `data/`:

- `clickpy-queries.sql`: exact ClickHouse queries.
- `clickpy-cuda-runtime-monthly-by-version.csv`: raw monthly downloads by
  package and version, 2025-01 to 2026-10.
- `clickpy-cuda-runtime-monthly-share-by-minor.csv` and
  `clickpy-cuda-runtime-quarterly-share-by-major.csv`: derived shares.
- `clickpy-monthly-totals-related.csv`: torch, cuda-toolkit, jax, cupy, ORT,
  vLLM and numpy.
- `pypistats-*.json` and `pepy-v2-*.json`, plus `pepy-per-version-monthly.csv`:
  cross-check sources.
- `pypi-torch-<ver>.json`: PyPI metadata for torch 2.6.0 to 2.14.1.
- `gh-llamacpp-release-assets.csv`, `gh-llamacpp-sampled-release-cuda-assets.csv`
  and `gh-ollama-release-assets.csv`: GitHub release asset counts.
- `crates-cudarc.json` and `crates-candle-core.json`: crates.io API.
- `dockerhub-nvidia-cuda-by-minor.csv`: Docker Hub tag summary.
- `pages/`: saved NVIDIA release notes, the CUDA archive, the JetPack archive,
  three PyTorch packaging notices and the Steam video-card survey.
