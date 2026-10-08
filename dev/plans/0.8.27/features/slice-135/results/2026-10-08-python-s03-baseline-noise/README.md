---
title: Slice 135 S03 installed Python baseline-only noise pilot
status: AUDITED_LOCAL_RAW_NOT_FROZEN_COMPARISON
target_release: 0.8.27
---

# Installed Python S03 baseline-only noise pilot — 2026-10-08

The [fixture feasibility result](../2026-10-08-python-s03-feasibility/README.md)
preceded this pilot. The [campaign runner](../../../../../../../scripts/slice135_python_s03_baseline_campaign.py)
ran five fresh-process, fresh-database blocks at each 32- and 256-row size,
with 20 seconds idle between blocks. Each block made 100 unprofiled installed
Python calls per case: filter, three pinned temporal instants, bounded graph,
provenance-backed evidence and memory-load search. The baseline source was
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the installed wheel hash was
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
The same fixture and wheel were used in the feasibility result.

All **10/10 blocks** and **7,000/7,000 materialized observations** passed
fixture checks and the [independent campaign auditor](../../../../../../../scripts/slice135_python_s03_baseline_audit.py).
Every reopened database had the expected 41 or 265 nodes, one graph edge,
the pinned validity windows and canonical evidence body, and SQLite integrity
`ok`. No block had an environment invalidator, warning or measured-child swap
event. Child user+system CPU ranged from 32.65 to 33.78 seconds per block;
peak RSS ranged from 436,900 to 438,068 KiB. The pilot's measured wall times
were about 3.6–3.7 seconds per block; parallel embedding work can make summed
CPU time exceed wall time.

The table gives the median of five block percentiles and the range of those
block percentiles divided by their median. Percentiles use nearest rank. The
full five values and exact observations are in the independent audit and raw
receipts.

| Size | Case | Median block p50 (ms) | p50 span | Median block p95 (ms) | p95 span |
| --- | --- | ---: | ---: | ---: | ---: |
| 32 | Evidence | 17.096 | 1.79% | 18.260 | 18.82% |
| 32 | Filter | 9.714 | 2.19% | 10.461 | 1.39% |
| 32 | Graph | 0.851 | 9.86% | 1.030 | 5.86% |
| 32 | Memory load | 0.219 | 10.79% | 0.297 | 5.58% |
| 32 | Temporal boundary | 0.089 | 15.30% | 0.116 | 16.36% |
| 32 | Temporal early | 0.103 | 12.11% | 0.269 | 14.84% |
| 32 | Temporal late | 0.097 | 11.29% | 0.127 | 21.59% |
| 256 | Evidence | 17.235 | 1.48% | 18.383 | 3.08% |
| 256 | Filter | 9.749 | 4.53% | 10.429 | 6.67% |
| 256 | Graph | 0.793 | 10.24% | 1.039 | 7.32% |
| 256 | Memory load | 0.862 | 3.50% | 0.985 | 7.22% |
| 256 | Temporal boundary | 0.089 | 9.86% | 0.113 | 8.21% |
| 256 | Temporal early | 0.109 | 14.21% | 0.279 | 16.85% |
| 256 | Temporal late | 0.096 | 11.57% | 0.126 | 18.64% |

The larger p95 spread in some 32-row evidence and submillisecond temporal
cells means small candidate deltas in those cells will be inconclusive. A
paired result must show all five within-pair deltas and this baseline spread;
neither a single delta nor a sign alone is an equivalence verdict. One
hundred samples support p50/p95 here. **p99 is unsupported** below 1,000
valid observations per cell. No new numerical release gate is introduced.

The local raw directory is
`/tmp/slice135-s03-python-baseline-noise-a3f59356b`. It retains the exact
pilot manifest, 10 block commands and run-order records, environment and GNU
Time resources, 7,000 raw attempts, 10 SQLite databases and per-block
independent audits. Its verified 154-file `SHA256SUMS` has SHA-256
`be72f0416ee054f9505b414f92e55780d0c29065a10e938f75d97ca87011ec83`.
The manifest SHA-256 is
`67a073c0a86b2a241d1f4ea245d77df5b649b5f84dba206c55467ffef347a8fa`;
the run-order SHA-256 is
`56efa241aeacc44d6c66ad37ff99498465b07c5af11db3a286a235b963300140`.
The independent campaign audit SHA-256 is
`d0c7e017ecfcfe101fdce4fa91fcd715471161c9604aac963bb0fbd4e426ecaa`.
The separate negative-control outcomes are retained at
`/tmp/slice135-s03-python-baseline-negative-controls/results.json`: the
auditor rejected a swapped block order, a false filter result and a changed
persisted evidence body.

This qualifies baseline noise for the selected S03 Python fixture and the
100-sample p50/p95 reporting rule. The broader protocol remains a draft.
Before candidate timing, review and freeze the exact paired wrapper, wheel
and runner hashes, five alternating version pairs, 20-second idle, corpus
digests, invalidators and within-pair reporting rule. This baseline-only
campaign provides no 0.8.27 latency comparison or production-traffic weight.
