---
title: Slice 135 installed Python S03 functional feasibility
status: AUDITED_LOCAL_RAW_NOT_FROZEN_COMPARISON
target_release: 0.8.27
---

# Installed Python S03 functional feasibility — 2026-10-08

The [S03 runner](../../../../../../../scripts/slice135_python_s03.py) combines
existing human-authored query-correctness shapes: the cross-binding X1 search
fixture, pinned validity-window tests, Slice 60 bounded graph traversal, Slice
50 provenance-backed evidence, and the Slice 135 S01 32/256-row corpus. It
uses a real database and the installed Python wheel at each source identity.
The two corpus sizes are a workload proxy, not a claim that they reproduce
production traffic. No external benchmark answers are scored here.

| Source | 32 rows | 256 rows | Wheel SHA-256 |
| --- | --- | --- | --- |
| 0.8.26 `f99e002f0d2e4002f3694c9f8d4986b56089edaa` | 7/7 fixture cases | 7/7 fixture cases | `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282` |
| 0.8.27 candidate `3f29d649d0213e595c0dab251a449d92fd625792` | 7/7 fixture cases | 7/7 fixture cases | `39d4af3060ca4b72d7ad16211fa0ba44a8992feef87040f3513a262cbdc359a4` |

Each run seeded one fresh SQLite database, drained projection work, closed and
reopened the installed SDK, and materialized one result for each filter,
temporal early/boundary/late, graph, evidence, and memory-load case. The
independent [auditor](../../../../../../../scripts/slice135_python_s03_audit.py)
reopened all four raw receipts and databases. It checked exact fixture
results, the half-open temporal boundary, retained canonical bodies and
windows, the graph edge, corpus count, and SQLite integrity. The 32-row
databases each contained 41 nodes and one edge; the 256-row databases each
contained 265 nodes and one edge. Two negative controls changed a claimed
filter result and the persisted canonical evidence body. Both were rejected.

The raw directories are local pending the final Phase 1 retention review:

| Size | Version | Raw directory | Raw JSON SHA-256 | Database SHA-256 |
| --- | --- | --- | --- | --- |
| 32 | baseline | `/tmp/slice135-s03-python-baseline-feasibility-01` | `b39cbb03d8c50946d57d726cfad090250bd22d4e2c77abea65a91a6dbbc53e4b` | `9632bba91bbeb4a3b8637a154b726fcac40d9e353db856c72376fc016226faaa` |
| 32 | candidate | `/tmp/slice135-s03-python-candidate-feasibility-01` | `9bbab13b1bd444191a1019c943aba47e6b71d3c8651900044440c83311ee1ed2` | `2058ed1ed7e1d9ff94c50c14efab9aacb6ff11974d4d6f9ed42915ebe1672e64` |
| 256 | baseline | `/tmp/slice135-s03-python-baseline-feasibility-256-01` | `ba366d6e0a78f9f943ac640c53ff757147c4a3b4c90a72060f43b9a198c32648` | `f1219f7272432b9498bbcb4af507ba6480d47de2fb19fbad80cc24786ab11633` |
| 256 | candidate | `/tmp/slice135-s03-python-candidate-feasibility-256-01` | `b5886ac1b78896b01e0add18eaf45358529aacc483d028814b64d652ea99123f` | `731c292167e308f29b7bd7b37ddeca52b8482d949007800ff55f83e741643241` |

The runner SHA-256 is
`116f1e206970a257119fee34b2deb034178cb11154bcdf8a002af6cbc9480c78`;
the current auditor SHA-256 is
`5745bc8bebe41b7afa68fc0a91a753a75daf966b4c78541fbc37a2e8732e1373`.
The four feasibility receipts were re-audited after the auditor added
nearest-rank and aggregate-cost summaries; their raw JSON and database hashes
above did not change.
Each raw JSON binds the runner, X1 fixture, S01 corpus helper, and S02 graph
helper bytes. The prior S01 receipts bind each wheel to its clean source.
The negative-control outcomes are retained at
`/tmp/slice135-s03-negative-controls/results.json`.

These are **one-shot functional feasibility runs**. The captured durations
have no baseline noise qualification, frozen pair order, resource envelope,
repeated observations, or supported percentile estimate. They must not be
used as a 0.8.26/0.8.27 latency comparison. Next: qualify repeated baseline
blocks, freeze the S03 subset, then run paired, independently audited cells
and use the same fixed mix for Pareto and coverage attribution. Graph and
evidence reuse S02's established fixture; S03 does not replace S02's whole
system lifecycle or its fault matrix.
