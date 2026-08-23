# SCALE-02 rank-boundary result

## Decision

Recommend streamed BM25 boundary-tie completion with shipped reader defaults.
It passes the scale-adjusted 25/40/50 ms p50 policy through 50k, preserves exact
ordered retrieval, and avoids mmap128's roughly 0.5–1.0 GiB additional RSS.
Production landing remains a post-result HITL decision.

The original fixed-20-ms A0 envelope remains 17,272 records. This off-shoot is
a separate algorithm-selection result, not a retroactive rewrite of that
receipt or a capacity guarantee.

## Result

The approved
[`rank-boundary.v1.json`](../../experiments/configs/scale-02/rank-boundary.v1.json)
ran a balanced full factorial: shipped fallback versus streamed boundary-tie
completion, crossed with default versus mmap128 readers, at 25k, 40k, and 50k.
Each cell used five fresh databases; all 60 repetitions completed with zero
errors and zero timeouts.

| Boundary path | Reader | Records | Steady p50 | Steady p99 | Throughput | Peak RSS | Policy |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| shipped fallback | default | 25,000 | 24.01 ms | 110.71 ms | 31.62 qps | 381 MiB | control |
| shipped fallback | default | 40,000 | 49.80 ms | 260.58 ms | 13.92 qps | 399 MiB | control |
| shipped fallback | default | 50,000 | 108.32 ms | 327.45 ms | 8.44 qps | 424 MiB | control |
| streamed tie | default | 25,000 | 8.60 ms | 39.53 ms | 84.63 qps | 381 MiB | pass |
| streamed tie | default | 40,000 | 10.54 ms | 24.24 ms | 85.68 qps | 398 MiB | pass |
| streamed tie | default | 50,000 | 12.95 ms | 30.74 ms | 69.47 qps | 424 MiB | pass |
| shipped fallback | mmap128 | 25,000 | 9.18 ms | 36.21 ms | 85.13 qps | 1,359 MiB | control |
| shipped fallback | mmap128 | 40,000 | 25.28 ms | 132.45 ms | 26.15 qps | 1,393 MiB | control |
| shipped fallback | mmap128 | 50,000 | 67.09 ms | 190.84 ms | 14.37 qps | 1,416 MiB | control |
| streamed tie | mmap128 | 25,000 | 6.39 ms | 15.93 ms | 140.55 qps | 1,046 MiB | pass |
| streamed tie | mmap128 | 40,000 | 10.19 ms | 25.95 ms | 87.63 qps | 961 MiB | pass |
| streamed tie | mmap128 | 50,000 | 13.70 ms | 41.41 ms | 63.33 qps | 937 MiB | pass |

The 95% upper bounds for streamed/default are 9.96/43.08 ms p50/p99 at 25k,
10.60/24.24 ms at 40k, and 13.02/30.74 ms at 50k. These satisfy the registered
25/40/50 ms p50 budgets, 150 ms p99 budget, resource ceiling, and 25k
non-regression rule.

## Fidelity and mechanism

Preflight reproduced the shipped fallback counts: 20/100 queries at 25k,
41/100 at 40k, and 58/100 at 50k. The bundled SQLite 3.53.2 query-plan witness
reported no temporary ORDER BY b-tree for the streamed statement.

Across preflight and the six final equivalence cells:

- every ordered top-100 candidate signature and top-10 public-result signature
  matched the forced full-sort control;
- streamed cells recorded zero full-sort fallbacks;
- the largest observed completed boundary group was 33 rows and the largest
  scan consumed 123 rows;
- every observed connection used WAL, and every writer used
  `synchronous=NORMAL`.

The result supports the causal hypothesis: the growing latency came from the
fallback's full-match stable sort. Completing only the BM25 group tied across
the top-100 boundary removes that sort while retaining the cursor tie-break.
mmap128 materially helps the shipped fallback but adds no useful 40k/50k
benefit after the algorithmic fix.

## Evidence

- [safe receipt](../../experiments/runs/scale-02-rank-boundary-20260823T0147Z-c5ade6de/record.json)
- external artifact-manifest SHA-256:
  `bad0098141509c6b960e0cad95a149020543c8f0fe375ab3ce9b116b45ee594b`
- candidate source: `9e507553f954c56d5c6177eabf1750faddf3acfd`
- authorization: `seq-266`

External databases and content-bearing query comparisons remain outside the
repository under `data/performance-benchmarking/scale-02/rank-boundary-9e507553/`.
