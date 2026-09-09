---
title: 0.8.25 Slice 71 — AC-013 vector-latency investigation
status: ENVIRONMENT_INVALID
date: 2026-09-08
candidate: 5546585da8f5893f903dbde8d1a885c483061c1b
baseline: 4fc1b890a11ebfaa8f11b15823656e856002807a
---

# AC-013 vector-latency investigation

## Result

The release-mode 10k/384-dimension/1,000-sample campaign completed in sealed
`B,C,C,B,B,C` order. Every repetition failed AC-072's p50 <= 80 ms gate. The
baseline was stable; the candidate p50 was stable but its p99 range violated
the preregistered 20% environment-validity limit. The controlling result is
therefore `environment_invalid`, not `pre_existing_gate_failure`.

| Arm | p50 ms | p99 ms | Seed write ms | Drain ms |
| --- | ---: | ---: | ---: | ---: |
| B1 | 165 | 174 | 1749 | 256 |
| C1 | 201 | 260 | 2901 | 2153 |
| C2 | 202 | 311 | 4215 | 681 |
| B2 | 165 | 176 | 586 | 1863 |
| B3 | 164 | 172 | 1772 | 245 |
| C3 | 202 | 211 | 4836 | 186 |

Baseline range/median is 0.61% for p50 and 2.30% for p99. Candidate is 0.50%
for p50 and 38.46% for p99. All raw logs and their digests are bound in
`dev/plans/runs/0.8.25-slice-71/receipt.v1.json`.

## Attribution correction retained

The deterministic RED trace observed 24 post-filter source lookups for 24 FTS
hits. GREEN moves active-barrier eligibility into the edge FTS and edge-vector
SQL/hydration paths, where node paths already enforced it, and removes the
common per-hit post-filter. The trace then observes zero redundant lookups;
real-database node FTS, edge FTS, edge-vector, and vector pre-truncation barrier
tests remain green.

This proved and corrected one N+1 mechanism, but it did not make AC-072 pass.
No second treatment, threshold relaxation, fixture change, or semantic
weakening was attempted after observing the sealed result.

## Disposition

Slice 71 stops before bulk-ingest execution. A resumed run needs an owner-
approved protocol for obtaining environment-valid AC-013 evidence and deciding
whether further profiling is in scope. The existing raw results remain
negative/invalid evidence and must not be relabeled.
