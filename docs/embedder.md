# Default Embedder

FathomDB can embed your documents for you with a built-in, in-process embedder,
so you do not have to wire up an embedding model yourself. It is **opt-in**: a
fresh engine has no embedder configured and vector writes fail with
`EmbedderNotConfiguredError` until you either enable the default embedder or supply
your own (Rust only, today).

> Status: 0.8.27 release documentation. The ANN-fidelity
> recall@10,
> measured on the pre-fusion **vector stage** (the SUT the 0.90 floor gates), is
> **0.896** (95% CI 0.864–0.925, N=7,667) and holds the **0.90 floor under the
> one-sided CI gate** (`recall_ci_hi ≥ 0.90`) — see
> [Caveats](#caveats-and-limitations).

## What it is

| | |
|---|---|
| Model | `BAAI/bge-small-en-v1.5` |
| Dimensions | 384 |
| Runtime | `candle-transformers` BERT, pure-Rust, in process (no Python, no sidecar) |
| Pipeline | WordPiece tokenization (max 512 tokens) → mean-pool → L2-normalize → mean-centering → sign-bit quantization → bit-KNN (K=192) → f32 rerank → top-10 |

Documents longer than 512 tokens are truncated to the model's context window.

## Enabling it

The default embedder is gated behind a build feature so users who never use it
pay no dependency or binary-size cost. Install the embedder-enabled
distribution, then opt in at `open`.

### Python

```python
from fathomdb import Engine

engine = Engine.open("mydb.sqlite", use_default_embedder=True)
report = engine.open_report()
print(report.default_embedder.name)   # "fathomdb-bge-small-en-v1.5"
```

The published Python wheel already includes the default embedder; there is no
`default-embedder` extra to install. Install the package normally:

```bash
pip install fathomdb
```

### TypeScript

```ts
import { Engine } from "fathomdb";

const engine = await Engine.open("mydb.sqlite", { useDefaultEmbedder: true });
console.log(engine.openReport().defaultEmbedder.name); // "fathomdb-bge-small-en-v1.5"
```

The embedder-enabled native binary is larger; see your platform package notes.

### Default OFF

If you do not pass the flag, the engine opens normally but has no embedder, and
vector writes raise `EmbedderNotConfiguredError`. This is unchanged from earlier
releases. Supplying your own custom embedder is available in the Rust API today;
custom Python/TypeScript embedders are planned for a later release.

## First use: weight download

The model weights are **not** bundled. The first time you open an engine with
the default embedder, FathomDB downloads the pinned weight files (~133 MB total)
from a fixed Hugging Face URL set, caches them under your platform cache
directory, and verifies every file by sha256 before loading. Subsequent opens
read from the cache and do no network I/O.

This first-use download is the single, narrowly-scoped exception to FathomDB's
"no implicit network access" rule, and it is **visible**: the open report records
what happened.

```python
report = engine.open_report()
print(report.embedder_download_ms)   # Some(ms) on a cold cache, None when warm
for ev in report.embedder_events:    # per-file url, bytes, sha256, cache path
    print(ev)
```

Notes:

- **Offline / pre-warming.** Warm the cache ahead of time (e.g. in CI or an
  air-gapped build step) so the first real open does no network I/O. A cold
  cache with no network will fail the open with a clear loader error rather than
  hanging silently.
- **`HF_TOKEN`.** If set, it is sent as a bearer token for token-gated mirrors.
  The public `bge-small-en-v1.5` does not require one. No other credential
  source is consulted, and the token is never persisted.
- **Integrity.** A file whose sha256 does not match the pinned value is removed
  and the open fails; there is no trust-on-first-use.

## Mean-centering

Real embeddings cluster on a narrow cone of the vector space, which hurts
sign-bit quantization. FathomDB corrects for this by subtracting a per-workspace
**corpus-mean vector** before the sign-bit step (the full-precision rerank stays
un-centered). The mean is computed once, from the first 256 vectors you ingest,
pinned in the workspace, and not silently recomputed thereafter.

A consequence worth knowing: if the **first 256 documents** you ingest are not
representative of the rest of your corpus (for example, you load one topic
first and pivot later), the pinned mean can be skewed and retrieval quality may
suffer. The remedy is to reindex; an automatic reindex/refresh path is planned
for a later release.

## GPU acceleration (opt-in)

CUDA is an opt-in **artifact capability**, not an end-user rebuild step. A
supported CUDA-capable artifact contains both CPU and CUDA paths; CPU-only
artifacts remain usable without an NVIDIA driver. GPU acceleration is most
valuable for bulk embedding (initial ingest, re-indexing, offline evaluation).

Scope note: when an engine resolves `FATHOMDB_EMBED_DEVICE` to CUDA (whether
`auto` on a CUDA-capable artifact or forced `cuda:N`), that device is used for
**all** embedding the engine instance performs — both ingest **and** per-query
embedding of the query string. What stays **CPU-only / 1-bit (Hamming) /
deterministic regardless** is the *retrieval* machinery: the stored sign-bit
vector index, the Hamming scan over it, and RRF fusion. Because query embedding
follows the selected backend, you should query a workspace with the **same
backend that built its index** (see the cross-backend discipline below) — that is
the point of the env knob, not a footgun.

The artifact builder enables CUDA support once. Users then select its runtime
policy without recompiling:

1. **Artifact build capability.** The release/package builder selects the
   backend at build time:
   - `embed-cuda` — NVIDIA CUDA (needs the CUDA toolkit, e.g. 12.6).
   - `embed-metal` — retained build support; the public Slice 70 selection
     policy currently does not expose a Metal request.
   Application users of a supported CUDA-capable package do **not** rebuild it.
   For local package development, build the Python extension on the **main
   checkout** (never a worktree):
   `maturin develop --features pyo3/extension-module,embed-cuda`.
2. **The `FATHOMDB_EMBED_DEVICE` environment variable** selects the device at
   runtime: `auto` · `cpu` · `cuda:N` (GPU N). The unset/default policy is `auto` only on a CUDA-capable artifact;
   it is resolved once, when the engine opens, and the result is exposed in `OpenReport`.
   `auto` probes CUDA and records a typed CPU result when CUDA is unavailable;
   a CPU-only artifact records `cuda_not_compiled`; `cpu` never probes CUDA; and
   `cuda:N` is forced and raises a typed open error if it cannot be used. Bare `cuda`, `metal`,
   whitespace/case variants, and other spellings are configuration errors —
   embedding never silently changes a forced GPU request into CPU execution.

The device is **not** part of the embedder identity. Python and TypeScript expose
the resolved open-time evidence as `embedder_device_resolution` /
`embedderDeviceResolution`; selection remains the environment variable rather
than a new binding-specific setting. `FATHOMDB_RERANK_DEVICE` independently
selects the optional cross-encoder with the same exact `auto` · `cpu` ·
`cuda:N` grammar. A forced `cuda:N` reranker request fails typed rather than
running CE on CPU.

### CUDA library and GPU requirements

The CUDA artifacts are built with CUDA 12.6 and need:

- **An NVIDIA driver of 560.28.03 or later** (the CUDA 12.6 minimum). A
  newer driver, CUDA 13 drivers included, runs them.
- **A GPU of compute capability 7.5 or later on x86_64** (Turing: GeForce
  RTX 20 and GTX 16 series, Quadro RTX and T series, Tesla T4, and every
  later GPU). The Jetson artifacts target the AGX Orin. A GPU below the
  floor is refused with `cuda_incompatible` before FathomDB builds a device
  on it. `auto` tries only the first visible GPU, so on a host with an older
  GPU at index 0 set `CUDA_VISIBLE_DEVICES` to a supported one.
- **CUDA 12 cuBLAS and cuRAND libraries** (CUDA 12.x; tested with cuBLAS
  12.1 to 12.9). A CUDA toolkit provides them, as do the PyPI wheels. If
  one is missing, reports another major, or lacks a function FathomDB uses,
  FathomDB refuses it before any GPU work with the typed kind
  `cuda_library_unavailable` (reason `missing`, `wrong_major` or
  `missing_symbol`): a forced `cuda:N` fails with the library, file,
  version and remedy, and `auto` runs on CPU with that reason. The process
  writes one `fathomdb-cuda-libraries {json}` line to stderr. The remedy,
  which also covers a host with only CUDA 13 libraries:

  ```sh
  pip install nvidia-cublas-cu12 nvidia-curand-cu12
  export LD_LIBRARY_PATH="$(python3 -c 'import nvidia.cublas as m; print(m.__path__[0])')/lib:$(python3 -c 'import nvidia.curand as m; print(m.__path__[0])')/lib:$LD_LIBRARY_PATH"
  ```

  The CUDA library loader does not search Python's site-packages on Linux,
  so both directories must be on `LD_LIBRARY_PATH`.

`fathomdb doctor cuda-libraries` shows which files a process loads and why
one is refused ([CLI reference](reference/cli.md)); the open report's CUDA
device facts list them too (`cuda_libraries` in Rust and Python,
`cudaLibraries` in TypeScript).

FathomDB-built artifacts (the wheels, npm packages and CLI) link the CUDA
runtime statically, so their CPU mode loads on a host without `libcudart`.
A Rust project that builds `fathomdb-embedder` from crates.io with
`embed-cuda` (or `tegra-pool`) gets the published fork crates and links
`libcudart` dynamically through upstream `candle-kernels` 0.10.2: set
`LIBRARY_PATH` to the CUDA library directory to link, and keep `libcudart`
loadable at run time.

### Embedding and cross-encoder on one GPU

`FATHOMDB_EMBED_DEVICE=cuda:0` and `FATHOMDB_RERANK_DEVICE=cuda:0` may select
the same GPU in one process. They create independent Candle model instances;
there is no resource manager, device reservation, or hard Candle memory cap.
GPU utilization and memory observations are sampled device-wide values, not a
per-request allocation or performance guarantee. The vector index, bit-KNN,
exact-f32 database rerank, FTS, RRF fusion, graph, and storage remain CPU-only
regardless of where embedding or CE runs.

### Node.js on Jetson: load fathomdb first

This applies to a Node.js addon built with CUDA (`embed-cuda` or
`rerank-cuda`) on an NVIDIA Jetson running Linux AArch64. That includes the
`fathomdb-linux-arm64-gnu-tegra` package for a confirmed classic Jetson Orin,
introduced in 0.8.27. It is installed as an exact-version tarball from the
Tegra Pages site, at the same version as `fathomdb`, after
checking its published SHA-256; see
[TypeScript install](install/typescript.md#classic-jetson-orin-with-cuda).
The generic Linux AArch64 npm package is CPU-only and does not need any of
this. Python's Tegra wheel starts CUDA at import instead; see "Jetson: private
CUDA memory pool" below.

The 0.8.28 Tegra Node addon build includes `tegra-pool`, creating a private
pool at first CUDA device use when its Tegra checks pass. The 0.8.27 Tegra
addon did not include the private pool. The generic arm64 npm package remains
CPU-only.

Starting CUDA at import is an AArch64-only exception: the Linux x86_64 npm
package never probes the CUDA driver at import. A missing driver never fails
the import on either.

On a Jetson, the CUDA driver must reserve one large range of the process's
virtual address space when it starts. On the Jetson AGX Orin 64 GB that range
is about 61 GiB. Node.js scatters its JavaScript heap across the same region
as the heap grows. Once an application holds roughly 70 MiB or more of
JavaScript data, CUDA may be unable to start at all. The FathomDB addon
therefore starts CUDA as soon as Node loads it, which secures that first
reservation, so later heap growth can no longer stop CUDA from starting.
CUDA's default memory pool can still need further address space later; see
"Steady embeddings can be slower" below.

**What to do:** load fathomdb before your application builds large in-memory
data. Either make it the first import of your entry module:

```ts
import { Engine } from "fathomdb"; // first import of the entry module
```

or start Node.js with it preloaded, which runs before any application code:

```bash
node --import fathomdb app.js
# or
NODE_OPTIONS=--import=fathomdb node app.js
```

**What to expect:**

- The process's virtual size (`VSZ`, `VmSize`) grows by about 61 GiB right
  after the import. This is reserved address space, not RAM. Resident memory
  grows by about 1–3 MiB. The import takes about 12 ms longer, or about 120 ms
  longer when the GPU has been idle.
- CUDA starts once per process. Loading fathomdb first inside a
  `worker_threads` worker works the same way.
- When every component that the addon was built to run on CUDA is set to
  `cpu` at the time fathomdb is loaded, the addon does not start CUDA at all.
  That means `FATHOMDB_EMBED_DEVICE` if its embedder was built with CUDA, and
  `FATHOMDB_RERANK_DEVICE` if its cross-encoder was built with CUDA.
- `FATHOMDB_CUDA_EARLY_INIT=off` turns early start-up off. Use it only if
  you must not reserve the address space before opening an engine. A large
  heap can then stop CUDA from starting again.
- With no visible GPU, the import still succeeds silently, and `auto` uses
  the CPU.

**Symptom of a late import.** If `cuInit` fails after the heap grew before
fathomdb was loaded, a forced `cuda:N` open fails with `EmbedDevicePolicyError`
(`FDB_EMBED_DEVICE_POLICY`, kind `cuda_probe_failed`), or
`RerankerDevicePolicyError` for the cross-encoder. The message may continue
with `cuInit returned CUDA_ERROR_OUT_OF_MEMORY` and name both remedies when
the latest probe of that component still records that failure. Concurrent
probes of the same component can replace this best-effort hint; the error
code, kind and forced-CUDA refusal remain stable. Under
`auto` the engine uses the CPU instead, and
`openReport().embedderDeviceResolution.reason` is `cuda_probe_failed` for
the failed embedder probe.

**Why a late import can still fail.** Loading fathomdb only helps if the
address space still has room. In measurements on the AGX Orin 64 GB, a heap
of about 74 MiB before the first CUDA use made CUDA fail to start in about
half of the processes, and a heap of about 160 MiB or more in all of them.
Freeing data or forcing garbage collection did not help. Load fathomdb
earlier instead of retrying.

**Steady embeddings can be slower.** CUDA's default memory pool on the
Jetson needs a further contiguous range of about 20 GiB. When the process
layout cannot provide it, FathomDB falls back to synchronous GPU allocation.
Steady-state embeddings are then about 1.8–2.8 times slower: about 25 ms
against 9–15 ms per embedding on the AGX Orin 64 GB. Loading fathomdb first
does not prevent this, and most Node.js processes in the measurements took
this path. Results are correct either way.

These measurements come from one Jetson AGX Orin 64 GB (L4T R36, CUDA 12.6).
Other Jetson models are unmeasured. So are non-Tegra AArch64 Linux CUDA hosts
such as Grace Hopper, GB10 and SBSA servers, which also build this behaviour.

### Jetson: private CUDA memory pool

Since 0.8.28, FathomDB on the Jetson AGX Orin 64 GB has its own CUDA
memory pool, capped at 3 GiB, that hands freed memory back to the system at
once. Every allocation that fits succeeds, and the memory is returned when
the work ends, without the slower synchronous path described above.

Every FathomDB build with CUDA on AArch64 Linux includes the pool:
`embed-cuda` and `rerank-cuda` imply the `tegra-pool` Cargo feature, and
naming `tegra-pool` as well is accepted and redundant. That covers the Tegra
Python wheel (`fathomdb==<version>+tegra`), the Tegra Node package and a Rust
project built with `embed-cuda`. x86_64 builds compile none of it. Other
Jetsons and other AArch64 CUDA hosts keep the earlier behaviour unless the
pool mode is `on` and they pass the remaining checks.

- **Python starts CUDA at import.** The Tegra wheel starts the CUDA driver
  once when `fathomdb` is imported, under the same rules as the Node addon
  above (skipped when every CUDA-capable policy is `cpu`, or with
  `FATHOMDB_CUDA_EARLY_INIT=off`). In Python and Node the pool is used
  only if that start-up ran.
- **Rust needs no extra call.** A Rust process has no import hook. The pool
  accepts the CUDA start-up that FathomDB's own device check makes when an
  engine opens with CUDA, so a Rust program gets the pool without calling
  anything (its report shows `module_load_init` `not_at_load` with reason
  `private_pool`). `FATHOMDB_CUDA_EARLY_INIT` has no effect in Rust; turn
  the pool off with `cuda_pool = off`.
- **Other AArch64 hosts start CUDA at import too.** On an AArch64 Linux host
  that is not a Jetson (Grace, Graviton, SBSA servers), a Python extension
  built from source with CUDA also starts CUDA at import, so the `fork` rule
  below applies there as well. Node already behaves this way. No published
  artifact targets these hosts.
- **`fork` breaks CUDA in children.** After `import fathomdb`, a
  `multiprocessing` child created with the `fork` start method cannot use
  CUDA. Use `spawn` or `forkserver` (the Linux default from Python 3.14), or
  set `FATHOMDB_CUDA_EARLY_INIT=off`. This can break code that worked with
  0.8.27.
- **Settings.** Each is an open setting in every SDK (Rust `EngineConfig`,
  Python `EngineConfig` or keyword, TypeScript `engineConfig`) and a flag
  on `fathomdb doctor gpu`, `reranker-gpu` and `cuda-allocator`. Its
  environment variable applies only when no open sets it:
  - `cuda_pool` (`cudaPool`, `--cuda-pool`): `auto` (default), `on`, or
    `off` (the earlier behaviour); fallback `FATHOMDB_POOL_MODE`;
  - `cuda_pool_max_size` (`cudaPoolMaxSize`, `--cuda-pool-max-size`): the
    cap in bytes, at least 32 MiB (the CLI flag and the environment also
    take `<n>G` or `<n>M`); fallback `FATHOMDB_POOL_MAXSIZE`;
  - `cuda_pool_release_threshold` (`cudaPoolReleaseThreshold`,
    `--cuda-pool-release-threshold`): `0` or `max`; fallback
    `FATHOMDB_POOL_RELEASE_THRESHOLD`. With `0` (the default), freed GPU
    memory returns to the system automatically; FathomDB never holds freed
    memory. With `max`, freed GPU memory stays reserved for reuse while
    any FathomDB model is open in the process, and is returned when the
    last one closes. "Closes" means the model is dropped, which can be
    shortly after `close` returns.

  A malformed environment value turns the pool off; it never stops the
  process. `cuda_pool = off` turns the pool off; it does not stop the Node
  and Python import-time CUDA start-up, which `FATHOMDB_CUDA_EARLY_INIT`
  controls.
- **One pool configuration per process.** The GPU pool settings
  (`cuda_pool`, `cuda_pool_max_size`, `cuda_pool_release_threshold`) are
  fixed once per process: by the first open that passes any of them, or, if
  none does, by the `FATHOMDB_POOL_*` environment variables (or their
  defaults) at the first GPU use. A later open that sets one of them to a
  different value fails with an invalid-argument error naming the setting
  and the value in force. Closing every engine does not reset them. Opens
  that omit them never fail this way. An open that sets any of them fixes the
  configuration on every platform, including machines where the pool never
  engages (x86, discrete GPUs, CPU-only engines). Whether an open that omits
  them fixes the configuration depends on whether it builds a GPU device; set
  the same values on every open, or on none.

  In practice: pick one set of values per process, or leave them unset. A
  test suite that runs in one process (for example a single pytest run)
  must not open engines with different pool settings; run such cases in
  separate processes.

  A `cuda_pool_max_size` larger than the GPU's memory cannot be checked
  until the pool is created; the pool is then turned off, and the allocator
  report's reason is `invalid_setting`.
- **Usable size.** On JetPack 6 a capped pool serves about a third of its
  cap, so the 3 GiB default serves about 1 GiB (FathomDB's peak measured on
  the Orin was 288 MiB), and `FATHOMDB_POOL_MAXSIZE` buys about a third of
  its value. A cap near total device memory is not recommended.
- **When the pool is full**, the request fails with
  `CudaPoolExhaustedError`, and the next request runs normally.
- **Module-level models keep their memory.** `embed_batch_cls` /
  `embedBatchCls`, module-level `rerank`, the Rust SDK's standalone
  functions and the engine's reranker keep one model for the life of the
  process, so their pool memory is held until it exits, and under `max` the
  pool keeps its reservation while one of them is open. An engine's own
  embedder returns its memory when the engine closes.
- **Do not reset the GPU context underneath FathomDB.** If another library in
  the same process resets the CUDA primary context, FathomDB raises
  `CudaContextLostError` and writes one `fathomdb-cuda-context-lost` line to
  stderr; closing afterwards can still crash the process. This is
  unsupported in 0.8.28.

`fathomdb doctor cuda-allocator` shows which allocator a process gets and
why ([CLI reference](reference/cli.md)). Each SDK's open report carries the
same decision on the CUDA device facts: `cuda_allocator` in Rust and Python,
`cudaAllocator` in TypeScript.

### Measured speedup

Re-embedding a 2,000-document corpus, single-stream, on one RTX 3090 vs CPU
(`cargo run --release --example gpu_speedup`):

| Path | CPU | CUDA (`cuda:0`) | Speedup |
|------|-----|-----------------|---------|
| per-document `embed()` | 17.8 docs/s | 276.8 docs/s | **15.6×** |
| batched `embed_batch()` | 8.8 docs/s | 821.8 docs/s | **93×** |

Bulk re-embed end-to-end (CPU per-doc → CUDA batched) is **~46×**. On CPU,
batching is *slower* than per-document calls (padding overhead with no
parallelism to amortize it); on GPU, batching wins decisively. A 27-hour CPU
re-embed becomes minutes.

### Cross-backend discipline

Stored vectors are valid only for the exact embedding function that produced
them — model, revision, **and backend numerics**. `EmbedderIdentity`
(name/revision/dim) is the cheap pre-filter: it catches a model/revision/dim
mismatch but **not** numeric divergence between two backends sharing one
identity.

Since 0.8.18 the engine also runs a **vector-equivalence self-check** at open:
45 pinned probes are compared against a stored reference baseline, and a
divergence past the floor sets `dense_disabled` so every vector-dependent arm
refuses at query time with `VectorEquivalenceMismatchError`. The FTS-only path
(`search_text_only` / `searchTextOnly`) stays serviceable.

⚠ **Since 0.8.20 that verdict is cached.** The engine fingerprints the embedder
identity, the pinned mean vector, the probe fixture, the divergence floors and
the stored baseline; an open whose fingerprint is unchanged does **zero** probe
embeds and reuses the previous verdict. So `dense_disabled` reports the arm's
status *as verified at the last open whose fingerprint differed*, not a fresh
per-open re-verification — a same-identity backend drift (CPU↔GPU, a rebuilt
library or driver) is no longer caught per-open. An identity *change* still
refuses the open ahead of any cache. **The self-check is not tamper evidence**:
nothing at rest is authenticated, so an actor with write access to the database
file can rewrite the baseline, the cached marker, or the vectors. Do not read
`dense_disabled` as a tamper signal.

A **same-backend build-and-read** discipline therefore remains the conservative
default: read an index with the same backend that built it.

In practice the divergence for this model is tiny: measured CPU-vs-CUDA cosine
was **0.99999983** (max per-component Δ ≈ 1.6e-7), and at the 1-bit sign-bit
quantization the retrieval path actually uses, **0 of 6,144 bits disagreed** —
the CUDA-built and CPU-read codes were identical on the probe set. The
same-backend rule is the conservative default; the probe-set guard will replace
it with an enforced, calibrated tolerance.

## Caveats and limitations

- **Recall.** The ANN-fidelity measurement — how faithfully the 1-bit
  sign-quant index reproduces the same model's exact f32 top-10 — is
  **recall@10 = 0.896** (95% CI 0.864–0.925) on the real bge-small embedder
  (N=7,667, K=192, mean-centering), measured on the pre-fusion **vector stage**
  (the SUT the floor gates). The **0.90 floor holds under the one-sided CI
  gate** (`recall_ci_hi 0.925 ≥ 0.90`). This is an ANN/quantization-fidelity
  number, **not** an IR-relevance number. (An earlier **0.937** figure was
  measured on a different, pre-correction `search()` SUT; the 0.937→0.896
  difference is a measurement-SUT change, not a fidelity regression, and is not
  caused by embedder pooling. An earlier ~0.83 dev-box figure was a measurement
  artifact — exclude-after-top-10 plus body-string ground truth over a corpus
  with duplicate bodies.) A full-scale N=1M run remains infeasible on commodity
  hardware; the N≈7.7k value is treated as a near-upper bound (recall declines
  slowly with N).
- **Retrieval latency grows with corpus size.** There is no ANN index — the
  vector arm is a full scan. See
  [compatibility § performance posture](compatibility/index.md).
- **Topic-drift mean** (see above): pinned on the first 256 docs; reindex to
  refresh.
- **Custom Python/TypeScript embedders** are not exposed as of 0.8.20; the
  binding surface is binary (built-in default embedder, or none).

## Upgrading an existing workspace

A workspace that was previously opened **without** the default embedder recorded
a `fathomdb-noop` embedder identity. Re-opening it with the default embedder
fails closed with an identity mismatch — this is intentional: vector identity
belongs to the embedder, and silently re-embedding under a new model would
corrupt retrieval. To adopt the default embedder on existing data, create a
fresh workspace and re-ingest (wipe-and-rewrite). There is no in-place swap.

## See also

- `OpenReport` fields: [Python](install/python.md), [TypeScript](install/typescript.md)
- Vector identity rationale: [Embedder Identity](positions/embedder-identity.md)
