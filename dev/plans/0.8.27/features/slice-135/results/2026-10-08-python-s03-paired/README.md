---
title: Slice 135 S03 installed Python paired comparison
status: AUDITED_SUBSET_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Installed Python S03 paired comparison — 2026-10-08

The [baseline-only noise pilot](../2026-10-08-python-s03-baseline-noise/README.md)
preceded this campaign. The [executable comparison protocol](../../s03-python-comparison-protocol.json)
was committed at `d9a684096` before candidate timing; its exact SHA-256 is
`b32a4ec20e72ff6ffd8fadda6bd1255f451691f32b5dc38b8b5376938c5c0d14`.
It binds 0.8.26 source `f99e002f0d2e4002f3694c9f8d4986b56089edaa`
and wheel `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`
against candidate product source `3f29d649d0213e595c0dab251a449d92fd625792`
and wheel `39d4af3060ca4b72d7ad16211fa0ba44a8992feef87040f3513a262cbdc359a4`.
The candidate source contains both off-ladder landings and the later
search/graph error repairs. The package version string remains 0.8.26 and is
not used as candidate identity. The separate [provenance check](slice135-s03-python-paired-provenance-d9a684096.json)
rechecked both clean source checkouts, wheel members, installed native bytes
and every retained command.

The [campaign runner](../../../../../../../scripts/slice135_python_s03_pair_campaign.py)
completed five alternating version pairs at each 32- and 256-row corpus size,
with 20 seconds idle between blocks. Each fresh-process, fresh-database block
made 100 unprofiled installed Python calls per case through a materialized
result: filtered search, three pinned temporal instants, bounded graph,
provenance-backed evidence and memory-load search. All **20/20 blocks** and
**14,000/14,000 observations** passed the independent
[raw and reopened-state audit](slice135-s03-python-paired-audit-d9a684096.json).
Each reopened database had the expected canonical bodies, validity windows,
41 or 265 nodes, one graph edge and SQLite integrity `ok`. No environment
invalidator, host warning, measured-child swap or major fault was recorded.
Candidate peak child RSS ranged 301,992–303,480 KiB, against baseline
437,264–437,860 KiB. RSS is a process-level diagnostic, not a complete
allocator or ownership attribution.

The table gives descriptive **pooled** nearest-rank percentile deltas from
500 raw observations per version and cell; positive means the candidate is
slower. The [audit JSON](slice135-s03-python-paired-audit-d9a684096.json)
retains all five within-pair p50/p95 deltas and the source raw hashes. The
positive-pair count is for p50. No valid slow sample was discarded.

| Size | Case | Baseline p50 (ms) | Candidate p50 (ms) | p50 delta | p95 delta | Positive p50 pairs |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| 32 | Evidence | 17.181 | 17.522 | +1.989% | +0.231% | 4/5 |
| 32 | Filter | 9.828 | 9.841 | +0.130% | −1.843% | 2/5 |
| 32 | Graph | 0.863 | 0.848 | −1.761% | −0.065% | 2/5 |
| 32 | Memory load | 0.214 | 0.199 | −6.944% | −7.634% | 0/5 |
| 32 | Temporal boundary | 0.095 | 0.090 | −5.285% | −0.050% | 3/5 |
| 32 | Temporal early | 0.115 | 0.106 | −7.535% | −6.133% | 2/5 |
| 32 | Temporal late | 0.101 | 0.097 | −4.089% | −4.479% | 1/5 |
| 256 | Evidence | 17.337 | 18.007 | +3.863% | +1.983% | 5/5 |
| 256 | Filter | 9.717 | 9.880 | +1.676% | +1.847% | 4/5 |
| 256 | Graph | 0.843 | 0.838 | −0.681% | −1.730% | 2/5 |
| 256 | Memory load | 0.848 | 0.768 | −9.424% | −6.269% | 0/5 |
| 256 | Temporal boundary | 0.088 | 0.086 | −2.512% | −3.705% | 2/5 |
| 256 | Temporal early | 0.107 | 0.107 | −0.104% | +0.513% | 3/5 |
| 256 | Temporal late | 0.101 | 0.097 | −3.098% | −1.522% | 1/5 |

The 256-row evidence p50 increase appears in all five pairs and exceeds the
baseline pilot's 1.48% five-block p50 span for that cell; it is a performance
investigation lead. The 32-row evidence delta is close to its 1.79% pilot
span and changed sign in one pair. The memory-load p50 decrease appears in
all five pairs at both sizes, but this fixed fixture does not establish a
general search speedup. Several submillisecond temporal and graph deltas
change sign by pair. **p99 is unsupported** with fewer than 1,000 samples per
version and cell. There is no numerical equivalence threshold or release
performance verdict from this subset.

Under this deliberately equal-frequency mix, evidence plus filtered search
accounted for 95.21% of candidate elapsed query cost at 32 rows and 93.49%
at 256 rows. This is an S03 workload cost proxy, not production-traffic
weight or sampled CPU attribution. The full Pareto matrix must combine the
declared operation mix, CPU/queue ranking, branch coverage and severe rare
paths. The evidence lead merits call-boundary attribution alongside the
earlier engine and SDK vector, hybrid and lifecycle leads.

The [negative controls](slice135-s03-python-paired-negative-d9a684096.json)
show that the independent auditor rejected a swapped run order, one false
filter observation and a changed persisted body. Their file SHA-256 is
`aa9fddbf08da7e4bd1264079fe2bc8ba7d0a77536ec701da275267bec6f0099f`.
The provenance result SHA-256 is
`2a207dead3792930ca47ecb04d28d6f79694269490f7d8c9305dfb975c0ab455`;
the independent audit SHA-256 is
`c9302229bfb97c91ad76be34672b7cafd8eadcb125cc8ae68e698e45eb19734d`.

The local raw archive is `/tmp/slice135-s03-python-paired-d9a684096`. It
retains 20 commands, run-order and environment records, GNU Time resources,
14,000 raw attempts, 20 SQLite databases and block audits. Its verified
303-file `SHA256SUMS` has SHA-256
`2038321c97c434897b260b2dfa04c26012a29dc32a9eb1f5903ff92f450d0bb0`.
The campaign manifest SHA-256 is
`3d4eaf46b45750ff296c179b9baa6fd3bf77fa2a08d57256621afeb2d5c6ff6f`;
run-order SHA-256 is
`68f3ee4213de61593eb42c46c4f29d1b50c58c3c23bf50549d18f0343fa1e856`.
This selected S03 comparison expands system latency and the Pareto cost
proxy. It does not close the broader Phase 1 protocol, robustness or logic
matrices, or begin dedicated gold-answer scoring.
