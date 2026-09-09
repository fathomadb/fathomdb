---
title: 0.8.25 Slice 71 — AC-013 vector-latency investigation
status: ENVIRONMENT_INVALID
date: 2026-09-08
candidate: 5546585da8f5893f903dbde8d1a885c483061c1b
baseline: 4fc1b890a11ebfaa8f11b15823656e856002807a
---

# AC-013 vector-latency investigation

## Exploratory result

The release-mode 10k/384-dimension/1,000-sample campaign completed in intended
`B,C,C,B,B,C` order. Every repetition failed AC-072's p50 <= 80 ms gate. The
baseline was stable; the candidate p50 was stable but its p99 range violated
the preregistered 20% environment-validity limit. The controlling result would
therefore be `environment_invalid`, not `pre_existing_gate_failure`.
However, independent review established that the manifest and evidence were
first committed together. The observation was not preregistered and is not
admissible acceptance evidence.

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
`dev/plans/runs/0.8.25-slice-71/exploratory-receipt.v1.json`; raw logs are
under `ac013-exploratory/`.

## Admissible result

The corrected manifest and capture contract were committed before rerun. The
same six cells completed in `B,C,C,B,B,C` order with all host-pressure checks
valid. Every repetition again failed AC-072 p50. Candidate p99 range/median is
84/210 = 40.00%, so the mechanically derived controlling classification is
`environment_invalid`.

| Arm | p50 ms | p99 ms | Seed write ms | Drain ms |
| --- | ---: | ---: | ---: | ---: |
| B1 | 163 | 173 | 1834 | 82 |
| C1 | 200 | 210 | 4829 | 193 |
| C2 | 201 | 210 | 5104 | 189 |
| B2 | 165 | 174 | 1846 | 90 |
| B3 | 163 | 172 | 1548 | 573 |
| C3 | 201 | 294 | 5096 | 183 |

Baseline range/median is 1.23% for p50 and 1.16% for p99. Candidate is 0.50%
for p50 and 40.00% for p99. Every cell retained 1,000 samples, SQLite/runtime
identity, start/end load and memory, zero swap deltas, `k10temp:Tctl` below the
90 C limit, and empty competing-process inventories. Raw logs and digests are
bound in `dev/plans/runs/0.8.25-slice-71/receipt.v1.json`.

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

The revised campaign confirms `environment_invalid`. The existing first-run
raw results remain exploratory and must not be relabeled. The approved stop
condition blocks bulk-ingest execution and any unregistered further AC-013
treatment. Slice 71 remains blocked for an owner-approved protocol change.
