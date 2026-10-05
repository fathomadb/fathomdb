# Slice 110 early `cuInit`: verification

Host: Jetson AGX Orin 64 GB, L4T R36.5.2, driver 540.5.0, CUDA 12.6. Node
25.9.0, 24.15.0 and 26.10.0 under nvm. All runs are fresh processes, one at a
time, with each series holding the shared GPU lock. Forced-CUDA series set
`FATHOMDB_EMBED_DEVICE=cuda:0`, `FATHOMDB_RERANK_DEVICE=cuda:0` and
`FATHOMDB_GPU_ALLOCATION_WITNESS=1`. A full run opens with the default
embedder, checks the open report and the witness, embeds 11 times, reranks
twice and closes.

Files:

- [consumer.mjs](consumer.mjs), [run-series.sh](run-series.sh) and
  [scenarios.sh](scenarios.sh) are the consumer and runners used. Paths come
  from arguments.
- [stage-installed.sh](stage-installed.sh) builds the installed form: the
  thin main and the platform package are packed and installed offline, with
  lifecycle scripts disabled, into a fresh directory outside the source tree.
- [analyze.py](analyze.py) produced [analysis.txt](analysis.txt).
- [red-green.txt](red-green.txt) records the red and green test output.
- [artifacts.sha256](artifacts.sha256) lists the artifact hashes.
- [sample-logs/](sample-logs/) holds one log per kind of series. The other
  per-run logs are omitted.
- [experiment/](experiment/) holds the earlier experiment's analyses and its
  hook patch, with local paths redacted. That experiment established the
  heap-size thresholds and the cost of the heavier variants.
- Fix round 3 (§ "Fix round 3" below) adds
  [worker-consumer.mjs](worker-consumer.mjs),
  [analysis-round3.txt](analysis-round3.txt),
  [sample-logs-round3/](sample-logs-round3/) and
  [python-smoke-round3.txt](python-smoke-round3.txt), and appends to
  [red-green.txt](red-green.txt) and [artifacts.sha256](artifacts.sha256).

## Results ([analysis.txt](analysis.txt))

| Scenario | Node | Form | Runs | Result |
| --- | --- | --- | --- | --- |
| Import first, then 400k objects (69–74 MiB heap) | 25.9.0 | in-tree | 15 | 15 pass |
| Import first, then 1M objects (157–181 MiB heap) | 25.9.0 | in-tree | 15 | 15 pass |
| Import first, then 400k / 1M objects | 24.15.0 | in-tree | 5 + 5 | 10 pass |
| Import first, then 400k / 1M objects | 26.10.0 | in-tree | 5 + 5 | 10 pass |
| Late import: 1M objects, then import | 25.9.0 | installed | 5 | 5 refused at open, with the new message |
| Late import under `node --import fathomdb` | 25.9.0 | installed | 5 | 5 pass |
| Late import under `NODE_OPTIONS=--import=fathomdb` | 25.9.0 | installed | 3 | 3 pass |
| Small consumer, no heap growth | 25.9.0 | in-tree | 10 | 10 pass |
| Small consumer, no heap growth | 25.9.0 | installed | 10 | 10 pass |
| Forced CPU (`cpu` for both) | 25.9.0 | in-tree | 3 | 3 pass on CPU |
| `CUDA_VISIBLE_DEVICES=` with `auto` | 25.9.0 | in-tree | 3 | 3 pass on CPU (`no_visible_cuda_device`) |

Every run wrote nothing to stderr. All 78 passing CUDA runs and all 6 CPU runs
had the same leading embedding values. Witness deltas were 106–144 MiB.

The 5 late-import refusals were `FDB_EMBED_DEVICE_POLICY`, kind
`cuda_probe_failed`, at a heap of 174–176 MiB, each with this message:

```text
cuda:0 requested but unavailable: CudaProbeFailed; cuInit returned
CUDA_ERROR_OUT_OF_MEMORY: the CUDA driver could not reserve its range of the
process address space, which is usually fragmented by a large JavaScript heap;
import fathomdb before building large in-memory data (at the top of the entry
module), or start Node with --import fathomdb
```

The message is one line; it is wrapped here. Their import added only about
0.84 GiB of virtual size: `cuInit` had failed, so no reservation existed. With
`--import`, the same script found fathomdb already loaded (its own import took
0.9 ms) and passed.

### Import cost

`CONSUMER_MODE=import`, installed form, Node 25.9.0, 10 runs each:

| Build and environment | Import median (min–max) | VmSize delta | VmRSS delta (median) |
| --- | --- | --- | --- |
| Early `cuInit` (default `auto`) | 32.4 ms (30.7–141.8) | 63,787 MiB | 11 MiB |
| Same build, both policies `cpu` (hook skipped) | 20.9 ms (18.7–27.1) | 821 MiB | 10 MiB |
| Review fix 2 build, no hook | 21.0 ms (19.9–26.6) | 821 MiB | 8 MiB |

The hook therefore adds about 11 ms per import, 1–3 MiB of RSS, and about
61.5 GiB of virtual size, which is reserved address space rather than memory.
The first run of every series that called `cuInit` imported in 134–144 ms,
the no-device series included. The two series without a `cuInit` call
started at 21–22 ms. The first `cuInit` after the GPU has been idle, even for
the few seconds between series, therefore costs about 110 ms more. The
medians above exclude no runs.

### Synchronous and stream-ordered paths

As in earlier rounds, the path is inferred from the bimodal steady embed (over
18 ms is synchronous). Across the 78 passing CUDA runs, 51 were synchronous
and 27 stream-ordered. The ratio of medians (synchronous over stream-ordered)
was 1.79–2.22 in the 7 series with both paths. Together with the 10 earlier
series of Slice 110, the range was 1.8–2.4 times. Fix round 3 (below)
widened it to 1.8–2.8, the figure now used in the receipt and the docs.

### Not compiled elsewhere

`nm` on the built CUDA addon shows the constructor
`initialize_cuda_driver_at_load` and the embedder's `initialize_cuda_driver` /
`last_cuda_driver_init`, and the binary contains the hint text. A release
`default-embedder,default-reranker` build of the same crate on this host has
neither the symbols nor the text. On other targets the
`target_arch = "aarch64"` gate removes them by construction.

## Fix round 3: `cuInit` at module registration

The review of the early-cuInit round found that a shared-library constructor
runs inside the dynamic loader under its lock. The call now runs from the
napi-rs 2.16.17 `module_exports` callback. napi-rs registers that callback
from its own constructor, which only stores a function pointer, and calls it
inside `napi_register_module_v1` after the exports are registered: on the
loading thread, once `dlopen` has returned, once per env. A
`std::sync::Once` keeps the `cuInit` to one per process.
`FATHOMDB_CUDA_EARLY_INIT=off` turns it off. Each probe now records its own
`cuInit` outcome, and a refusal's hint reads its own probe's entry, so the
reranker's memoized refusal keeps its cause.

The addon `3ecc99e6…` was built from `377ab9569` with the same recipe. `nm`
shows `cuda_early_init::EARLY_INIT` and napi-rs's
`__napi__explicit_module_register` registration. No constructor of ours calls
`cuInit`. The new unit test `loading_the_addon_library_does_not_initialise_cuda`
failed against the constructor build (`Some(Initialized)`, want `None`) and
passes now ([red-green.txt](red-green.txt)).

Results ([analysis-round3.txt](analysis-round3.txt)). No run wrote to
stderr, and every CUDA run had the same leading embedding values.

| Scenario | Node | Form | Runs | Result |
| --- | --- | --- | --- | --- |
| Import first, then 400k objects (68–79 MiB heap) | 25.9.0 | in-tree | 15 | 15 pass |
| Import first, then 1M objects (157–182 MiB heap) | 25.9.0 | in-tree | 15 | 15 pass |
| Import first, then 400k / 1M objects | 24.15.0 | in-tree | 5 + 5 | 10 pass |
| Import first, then 400k / 1M objects | 26.10.0 | in-tree | 5 + 5 | 10 pass |
| First load inside a `worker_threads` worker, then 1M objects in the worker | 25.9.0 | in-tree | 5 | 5 pass; worker import +64125–65141 MiB VmSize |
| Late import, 1M objects | 25.9.0 | installed | 5 | 0 pass: all `FDB_EMBED_DEVICE_POLICY` / `cuda_probe_failed` with the hint, heap 174–176 MiB |
| Late import under `node --import fathomdb` | 25.9.0 | installed | 5 | 5 pass |
| Small consumer | 25.9.0 | in-tree / installed | 10 + 10 | 20 pass |
| `FATHOMDB_CUDA_EARLY_INIT=off`, import only | 25.9.0 | installed | 10 | 10 pass; no reservation (+821 MiB VmSize) |
| `FATHOMDB_CUDA_EARLY_INIT=off`, forced CUDA, small | 25.9.0 | installed | 3 | 3 pass |

Import cost, median of 10 installed imports on Node 25: 33.2 ms with the
hook, 21.1 ms with it skipped (both policies `cpu`), 20.9 ms for the review
fix 2 build without a hook, 20.3 ms opted out. That is about +12 ms, +3 MiB
of RSS and +61.5 GiB of VmSize. The slowest import of every series whose
measured import called `cuInit` took 131–144 ms (the first `cuInit` after
the GPU had been idle); the series without a `cuInit` call stayed under
23 ms.

Paths: of the 83 passing CUDA runs, 65 were synchronous and 18
stream-ordered. The ratio of medians was 1.91–2.82 in the 9 series with both
paths; five of those series had a single stream-ordered run. Across all 26
such series of Slice 110 the range is 1.79–2.82 (synchronous medians
24.7–28.2 ms, stream-ordered 9.2–14.6 ms), so the docs now say about
1.8–2.8 times.

Other checks on this round's tree: `scripts/tests/test_tegra_node_early_cuinit.sh`
passed 3 / 3 in-tree and installed and failed 3 / 3 on the no-hook addon;
the vendored cudarc tests passed 14 / 14; `device_policy` 7 / 7 and
`slice71_reranker_policy` 8 / 8 with default and CUDA features;
`tegra_fragmented_va_cuda` 10 / 10. The Tegra Python wheel was rebuilt
(`d861de34…`) and passed 10 / 10, 5 normal and 5 fragmented
([python-smoke-round3.txt](python-smoke-round3.txt)).

The constructor named under "Not compiled elsewhere" above no longer exists.
The embedder's `cuda_driver_init` items and the probes' record calls compile
into every aarch64 Linux CUDA build (Node addon, Tegra Python wheel, CLI);
only the napi hook and the hint are Node-only.
