# Performance Gauntlet v1.2 — FathomDB 0.8.26 results

**0.8.26 run:** `gauntlet-0.8.26-20260921-c`  
**Source:** FathomDB 0.8.26, commit
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`  
**0.8.25 retrieval comparison:**
`gauntlet-0.8.25-search01-locomo-20260921`, released tag `v0.8.25`, commit
`e2934867436eb9859f31574ad05abf632314906a`  
**Disposition:** directional results; all 10 0.8.26 cells and both selected
0.8.25 retrieval cells passed

## GPU execution boundary

All corpus embedding cells were run with the CUDA-enabled binaries, device
`cuda:0`, and pinned RTX 3090 UUID
`GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`. The earlier AC-073 CPU attempt is
invalid and excluded from these results. SEARCH-01 and LOCOMO A0 are
historically FTS-only workloads and perform no neural corpus encoding. The
CPU arm in the cross-encoder profile is an intentional comparison arm.
GRAPH-RETRIEVAL-01's one-time seed qualification also used forced CUDA on that
RTX 3090. It matched all 300 historical top-10 rankings before freezing the
top-20 seeds. Its later native graph-expansion stage performs no encoding and
therefore correctly ran without GPU work.

## Retrieval quality and fidelity

This section contains retrieval scores only. It excludes latency, throughput,
ingest time, and stress timing.

| Workload | Retrieval metric | FathomDB 0.8.25 | FathomDB 0.8.26 | Change |
| --- | --- | ---: | ---: | ---: |
| AC-073 | Vector-stage fidelity recall@10 | 0.772 (CI 0.743–0.798) | 0.772 (CI 0.743–0.798) | 0.000 |
| AC-075 | TC-5 fidelity recall@10 | 0.958 (CI 0.938–0.974) | 0.954 (CI 0.936–0.971) | −0.004 |
| SCALE-02 | Retrieval-quality score | Not measured | Not measured | n/a |
| SEARCH-01 | Strict evidence recall@5 | 0.647809 | 0.647809 | 0.000000 |
| SEARCH-01 | Strict evidence recall@10 | 0.697451 | 0.697451 | 0.000000 |
| SEARCH-01 | Negative-query abstention | 0.008000 | 0.008000 | 0.000000 |
| LOCOMO A0 | Recall@1 | 0.343750 | 0.343750 | 0.000000 |
| LOCOMO A0 | Recall@5 | 0.593099 | 0.593099 | 0.000000 |
| LOCOMO A0 | Recall@10 | 0.667318 | 0.667318 | 0.000000 |
| LOCOMO A0 | MRR | 0.446271 | 0.446271 | 0.000000 |
| LOCOMO A0 | nDCG@10 | 0.465329 | 0.465329 | 0.000000 |
| LOCOMO A0 | Temporal evidence recall | 0.741433 | 0.741433 | 0.000000 |

The release-specific comparison is therefore narrow but clear:

- AC-073 was unchanged.
- AC-075 moved by −0.004, or −0.4 percentage points. Its 95% intervals overlap
  substantially.
- SCALE-02 is an efficiency and scale workload. It does not compute relevance,
  recall, MRR, or nDCG, so it has no retrieval-quality value to compare.
- SEARCH-01 and LOCOMO A0 were unchanged at the displayed precision in the
  release-specific comparison. SEARCH-01 completed 10,506 document ingests and
  4,597 retrievals. LOCOMO scored 1,536 eligible questions after excluding four
  questions without evidence.

### Older retrieval references

These references are useful reproducibility checks, but they are not 0.8.25
release measurements.

| Workload | Retrieval metric | Earlier FathomDB run | FathomDB 0.8.26 | Change |
| --- | --- | ---: | ---: | ---: |
| SEARCH-01 | Strict evidence recall@5 | 0.6478 | 0.647809 | Same at reported precision |
| SEARCH-01 | Strict evidence recall@10 | 0.6975 | 0.697451 | Same at reported precision |
| SEARCH-01 | Negative-query abstention | 0.008000 | 0.008000 | 0.000000 |
| LOCOMO A0 | Recall@1 | 0.343750 | 0.343750 | 0.000000 |
| LOCOMO A0 | Recall@5 | 0.593099 | 0.593099 | 0.000000 |
| LOCOMO A0 | Recall@10 | 0.667318 | 0.667318 | 0.000000 |
| LOCOMO A0 | MRR | 0.446271 | 0.446271 | 0.000000 |
| LOCOMO A0 | nDCG@10 | 0.465329 | 0.465329 | 0.000000 |
| LOCOMO A0 | Temporal evidence recall | 0.741433 | 0.741433 | 0.000000 |

The earlier SEARCH-01 run used candidate `v0.8.22-81-gbf2d88a0`. The earlier
LOCOMO A0 run used commit `ac888267`. Both older references also agree with the
new release-specific 0.8.25 and 0.8.26 measurements at the displayed precision.

AC-073 and AC-075 are ANN-fidelity measurements against same-model exact-f32
ground truth. They measure approximation fidelity, not semantic relevance to
human-authored qrels. SEARCH-01 and LOCOMO A0 are relevance/evidence retrieval
measurements.

### GRAPH-RETRIEVAL-01 historical-300 result

This is a new 0.8.26 directional measurement, so no 0.8.25 result exists. The
control and both treatment arms used the same qualified MuSiQue top-20 seeds.

| Retrieval metric | Fused control | Native depth 1 | Native depth 2 |
| --- | ---: | ---: | ---: |
| Complete support@5 | 0.443333 | 0.443333 | 0.443333 |
| Complete support@10 | 0.610000 | 0.610000 | 0.610000 |
| Complete support@20 | 0.916667 | 0.916667 | 0.916667 |
| Support recall@5 | 0.658333 | 0.658333 | 0.658333 |
| Support recall@10 | 0.770000 | 0.770000 | 0.770000 |
| Support recall@20 | 0.916667 | 0.916667 | 0.916667 |
| nDCG@5 | 0.630227 | 0.630227 | 0.630227 |
| nDCG@10 | 0.675379 | 0.675379 | 0.675379 |
| nDCG@20 | 0.723144 | 0.723144 | 0.723144 |
| First-support MRR | 0.756240 | 0.756240 | 0.756240 |

Both graph treatments were complete no-ops: candidate-change and useful-
expansion rates were `0.000000`, while the no-op rate was `1.000000`. All
complete-support deltas and their 95% bootstrap intervals were exactly zero.
The graph path executed successfully but did not improve this retrieval
projection.

## Timing, throughput, and system performance

The measurements below are intentionally separate from retrieval quality.
They are directional system-performance observations.

### Side-by-side captured measurements

| Cell | FathomDB 0.8.25 | FathomDB 0.8.26 | Directional observation |
| --- | --- | --- | --- |
| AC-076 | p50 1 ms; p99 3 ms | p50 1 ms; p99 2 ms | p50 flat; p99 −1 ms |
| AC-072 | p50/p99: 70/79, 70/77, 70/77 ms | p50/p99: 69/75, 70/78, 70/77 ms | similar distributions |
| AC-081 | ratio median 2.855; mean 3.355 | ratio median 3.576; mean 3.400 | median +25.2%; mean +1.3% |
| AC-073 stress | p99 391 ms | p99 418 ms | +27 ms |
| CE profile | baseline | CUDA standalone p95 ratio 1.182 | informational 1.10 ratio exceeded |

AC-073's 0.8.26 retrieval-query latency was p50 36 ms and p99 47 ms. There is
no corresponding value in the retained 0.8.25 AC-073 receipt, so it is not
shown as a side-by-side comparison.

### FathomDB 0.8.26 SCALE-02 efficiency ladder

| Documents | Steady p50 ms | Steady p99 ms | Ingest documents/s |
| ---: | ---: | ---: | ---: |
| 10,000 | 3.629 | 15.792 | 6,953 |
| 17,272 | 5.916 | 26.314 | 6,723 |
| 25,000 | 8.033 | 36.314 | 5,962 |
| 40,000 | 9.959 | 24.519 | 5,773 |
| 50,000 | 12.180 | 27.856 | 5,732 |

Protected writes passed three repetitions for each fixture. Median total times
were 1,428 ms for SCALE-02 and 1,269 ms for AC-013.

### GRAPH-RETRIEVAL-01 native-expansion latency

These measurements cover graph expansion and evidence resolution after seed
qualification; they do not include encoding.

| Treatment | p50 ms | p95 ms | p99 ms |
| --- | ---: | ---: | ---: |
| Native depth 1 | 2.749 | 10.152 | 21.587 |
| Native depth 2 | 3.682 | 21.488 | 41.417 |

## Evidence locations

- Graph benchmark implementation smoke and GPU qualification:
  [`GRAPH-BENCHMARKS-SMOKE-0.8.26.md`](GRAPH-BENCHMARKS-SMOKE-0.8.26.md)
- Qualified GRAPH-RETRIEVAL-01 seed manifest:
  [`results/graph-retrieval-01-seed-manifest.v1.json`](results/graph-retrieval-01-seed-manifest.v1.json)
- GRAPH-RETRIEVAL-01 historical-300 receipt:
  [`results/graph-retrieval-01-historical-300-0.8.26.record.json`](results/graph-retrieval-01-historical-300-0.8.26.record.json)
- Campaign summary:
  `data/performance-benchmarking/gauntlet-v1.2/runs/gauntlet-0.8.26-20260921-c/gauntlet-summary.json`
- 0.8.25 SEARCH-01 and LOCOMO comparison summary:
  `data/performance-benchmarking/gauntlet-v1.2/runs/gauntlet-0.8.25-search01-locomo-20260921/gauntlet-summary.json`
- Human summary:
  `data/performance-benchmarking/gauntlet-v1.2/runs/gauntlet-0.8.26-20260921-c/gauntlet-summary.md`
- SCALE-02 raw artifacts:
  `~/.local/share/fathomdb-experiments/performance-gauntlet-v1.2/0.8.26/scale02-raw/gauntlet-0.8.26-20260921-c`
- LOCOMO raw artifacts:
  `~/.local/share/fathomdb-experiments/performance-gauntlet-v1.2/0.8.26/locomo-raw`

Both campaign summary states are `passed` and record hashes for every retained
artifact.
