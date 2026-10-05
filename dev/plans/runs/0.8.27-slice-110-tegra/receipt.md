# Slice 110 Tegra NAPI qualification receipt

**Date:** 2026-10-04  
**Candidate:** `366e1bc3d4e4df2a8bb8cd9268ccfdae0e02b08a`  
**Product-code baseline:** `87670f61d` (the candidate adds documentation only)  
**Host:** Jetson AGX Orin, Linux `aarch64`, L4T kernel `5.15.199-tegra`, CUDA
12.6 / NVIDIA driver 540.5.0, GPU UUID
`GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82`.

## Disposition

| Row | Result | Evidence |
| --- | --- | --- |
| Node 25 thin-main and installed CPU package | PASS | Fresh external consumer, runtime surface, subscriber, reopen, typecheck, and CPU embedding all pass. |
| Tegra `embed-cuda,rerank-cuda` package build and installed package | PASS | The host-native CUDA package builds and a fresh external consumer passes the same package, surface, subscriber, reopen, and typecheck checks. Forced CPU embedding and reranking through this CUDA-capable package also pass. |
| Node NAPI forced-CUDA runtime | FIXED for the allocator failure; pending independent review | Root cause and fix: [allocator fallback](#allocator-root-cause-and-fix). With the fix, 100 / 100 fresh forced-CUDA processes passed open, embed, rerank and the allocation witness (Node 25, 24 and 26; in-tree and installed). Open limitations: `cuInit` failure in very heap-heavy processes and a slower fallback path. |

The sections before [the allocator fix](#allocator-root-cause-and-fix) record
the investigation while this row was OPEN; their OPEN wording is historical.
The open row was a runtime-capacity/availability finding, not a claim that the
artifact lacks CUDA. It preserves the architecture's policy boundary: the
JavaScript thin main keeps the stable public API, while the platform package
owns the native binary and CUDA capability. The package can be built and
installed correctly even when a forced runtime policy must refuse an
unavailable CUDA device. The public typed refusal is therefore the consumer
contract; the underlying driver error is retained in diagnostic evidence for
release triage.

## Exact source, tools, and artifacts

The isolated Jetson source tree came from a local `git archive` of the
candidate. Archive SHA-256:
`9dfaaa9a994e167486f05a5bc91f582ad5ee257188650454255a277fe4c45d52`.
The copied `src/ts/package-lock.json` SHA-256:
`36ed30648b87f581f4abb608ff1774dbcd5a79e7b3c94524d2bcf55749eb5709`.

The build used Node `v25.9.0` / npm `11.12.1`, Rust `1.95.0`, GCC `11.4.0`,
and NVCC CUDA `12.6.68`. The CUDA build used `CUDA_COMPUTE_CAP=87`,
`CUDA_PATH=/usr/local/cuda-12.6`, the aarch64 CUDA library directory, and
`embed-cuda,rerank-cuda`.

| Artifact | SHA-256 |
| --- | --- |
| CPU `.node` | `7601325d16e6639e44a179c0913f870bddba427750cf392dd42004cce3457a8f` |
| CPU main tarball | `4c71a9a951776ffba54cc48a92ee5820ecc0772f2a5c53978b5758d9694e3227` |
| CPU platform tarball | `4e3779444adf01ca5dd6c9347041b29957f5824654cbc76c71b946b3cd90a8ef` |
| CUDA/rerank `.node` | `76b55ea990f052d18f80a522649a0de0927d6bcd789248225e06cb28c5482322` |
| CUDA/rerank main tarball | `4c71a9a951776ffba54cc48a92ee5820ecc0772f2a5c53978b5758d9694e3227` |
| CUDA/rerank platform tarball | `5e18da510d299721bace18bc1d1eb625c9d196b3cdccc8243f55e05c49ad5f9b` |

For both rows, the installed external consumer's resolved
`fathomdb-linux-arm64-gnu` binary had the matching build hash. Packed-file
inspection found no test hooks, tests, or `node_modules` payload. The CUDA
binary records no `RPATH` or `RUNPATH`; direct `NEEDED` entries are only
`libstdc++.so.6`, `libgcc_s.so.1`, `libm.so.6`, `libc.so.6`, and
`ld-linux-aarch64.so.1`. CUDA's driver library is dynamically discovered by
CUDARC; `LD_DEBUG=libs` confirmed a successful load of
`/usr/lib/aarch64-linux-gnu/nvidia/libcuda.so`.

## Installed-consumer checks

Each package pair was installed offline with lifecycle scripts disabled into a
fresh directory outside the source tree. Under Node 25, both consumers passed:

- exact 17 native exports and exact 44 `Engine` prototype names from
  `entry-runtime.json`;
- absence of `ForTest` and `forcePanic` exports;
- write subscriber delivery (two events), string `droppedRecordsTotal`, close,
  rejection after close, and successful reopen;
- invalid typed configuration rejects with `RangeError` before a database is
  created; and
- external consumer TypeScript typecheck.

The CPU artifact opened `useDefaultEmbedder` with `FATHOMDB_EMBED_DEVICE=cpu`,
produced a 384-dimensional vector, and reported the CPU device. The
CUDA/rerank artifact, forced to CPU for a deterministic check, produced the
same 384-dimensional embedding and two non-null cross-encoder rerank scores
(`0.9999354806829863` and `0.000012546185781769715`).

## Forced-CUDA runtime investigation

The normal public Node error is a typed `FDB_EMBED_DEVICE_POLICY` with
`kind: cuda_probe_failed` and message
`cuda:0 requested but unavailable: CudaProbeFailed`. An isolated temporary
diagnostic print in the NAPI dependency showed the hidden cause during a
10-process Node 25 run:

```text
DriverError(CUDA_ERROR_OUT_OF_MEMORY, "out of memory")
```

The diagnostic source was restored before this receipt; before/after hashes
matched for both edited Rust files. The diagnostic repeat log SHA-256 is
`8238b7d7dd36d8e5d99bdbae1d2c7a6f397cba80d7323f0fa7854976072b81e3` and
the restoration record SHA-256 is
`9606fe9fdff2b1c6c33d0f2cdfd5de5fd6f42d7e120cdcca37856f95ab4c2eef`.

A focused no-rebuild A/B used the same installed CUDA package and a library
path containing both the NVIDIA driver and CUDA toolkit:

| Runtime | Clean forced-CUDA `Engine.open` runs |
| --- | --- |
| Node 24.15.0 | 5 / 10 pass |
| Node 25.9.0 | 3 / 10 pass |
| Slice 103 Python wheel | 5 / 5 pass |

Host `MemAvailable` stayed about 51.52 GB before every run. `nvidia-smi`
reported no compute processes but, on this integrated Orin, reports GPU memory
as `N/A` / unsupported; it cannot establish a sole-consumer or free-GPU-memory
precondition. The shared-host process snapshot was retained and no unrelated
process was terminated. `tegrastats` is available for a future reserved-host
repeat. The A/B evidence SHA-256 values are
`2c32d0f27ce34c180e871147117decbe4cf354694605a4b0393694cd67a9b0bf`
(Node), `3e8c8c003644ce54fdcb735a8bc0da7d71c53f6b20b2e6d120484c9f2c216604`
(Python), and
`74a309d482c3f2535e5d67d22aed5378cf518915f71486ebd254c75f2799734c`
(host snapshot).

A standalone CUDA Driver API probe (`cuInit` and `cuDeviceGetCount`) succeeds
with either toolkit-only or driver-plus-toolkit library paths. The NAPI
intermittency affects both Node 24 and Node 25, while the Python wheel passes
the focused sequence, so the evidence does not support treating this as a
Node-25-only loader failure or ordinary host-RAM exhaustion.

An isolated temporary Candle probe using the same `embed-cuda` feature and the
same `Device::new_cuda(0)` plus `Tensor::zeros` operation passed 10 / 10 on
the process main thread and 10 / 10 on a spawned Rust thread. This does not
support thread placement as the immediate cause. During a subsequent Node 25
failure, `tegrastats` reported RAM `13364/62828MB`, swap `2/31414MB`, and
`119x4MB` largest free blocks, with GPU utilization `0%`. It points to an
allocator/fragmentation or NAPI process-context condition that needs a
reserved-host follow-up. The temporary example was removed. The Candle
thread-placement, Node failure, and tegrastats evidence hashes are
`a2fa1f337c469b8a84ec7a98841dd194b516793adf84f897ffdc7d98e44f5fd8`,
`3a52c27e7a2064f37e65f86a65bb1dd684a345d4224ff5993500de62d3bcc6c3`, and
`aeadab1c6cb120e581d2fcd2e2ff73d030bcf725e3ea71159e5d4f371f657001`.

## Scope control

## Post-reboot requalification — candidate `c2f39b29`

The Jetson was updated and freshly rebooted at `2026-10-04T14:12:55Z`.
This repeat used current release head `c2f39b29d3af4f485470424ade3504504390acf6`;
reviewed product code `87670f61d` is an ancestor. The source archive SHA-256 is
`a1c9479115b7081d0a79f2d9705fbd96d6014d18f21b1aa078e2cfa40075b333`; the
lockfile hash is `36ed30648b87f581f4abb608ff1774dbcd5a79e7b3c94524d2bcf55749eb5709`.

The rebooted host is L4T R36.5.2, kernel `5.15.199-tegra`, driver 540.5.0,
CUDA 12.6.68, GCC 11.4.0, and Rust/Cargo 1.95.0. The candidate used Node
`v25.9.0` archive SHA-256
`8fb4283301b8c720fc9f18bffff0f659e72cc14d0cf207a3bb411808aaa73a57` and
npm 11.12.1. Before the runs it had 58 GiB available RAM, zero visible NVIDIA
compute processes, and unset `CUDA_VISIBLE_DEVICES`.

The CPU artifact and fresh installed consumer repeat PASS: the 17/44 runtime
surface, no test hooks, subscriber delivery, close/reopen, typed refusal,
typecheck, and 384-dimensional CPU embedding all pass. The CPU `.node` hash
is `7601325d16e6639e44a179c0913f870bddba427750cf392dd42004cce3457a8f` and
the new CPU platform tarball hash is
`3baf963379ecc4dd5500250d0c98a2c9e74a9f152f023c4525f555bffd2ed3f5`.

The matching CUDA/reranker artifact also builds and packages successfully.
Its `.node` hash is
`f2f389ea49b9668ef6bcef29087dfd9ae20ba1f5b8f4ef1c8522884740ab02e6`,
and the installed binary has that hash. The new CUDA platform tarball hash is
`1b66f74c1fc69ed95d6811ab6bf5074ac7c5527fd53b14327762850fa4c8844f`.
It retains the same direct `NEEDED` set and no `RPATH` or `RUNPATH`.

The first CUDA build attempt failed before artifact creation because the host
update left the CUDA toolkit `bin` directory off `PATH`. CUDARC reported:

```text
`nvcc --version` failed.
Err(Os { code: 2, kind: NotFound, message: "No such file or directory" })
```

The identical build passed with `PATH=/usr/local/cuda-12.6/bin:$PATH`; no
product source changed. CUDARC invokes `nvcc` by name even when `CUDACXX` and
`CUDA_PATH` are set. This records this host setup fact and does not establish
a defect in a repository wrapper.

Forced-CUDA Node 25 runtime remains **OPEN** after the reboot. Five clean
external-consumer processes, each with forced CUDA embedding and reranking,
produced 1 PASS and 4 typed `CudaProbeFailed` refusals. The successful process
produced a 384-dimensional CUDA embedding, non-null rerank scores, and an
in-process allocation witness: 111,144,960-byte delta above its
67,108,864-byte floor, plus a 1,076,256,768-byte control delta. The repeated
GPU evidence hash is
`bc611ba059a079a0fca6815032098ebfdbac9d10c48810705da2504cc50bb035`.

The failures occurred with about 60.2 GB `MemAvailable`, no visible compute
processes, and Orin GPU memory still unsupported (`N/A`) through `nvidia-smi`.
The host cannot demonstrate the witness's sole-consumer precondition; four
fresh-process refusals prevent qualification.

A read-only localization check passed 10 / 10 direct loads of the installed
platform module, then passed only 3 / 10 forced-CUDA
`Engine.open(useDefaultEmbedder: true)` calls with no embed, rerank, or
witness work. Its evidence hash is
`ae76960e78240ecb232ac03a6395d8baae9992fca0bc795be4063022fd3573ef`.
The instability is after native-module loading in the CUDA open/probe path.
The earlier candidate-bound diagnostic exposed the underlying CUDARC
`DriverError(CUDA_ERROR_OUT_OF_MEMORY, "out of memory")`; rebooted runs retain
the same public refusal without changing source to re-instrument it.

A V8 heap comparison did not identify a remedy: 10 forced opens each yielded
default 2 / 10 pass, `--max-old-space-size=256` 3 / 10, and
`--max-old-space-size=512` 2 / 10. The small difference is not reliable
evidence that V8 heap reservation causes the CUDA probe failure.

A final temporary instrumentation distinguished the probe stages while keeping
`FATHOMDB_RERANK_DEVICE=cpu`: `Device::new_cuda(0)` passed, then the first
`Tensor::zeros(1, DType::F32, &device)` failed with the exact driver error
`DriverError(CUDA_ERROR_OUT_OF_MEMORY, "out of memory")`. The preceding clean
process passed; the failing process had 58.1 GB `MemAvailable`, and concurrent
`tegrastats` showed RAM `5504/62828MB`, `36x4MB` largest free blocks, swap
`0/31414MB`, and GPU frequency 0%. This identifies the minimal CUDA tensor
allocation, rather than driver discovery, device creation, NAPI module load,
or reranker probing, as the failure stage. It remains a platform-runtime
finding: the evidence does not justify a product fix.

The temporary source was restored to SHA-256
`c1089fa0c9e79ef95909c0ed9597443f75144f6d324f7de2ec018f10e3f0d0b1`.
Stage diagnostic and V8 evidence hashes are
`64d6de2811f1c388f779dfbf5d620497177692e838553a67f3fea8d351b96526` and
`723fdf6bd23cd3c4461c8672e97b639fbd8f035b50945cbe66039e67afba47e0`.

The CPU and CUDA package rows remain PASS. The forced Node CUDA runtime row
remains OPEN pending reliable clean-process evidence and an explanation or
repair of the first CUDA allocation failure.

## Idle-host memory attribution and allocator check

After Memex CI run 37235196745 completed, its Jetson jobs had passed and no
Memex worker or pytest process remained on the host. A fresh external Node 25
consumer installed the same reviewed `c2f39b29` source with npm 11.12.1,
Rust 1.95.0 and CUDA 12.6.68. The main package SHA-256 was
`4c71a9a951776ffba54cc48a92ee5820ecc0772f2a5c53978b5758d9694e3227`,
the CUDA platform package was
`9d744536f406706880e57b93f379a50401575ac03c86d9c8e5efe63839085c40`,
and its restored installed `.node` was
`929f7dbfa950c1ba8fbb3760b365dfa9b33e6887696e9ef9d984f9542df8ff80`.
The five-run installed-package log SHA-256 is
`4986dfce91a03af0afa8bfa341237550dba3ed561c8df06e7f31e6fbf59d46c8`.

Three of five clean forced-CUDA open, embed, rerank and allocation-witness runs
passed; two refused during open with typed `CudaProbeFailed`. At the failing
attempt, a context-retaining CUDA Driver API probe measured free memory
`53,818,724,352` then `53,831,561,216` bytes, out of
`65,879,896,064` total. `MemAvailable` and `MemFree` were steady at about
52.9 GB and 47.0 GB, and `CmaFree` stayed `256,060` KiB across all five
starts. `nvidia-smi` named no compute process; accessible GPU-device file
descriptors named none. The graphical desktop, Airlock and Codex processes
were present, but available telemetry attributes no CUDA allocation to them.
The valid free-memory probe log SHA-256 is
`6e6afd09ae49e1e195f741a6b365a45d2c94099f8b99f44ae633013f6dc0bddb`.

A temporary same-process probe localized the misleading out-of-memory result:
in five failing Node opens, `Device::new_cuda(0)` succeeded, then the first
`Tensor::zeros(1, F32, ...)` returned `CUDA_ERROR_OUT_OF_MEMORY`. In each
failure, raw CUDARC `malloc_sync(4)` and `free_sync` succeeded on the same
thread and context, while `CudaStream::alloc_zeros::<u8>(4)` failed with the
same error. Its log SHA-256 is
`7da5444bda5c65c062c876a94e2bd0664dec57390b5502b42edb982a3ef8bbbd`.
This rules out aggregate GPU capacity and an ordinary four-byte synchronous
allocation as the cause. It narrows the fault to the stream allocation/zero
path; `alloc_zeros` combines asynchronous allocation and memset, so the
evidence does not yet choose between those stages or identify an external
memory owner. The temporary source was restored byte-for-byte to SHA-256
`c1089fa0c9e79ef95909c0ed9597443f75144f6d324f7de2ec018f10e3f0d0b1`;
the packaged consumer binary hash above was restored after diagnostics.
The [five-run log](idle-host-evidence/controlled-gpu-rerun.txt),
[valid memory probe](idle-host-evidence/memory-before-after-failure-v2.txt),
[same-process allocator log](idle-host-evidence/allocator-probe-runs.txt),
[package hashes](idle-host-evidence/installed-artifacts.sha256), and
[probe source](idle-host-evidence/cuda-mem-info.c) are retained with this
receipt.

No product code changed during these diagnostics. This receipt records the
independent Tegra investigation; the isolated remote build and consumer
directories can be removed after the retained evidence is verified.

## Allocator root cause and fix

**Branch:** `llm/slice110-tegra-allocator-fix`. Product-code fix commit:
`954ddd760` (vendored cudarc); the failing tests precede it at `78a7fc01b`
and `1fed05e5b`. Same host, driver, toolkit and toolchain as above (Node
25.9.0, 24.15.0 and 26.10.0 under nvm; Rust 1.95.0; NVCC 12.6.68).

### Root cause

The driver reports `CU_DEVICE_ATTRIBUTE_MEMORY_POOLS_SUPPORTED`, but the
device's default memory pool needs one contiguous 20960 MiB range of the
process's virtual address space inside [8 GiB, 128 GiB). Node/V8 scatters heap
pages through that window. When no such range is left,
`cuDeviceGetDefaultMemPool` and every `cuMemAllocAsync` return
`CUDA_ERROR_OUT_OF_MEMORY` with about 53 GB free, while synchronous
`cuMemAlloc` works. A plain C program reproduces it 20 / 20 by mapping three
4 KiB `PROT_NONE` pages at 38, 68 and 98 GiB before `cuInit`, and passes 20 / 20
without them. A placement rule (pool available iff the largest driver
reservation is at least 30.6 GiB or the largest hole in the window is at least
20960 MiB) matched 2149 / 2149 C runs and 63 / 63 earlier Node runs. See
[driver-isolation-evidence/results.md](driver-isolation-evidence/results.md),
[minimal_repro.c](driver-isolation-evidence/minimal_repro.c) and the unposted
[NVIDIA report draft](driver-isolation-evidence/nvidia-report-draft.md).

cudarc 0.19.7 (pulled in by the Candle fork) decides stream-ordered versus
synchronous allocation once per context from the pool attribute alone, with no
fallback, and the field is crate-private. Candle's first `Tensor::zeros` in
the CUDA probe therefore failed whenever the layout was fragmented, and the
product correctly refused with typed `FDB_EMBED_DEVICE_POLICY` /
`cuda_probe_failed`.

### Fix

`third_party/cudarc-0.19.7` is the published crate (Cargo.lock checksum
`1cea5f10a99e025c1b44ae2354c2d8326b25ddbd0baf76bde8e55cfd4018a2cc`), routed
by a root `[patch.crates-io]` entry and governed by the pin-rot gate (exact
path, local lock entry, licenses, patch note and tree digest). Its only change,
in `src/driver/safe/core.rs`:

- a context uses stream-ordered allocation only if pools are supported **and**
  `cuDeviceGetDefaultMemPool` succeeds; otherwise `cuMemAlloc`. The decision
  is made once per context in all four constructors, with no environment
  switch, and every alloc and free branch reads it;
- zero-byte synchronous requests return a null pointer (`cuMemAlloc(0)` is
  `CUDA_ERROR_INVALID_VALUE`, `cuMemAllocAsync(0)` returns null), and a null
  pointer is never passed to `cuMemFree`.

`FATHOMDB-PATCH.md` and `fathomdb-alloc-fallback.patch` (applies with
`patch -p1` to the published crate and reproduces the tree) sit in the vendor
directory for upstreaming. cudarc 0.17.8 (`ug-cuda`) still resolves from
crates.io. The forced-CUDA refusal contract is unchanged; its fake-provider
tests run in the full gate.

### Red and green

| Check | Before the fix | After the fix |
| --- | --- | --- |
| `fathomdb-embedder` test `tegra_fragmented_va_cuda` (fragmented layout, real Candle probe, tensor round trip, zero-element tensors) | 6 / 6 runs fail: `cuda_probe_failed`; direct probe `DriverError(CUDA_ERROR_OUT_OF_MEMORY, "out of memory")` | 11 / 11 pass; default pool `CUDA_ERROR_OUT_OF_MEMORY`, context on the synchronous path |
| Vendored unit tests (`scripts/tests/test_vendored_cudarc.sh`) | zero-length sync allocation fails with `CUDA_ERROR_INVALID_VALUE`; selection tests do not compile | 5 / 5 pass |
| Upstream cudarc driver tests, same features, raw-context tests excluded | 54 pass, 2 fail | 60 pass (54 + 5 new + 1 timing test), 1 fail |

The two upstream failures on the published crate are `test_pinned_copy_is_faster`
(a timing assertion that passed on the patched run) and
`test_unified_memory_host` (fails on both). The excluded `from_raw_context` and
non-primary-context tests crash with `SIGSEGV` on the published crate as well.

### Full-path Node verification

Fresh processes, one at a time, under the shared GPU lock, with
`FATHOMDB_EMBED_DEVICE=cuda:0`, `FATHOMDB_RERANK_DEVICE=cuda:0` and
`FATHOMDB_GPU_ALLOCATION_WITNESS=1`. Each run opens with the default embedder,
checks the open report and witness, embeds 11 times, reranks twice and
closes. The installed form packed the thin main and the
`fathomdb-linux-arm64-gnu` platform package and installed both offline, with
lifecycle scripts disabled, into a fresh consumer directory outside the source
tree; its resolved binary matched the built hash.

| Form | Node 25.9.0 | Node 24.15.0 | Node 26.10.0 | Forced CPU (Node 25) |
| --- | --- | --- | --- | --- |
| In-tree `dist/` | 30 / 30 | 10 / 10 | 10 / 10 | 3 / 3 |
| Installed package | 30 / 30 | 10 / 10 | 10 / 10 | 3 / 3 |

The product does not report which allocator a context chose, so the path is
inferred from steady embed latency, which is bimodal: 72 of the 100 CUDA runs
were synchronous (24.5–32.7 ms steady median) and 28 stream-ordered
(8.8–17.3 ms). Medians: synchronous 25.7 ms, stream-ordered 10.9 ms in-tree
and 12.4 ms installed, so the fallback costs about 2.1–2.4 times per steady
embed. First embed was 59–62 ms against 47 ms; open time was unchanged at about
1.4 s. Every witness delta cleared the 64 MiB floor (105–164 MiB). All 100
CUDA embeddings had identical leading values, as did all six CPU ones.
Runner, consumer, analysis and hashes are in
[fix-verification](fix-verification/); the committed `run-series.sh` is the
shellcheck-clean form of the runner used, with the same behaviour.

| Artifact | SHA-256 |
| --- | --- |
| CUDA/rerank `.node` (built and installed) | `dd625fd7de9e3d352fad3bc9718f2af26e29872164abe817ce76771d05aac5ce` |
| CUDA/rerank platform tarball | `b4f41e454255bd767ad515e7b50df8c28f748a83e012c4b645edecdab2cca794` |
| Main tarball | `5c7ab8b1779041ee393a9d1cee1e882a731220f778f7c164caee9b354b2a133b` |

### Blast radius

The vendored cudarc is compiled only where Candle's CUDA backend is: the Node
`embed-cuda`/`rerank-cuda` artifact, the Python CUDA wheels (x86_64 manylinux
and the host-native Tegra wheel), the CLI with `embed-cuda`/`rerank-cuda`, and
the TC-5 CUDA benchmark. CPU, Metal and no-feature builds do not compile it.
On devices whose default pool is available, including discrete GPUs, the
selection keeps upstream's stream-ordered path; the only addition is one
`cuDeviceGetDefaultMemPool` call per context. Root patches do not propagate:
downstream Rust builds of the published crates with `embed-cuda` still resolve
the unpatched crates.io cudarc.

### Tegra Python wheel

`scripts/release/build-python-cuda-tegra.sh` built
`fathomdb-0.8.26+tegra-cp310-abi3-linux_aarch64.whl` (sha256
`d7d1b651a1fb6120b7d0e6d85fa73f1bd8418f2336ec8beaa67143ef2efcdc73`) with
maturin 1.14.1; `cargo tree` for that build resolved the vendored cudarc. The
wheel was installed into a scratch venv (never `pip install -e` from the
worktree) and driven by `fix-verification/python_smoke.py`, which forces
`cuda:0` with the allocation witness on, opens, writes and searches. With
`fragment` it maps the same three PROT_NONE blockers before importing
fathomdb. On a quiet host, all 10 fresh processes passed (5 normal, 5
fragmented), each reporting the `cuda` effective device, with witness deltas
of 105–140 MiB
([python-smoke-quiet.txt](fix-verification/python-smoke-quiet.txt)).

An earlier series of 10 ran while a heavy cargo test was using the same
unified memory, and 8 passed. Both failures were the witness's
`insufficient_delta` check (one fragmented, one normal), not device
resolution. The witness reads a system-wide `cuMemGetInfo` counter, so it
needs the process to be the only GPU consumer.

### Open limitations

- In very heap-heavy Node processes `cuInit` itself can fail with
  `CUDA_ERROR_OUT_OF_MEMORY` before any context exists, apparently when no
  4 GiB hole is left in the window. In the explicit-pool experiment's
  400k-object variants 25 of 68 processes created no CUDA context
  ([analysis](explicit-pool-experiment/analysis.txt)). The allocator fallback
  does not address it; forced CUDA still refuses with the typed error there.
- The synchronous path is about 2.1–2.4 times slower per steady embed.
- The Slice 110 runtime row is fixed for the allocator failure but remains
  **IN_PROGRESS** pending independent review.

### Evidence retained and omitted

The four experiment directories are committed after pruning, following the
idle-host curation: reproducer sources, analysis scripts, patches, summaries
and the NVIDIA report draft. Local paths in scripts were replaced by
`<worktree>` and `<scratch>`. The earlier series drivers are archived as
`*.sh.txt` because they are run records with redacted paths, not maintained
scripts. Not committed (left on the Jetson host): `maps-all.tar.xz`, all raw
per-run logs (`logs*/`, `c-logs/`, `cuinit-threshold/`, `replay/`), the
`strace` output, and the build logs.
