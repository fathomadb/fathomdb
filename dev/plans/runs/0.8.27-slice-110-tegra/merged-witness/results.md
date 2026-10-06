# Slice 110 merged-source allocation witness on the AGX Orin

Date: 2026-10-06. Host: Jetson AGX Orin 64 GB, L4T R36.5.2, CUDA 12.6.68,
driver 540.5.0, Node v25.9.0.

## Artifact

Source: merge commit `a25d063cd` (the later `release/0.8.27` commits to
`825f31cc5` change documentation only). CUDA N-API addon built with
`embed-cuda,rerank-cuda` in release mode, staged in the installed form with
`../fix-verification/stage-installed.sh` (offline install, lifecycle scripts
disabled, consumer outside the source tree).

| Item | SHA-256 |
| --- | --- |
| Native addon (built and installed copy identical) | `f66a394c699e1dc7a7894b6e66a995223eff2e2b728007ca0e1de3e1ee9da635` |
| Platform tarball `fathomdb-linux-arm64-gnu-0.8.26.tgz` | `87ba19783fd4a12f972ff3524aa1289acbf07879a2520303deea3d2e2a48f917` |
| Main tarball `fathomdb-0.8.26.tgz` | `5c7ab8b1779041ee393a9d1cee1e882a731220f778f7c164caee9b354b2a133b` |

The addon digest matches the merged-source Orin build recorded in the Slice 110
status.

## Series

Each run is a fresh Node process running `../fix-verification/consumer.mjs`
through `../fix-verification/run-series.sh` (open, embed, rerank, close). The
whole set ran under the shared GPU lock. Before the quiet series, no other
process held measurable CPU or GPU load, and MemAvailable was 51.0 GB.

| Series | Condition | Runs | Outcome | Witness `deltaBytes` (floor 67,108,864) |
| --- | --- | --- | --- | --- |
| quiet-cuda | `cuda:0`, witness on, sole GPU and host-memory consumer | 20 | 20 / 20 pass | 140,099,584 – 171,958,272; median ≈ 146.4 M |
| cpu | `cpu`, witness off | 5 | 5 / 5 pass | n/a |
| churn-cuda | `cuda:0`, witness on, `churn.py` concurrently touching and releasing 2 GiB of host memory every 0.8 s | 5 | 5 / 5 pass | 338,247,680 – 372,924,416 |

Quiet per-run deltas, in run order: 143622144, 146817024, 143622144,
143876096, 148004864, 150241280, 146436096, 146305024, 142565376, 140099584,
146178048, 159731712, 146812928, 171958272, 146309120, 152125440, 146698240,
142618624, 146956288, 149467136. The smallest quiet delta is 2.09 times the
floor.

## Interpretation

- On the quiet host the merged installed package passes the witness in
  every run, with a margin of more than twice the floor.
- The witness reads `cuMemGetInfo`, which on this integrated GPU is a
  system-wide counter (`SOLE_GPU_CONSUMER_PRECONDITION` in
  `fathomdb-embedder/src/gpu_witness.rs`). In the churn control, another
  process's host-memory activity moved the measured delta by about 200 MB.
  In these runs that activity happened to raise it. Memory released by
  another process inside the measurement window lowers it by the same
  mechanism, and a large enough release makes the delta negative.
- The earlier witness-enabled repeat recorded in the Slice 110 status had a
  negative delta in its second process, with CUDA execution otherwise passing.
  That matches this mechanism. Inferred, not reproduced: that run's concurrent
  host activity was not recorded. The GPU-heavy 0.8.28 pool-study series and
  builds ran on this host on 2026-10-05. A negative witness delta on this
  device shows that the sole-consumer precondition was violated. It does not
  show a CUDA allocation failure.
- Steady embed medians are bimodal, about 9–13 ms and about 25–31 ms. That
  matches the Slice 110 path mixture (default pool available or not at first
  allocation) and is not new.

`churn.py` is the interference control used above.
