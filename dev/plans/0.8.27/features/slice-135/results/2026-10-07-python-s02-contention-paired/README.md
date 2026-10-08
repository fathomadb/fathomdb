---
title: Slice 135 installed Python S02 bounded-contention paired diagnostic
status: AUDITED_LOCAL_RAW_PENDING_RETENTION
target_release: 0.8.27
---

# Installed Python S02 contention pair — 2026-10-07

The [frozen subset](../../s02-python-contention-comparison-protocol.json) was
committed at `cdb890c1b` before paired timing. Its SHA-256 is
`f695109dd82b983aa8a3dbad5cc3fc2f11f6a257fd18e7e15d3498b678ef2f0b`.
The clean 0.8.26 source was `f99e002f0d2e4002f3694c9f8d4986b56089edaa`;
the repaired 0.8.27 candidate was
`3f29d649d0213e595c0dab251a449d92fd625792`, with Rust crate tree
`c8eaaa19a0e0e876326b778733ace354cc7e21e8`. Installed wheel SHA-256
values were `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`
and `39d4af3060ca4b72d7ad16211fa0ba44a8992feef87040f3513a262cbdc359a4`,
respectively. The source package version string is not the comparison
identity.

## Workload and qualification

Each fresh process opened one real database with an installed Python wheel,
wrote 32 corpus and three graph nodes plus one edge, configured and drained
the vector projection, and synchronized an eight-write thread with an
eight-cycle reader thread on the same engine handle. Reader cycles exercised
get, text, vector and graph-neighbor calls. After contention, each process
drained projection work, checked evidence, erased graph and contended
sources, closed, reopened and checked persisted state. The timer ran from
initial open through reopened close. It included the whole product sequence,
while the independent auditor inspected direct SQLite state after timing.
Every measured process had actual writer/reader call-interval overlap; the
minimum independently recomputed overlap was six of eight reader cycles.

The baseline-only pilot retained an initial invalid attempt: resolving the
virtual-environment Python symlink selected the system interpreter. The
launcher was corrected and five later blocks passed, each with one warmup
and three measured sequences. The 15 valid pilot medians ranged from
5,407.131 to 5,484.336 ms by block, a 77.205 ms spread. Three blocks had
host-only paging warnings and no measured child swapped. The pilot and
negative controls preceded the frozen schedule of five alternating version
pairs, 20 measured sequences and one warmup per block, at least ten seconds
between serial blocks. No slow valid sample was trimmed or replaced.

## Paired observation

| Pair | Baseline p50 (ms) | Candidate p50 (ms) | Paired p50 change | Host paging warning |
| --- | ---: | ---: | ---: | --- |
| 1 | 5,430.458 | 5,449.432 | +0.349% | Yes |
| 2 | 5,430.705 | 5,447.013 | +0.300% | Yes |
| 3 | 5,426.604 | 5,444.822 | +0.336% | Yes |
| 4 | 5,415.786 | 5,445.352 | +0.546% | Yes |
| 5 | 5,425.064 | 5,436.748 | +0.215% | Yes |

All ten blocks and 200 measured sequences passed independent source, wheel,
runner, raw-output, GNU Time, overlap, evidence, erasure and reopened-state
checks. Each block also had one valid warmup. The independent SQLite oracle
found `integrity_check=ok`, 32 retained corpus nodes and zero erased graph
nodes, graph edges or contended nodes in every process. Pooled nearest-rank
whole-sequence p50 was **5,427.298 ms** on 0.8.26 and **5,447.553 ms** on the
candidate, a **+0.373%** descriptive change. Pooled p95 was 5,510.206 and
5,502.308 ms, respectively, a **−0.143%** change. The median paired-block
p50 change was +0.336%, ranging from +0.215% to +0.546%. P99 is unsupported
by this sample size.

Eight of ten blocks had host-only swap-counter drift; every pair includes at
least one warning-bearing block. Measured children had zero swap events. No
warning-free paired sensitivity estimate exists, so this run neither proves
equivalent system latency nor establishes a release performance verdict.
Measured-child peak RSS block medians ranged from 451,812 to 452,124 KiB on
the baseline and 317,520 to 317,812 KiB on the candidate. That is a resource
observation at this process boundary, not an attribution of the memory
difference to a specific component. Contention and the long reopened-open
stage of earlier S02 runs still need cross-boundary attribution.

The independent campaign auditor rejected two altered retained-campaign
controls: swapped block order and a falsified call-overlap count. The original
order and raw sample hashes were unchanged. Its three focused tests passed;
Ruff passed with `--no-cache`. No new full agent gate was run for this subset.

The local raw campaign is
`/tmp/slice135-python-s02-contention-paired-cdb890c1`. It includes the
frozen protocol snapshot, independent auditor, accepted audit, negative
controls, all process receipts and a verified `SHA256SUMS` covering 2,167
files. The manifest SHA-256 is
`3ce5d9eaefc7d565ebdfc9f8238919c0096f272301ef32f92adbb21500d6409d`.
The five accepted baseline pilot blocks remain at
`/tmp/slice135-python-s02-contention-pilot-block-02` through `-06`; the
initial invalid attempt remains at `-01`. Raw archive retention is deferred
to the end of Phase 1. TypeScript and Rust SDK bounded contention, S03, the
full robustness and coverage matrices, and the exact-candidate Phase 1
checkpoint remain open.
