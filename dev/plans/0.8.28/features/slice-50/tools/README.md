# Slice 50 drain qualification

`slice50_drain_scaling.rs` uses the frozen AC-013 Zipfian corpus and dense
FNV/xorshift embedder, format 34, default connection cache, 768 dimensions
and normal worker count. It times write and drain separately, asserts the
vector count, and hashes all vector fields except timestamps, vector-row
mappings, terminal states and the projection cursor. Synthetic embedding
runs on CPU; no BGE or GPU is used.

From the repository root (Python 3.12+):

```sh
python3 -m unittest discover -s dev/plans/0.8.28/features/slice-50/tools
python3 dev/plans/0.8.28/features/slice-50/tools/drain_scaling.py --scratch /tmp/slice50-scaling build --sha 630af0bfc --label base
python3 dev/plans/0.8.28/features/slice-50/tools/drain_scaling.py --scratch /tmp/slice50-scaling build --sha HEAD --label head
python3 dev/plans/0.8.28/features/slice-50/tools/drain_scaling.py --scratch /tmp/slice50-scaling run
python3 dev/plans/0.8.28/features/slice-50/tools/drain_scaling.py --scratch /tmp/slice50-scaling analyze
```

Build exports contain the selected source plus this checkout's probe. Builds
refresh exported source timestamps and share one serialized Cargo target to
limit disk consumption, copying each
finished binary before building the next label. Use a new scratch directory
for another experiment. Do not run other Cargo builds against that target
concurrently. The source commit and copied executable are recorded per label.

Runs interleave base/head at 10k, 30k and 100k, reverse order on the second
repeat, then run one reported-only overlap cell per label. A file lock
serializes each experiment; start load must be ≤2.0 (up to ten minutes wait),
and load is sampled during each run. A cell exceeding 2.5 is repeated once;
analysis uses the final attempt and reports any remaining flagged attempt.
Raw JSONL, binary metadata and the summary stay in scratch, outside the repo.

The predeclared rule is head 100k:10k median cost/vector ≤1.3 and head 10k
median ≤1.05 × base; all logical digests must match. The 100k paired speedup
is reported separately. A failed rule is reported, never loosened after a
run. The probe itself accepts `SLICE50_N`, `SLICE50_DIM`, `SLICE50_MODE`
(`held` or `overlap`) and `SLICE50_DRAIN_TIMEOUT_MS`; qualification uses the
fixed defaults except matrix size/mode. These are test knobs, not product
configuration.
