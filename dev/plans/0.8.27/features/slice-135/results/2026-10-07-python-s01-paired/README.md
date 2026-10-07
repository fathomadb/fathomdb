---
title: Slice 135 installed Python S01 paired campaign
status: S01_PYTHON_PAIRED_DIAGNOSTIC_COMPLETE
target_release: 0.8.27
---

# Installed Python S01 paired campaign

The [frozen subset protocol](../../s01-python-comparison-protocol.json) governs
the installed 0.8.26 baseline and post-Slice-132 candidate comparison. The
baseline source is `f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the
candidate wheel came from `b2ac8081e79a6e626d01f5f97331be331f1cc16d`.
The candidate wheel filename still carries version 0.8.26 before the release
version bump, so the SHA-256 in the protocol identifies its actual bytes.
Product source and `Cargo.lock` did not change between that source commit and
the protocol freeze. No candidate timing was examined before the protocol's
first revision.

The first 32-row baseline block is retained as
[an invalidated attempt](invalid-pre-p99-fix/attempt.json). Its wrapper marked
the block valid and wrote 1,000 warm observations per query shape, but its
summary still labeled p99 unsupported. A failing focused test exposed that
inconsistency. Protocol SHA-256
`13188f0b303f87af1289371ee82dd4889c6e84a3abab7ecb7ba1f659ec370ec4`
and block runner SHA-256
`550e9729059a88c92e085d0a59b0b3614467088dc8e77c623cafafe4af748f3a`
identify that attempted revision. The raw samples are preserved, but this
block is excluded from all paired summaries. The corrected protocol pins a
new block runner hash and restarts pair 1 from a fresh database.

The corrected campaign then ran **five alternating baseline/candidate pairs**
at each of 32 and 256 rows, with 1,000 warm calls per text, vector-bearing and
hybrid query shape per block. The [independent audit](independent-audit.json)
checked all twenty valid block receipts, 60,120 materialized calls, seeded IDs
and result branches, archived wheel bytes, frozen protocol and runner hashes,
environment snapshots, block order and idle interval. It recomputed every
nearest-rank p50/p95/p99 and paired delta from raw observations. A deliberately
wrong text ID and a wrong reported p99 both fail its focused tests. The
[hash manifest](SHA256SUMS) covers all retained files; the exact
[block runner](block-runner.py), [remaining campaign driver](remaining-driver.py)
and [run-order log](run-order.jsonl) are retained. Pair 1 at 32 rows was run
manually, with its exact commands recorded in its two block receipts; the
driver covers the remaining eighteen blocks.

## Paired query-call latency

Negative deltas mean the candidate was faster. Each entry is the median of
five within-pair percentage deltas, followed by the full observed pair range.
These are installed Python method-call timings through materialized results;
corpus setup, model loading and semantic verification are outside the query
timer. The vector-bearing shape calls the public hybrid `search` method with
no lexical match; it is not a separate vector-only API. All pair values and
session-first observations are in the audit and raw block files.

| Rows | Query | p50 median [range] | p95 median [range] | p99 median [range] |
| --- | --- | ---: | ---: | ---: |
| 32 | Text | +0.15% [−5.80%, +2.73%] | −2.08% [−12.46%, +0.51%] | −1.47% [−8.44%, +14.63%] |
| 32 | Vector-bearing | −2.12% [−6.11%, −1.48%] | −4.01% [−5.64%, −1.87%] | −5.83% [−12.12%, −1.41%] |
| 32 | Hybrid | −1.48% [−3.38%, −0.14%] | −0.38% [−6.09%, +1.92%] | −0.66% [−13.02%, +4.99%] |
| 256 | Text | −0.81% [−8.12%, +6.99%] | −4.22% [−16.56%, +8.94%] | +2.26% [−27.41%, +8.45%] |
| 256 | Vector-bearing | −0.09% [−1.35%, +2.25%] | −0.99% [−4.50%, +1.84%] | +1.44% [−13.82%, +3.34%] |
| 256 | Hybrid | +0.38% [−2.20%, +1.06%] | −0.74% [−2.70%, +2.32%] | −2.94% [−5.77%, +7.74%] |

The 32-row vector-bearing cell was faster in all five pairs at the three
reported percentiles. The other cells have mixed signs or broad ranges.
These observations diagnose this narrow workload; they do not establish a
general product speed verdict or a release threshold. The 1,000-sample p99
is the minimum declared tail sample count, so its five-pair range matters more
than a single point estimate.

## Whole-worker resources and validity

The table below reports ranges across five blocks per role. Wall time covers
the complete worker from its raw start to finish, including database setup;
GNU Time child CPU and peak RSS also cover the whole worker. CPU seconds can
exceed wall seconds because the embedder uses multiple threads. Full per-block
user/system CPU, filesystem block I/O, major faults and swap counts are in the
independent audit.

| Rows | Role | Worker wall seconds | Child user CPU seconds | Peak RSS MiB |
| --- | --- | ---: | ---: | ---: |
| 32 | 0.8.26 baseline | 25.08–26.05 | 257.06–261.92 | 441.5–442.4 |
| 32 | 0.8.27 candidate | 24.52–25.50 | 252.74–260.12 | 343.7–357.5 |
| 256 | 0.8.26 baseline | 31.31–31.45 | 298.56–299.35 | 445.0–446.4 |
| 256 | 0.8.27 candidate | 29.08–29.24 | 282.65–284.79 | 350.3–365.5 |

All twenty corrected blocks passed the declared invalidators; every measured
child reported **zero swap events and zero major faults**. Nine blocks had
small changes in the **host** swap counter, retained as warnings. Excluding
any pair containing a warning leaves only two clean pairs at 32 rows and one
at 256 rows, too few for a meaningful clean-pair trend. The audit exposes
those clean-pair deltas and labels their inference insufficient. Host activity
therefore limits confidence even though no measured child swapped.

This is an S01 Python subset result. It does not close the Rust and TypeScript
installed boundaries, the accepted-operation functional register, S02's whole
system sequence, the engine cells, or the other Phase 1 matrices. The raw
semantic assertions are basic workload validity, not independent relevance
or answer gold. The [full-gate disposition](verification.md) records two
open repository test failures and the successful standard Slice 135 harness
registration on this commit.
