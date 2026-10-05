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
series of Slice 110, the range is 1.8–2.4 times. That is the figure used in
the receipt and the docs.

### Not compiled elsewhere

`nm` on the built CUDA addon shows the constructor
`initialize_cuda_driver_at_load` and the embedder's `initialize_cuda_driver` /
`last_cuda_driver_init`, and the binary contains the hint text. A release
`default-embedder,default-reranker` build of the same crate on this host has
neither the symbols nor the text. On other targets the
`target_arch = "aarch64"` gate removes them by construction.
