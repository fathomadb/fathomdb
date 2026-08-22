# SCALE-02 compact FTS follow-up result

## Decision

Use the production rank-fast FTS path with shipped SQLite reader defaults for
the formal SCALE-02 10k rerun. Do not add mmap or enlarged-cache settings.

The HITL approved `rank_default` plus production landing on 2026-08-22. The
decision is recorded as `seq-264` in
[`dev/steward/steward-ledger.jsonl`](../../dev/steward/steward-ledger.jsonl).
The approved executable contract is
[`a0-envelope.v2.json`](../../experiments/configs/scale-02/a0-envelope.v2.json).

## Results

The compact follow-up used five fresh 10k databases per performance cell. All
cells completed without errors or timeouts. The values below are 95% upper
bounds across repetitions; lower is better.

| Cell | Steady p50 | Steady p99 | Peak RSS | Decision |
| --- | ---: | ---: | ---: | --- |
| Existing full sort, defaults | 29.09 ms | 53.37 ms | 218 MiB | Reject: p50 exceeds 20 ms policy |
| Rank fast, defaults | 9.50 ms | 44.25 ms | 230 MiB | Select: lowest-footprint eligible cell |
| Rank fast, 128 MiB mmap | 2.99 ms | 11.27 ms | 760 MiB | Reject: extra memory is unnecessary |
| Rank fast, 64 MiB cache | 3.30 ms | 15.29 ms | 570 MiB | Reject: extra memory is unnecessary |
| Rank fast, 256 MiB mmap and 64 MiB cache | 3.02 ms | 11.26 ms | 1.1 GiB | Reject: highest footprint without a decision benefit |

The selected cell reduced measured steady p50 from 28.95 ms to 9.36 ms and
increased throughput from 33.71 to 97.63 queries per second, while peak RSS
rose by about 12 MiB. Retrieval remained exactly equivalent on all 100 TC-5
queries and all 32 ANSWER-01 queries. The rank-fast route served 100 of those
132 queries; the other 32 safely used the full-sort fallback.

Concurrency improved aggregate throughput from 97.63 queries per second at
one worker to 191.13 at two and 296.37 at four. Tail latency increased at four
workers, so the registered formal latency treatment remains concurrency one.

The aggregate evidence is in the
[`tuning receipt`](../../experiments/runs/scale-02-fts-tuning-20260822T1851Z-51e41245/record.json)
and [`selection receipt`](../../experiments/runs/scale-02-fts-selection-20260822T1859Z-946ebdc2/record.json).
The external artifact manifest has SHA-256
`f0b029f282163f46e36cbbc4dc40e8409dd4b93f093963e7a18ba17381b0ad16`.

## Trade-offs and boundary

- Reader tuning can reduce latency further, but costs roughly 2.6 to 4.9 times
  the selected cell's peak memory. It is not needed to pass the advisory
  policy and would make the default more host-sensitive.
- The production fast path applies only to eligible direct, unfiltered,
  node-only text retrieval. Edge-bearing databases and an ambiguous score
  boundary retain the full-sort path. The experiment-only force-full-sort
  control remains available for comparison; it is not a production setting.
- This decision establishes the treatment for the formal 10k rerun. It does
  not by itself establish a product capacity guarantee.

## Formal scale envelope

The approved treatment completed five fresh repetitions at each attempted
point. The 10k and 17,272 points passed every criterion. The 25k point failed
only the 20 ms steady-p50 criterion, so the registered stop rule prohibited
40k and 50k.

| Records | Corpus composition | Steady p50 | Steady p99 | Throughput | Peak RSS | Result |
| ---: | --- | ---: | ---: | ---: | ---: | --- |
| 10,000 | all real | 9.41 ms | 42.59 ms | 96.58 qps | 204 MiB | Pass |
| 17,272 | all real | 14.68 ms | 82.84 ms | 51.51 qps | 216 MiB | Pass |
| 25,000 | 17,272 real + 7,728 derived | 23.90 ms | 112.21 ms | 31.61 qps | 232 MiB | Fail: p50 |

All attempted points had zero errors and zero timeouts. Mutation-to-ready p99
was 9.35 ms, 9.51 ms, and 15.19 ms respectively, well within the 5,000 ms
policy. The formal receipts are the [10k](../../experiments/runs/scale-02-a0-10000-20260822T2239Z-e993dd61/record.json),
[17,272](../../experiments/runs/scale-02-a0-17272-20260822T2241Z-c543eeb9/record.json),
and [25k](../../experiments/runs/scale-02-a0-25000-20260822T2245Z-ee93a826/record.json)
records.

The measured advisory A0 envelope on this host is therefore 17,272 records.
The 25k result is the first observed failing point, not evidence of a hard
capacity limit. Reopen SCALE-02 only if the workload, policy, host, inputs, or
production search behavior changes.
