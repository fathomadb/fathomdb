---
title: Slice 135 installed TypeScript S02 contention paired diagnostic
status: AUDITED_LOCAL_RAW_PENDING_RETENTION
target_release: 0.8.27
---

# Installed TypeScript S02 bounded contention — 2026-10-08

The [frozen subset](../../s02-ts-contention-comparison-protocol.json) was
committed at `eed3d4728` before candidate timing. Its SHA-256 is
`dc9f7aa4b08ba3480debb1565bf0ecaa56526dc58663a2eb0b9be1913e5e550c`.
The installed baseline npm packages are bound to source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the repaired candidate
packages are bound to `3f29d649d0213e595c0dab251a449d92fd625792`.
This is a version comparison at the installed TypeScript SDK boundary, not
an engine-only or Rust SDK result.

Each fresh-process sequence opened one real database, seeded 32 corpus nodes
and a three-node/one-edge provenance graph, configured and drained the vector
projection, then ran eight writer calls and eight reader cycles on the same
engine handle. Readers asserted retained `get`, text, vector and graph
behavior while writers committed distinct rows. The sequence checked
projection readiness, evidence resolution, erasure, and SDK state after
close/reopen. Independent direct SQLite verification ran **after** the whole
product timer. The [feasibility receipt](../2026-10-07-ts-s02-contention-feasibility/README.md)
and [five-block baseline pilot](../2026-10-08-ts-s02-contention-baseline-pilot/README.md)
preceded the frozen schedule.

## Paired result

Five alternating version pairs supplied 20 measured fresh processes plus one
warmup per block, with at least 20 seconds idle between blocks. All ten
blocks and **100 measured sequences per version** passed independent raw,
installed-artifact, timeline, resource and reopened-state audit. The minimum
actual writer/reader overlap was six of eight reader cycles in a block;
every sequence had nonzero overlap. The auditor recomputed every duration and
nearest-rank percentile from the 200 raw observations.

| Whole product interval | 0.8.26 baseline | 0.8.27 candidate | Candidate change |
| --- | ---: | ---: | ---: |
| Pooled p50 | 5,058.285 ms | 5,234.760 ms | +3.489% |
| Pooled p95 | 5,310.556 ms | 5,301.865 ms | −0.164% |

All five within-pair p50 changes were positive, ranging from +2.983% to
+4.210%, with median +3.408%. Four pairs had host-only swap-counter warnings;
the one warning-free pair had a +3.152% p50 change. No measured child
swapped. The pooled p95 and within-pair p95 signs differ, so the small pooled
p95 decrease is not evidence of a tail improvement. There is no supported
p99, equivalence verdict or whole-release performance verdict from this cell.
The repeated p50 increase is a TypeScript whole-sequence attribution lead.

The separate [paired auditor](../../../../../../../scripts/slice135_ts_s02_contention_pair_audit.py)
reopened every raw JSON receipt and SQLite database and rejected two retained
negative controls: a changed block order and a false overlap claim even
after the intermediate SHA-256 chain was updated. Its three focused tests,
the workload/block/campaign tests and Ruff pass. The measured campaign
contains its own independent audit, negative-control script and output,
frozen protocol, source snapshots, installed npm archives, block commands,
resources and raw databases. Its verified `SHA256SUMS` covers **2,220 files**
and has SHA-256
`3b5613931c12bb70fc1aa03d0822682607864106792f9b4b7ec2164bae4de84b`.
The local raw directory is
`/tmp/slice135-ts-s02-contention-paired-eed3d4728`; final archive retention
is deferred until the end of Phase 1.

This result closes the installed TypeScript bounded-contention measurement
subset. Rust SDK candidate-only contention, the broader S03 and four-area
matrices, final source-bound checkpoint and full gate remain open.
