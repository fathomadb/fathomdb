---
title: 0.8.32 Prework Slice 8 — CUDA Version Review, Standing Tripwires, and Slice 40 Venue Feasibility
status: COMPLETE
target_release: 0.8.32
slice: 8
date: 2026-11-24
---

# 0.8.32 Prework Slice 8 — CUDA Version Review, Standing Tripwires, and Slice 40 Venue Feasibility

Read-only audit of `/mnt/data/worktrees/release-0.8.32` (`dev/0.8.32` @ `38a1594bf`), paired with a `2026-11-24` primary-source web refresh per [`AGENTS.md`](../../../../AGENTS.md) § 1, [`cuda-version-support-review.md`](../../../design/cuda-version-support-review.md), [`ADR-0.8.28-cuda-version-support-policy.md`](../../../adr/ADR-0.8.28-cuda-version-support-policy.md), [`overflow-chunking-reevaluation.md`](../../../design/overflow-chunking-reevaluation.md), and [`slice-0.md`](./slice-0.md). No repository files, host settings, drivers, or toolchains were modified.

Every empirical claim carries one epistemic label: **MEASURED** (verified directly in local checkout files, hardware outputs, or previously saved primary JSON), **REPORTED** (stated by an external primary/secondary source retrieved `2026-11-24`), **INFERRED** (auditor reasoning from evidence), or **UNKNOWN** (signal unavailable without shell egress or direct SQL/API access; never fabricated).

---

## 1. Prior CUDA Review Lineage (`0.8.28`–`0.8.31`)

1. **`0.8.28` Slice 31 baseline (`2026-10-11`):** [`0.8.28` adoption study](../../0.8.28/features/slice-31/research/cuda-version-adoption.md), [`0.8.28` proposal](../../0.8.28/features/slice-31/research/cuda-version-support-proposal.md), and [`ADR-0.8.28-cuda-version-support-policy.md`](../../../adr/ADR-0.8.28-cuda-version-support-policy.md) established the importance matrix (`11.x` none; `12.6` tier 1 shipping floor; `other 12.x` covered via compatibility cells; `13.0` tier 1 next; `13.2` low; `13.4+` compile-check only), Turing (`sm_75`) x86_64 / Orin (`sm_87`) PTX floors, driver-library loading guard (`cuda_library_unavailable`), and `windchill3` R580 retention.
2. **`0.8.29` refresh (`2026-11-10`):** [`0.8.29/research/cuda-version-adoption.md`](../../0.8.29/research/cuda-version-adoption.md) measured PyPI `torch==2.15.0` (`2026-10-28`) pinning `cuda-toolkit==13.2.2`, noted PyTorch dropping CUDA 12.6 wheels in `2.15` and removing `13.0` from nightlies due to CUDA compiler reconvergence bug `6111901` (present `12.8`–`13.2.1`, fixed `13.2.2`), recorded CUDA `13.5`/R620 and R580 LTS EOL June 2028, proposed a delivery hold on `13.0` until `6111901` exposure is assessed, and left ClickPy/Pepy/GitHub download shares `UNKNOWN`.
3. **`0.8.30` refresh (`2026-11-17`):** [`0.8.30/research/cuda-version-adoption.md`](../../0.8.30/research/cuda-version-adoption.md) and [`cuda-policy-observations-2026-11-17.json`](../../0.8.30/research/data/cuda-policy-observations-2026-11-17.json) retained all `ADR-0.8.28` ratings unchanged and recorded ClickPy/Pepy/GitHub shares `UNKNOWN`.
4. **`0.8.31` disposition (`2026-11-23`):** Owner ruling A1 in [`0.8.31/plan.md`](../../0.8.31/plan.md) deferred [`cuda-delivery-plan.md`](../../0.8.31/cuda-delivery-plan.md) (`TC-d244fe8e`, `TC-b267d940`, `TC-48442a08`) out of `0.8.31`; `0.8.31` shipped (`v0.8.31`) with the `0.8.30` CUDA 12.6 stack (`CUDA_TOOLKIT_VERSION='12.6.3'` manylinux x86_64 wheel, host `nvcc 12.6.68` x86_64 N-API, and classic Jetson Orin L4T R36 / JetPack 6 Tegra Pages artifacts), pinned fork cohort `cudarc-fathomdb =0.21.0` and `candle-*-fathomdb =0.13.0`. [`0.8.32/scope.md`](../scope.md) § Out of scope retains CUDA 13 delivery deferred post-`0.8.35` or to a dedicated release.

---

## 2. Fresh CUDA Ecosystem Signals (`2026-11-24`)

### 2.1 Signals Unavailable Without Direct Shell Network / SQL Access (`UNKNOWN`)

| Signal | Availability status (`2026-11-24`) | Notes |
| --- | --- | --- |
| ClickPy monthly runtime-wheel shares (`sql-clickhouse.clickhouse.com`, `pypi.pypi_downloads_per_day_by_version`) | **UNKNOWN** | Requires direct ClickHouse SQL access; inaccessible via `search_web`. |
| pepy.tech 91-day per-version counts & pypistats.org comparable-month cross-check denominators | **UNKNOWN** | `search_web` surfaced only a rolling `nvidia-cuda-runtime-cu12` headline (~321k/day, ~3.1M/wk, ~15.6M/mo — **REPORTED**, [pypistats.org](https://pypistats.org/packages/nvidia-cuda-runtime-cu12)), lacking `-cu11`/`nvidia-cuda-runtime` comparable denominators. |
| GitHub release-asset download counts (`ggml-org/llama.cpp`, `ollama/ollama`) | **UNKNOWN** | Asset version availability is **REPORTED** below; live `download_count` splits require GitHub REST API access. |
| Installed host driver distribution & FathomDB consumer CUDA 12 vs. 13 demand | **UNKNOWN** | No public driver census exists; Oct 2026 Steam GPU-model share (~1.54% GTX 1060, ~1.21% GTX 1050 Ti — **REPORTED**, [Steam Survey](https://store.steampowered.com/hwsurvey/videocard/)). |

### 2.2 Observed Ecosystem Signals (`2026-11-24`)

| Signal | Label | Observation (`2026-11-24`) | Primary / Cited Source |
| --- | --- | --- | --- |
| PyTorch releases & defaults | **REPORTED** | `torch 2.15.0` (`2026-10-28`) remains latest stable (`2.16.0` targeted `2026-12-17`). Default Linux wheel pins **CUDA 13.2.2** (`13.4.3` alternative); `2.14` was the final release with CUDA 12.6 wheels (`2.15+` is 13.x-only). | [PyTorch 2.15 blog](https://pytorch.org/blog/pytorch-2-15-release-blog/), [PyPI `torch 2.15.0`](https://pypi.org/pypi/torch/2.15.0/json) |
| CUDA Toolkit & driver branches | **REPORTED** | Latest toolkit remains **CUDA 13.5** (`2026-11-04`). Table 2 branches: `13.0` -> `R580`, `13.1` -> `R590`, `13.2` -> `R595`, `13.3` -> `R610`, `13.4` -> `R615`, `13.5` -> `R620`. Table 3 minor-compat: `13.x` >= `580`, `12.x` `525`–`<580`, `11.x` `450`–`<525`. | [CUDA Toolkit Release Notes](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html), [Minor Version Compatibility](https://docs.nvidia.com/deploy/cuda-compatibility/latest/minor-version-compatibility.html) |
| Driver branch lifecycle & architectures | **REPORTED** | `R580` LTSB EOL **June 2028** (last branch for Maxwell/Pascal/Volta); `R595` PB EOL **March 2027**; **`R620` LTSB** released **November 2026** (EOL **September 2029**). | [NVIDIA Supported Drivers Matrix](https://docs.nvidia.com/datacenter/tesla/drivers/supported-drivers-and-cuda-toolkit-versions.html) |
| Compiler safety (`6111901`) | **REPORTED** / **UNKNOWN** | CUDA `13.2.2` fixed compiler reconvergence bug `6111901` (present `12.8`–`13.2.1`). Shipped `12.6` predates `12.8`; exact exposure of `candle-kernels-fathomdb` on `13.0` remains **UNKNOWN**. | [CUDA 13.2.2 Release Notes](https://docs.nvidia.com/cuda/archive/13.2.2/cuda-toolkit-release-notes/index.html) |
| Jetson / JetPack (`Orin` & `Thor`) | **REPORTED** | **JetPack 6.2.3** (`L4T 36.5.2`, Ubuntu 22.04, **CUDA 12.6.10**) remains active for Orin. **New delta:** **JetPack 7.2.2** (`L4T 39.2.2`, Ubuntu 24.04, **CUDA 13.2.2**, Nov 2026) released. | [NVIDIA JetPack Archive](https://developer.nvidia.com/embedded/jetpack-archive) |
| `vllm` | **REPORTED** | **New threshold crossed:** `v0.33.0` / `v0.34.0` (`2026-11-16`) discontinued standard CUDA 12.9 builds and moved official release wheels/images exclusively to **CUDA 13.0 (`cu130`)**. | [vLLM Releases](https://github.com/vllm-project/vllm/releases) |
| `onnxruntime-gpu` | **REPORTED** | `1.32.0` (mid-Nov 2026): default packages remain **CUDA 13.0** + cuDNN `9.x`, with separate `cuda12` (`12.8+`) builds still distributed. | [PyPI `onnxruntime-gpu`](https://pypi.org/project/onnxruntime-gpu/) |
| `llama.cpp` & `ollama` | **REPORTED** | `llama.cpp v1.0.0` (`2026-11-13`) retains **both CUDA 12 (`12.4`/`12.8`) and CUDA 13 (`13.5`)** assets. `ollama` (`v0.40.18+`) **dropped JetPack 5 (`11.4`)** assets. | [llama.cpp Releases](https://github.com/ggml-org/llama.cpp/releases), [Ollama Releases](https://github.com/ollama/ollama/releases) |
| `cudarc` & `candle-core` | **REPORTED** / **MEASURED** | Upstream `candle-core 0.11.1` depends on `cudarc ^0.19.10` (`cuda-11040`..`cuda-13040`). **MEASURED:** FathomDB [`Cargo.toml`](../../../../Cargo.toml) pins qualified fork cohort `cudarc-fathomdb =0.21.0` and `candle-*-fathomdb =0.13.0`. | [docs.rs `cudarc`](https://docs.rs/crate/cudarc/latest), [`Cargo.toml`](../../../../Cargo.toml) |
| `jax` & `cupy` | **REPORTED** | Both retain parallel CUDA 12 and CUDA 13 distributions. | [JAX Installation](https://docs.jax.dev/en/latest/installation.html), [CuPy Installation](https://docs.cupy.dev/en/stable/install.html) |
| Distros & cloud GPU images | **REPORTED** | Ubuntu 24.04 `noble-updates` ships `nvidia-driver-580` (`580.178.04`) and `nvidia-driver-595`; AWS Ubuntu 24.04 Base GPU DLAMIs default to `/usr/local/cuda-13.5`. | [Supported Drivers Matrix](https://docs.nvidia.com/datacenter/tesla/drivers/supported-drivers-and-cuda-toolkit-versions.html) |

### 2.3 Importance Table Comparison, Test Host Validity, and Deferral Check

- **ADR-0.8.28 importance ratings verdict (INFERRED):** **No tier change is warranted for `0.8.32`.** `11.x` remains unsupported (`none`); `12.6` remains **tier 1, shipping floor**; `other 12.x` covered; `13.0` **tier 1 next**; `13.2` **low**; `13.4+` **compile-check only**.
- **Test-host validity for `0.8.32` (MEASURED):** `windchill3` (`580.178.04` + `/usr/local/cuda-12.6`, RTX 3090 GPUs `0, 1`, K620 GPU `2` display-only/unused) and classic Jetson Orin (`10.83.10.13`, JetPack 6 / CUDA `12.6.68`) remain fully valid without alteration.
- **CUDA 13 deferral verdict (INFERRED):** Keep CUDA 13 deferred post-`0.8.35` per [`0.8.32/scope.md`](../scope.md) § Out of scope; no `ADR-0.8.28` amendment required unless owner elects to pull CUDA 13 forward.

---

## 3. Ready-to-Paste `dev/plans/0.8.32/plan.md` CUDA Review Summary Paragraph

> **CUDA version review (`2026-11-24`, `prework/slice-8.md`):** Primary-source refresh conducted `2026-11-24` against [`ADR-0.8.28-cuda-version-support-policy.md`](../../../adr/ADR-0.8.28-cuda-version-support-policy.md) and the `0.8.28`/`0.8.29`/`0.8.30` reviews (`ClickPy` SQL, `Pepy` API, and GitHub asset download-count splits remain `UNKNOWN` without shell network access). **Changes observed:** `vLLM` (`v0.33.0`/`v0.34.0`, Nov 2026) joined `PyTorch 2.15.0` (CUDA `13.2.2` default) in dropping standard CUDA 12 wheels for CUDA `13.0+`; `Ollama` (`v0.40.18+`) dropped JetPack 5 (`11.4`); NVIDIA published `CUDA 13.5` / `R620` LTSB (EOL Sept 2029) and `JetPack 7.2.2` (`L4T 39.2.2`, CUDA `13.2.2`, Nov 2026) while keeping `JetPack 6.2.3` (`CUDA 12.6.10`) active for Orin and `R580` LTSB active through June 2028; `llama.cpp v1.0.0`, `onnxruntime-gpu 1.32.0` (`cuda12` build), `JAX`, and `CuPy` retain CUDA 12 support. **Importance table & host posture:** All `ADR-0.8.28` importance ratings (`11.x` none; `12.6` tier 1 shipping floor; `other 12.x` covered; `13.0` tier 1 next; `13.2` low; `13.4+` compile-check only) remain **unchanged** for `0.8.32`, `windchill3` (`580.178.04` + `/usr/local/cuda-12.6`, RTX 3090 GPUs `0,1` and K620 GPU `2` unused) and Orin JetPack 6 (`12.6`) remain valid for what `0.8.32` ships, and CUDA 13 delivery (`0.8.31/cuda-delivery-plan.md`, gated on compiler issue `6111901` kernel-exposure resolution) stays **deferred** post-`0.8.35` with no `ADR-0.8.28` amendment required unless the owner elects to pull it forward.

---

## 4. Standing Tripwire Verification (`T1`–`T3`) Against `0.8.31` Shipped State

| Tripwire | Trigger criterion | `0.8.31` / `38a1594bf` verification evidence | Status (`2026-11-24`) | Action for `0.8.32` |
| --- | --- | --- | --- | --- |
| **`T1`** | Answer-level eval on session/doc stores shows loss traced to evidence past 512 wordpieces | **MEASURED:** No answer-level overflow probe ran in `0.8.31` (slated for `0.8.35` Tracks 1–2). | **NOT FIRED** (unmeasured) | Retain parked. |
| **`T2`** | Fusion weights or default embedder / max sequence tokens change | **MEASURED:** `fusion.rs`: `RRF_K = 30.0`, `RRF_WEIGHT_TEXT = 3.0`, `RRF_WEIGHT_VECTOR = 1.0`; `candle_bge.rs`/`open.rs`: `DEFAULT_EMBEDDER_NAME = "fathomdb-bge-small-en-v1.5"`, `MAX_SEQUENCE_TOKENS = 512`. Both pinned unit tests intact. | **NOT FIRED** | Preserve zero-behavior-change invariants across `search.rs` split; re-check at close. |
| **`T3`** | Exact-scan kernel (`1–2M` brief Path A) lands | **MEASURED:** Zero `packed_popcount`/`exact_scan`/`centroid_id` symbols in `src/`; `search.rs` still runs two-phase `vec0` bit-KNN; `E1` unrun. | **NOT FIRED** | Retain parked; watch `database_bytes` headroom (`200.4 MB` at 100k). |

**Governance gap found:** Standing ledger todo `TC-2a9f89fa-5302-44b7-a14e-97b9d9cbde90` in [`dev/todos-and-considerations-ledger.jsonl`](../../../todos-and-considerations-ledger.jsonl) still carries `"target_release": "0.8.29"`; retarget via `ledgerwrite` at `0.8.32` start/close.

---

## 5. Slice 40 Venue Feasibility Check (`R32-05` Power-Loss & Fuzzing)

### 5.1 Power-Loss / Torn-Write Venue (`HANDOFF-03` Option 1: Disposable VM + `dm-flakey`)

| Capability / Probe | Observed State (`2026-11-24`, **MEASURED**) | Feasibility Impact |
| --- | --- | --- |
| Host loopback + `dmsetup` on host | `/dev/mapper/control` (`Permission denied`), `/dev/loop*` (`nobody:nogroup 0660`), `libvirtd` inactive, `dm-dust` absent. | **Blocked without host `sudo`:** cannot run `dmsetup`/`losetup` directly on host as `coreyt`. |
| User-mode QEMU/KVM (`qemu-system-x86_64` + `/dev/kvm`) | `/dev/kvm` has POSIX ACL `user:coreyt:rw-` (`kvm_amd` loaded); `qemu-system-x86_64 8.2.2`, `qemu-img`, `xorriso`, `genisoimage`, `mkisofs`, `cpio`, `gzip`, `busybox`, `fakeroot`, `mksquashfs`, `/usr/share/OVMF/` installed; `/mnt/data` has `576 GiB` free. | **Feasible with zero host `sudo` or host mutation:** inside a disposable QEMU/KVM guest, guest `root` runs `modprobe dm-flakey`, `losetup`, `dmsetup create ... flakey`, `mkfs.ext4`, FathomDB commit/WAL/checkpoint stress, real dropped/torn/reordered flushes, and hard QEMU power-off. |
| Guest OS image / cloud-init tooling | `/boot/` absent in sandbox; `cloud-localds`, `guestfish`, `virt-builder`, `debootstrap`, `mmdebstrap` absent; no Ubuntu x86_64 guest image staged yet under `/mnt/data/`. | **Pre-Slice-40 prep needed:** download & SHA-256-pin an Ubuntu 24.04 (`glibc >= 2.39`) cloud/minimal image + kernel verified to include `dm_flakey.ko` (`linux-generic`/`linux-modules-extra`, not stripped `linux-kvm`), and generate NoCloud `cidata` ISO via `genisoimage`/`xorriso`. |

### 5.2 Coverage-Guided Fuzzing Venue (`R32-05`)

| Capability / Probe | Observed State (`2026-11-24`, **MEASURED**) | Feasibility Impact |
| --- | --- | --- |
| Nightly Rust + sanitizer runtimes | `nightly-2026-08-01-x86_64-unknown-linux-gnu` installed with `librustc-nightly_rt.{asan,dfsan,lsan,msan,rtsan,tsan,ubsan}.a`; `gcc-13` / `g++-13` installed. | Ready for `-Zsanitizer=address` via `rustc`'s internal LLVM. |
| Fuzzing runner & LLVM tools | `cargo-fuzz` not installed; `clang` and `llvm-profdata`/`llvm-cov` absent. | Install `cargo-fuzz --locked` in Slice 10 prep and dry-run a minimal `libfuzzer-sys` + `ASan` link & coverage-tool check before offline durability execution. |

---

## 6. Numbered Proposals (`P8-01`–`P8-05`)

| ID | Proposal | Understood (H/M/L) | Risk (H/M/L) | Effort (S/M/L) | Recommendation | Target Slice / Rationale |
| --- | --- | --- | --- | --- | --- | --- |
| `P8-01` | Record the § 3 CUDA review summary paragraph in `dev/plans/0.8.32/plan.md`, retaining all `ADR-0.8.28` importance tiers unchanged, `windchill3` R580 (`580.178.04`) + `/usr/local/cuda-12.6` (K620 unused), and Orin JetPack 6 (`12.6`) for `0.8.32` | H | L | S | include | **Slice 10 (`Prework Prep`)** — satisfies `AGENTS.md` § 1 and `cuda-version-support-review.md` § 3 with zero scope churn. |
| `P8-02` | Keep CUDA 13 delivery (`0.8.31/cuda-delivery-plan.md`, `TC-d244fe8e`, `TC-b267d940`, `TC-48442a08`) deferred post-`0.8.35`; log `vLLM v0.33+`/`PyTorch 2.15` CUDA 12 drop + `JetPack 7.2.2` + `6111901` safety gate as inputs for that release | H | L | S | postpone | **Post-`0.8.35` / dedicated CUDA 13 release** — aligns with `0.8.32/scope.md`. |
| `P8-03` | Record standing tripwires `T1`–`T3` = NOT FIRED in `0.8.32/plan.md`, keep `search.rs` split zero-behavior-change (`overflow_chunking_*_tripwire` tests green), and retarget ledger todo `TC-2a9f89fa` from `0.8.29` to `0.8.32` via `ledgerwrite` | H | L | S | include | **Slice 10 (`Prework Prep`)** + Qualification close — closes stale `0.8.29` ledger target and protects `T2` constants. |
| `P8-04` | De-risk durability power-loss venue early: pin & stage an Ubuntu 24.04 (`glibc >= 2.39`) guest image + kernel verified to include `dm_flakey.ko` under `/mnt/data/worktrees/`, build NoCloud `cidata` via `genisoimage`/`xorriso`, and pass one user-mode `/dev/kvm` + `dm-flakey` boot smoke before the durability slice | H | M | M | include | **Slice 10 prep / Durability slice entry (`R32-05`)** — avoids discovering missing guest `dm_flakey.ko` or unprivileged host `dmsetup` mid-slice. |
| `P8-05` | Install `cargo-fuzz` (`--locked`) and run a minimal `nightly-2026-08-01` `libfuzzer-sys` + `ASan` link & coverage-tool check during Slice 10 prep so durability fuzzing builds offline cleanly | H | M | S | include | **Slice 10 (`Prework Prep`, `R32-05`)** — closes missing `cargo-fuzz` / `clang` toolchain ambiguity before feature work. |
