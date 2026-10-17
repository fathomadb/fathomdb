# FathomDB 0.8.27 cross-SDK FTS comparison

Run date: 2026-10-13. Host: AMD Ryzen Threadripper PRO 5945WX, x86-64 Linux, 62 GiB RAM. Repository revision at plan creation: `03ad8268e137daceeca872abbf82d3d0c24015a1`.

## Workload and method

- Python, TypeScript, and optimized Rust SDK drivers, each declaring SDK 0.8.27 and pinned to hashed local runtime, native, driver, wrapper, and CLI artifacts.
- 100 synthetic documents, 10 single-term queries, one designed relevant document and one exact ordered-ID oracle per query.
- `engine.search_text_only`, limit 10, fixed `valid_as_of` 1700000000, SDK call through materialized result.
- Two independent attempts from the **same** `suite.json` (SHA-256 `87b39529db62dd3cb7c51c59db1bf6ea5bf68845972ef185429df1da3af03a26`). Each has 10 balanced, serial repetitions per SDK and case, one fresh database per repetition, first query pass after populated reopen, no warmup. Thus each cell has 100 timed queries. Separate retrieval and latency/capacity cases use the same workload.
- The runner wrote execution configurations, raw samples, setup and resource witnesses, safe receipts, index rows, and a validated scoreboard. Both attempts contain six complete cells and 600 timed query samples. The 18 receipts include a six-cell pilot, then six cells per main attempt. Independent scoreboard rebuilds passed for both main attempts.

## Latency and capacity

Times are milliseconds per SDK call, measured through materialized results. Peak RSS is the driver process maximum; database size includes SQLite sidecars and is the same for all arms.

| Attempt | SDK | p50 ms | p95 ms | p99 ms | Peak RSS MiB | DB MiB |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 1 | Python | 0.534 | 0.669 | 0.883 | 43.5 | 0.559 |
| 1 | TypeScript | 0.558 | 0.862 | 1.023 | 93.0 | 0.559 |
| 1 | Rust | 0.471 | 0.573 | 0.706 | 17.1 | 0.559 |
| 2 | Python | 0.464 | 0.638 | 0.661 | 43.5 | 0.559 |
| 2 | TypeScript | 0.576 | 0.840 | 0.884 | 92.4 | 0.559 |
| 2 | Rust | 0.436 | 0.576 | 0.615 | 17.1 | 0.559 |

## Retrieval

Each SDK returned the exact designed ordered ID for all 10 queries in both attempts. Recall@10, MRR, and NDCG@10 were **1.00** for all three SDKs. This fixture checks binding agreement and the mechanical retrieval scoreboard; its easy queries do not establish general retrieval quality.

## Interpretation and limits

The local observed median order was Rust, Python, TypeScript in both attempts, with small absolute differences. The second Python median changed by about 0.070 ms. The suite reports `source_provenance: unverified` and `evidence_status: local_descriptive`, so these observations are not a controlled speed ranking. Rust used an optimized release build; Python and TypeScript loaded their present native binaries, but those builds were not independently attested to the same source revision and optimization settings. Driver peak RSS includes language runtime differences. The runner's derived queries-per-second metric is the reciprocal of summed call time at concurrency 1, not end-to-end throughput.

## Artifacts and rerun

- External artifact directory: `/tmp/fathomdb-sdk-compare-20261013/`. Artifact filenames below are relative to that directory. These artifacts are not committed to the repository.
- Pinned experiment: `suite.json`; fixture: `fixture.json`; arm manifests: `python-arm.json`, `typescript-arm.json`, `rust-arm.json`.
- Attempt 1: `suite-output/suite-manifest.json`, `suite-output/scoreboard.json`.
- Attempt 2: `suite-output-repeat/suite-manifest.json`, `suite-output-repeat/scoreboard.json`.
- Safe receipts and append-only index: `receipts/`; Rust build attestation: `rust-build.json`; local dependency inventories: `closure-evidence.json`, `python-pip-freeze.txt`; host snapshot: `host.json`.
- The historical external `run_suite.py` launcher invoked the repository's
  `experiments.sdk_benchmark` runner and `experiments.sdk_scoreboard`. To rerun,
  use the [cross-SDK configuration guide](../../experiments/configs/sdk/README.md)
  from a checkout with its own installed dependencies, the pinned `suite.json`,
  a **new** external output directory, and the original external receipts root.

The artifacts are under `/tmp` and should be moved to durable external storage before temporary-file cleanup if longitudinal comparison is needed. The plan contains absolute paths to those pinned files, so move the directory as a unit and update/re-hash the bindings before another run.
