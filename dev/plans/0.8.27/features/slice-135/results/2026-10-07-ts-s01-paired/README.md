---
title: Slice 135 installed TypeScript S01 paired diagnostic
status: VALID_PAIRED_DIAGNOSTIC_SENSITIVITY_LIMITED
target_release: 0.8.27
---

# Installed TypeScript S01 paired diagnostic

The [frozen comparison protocol](../../s01-ts-comparison-protocol.json),
SHA-256 `bbff7de33f874c4647488705759217ba451b6424ee603beee021ad24e9661bd2`,
was committed before these paired blocks. It compares exact 0.8.26 source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` with the post-Slice-132
candidate source `b2ac8081e79a6e626d01f5f97331be331f1cc16d`. The
candidate package still carries a pre-release `0.8.26` version string;
source and [archive hashes](../2026-10-07-ts-s01-noise-pilot/README.md)
identify the measured build. Both main and Linux x64 GNU native package
archives are retained with the baseline noise result.

The [campaign driver](campaign-driver.py) ran five alternating pairs per
corpus size, at 32 and 256 rows. Each of the 20 blocks made one session-first
call, one warm-up and **1,000 warm installed-TypeScript SDK calls** for each
of text, vector-bearing and hybrid retrieval. The corpus and query bytes
match the Python S01 workload. Each query timed the async SDK invocation
through the resolved, materialized result; corpus setup, model loading and
semantic verification were outside the per-call timer. The session-first
diagnostic follows reopen but is not a cold OS or model-cache measurement.
Node `v25.9.0`, the default BGE-small model and CPU embedding were held fixed.

The [independent audit](independent-audit.json) verified all **20 valid
blocks and 60,120 call attempts**, exact source and archived package bytes,
the frozen runner hashes, every seeded result assertion, nearest-rank
p50/p95/p99, command and environment records, and the complete
[run order](run-order.jsonl) with at least 20 seconds between blocks. A replay
from the retained receipts produced byte-identical audit JSON. The audit's
negative controls reject a wrong text ID, false p99 and wrong native
artifact. The [hash manifest](SHA256SUMS) covers every result file.
The immutable workload runner retains its pilot-era `status` string inside
each raw file; each block's `attempt.json` and `pilot-protocol.json` bind
that raw output to the frozen paired protocol and measured role.

Candidate-minus-baseline paired percentage deltas follow. Negative values
mean the candidate was faster. Each cell shows the median of five pairs and
the full observed range; these are descriptive, without a regression
threshold or significance claim.

| Rows | Query | p50 median (range) | p95 median (range) | p99 median (range) |
| --- | --- | --- | --- | --- |
| 32 | Text | +0.33% (−12.49% to +7.19%) | −0.49% (−1.98% to +1.96%) | −3.37% (−20.67% to +5.40%) |
| 32 | Vector-bearing | +1.58% (−3.26% to +3.04%) | +2.06% (−5.04% to +3.28%) | +3.50% (−9.35% to +5.70%) |
| 32 | Hybrid | −0.15% (−1.43% to +2.07%) | −0.28% (−7.40% to +3.18%) | −2.63% (−15.19% to +4.25%) |
| 256 | Text | −0.47% (−3.40% to +2.45%) | −0.61% (−6.00% to +2.97%) | −14.12% (−21.44% to −1.94%) |
| 256 | Vector-bearing | +0.02% (−0.18% to +3.77%) | +0.06% (−0.62% to +2.35%) | −1.27% (−7.71% to +13.02%) |
| 256 | Hybrid | +0.12% (−0.30% to +1.66%) | +1.02% (−1.12% to +3.29%) | +1.47% (−2.29% to +13.55%) |

All measured children recorded zero swap events and zero major faults. **Twelve
of 20 blocks** had small host swap-counter changes (one to eight pages per
block), retained as warnings. Just one pair at each size had no host warning,
so warning-free sensitivity is insufficient. The 256-row text p99 delta was
negative in all five pairs, but its size and reliability need a follow-up on
a quieter host or more warning-free pairs. Other cell ranges generally cross
zero. These observations do not establish a whole-product latency verdict.

Whole-worker peak RSS, which includes setup and model loading, ranged from
608–618 MiB for baseline versus 378–391 MiB for candidate at 32 rows, and
614–658 MiB versus 394–422 MiB at 256 rows. These resource figures are not
per-query attribution. The raw GNU Time CPU, I/O and memory records remain in
each block. The protocol's accepted release gates and broader S02 system
latency, robustness, Pareto and logic matrices remain separate and open.
The [full-gate verification](verification.md) ran all registered suites and
remains red on two previously observed failures; no release-green claim is
made from this diagnostic.
