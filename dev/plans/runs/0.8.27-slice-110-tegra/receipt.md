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
| Node NAPI forced-CUDA runtime | OPEN | Fresh processes intermittently fail the minimal CUDA probe with `CUDA_ERROR_OUT_OF_MEMORY`; a single successful open is not a qualification. |

The open row is a runtime-capacity/availability finding, not a claim that the
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

No product or release-state file changed. This receipt records the independent
Tegra investigation only; the isolated remote build and consumer directories
are agent-owned temporary material.
