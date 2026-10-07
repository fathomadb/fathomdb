---
title: Slice 135 installed Python S01 baseline functional probe
status: BASELINE_FUNCTIONAL_PROBE_TIMING_UNQUALIFIED
target_release: 0.8.27
---

# Installed Python S01 baseline functional probe

The source-independent [runner](runner.py) exercised the
[qualified installed 0.8.26 wheel](../2026-10-07-python-wheel-baseline/README.md)
from source `f99e002f0d2e4002f3694c9f8d4986b56089edaa`. It checked that the
imported Python package and native extension bytes match the wheel whose
SHA-256 is `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
The runner SHA-256 is in [SHA256SUMS](SHA256SUMS). The default CPU embedder
was available from the local cache. No paid model call was made.

For each corpus size, the runner wrote two fixed anchors and neutral filler
rows to a fresh real database, declared a searchable vector projection,
drained to `ready`, asserted the anchor rows via `read.get`, then reopened a
session for each query shape. The shapes were `search_text_only` with one
literal anchor, `search` with no lexical match and a vector-bearing result,
and `search` with a lexical anchor plus the vector-ready corpus. The latter
two are both the public hybrid method; the vector-labeled shape verifies a
vector result branch and the absence of a lexical match. These are workload
validity assertions, not relevance gold.

Both commands used the baseline installed virtual environment, the retained
wheel path, its exact hash, 100 warm samples, `FATHOMDB_EMBED_DEVICE=cpu`, and
the corresponding `--rows` value. Example for 32 rows:

```sh
env -u PYTHONPATH -u VIRTUAL_ENV FATHOMDB_EMBED_DEVICE=cpu \
  /tmp/slice135-python-venv-baseline-f99e002/bin/python \
  scripts/slice135_python_s01.py \
  --rows 32 --samples 100 \
  --wheel /tmp/slice135-python-wheel-baseline-f99e002/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --wheel-sha256 7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282 \
  --source-sha f99e002f0d2e4002f3694c9f8d4986b56089edaa \
  --output /tmp/slice135-python-s01-baseline-32-final.json
```

The [32-row](raw-32.json) and [256-row](raw-256.json) raw files each contain
one session-first call, one warmup, and 100 warm calls for each of three
shapes: 306 calls per corpus, 612 total. The
[independent check](independent-check.json) recomputed attempt counts, source
and wheel identities, positive nanosecond observations, known IDs, text
anchor, vector branch and hybrid anchor from the raw data. It found zero
semantic failures. Changing one retained text ID from `A` to `B` in memory
caused its semantic check to reject the mutation. A deliberately wrong wheel
hash was rejected before any workload output; see [wrong-wheel.log](wrong-wheel.log).
The [32-row](resource-32.txt) and [256-row](resource-256.txt) GNU Time child
resource reports are retained.

**Timing is unqualified.** This is one baseline block per corpus size, with
no full environment snapshots, repeated-block noise estimate, frozen S01
sample/uncertainty rule, or candidate pair. The `latency_ns` values in the raw
files are diagnostic only; no speed or tail comparison is made from them.
The next step is a repeatable block wrapper with environment/resource
invalidators, then baseline-only repeated blocks and protocol freeze before
candidate timing.
