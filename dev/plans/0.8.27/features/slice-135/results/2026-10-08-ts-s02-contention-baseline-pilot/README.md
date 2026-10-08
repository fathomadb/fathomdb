---
title: Slice 135 installed TypeScript S02 contention baseline noise pilot
status: AUDITED_BASELINE_ONLY_LOCAL_RAW
target_release: 0.8.27
---

# TypeScript S02 contention baseline pilot — 2026-10-08

The [functional feasibility pair](../2026-10-07-ts-s02-contention-feasibility/README.md)
established installed-package execution and a product timer that excludes
direct SQLite verification. This pilot then used the exact 0.8.26 source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`, installed npm archives
already pinned by the [TypeScript S02 subset](../../s02-ts-vector-repaired-comparison-protocol.json),
and unchanged contention runner SHA-256
`7f147e23746c78a25831fcb362b2bba5d01a4ac1068a12aef9ea79959d4e3ab4`.

Five separate baseline blocks ran serially, each with one warmup and three
measured fresh processes. Each process used one shared engine handle for
concurrent readers and writers, then checked projection readiness, evidence,
erasure and close/reopen behavior. An independent auditor re-opened every raw
receipt and SQLite database, verified exact installed module and archive
bytes, recomputed call-interval overlap and checked persisted rows. All 20
processes and 15 measured sequences passed; measured-process overlap was at
least seven of eight reader cycles. No attempt was replaced or trimmed.

| Block | Measured sequences | p50 (ms) | Minimum (ms) | Maximum (ms) | Host paging warning |
| --- | ---: | ---: | ---: | ---: | --- |
| 1 | 3 | 5,017.652 | 4,915.355 | 5,239.675 | 1 page |
| 2 | 3 | 5,099.465 | 4,974.498 | 5,127.372 | 5 pages |
| 3 | 3 | 5,060.005 | 5,054.658 | 5,090.025 | None |
| 4 | 3 | 5,062.213 | 4,973.358 | 5,098.890 | None |
| 5 | 3 | 5,081.023 | 5,042.144 | 5,117.539 | None |

The median block p50 was 5,062.213 ms; block medians spanned 81.814 ms.
Two blocks had host-only swap-counter drift, and measured children had zero
swap events. The independent audit checked at least ten seconds of idle time
between blocks two through five, exact runner bytes, environment identity,
resources, sample counts, raw hashes and producer statistics. It rejected an
altered retained block p50. Three observations per block do not support p95,
p99 or a version comparison.

The qualified pilot supports a proposed paired schedule of five alternating
version pairs, 20 measured fresh processes per block plus one warmup, and at
least 20 seconds idle between blocks. Freeze that schedule, hashes and
reporting rules before sampling candidate timing. Host paging warnings will
remain explicit, with warning-free sensitivity only if the paired run
actually supplies it. No equivalence or release-performance threshold follows
from this pilot.

Local raw blocks are `/tmp/slice135-ts-s02-contention-pilot-block-01` through
`-05`; each verified `SHA256SUMS` covers 47 files. The combined auditor,
negative control, driver and block manifests are at
`/tmp/slice135-ts-s02-contention-pilot-evidence`, whose verified manifest
SHA-256 is
`de691ad50d8daf13710a1f42b3850b4fed067940c95e80f9f391924c4b1f26c2`.
Raw archive retention remains at the end of Phase 1. The frozen paired
protocol and campaign, Rust SDK contention, S03 and the broader four-area
checkpoint remain open.
