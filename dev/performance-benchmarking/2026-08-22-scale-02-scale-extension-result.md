# SCALE-02 scale-adjusted extension result

## Decision

Use the scale-adjusted p50 policy `max(20 ms, records / 1000)` as a separate
advisory view; do not rewrite the original fixed-20-ms envelope.

- Keep shipped `rank_default` through 25k. It already passes the 25 ms budget,
  so extra reader memory is unnecessary there.
- `rank_mmap128` is the only tested profile eligible at 40k. It is an
  experiment-only result, not a production-default decision.
- No tested profile is eligible at 50k. Do not solve this with `cache64` or by
  relaxing the retained 150 ms p99 limit. The next 50k work, if authorized,
  should target the growing full-sort fallback share rather than another
  undirected cache-size sweep.

The original fixed-policy result remains a 17,272-record envelope. Under the
new scale-adjusted view, configured-as-is A0 reaches 25k; the tested mmap
treatment reaches 40k.

## Configured-as-is baselines

The 40k and 50k measurements used the exact approved
[`a0-envelope.v2.json`](../../experiments/configs/scale-02/a0-envelope.v2.json)
runtime, workload, inputs, seeds, and shipped reader defaults. `seq-266`
authorized measuring beyond the original stop. All points completed five fresh
repetitions with zero errors and zero timeouts.

| Records | Steady p50 | p50 budget | Steady p99 | Throughput | Peak RSS | Scale-adjusted result |
| ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 25,000 | 23.90 ms | 25 ms | 112.21 ms | 31.61 qps | 232 MiB | Pass |
| 40,000 | 48.83 ms | 40 ms | 260.15 ms | 14.02 qps | 276 MiB | Fail p50 and p99 |
| 50,000 | 109.49 ms | 50 ms | 326.36 ms | 8.49 qps | 310 MiB | Fail p50 and p99 |

Receipts: [25k](../../experiments/runs/scale-02-a0-25000-20260822T2245Z-ee93a826/record.json),
[40k](../../experiments/runs/scale-02-a0-40000-20260822T2311Z-be43f004/record.json),
and [50k](../../experiments/runs/scale-02-a0-50000-20260822T2324Z-f5c98234/record.json).

## Two-hypothesis off-shoot

The frozen
[`scale-hypotheses.v1.json`](../../experiments/configs/scale-02/scale-hypotheses.v1.json)
tested two accuracy-preserving reader profiles. Every cell used five fresh
databases and the formal concurrency-one workload. Each treatment-size cell
also compared ordered top-10 result signatures for all 100 TC-5 queries against
the full-sort control. All 600 comparisons matched; reader witnesses confirmed
the requested pragmas on every observed connection.

| Treatment | Records | Steady p50 | Steady p99 | Throughput | Peak RSS | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| 128 MiB mmap | 25,000 | 9.20 ms | 36.06 ms | 85.06 qps | 1,214 MiB | Pass |
| 128 MiB mmap | 40,000 | 25.30 ms | 132.08 ms | 26.25 qps | 1,272 MiB | Pass |
| 128 MiB mmap | 50,000 | 66.42 ms | 189.78 ms | 14.43 qps | 1,299 MiB | Fail p50 and p99 |
| 64 MiB cache | 25,000 | 14.46 ms | 82.28 ms | 53.19 qps | 1,137 MiB | Pass |
| 64 MiB cache | 40,000 | 36.39 ms | 196.02 ms | 19.27 qps | 1,455 MiB | Fail p99 |
| 64 MiB cache | 50,000 | 60.37 ms | 323.25 ms | 10.47 qps | 1,583 MiB | Fail p50 and p99 |

The safe aggregate [hypothesis receipt](../../experiments/runs/scale-02-scale-hypotheses-20260822T2328Z-55ce25d2/record.json)
binds the six cells. Its external artifact-manifest SHA-256 is
`96da584a4a49937f3baaccaed70d0207861584b957faf2e5b1e12c4ef76303ee`.

## Interpretation and boundary

Reader working-set capacity is a real part of the scaling problem: mmap128
cuts 40k median latency by 48% and p99 by 49% while preserving retrieval.
However, reader tuning alone does not sustain 50k. The rank-fast route served
80% of equivalence queries at 25k, 59% at 40k, and 42% at 50k; the remainder
used the exact full-sort fallback. This correlation makes fallback frequency
the strongest next hypothesis, though it does not by itself prove causality.

The mmap result costs about 1 GiB more process RSS than shipped defaults. The
cache64 treatment does not offer a better trade-off: it is slower, fails the
40k tail limit, and uses more memory than mmap at 40k and 50k. These are
single-host, derived-row efficiency measurements, not capacity guarantees or
real-corpus fidelity claims above 17,272 records.
