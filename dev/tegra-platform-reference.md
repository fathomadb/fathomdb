---
title: FathomDB on NVIDIA Tegra (Jetson) — platform reference
date: 2026-08-19
target_release: 0.8.23
desc: Durable reference for building, packaging, testing, and shipping FathomDB on Tegra/L4T — measured facts, structural constraints, and the traps
blast_radius: scripts/release/build-python-cuda-tegra.sh; scripts/release/glibc-floor-contract.sh; scripts/release/cuda-artifact-contract.sh; scripts/check-glibc-floor-doc-truth.sh; src/rust/crates/fathomdb-embedder/src/gpu_witness.rs; dev/platform-capabilities.json (see § 8 gap); docs/compatibility/index.md
status: living
---

# FathomDB on NVIDIA Tegra (Jetson)

## 0. What this document is, and is not

This is the **platform reference** for Tegra/L4T: the measured facts, the
structural constraints, and the traps that cost real debugging time. It is meant
to outlive release 0.8.23 and to be the first thing read by anyone touching
Tegra build, packaging, CI, or evidence work.

**It is not the decision record.** Requirements (`R80-*`), acceptance criteria
(`AC80-*`), and commissioning decisions (`D-*`) live in
`dev/design/0.8.23-aarch64-tegra.md`, which remains authoritative. Where this
document restates a decision it does so to explain *why the platform forces it*,
and it names the owning `D-`/`AC-` so the two cannot silently diverge. **If this
file and the design doc disagree, the design doc wins and this file is stale.**

Everything under "measured" was observed on the reference host below and is
reproducible there. Everything under "structural" is a property of the
ecosystem, not of our code, and will not be fixed by changing our code.

## 1. The reference host

Every measured value in this document comes from this machine.

| Property | Value |
|---|---|
| Board | NVIDIA Jetson AGX Orin Developer Kit |
| Device-tree compatible | `nvidia,p3737-0000+p3701-0005`, `nvidia,p3701-0005`, `nvidia,tegra234` |
| L4T | `R36 (release), REVISION: 5.2`, GCID 46426093, EABI aarch64 |
| Userspace lib dir | `usr/lib/aarch64-linux-gnu/nvidia` (from `/etc/nv_tegra_release`) |
| OS / libc | Ubuntu 22.04, **glibc 2.35** |
| Arch | `aarch64` |
| CUDA toolkit | 12.6, `nvcc` release 12.6, **V12.6.68** |
| Driver | 540.5.0 |
| GPU | `Orin (nvgpu)`, compute capability **8.7** (`sm_87`) |
| GPU UUID | `bbbe9f37-7028-556a-930b-54e5f3b67a82` |
| Memory | **Unified** — 65,879,896,064 bytes total, `CU_DEVICE_ATTRIBUTE_INTEGRATED = 1` |

The unified-memory property is not a detail. There is no discrete VRAM pool;
`cuMemGetInfo`'s "free" is system memory, shared with everything else on the box.
Every inference drawn from it must survive that fact.

## 2. Tegra is not `arm64-sbsa`, and nothing in the packaging stack knows it

**Structural.** CUDA for `aarch64` exists in two mutually incompatible flavors:

- **`arm64-sbsa`** — server-class Arm (Grace, Graviton, Ampere Altra).
- **Tegra / L4T** — Jetson.

They are not interchangeable in either direction. NVIDIA's `sbsa-linux` packages
ship **no SASS or PTX slices for Tegra iGPUs**, so a fat cross-flavor artifact is
not merely undesirable — it is not buildable.

**No packaging system can distinguish them.** This is the single fact that shapes
everything in §§ 3–5:

- **Python**: both a Tegra-linked and a generic aarch64 artifact carry the same
  wheel platform tag. There is no tag axis for SoC family, GPU, or CUDA flavor.
- **npm**: the complete matching key is `(process.platform, process.arch, libc)`.
  `process.arch` has no sub-architecture axis, so a Jetson Orin and an AWS
  Graviton both resolve as `linux` / `arm64` / `glibc`.
- **Rust**: there is no Tegra target triple. `aarch64-unknown-linux-gnu` covers
  both, which is why napi-rs — which derives package names mechanically from the
  triple — cannot express it either.

Evidence that this is mechanical rather than a policy we could argue with:
NVIDIA's own `nvidia-cuda-runtime-cu12` on PyPI ships exactly **one** aarch64
wheel and gives it to the SBSA build, with no `sbsa`/`tegra` marker anywhere in
the filename. There is nowhere to put one.

**Do not expect this to be fixed for Orin.** CUDA 13 unifies the Arm toolkits and
adds an `arm64-sbsa-jetson` target, but NVIDIA states the exception explicitly:
Orin (`sm_87`) "will continue on its current path for now." Orin is precisely the
target excluded from the unification.

## 3. Packaging

### 3.1 The rule

**Distribution name stays `fathomdb`.** The Tegra build is distinguished by a
**PEP 440 local version segment** (`+tegra`) served from a **first-party PEP 503
index** — not by a separate distribution name. Owning decision: **D-80.6-3**.

### 3.2 Why not a `fathomdb-tegra` distribution

Three reasons, in order of decisiveness.

**(a) Two distributions sharing the `fathomdb/` import package corrupts pip's
own bookkeeping.** This is worse than the "last one wins" shadowing it is
usually described as:

- pip does not detect, warn, or error (pypa/pip#4625, open since 2017; the only
  proposed fix, pip#14249, warns *after* the overwrite has happened).
- Both distributions remain listed as installed.
- The recording-installed-packages spec has **no concept of file-ownership
  arbitration between distributions**, so both `RECORD` files claim the same
  paths. A later `pip uninstall` of *either* deletes files the survivor still
  needs, leaving it installed-but-gutted — `.dist-info` intact, code gone
  (pypa/pip#8509, the ansible/ansible-base split).

That is a **corrupted-install** failure. It happens before any import, so no
import-time guard can catch it. It is strictly worse than the **wrong-build**
failure it would be adopted to prevent.

**(b) PyPI would reject the wheel anyway.** See § 3.3.

**(c) The suffix pattern is a name you own forever.** `cupy-cuda12x`,
`onnxruntime-gpu`, `paddlepaddle-gpu` all do it, and it is precisely the pattern
PEP 817's Motivation section singles out as broken. Retiring `tensorflow-gpu`
required the TensorFlow team to republish the name as a poison-pill stub that
raises on install.

### 3.3 PyPI rejects our wheel tag outright

**Structural, and load-bearing.** The artifact we build is bare
`linux_aarch64` — measured: `fathomdb-0.8.22-cp310-abi3-linux_aarch64.whl`.

**PyPI rejects a bare `linux_aarch64` platform tag with HTTP 400, under any
distribution name.** This is enforced in Warehouse's upload path by an
allow-list plus a `(many|musl)linux_(\d+)_(\d+)_(?P<arch>.*)` regex that a bare
`linux_` string structurally cannot match. The rationale is PEP 513's: a bare
`linux_*` tag records no distribution or system-library information, so
cross-machine compatibility cannot be assumed.

Note `linux_armv6l` and `linux_armv7l` *are* grandfathered carve-outs (legacy
Raspberry Pi). `linux_aarch64` is not among them.

**The consequence is that the two options are not symmetric**, and this is what
actually settles § 3.1:

| Option | Prerequisite |
|---|---|
| Separate distribution name on PyPI | `auditwheel repair` excluding `libcuda.so.1` and the CUDA runtime libs (which the CUDA EULA forbids redistributing anyway) → `manylinux_2_35_aarch64` → plus a glibc-floor exception |
| First-party index, same name | **None.** A Jetson's compatible-tag list already includes bare `linux_aarch64` |

### 3.4 This is what NVIDIA and PyTorch actually do

Not an analogy — the same problem, same solution:

- NVIDIA's Jetson PyTorch ships as distribution **`torch`** (the plain PyPI
  name), versioned `2.5.0a0+…nv24.08` — a PEP 440 local segment — tagged bare
  `linux_aarch64`, hosted at `developer.download.nvidia.com/compute/redist/jp/`.
- PyTorch's own docs now direct Jetson users at
  `--index-url https://pypi.jetson-ai-lab.io/jp6/cu126`, whose `torch`,
  `onnxruntime-gpu`, `vllm`, `xformers` etc. all carry canonical PyPI names.
- **JetPack and CUDA are encoded in the index path** (`jp6/cu126`), never in the
  distribution name and never in the wheel tag.

NVIDIA never places Jetson wheels on PyPI.

### 3.5 The cost this accepts, and the obligation it creates

A later `pip install -U fathomdb` resolving against PyPI **silently replaces the
Tegra build with the generic one**. This is observed in the wild
(facebookresearch/xformers#1193: NVIDIA's `torch 2.5.0a0+…nv24.08` displaced by
PyPI's `2.5.1`, silently losing memory-efficient attention).

This is accepted **only because it is detectable**. That makes detection a
requirement, not a nicety — **AC80-27** / **D-80.7-1**: `doctor gpu` must detect
a Tegra host carrying a non-Tegra `fathomdb` and report a named outcome plus the
repairing install command. Ship the naming decision without that check and the
decision's own justification is unmet.

Check the **`+tegra` local version segment**, not the presence of CUDA symbols —
a generic build on a Tegra host may expose those and still be wrong.

### 3.6 `--extra-index-url` is unsafe if printed naked

pip documents **no priority** between indexes; all are searched and the best
version wins. A bare `linux_aarch64` wheel is tag-compatible on Graviton, so a
non-Tegra aarch64 user who pasted the flag would receive an unloadable Tegra
build.

Two mitigations, both required:

1. Print the index string **only after Tier-1/Tier-2 Tegra detection succeeds**
   (§ 6). Detection-gating is what makes the command safe to print at all.
2. Pin exactly — `==<version>+tegra`, never floating.

Prefer documenting the `uv` form alongside pip, which is structurally safe:
`[[tool.uv.index]] explicit = true` plus a `[tool.uv.sources]` pin restricts the
index to `fathomdb` alone and prevents PyPI fallback for it.

`--index-url` (full replacement) is correct only if the index also mirrors the
whole dependency closure — PyTorch does this, we do not. Absent that, the honest
form is `--extra-index-url` + detection-gating + an exact pin.

### 3.7 npm: permanently out of scope

**D-80.7-3.** Not "out of scope for this slice" — out of scope permanently,
because npm *cannot express the distinction at all* (§ 2). The ecosystem's own
answer is to not distinguish: the one arm64+CUDA npm package found
(`@sweet-search/native-linux-arm64-gnu-cuda`) explicitly buckets "Jetson Orin,
Grace Hopper, and arm64 server GPUs" together, and the sole Jetson-specific npm
precedent (`@roboflow/tfjs-jetson`) has been unmaintained since 2021.

Scale marker for how hard adding an axis is: the `libc` field — *one* glibc/musl
distinction — took ~3 years to go from Yarn to documented npm support and still
carries an open lockfile-correctness bug.

If a future slice is forced to ship Tegra on npm, the only safe shape is a
distinctly-named package the main package **does not** list in
`optionalDependencies`, so npm can never auto-select it, with `doctor gpu`
printing the install command post-detection and the loader `dlopen`ing it if
present. Explicitly rejected: `node-llama-cpp`'s install-both-and-probe pattern —
sound on x64 where both binaries are loadable, invalid here where a Tegra-linked
binary cannot load on Graviton at all.

### 3.8 PEP 817 / 825 wheel variants — watch item, not a mechanism

**D-80.7-2. Do not schedule work against this.** Verified status as of
2026-08-19:

- PEP 817 is **Draft** and has been *split*; PEP 825 (also Draft, PEP-Delegate
  Paul Moore) covers **only the wheel file format**.
- The half we would actually need — **how a system's capabilities are detected**
  (the provider-plugin mechanism) — is explicitly deferred to future PEPs that
  are not yet written.
- **pip has zero support in any released version** (checked through 26.2.1). The
  only working client is an experimental `uv` side-build distributed outside
  PyPI.
- **There is no Tegra-aware provider.** WheelNext's `nvidia-variant-provider`
  exposes only `cuda_version_lower_bound`, `cuda_version_upper_bound`, and
  `sm_arch`, with no ARM/Jetson/Tegra/SBSA handling, and is not published to
  PyPI. `sm_arch` would not disambiguate us cleanly anyway — it describes compute
  capability, not which CUDA runtime flavor the binary was linked against.

**Why it still matters:** a variant-labelled wheel is
`{dist}-{version}-…-{platform}-{variant label}.whl`, so **the distribution name
never changes**. Keeping the name `fathomdb` is what makes eventual adoption
free; renaming now would buy a second migration later.

## 4. Building

### 4.1 Required environment

```sh
export PATH=/usr/local/cuda-12.6/bin:$PATH
export LIBRARY_PATH=/usr/local/cuda-12.6/targets/aarch64-linux/lib:$LIBRARY_PATH
export LD_LIBRARY_PATH=/usr/local/cuda-12.6/targets/aarch64-linux/lib:/usr/lib/aarch64-linux-gnu/nvidia:$LD_LIBRARY_PATH
export CUDA_COMPUTE_CAP=87
```

**`LIBRARY_PATH` is not optional.** Without it, linking fails:

```text
cannot find -lcudart
```

This is because on L4T the CUDA runtime libraries live at
`/usr/local/cuda-12.6/targets/aarch64-linux/lib` (with `lib64` a symlink to it),
not where an x86_64-shaped build expects them. The driver is likewise in an
L4T-specific subdirectory: `/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so.1`
(itself a symlink to `libcuda.so.1.1`), which is why `LD_LIBRARY_PATH` names it
separately.

### 4.2 Host-native, never a manylinux container

The Tegra CUDA runtime is **host-bound**. The artifact is built on the Jetson
against JetPack's CUDA 12.6. Entry point:
`scripts/release/build-python-cuda-tegra.sh`.

Consequence: the glibc floor is **2.35, not 2.28** (§ 5.1).

`--auditwheel skip` / `--compatibility linux` are honest here *only* because the
artifact never reaches a registry (§ 3.3, D-80.6-1). If that ever changes, this
justification evaporates and the auditwheel work in § 3.3's table becomes real.

**Measured build** (80.6 acceptance): 27 s warm, producing a 7,915,123-byte
wheel whose `*.dist-info/WHEEL` reads `Tag: cp310-abi3-linux_aarch64`. Always
confirm that tag from `WHEEL` rather than trusting the filename. The wheel is
`abi3`/`cp310`, so it installs under the **system** Python 3.10 — verifying it
in the repo's 3.13 `.venv` would test the editable install, not the artifact.
Verify in a throwaway venv built from `/usr/bin/python3`.

### 4.3 Toolchain axis

`nvcc` on this host reports `release 12.6, V12.6.68` — **identical to the x86_64
pin**, so `CUDA_NAPI_HOST_NVCC_VERSION` needs no per-target split. The **host
compiler** does need one, and is parameterized the same way 80.4 parameterized
compute capability (**D-80.6-4**, **AC80-8**): each x86_64 literal assertion
becomes the literal on an x86_64-axis variable plus a selector linkage. The
contract gate rejects a second toolkit-root or `nvcc` literal split off from the
shared pin, and rejects an N-API host CC re-pointed at the Tegra axis.

### 4.4 Python environment traps

- **`maturin` is only in the repo `.venv`.** Not on the system path. Any runbook
  or hosted job that assumes otherwise fails.
- **System `python3` is 3.10 and lacks `datetime.UTC`.** The repo `.venv` is
  3.13. `scripts/check-cuda-release-contract.py` imports `datetime.UTC` and dies
  on 3.10 with `ImportError: cannot import name 'UTC' from 'datetime'`. Put
  `.venv/bin` on PATH when running repo Python scripts:
  `PATH="$PWD/.venv/bin:$PATH"`. See § 8 — this is an unfixed latent gap for any
  environment without the venv.
- Native builds go through `pip install -e python/`, never manual
  `cargo build` + `cp`.

## 5. Contracts and gates

### 5.1 The glibc floor is per-family

**AC80-26.** `2.35` for `tegra`, `2.28` for every existing family, with the Tegra
floor **declared** in `scripts/release/glibc-floor-contract.sh` rather than
exempted from the gate.

The exception is **load-bearing, not cosmetic** — measured: the real Tegra
extension requires `GLIBC_2.34` and genuinely fails a 2.28 assertion. It excludes
nobody in the target audience, since L4T R36 *is* Ubuntu 22.04 / glibc 2.35, so
every Jetson capable of running the artifact already meets it.

### 5.2 The doc-truth gate needed restructuring, not re-pointing

Worth remembering as a class of bug. `scripts/check-glibc-floor-doc-truth.sh`
read:

```sh
grep -m1 -oE 'Measured glibc floor: [0-9]+\.[0-9]+'
```

**First match only**, compared against a single floor. The moment the doc carried
*two* claims, a wrong Tegra floor passed silently — exactly the drift the gate
exists to prevent. Claims are now per-family markered with **every** occurrence
of each marker checked, and fixtures prove a wrong **second** claim fails and
that a contradictory repeat is caught.

**Generalize this:** any gate that reduces a multi-valued fact to a single `-m1`
match becomes a silent no-op the moment a second value appears.

### 5.3 No publication capability

The contract forbids `twine`, `npm publish`, `maturin publish`, `maturin upload`,
and `cargo publish` inside the Tegra build wrapper. 0.8.23 **builds and proves;
it does not publish** (D-80.6-1).

## 6. Detection

Two-tier (**R80-8**). Do **not** use `uname`-reported architecture as the sole
signal — `jetson-containers` shipped exactly that and broke it when Jetson Thor
began reporting `tegra` in `uname -a` despite being SBSA-capable.

**Tier 1 — Tegra family, filesystem only, no subprocess.** Either signal alone
suffices:

- `/proc/device-tree/compatible` contains `nvidia,tegra`
  — measured here: `nvidia,p3737-0000+p3701-0005`, `nvidia,p3701-0005`,
  **`nvidia,tegra234`**
- `/etc/nv_tegra_release` exists — measured: `R36 (release), REVISION: 5.2`.
  This file also supplies the L4T revision.

**Tier 2 — resolve the Thor ambiguity.**

- Tier 1 false → does `/sys/firmware/acpi/tables` exist? Present means
  SBSA-capable non-Tegra hardware. **Measured here: absent** (`No such file or
  directory`), consistent with Tegra.
- Tier 1 true → `nvidia-smi --query-gpu=name --format=csv,noheader`, check for
  the **`nvgpu`** substring. Measured here: `Orin (nvgpu)` → classic Tegra iGPU.
  Absent (`NVIDIA Thor`) → Tegra-family but SBSA-capable.

**Two constraints on the Tier-2 implementation, both from real bugs:**

1. **Check the subprocess exit status before reading stdout.** An early Rust
   sketch did not, so a failing `nvidia-smi` with empty stdout was misread as
   SBSA-affirmative instead of failing closed.
2. **The `nvgpu` substring is unofficial and empirically observed.** No
   documented NVIDIA API confirms it; it could change across driver/JetPack
   versions without notice. Tier 1 must be tried first precisely because it is
   filesystem-only and does not depend on this. Tier 2 must degrade to a named
   `probe_failed`-shaped outcome — never a silent default — if `nvidia-smi` is
   missing, times out, or exits non-zero. Invoke it by **absolute path with a
   bounded timeout**.

## 7. GPU evidence on Tegra — where x86_64 assumptions break

This section exists because the x86_64 evidence lane is **structurally
unportable** to Tegra. Do not try to reuse it.

### 7.1 `nvidia-smi` returns `[N/A]` for the fields the x86_64 witness relies on

Measured, single command:

```text
$ nvidia-smi --query-gpu=name,uuid,driver_version,memory.total --format=csv,noheader
Orin (nvgpu), bbbe9f37-7028-556a-930b-54e5f3b67a82, 540.5.0, [N/A]
$ nvidia-smi --query-compute-apps=pid,used_memory --format=csv,noheader
[N/A], [N/A]
```

Three separate breaks:

- **`--query-compute-apps` returns `[N/A]`.** The x86_64 witness binds evidence
  to a PID through this. That binding cannot exist on Tegra. **This is the whole
  reason the separate Tegra witness lane exists.**
- **`memory.total` is `[N/A]`.** Unified memory; use the driver API instead.
- **The UUID has no `GPU-` prefix.** x86_64 returns `GPU-<uuid>`; Tegra returns
  the bare `<uuid>`. The driver-API bytes are identical, and this repo's
  `cuda_uuid_string` renders the prefixed form. **Any UUID comparison must
  normalize the prefix.**

The last one is a live latent bug outside the Tegra lane:
`scripts/release/verify-cuda-preflight-witness.py:335` asserts
`nvidia_smi_uuid == device["uuid"]` by **string equality**, which cannot hold on
Tegra. Tracked for 80.6.5.

### 7.2 The nvmap page pool absorbs allocations — the biggest trap here

**Measured, and it invalidated an earlier recorded fact.** Tegra's nvgpu/nvmap
driver maintains a page pool that absorbs freed GPU pages, so a *subsequent*
allocation is served from the pool and **`cuMemGetInfo` never observes it**.

Reproduced directly (256 MiB alloc/free cycles, then a model-sized allocation):

| Event | Free-memory delta |
|---|---|
| Allocation #1 (cold pool) | 1,032,192 bytes |
| Allocations #3, #4 (warm pool) | **0** |
| 133,466,304-byte "model" allocation (warm pool) | **0** |

A 133 MB allocation moving the counter by **zero bytes** is the failure mode.
Naive "allocate, then check free memory dropped" logic silently reports no GPU
activity on a machine that is definitely using the GPU.

**The mitigation is a pool-draining control allocation as a precondition**
(D-80.5-3), not a larger tolerance. Consequences to keep in mind:

- The technique depends on **undocumented nvmap behavior** a future JetPack could
  change. It would surface as **failing closed**, never as a false pass.
- The sole-GPU-consumer precondition is **stated, not enforced**. A concurrent
  GPU tenant could push the delta over the floor or mask it, so an unattributable
  delta is a named failure (**R80-12**).
- The witness holds up to 16 GiB of control blocks on a 61 GB unified-memory box.
  It is opt-in, `embed-cuda`-gated, and never runs in a shipped path.

### 7.3 Ordering invariant

`cuMemGetInfo` with **no current CUDA context returns 201
(`CUDA_ERROR_INVALID_CONTEXT`)**; it succeeds once a context exists. Sample order
is therefore a contract (D-80.5-1): construct the device → assert the ordinal
Candle *actually retained* matches the requested one → sample **before** → load
the model → sample **after** → run one real forward pass and record the vector
dimension.

The forward pass is not optional. Sampling alone proves bytes were claimed, not
that the model computed on the device.

### 7.4 The witness is opt-in — and that is a footgun

**D-80.6-6.** Production of the witness is gated behind
**`FATHOMDB_GPU_ALLOCATION_WITNESS`**, because producing it costs a second model
load plus control-block allocations, which on every CUDA open would be a runtime
contract change.

**If the variable is unset, `openReport.embedder_gpu_allocation_witness` is
`None`, no GPU allocation work happens, and the run still looks like a passing
smoke while proving nothing.** Any evidence run **must** set it and must confirm
the field came back non-`None`.

Gating is fail-closed (**R80-12**): unset/empty → no witness, no GPU work;
requested but unproducible (CPU fallback, or `embed-cuda` not compiled) → typed
error, never a silent `None`; unrecognized value → rejected, not read as off.

**A real witness, measured at 80.6 acceptance** from the installed wheel with
`FATHOMDB_GPU_ALLOCATION_WITNESS=1` and `FATHOMDB_EMBED_DEVICE=cuda:0` — use
these as the shape to sanity-check a future run against:

| Field | Value |
|---|---|
| `free_before_bytes` | 49,713,115,136 |
| `free_after_bytes` | 49,569,492,992 |
| `delta_bytes` | **143,622,144** |
| `delta_floor_bytes` | 67,108,864 (64 MiB) |
| `control_allocation_request_bytes` | 1,073,741,824 |
| `control_delta_bytes` | 1,077,563,392 |
| `embedded_vector_dim` | 384 |
| `schema` | `fathomdb.tegra-gpu-allocation-witness/v1` |

Note `delta_bytes` (143.6 MB) exceeds the 133,466,304-byte model — deltas are
always ≥ the true footprint, never less. Contrast § 7.2: the *same* allocation
size against a warm pool without the control step measured **0 bytes**. The
control allocation is what makes this number exist.

The floor is `>=`, deliberately not `> 0` (**D-80.5-2**): 64 MiB sits an order of
magnitude above the measured 0-byte idle jitter and about half the true model
footprint. It is recorded *in* the witness so the verdict stays re-derivable
(**R80-13**).

### 7.5 Driver access path

The CUDA driver API is reached through cudarc 0.19.7 re-exported by the pinned
Candle fork:

```rust
candle_core::cuda::cudarc::driver::result::mem_get_info()
```

Pure verdict logic, typed errors, floor comparison, UUID normalization, and
serialization are compiled **unconditionally**; only the real sampler is behind
`embed-cuda`. That is what makes the logic testable in CI on hosts with no GPU.
This mirrors the existing `CudaProvider` trait split in `device_policy.rs` —
follow that idiom rather than inventing a second one.

### 7.6 Device policy on a CUDA-compiled Tegra artifact

**R80-11**, corrected during 80.5 commissioning — the earlier phrasing was wrong:

| `FATHOMDB_EMBED_DEVICE` | Outcome |
|---|---|
| `cpu` | CPU path, CUDA **never initialized** (AC80-7) |
| unset / `auto` | **Selects `cuda:0`** and completes on the GPU (AC80-21) |
| `cuda:0` | Forced CUDA, witnessed (AC80-6) |

`auto` **does** select CUDA. A CPU resolution under `auto` on a CUDA-compiled
artifact is a failure, not a pass.

## 8. CI, runners, and known gaps

### 8.1 There is no hosted Tegra runner

GitHub-hosted `ubuntu-24.04-arm` is SBSA-class, not Tegra. Tegra evidence
requires a **self-hosted runner on real Jetson hardware**.

Pattern in use: a second, separately-registered runner instance under
`~/actions-runner/fathomdb/` (label `jetson-fathomdb`), alongside the existing
`memex` runner on the same host — a standard `svc.sh`-installed systemd unit. It
is a **new instance, not a shared or repurposed one**, so workflow-restricted
group discipline is not diluted by another repo's access. Labels are routing
hints, not access control.

### 8.2 x86_64 CUDA work cannot run on this host

The registered runner here is ARM64. x86_64 CUDA lane work (schema merges,
`cuda-preflight.sh` runs) targets `windchill3` and is worked manually over SSH.
That host carries **two** GPUs — an RTX 3090 and a Quadro K620 display card — so
**the intended GPU must be pinned by UUID, not by index**.

`CUDA_VISIBLE_DEVICES` alone is **insufficient** there: it pins host-native CUDA
processes but does **not** redirect `docker run --gpus '"device=0"'` (resolved by
the NVIDIA container runtime through host NVML) nor host-side
`nvidia-smi --id=0` — and those are the two selectors `cuda-preflight.sh`
actually uses.

### 8.3 `cuda-preflight.sh` required git — resolved

**Historic, kept because the shape recurs.** The script called
`git -C "$REPO_ROOT" rev-parse HEAD` at two sites, and `CANDIDATE_SHA` feeds
`build-input.json`, the witness payload, and
`verify-cuda-preflight-witness.py --candidate-sha`. A transfer workflow that
deliberately ships **no `.git`** died at the first call.

Resolved by `scripts/lib/cuda-candidate-sha.sh` (`resolve_cuda_candidate_sha`),
sourced and called once, early — before any Docker or model-cache work — so an
invalid override aborts before side effects. The script now contains **no `git`
invocation at all**. Three properties worth preserving in any future edit:

- **Both** sites needed it. The `repository_commit=` call ran unconditionally
  *before* the `CANDIDATE_SHA=` one, so fixing only the latter would still have
  died on a no-git remote.
- **One resolution, two consumers**, so `repository_commit=` and the witness's
  `candidate_sha` cannot disagree.
- **`FATHOMDB_CANDIDATE_SHA` fails closed** — validated as 40-character
  lowercase hex; set-but-invalid is an error, never a silent fall-through to
  `git`. The value is stamped into a witness as a **provenance claim**, so a
  bogus SHA silently accepted would forge provenance. Unset falls back to
  `git rev-parse`, unchanged.

The generalizable rule: **an environment override for a provenance value must
validate and abort, never degrade.**

### 8.4 Open gaps — do not describe these as green

| Gap | Detail |
|---|---|
| **`dev/platform-capabilities.json` has no Tegra entry** | The manifest is gated by `scripts/check-platform-capabilities.sh`, but its schema assumes **every platform is an npm loader triple** (`npm_package`, `package_dir`, `rust_target`). Tegra is a **Python-only artifact family with no npm package by ruling** (§ 3.7) and **no distinct Rust target** (§ 2), so it does not fit the existing row shape. Adding a row naively would either break the gate or require inventing an npm package name that D-80.7-3 forbids. **Unresolved — the manifest needs an artifact-family concept before Tegra can be represented in it.** |
| **`embed-cuda` clippy is not clean** | `cargo clippy -p fathomdb-embedder --features embed-cuda --all-targets -- -D warnings` fails with **7** errors. Verified pre-existing: the same command at `2dbe7c63` fails with **9**. 80.5 improved it 9 → 7. Remaining items are `tc5-benchmark`-only, dead in an `embed-cuda`-without-`tc5-benchmark` build. **Must not be described as green.** |
| **`ir_c_recall_run.rs:327` E0308** | `cargo check -p fathomdb-engine --test ir_c_recall_run --features embed-cuda` fails. Pre-existing, in an untouched crate. |
| **`datetime.UTC` on Python 3.10** | § 4.4. Works under the repo 3.13 venv; any hosted job on 3.10 dies on import. |
| **UUID string equality in the x86_64 verifier** | § 7.1. `verify-cuda-preflight-witness.py:335`. Tracked for 80.6.5. |
| **libtest has no "skipped" status** | The GPU-gated SKIP arm prints its named reason but the harness line still reads `ok`. Not a false pass in substance — nothing asserts GPU engagement — but it reads like one. |

## 9. Import-time co-installation guard

**AC80-25.** Retained, but **re-scoped** after D-80.6-3 was revised: under a
single distribution name pip will never install two `fathomdb` distributions, so
**this guard should never fire in the shipped configuration**. It stays because
it is cheap and because the source-build path lets a user hand-build a
locally-named sibling distribution — exactly the case it catches.

**It may not be cited as the reason the naming decision is safe.** That role
belongs to AC80-27 (§ 3.5). Do not delete it on the grounds that it is
unreachable, and do not implement one of the two checks and claim it covers the
other.

**Implementation note worth keeping — `importlib.metadata` is the wrong tool
here on both axes.** Measured:

- It costs **48 ms of a 91 ms `import fathomdb`** (it drags in `email` and
  `zipfile`).
- `packages_distributions()` returns `None` for `fathomdb` under an editable
  maturin install — this repo's own `.venv` — because the `RECORD` holds only
  `fathomdb.pth`. So it is **blind to one side of the collision it is meant to
  detect**.

The landed guard (`src/python/fathomdb/_coinstall.py`) is an `os`/`sys`-only
`sys.path` `.dist-info` name scan: **0.42 ms** at 18 distributions, **0.87 ms**
at 318 — a net ~1% on import. Any future revision must keep both properties:
cheap, and able to see editable installs.

**Re-measured at 80.6 acceptance, from the installed wheel** (6 runs of
`python -X importtime -c "import fathomdb"`, `_coinstall` self-time): 693, 576,
766, 761, 787, **1216** µs — range **0.58–1.22 ms**, mean ~0.80 ms. **One run
exceeded the 0.87 ms figure above by ~40%.** The *relative* claim held across
every run (0.91 %–1.34 % of total import), which is why this reads as ordinary
scheduler jitter on shared hardware rather than a regression. Recorded rather
than smoothed: **treat ~1 % of import as the durable claim, and 0.42–0.87 ms as
a best-case band, not a ceiling.**

## 10. Related documents

| Path | Role |
|---|---|
| `dev/design/0.8.23-aarch64-tegra.md` | **Authoritative** — requirements, acceptance criteria, and all `D-` decisions this file explains |
| `dev/plans/runs/0.8.23-slice-80-status.md` | What actually landed, per sub-slice, with evidence |
| `dev/design/0.8.23-slice-10-cuda-contract.md` | The x86_64 CUDA contract this lane deliberately does not disturb |
| `dev/design/0.8.23-gpu-artifacts.md` | Runner group discipline precedent |
| `docs/compatibility/index.md` | Public per-family glibc floors (gated by `check-glibc-floor-doc-truth.sh`) |
| `dev/platform-capabilities.json` | Platform manifest — **does not yet represent Tegra**, § 8.4 |
