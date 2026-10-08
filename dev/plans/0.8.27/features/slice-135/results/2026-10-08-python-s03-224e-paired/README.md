---
title: Slice 135 installed Python S03 and equal-frequency query mix at 224e44c59
status: AUDITED_PHASE1_SUBSET
target_release: 0.8.27
---

# Installed Python S03 and query mix

The [S03 protocol](../../s03-python-224e-comparison-protocol.json), SHA-256
`bc098de2cece9e7e5d837981145e0075d2b4dddf6610fc031ad08beed21fb3e8`,
was committed before candidate timing. It compares baseline source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` and candidate source
`224e44c593c13d86ece648adabe445723db04070` through installed Python
wheels. The candidate wheel SHA-256 is
`ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`.
Fresh real SQLite databases, model use and materialized answers were checked
in 20 alternating blocks: 100 calls for each S03 path per block, or 500
per path and version at each corpus size. All blocks passed the
[independent raw/order/resource and persisted-state audit](independent-audit.json)
with zero warnings, semantic failures or measured-child swaps. Its raw input
and run-order hashes are recorded in that audit. The S03 and query-mix
negative tests passed (`10 passed` across their two test files); they reject
semantic tampering and reordered blocks.

| Corpus | Path | Pooled p50 change | Median five-pair p50 change | Pair range |
| --- | --- | ---: | ---: | ---: |
| 32 rows | Evidence | +2.668% | +2.671% | -0.717% to +4.640% |
| 32 rows | Filter | +2.165% | +2.011% | -1.065% to +6.796% |
| 256 rows | Evidence | +2.642% | +2.741% | -1.146% to +6.465% |
| 256 rows | Filter | +0.952% | +1.545% | -4.608% to +4.851% |
| 256 rows | Memory | -15.228% | -13.945% | -20.756% to -11.819% |

Other S03 path values, all p95 values and every observed sample are retained
in the audit and local raw campaign. Each measured S03 path has 500 samples
per version and size, so p99 is unsupported under the frozen 1,000-sample
rule. The pair ranges are descriptive; five pairs do not establish a
significance or equivalence result.

The separately frozen [equal-frequency query-mix protocol](../../phase1-python-query-mix-224e-protocol.json),
SHA-256 `6f04fa9e044a867db754216ba80eb96bea6f5b6cee08f6bdbd84b8429df5bffb`,
selects 500 calls per path and version from the independently audited S01
and S03 campaigns. Its [recomputed ranking](phase1-query-mix-audit.json)
finds the smallest prefix exceeding 80% of elapsed call time:

| Corpus | Candidate ranking prefix | Share of ten-path elapsed time |
| --- | --- | ---: |
| 32 rows | Evidence, filter, vector, hybrid | 97.01% |
| 256 rows | Evidence, vector, filter, hybrid | 96.20% |

This ranking is an equal-frequency installed-Python proxy across separately
prepared S01 and S03 fixtures, not observed production traffic. It ranks
materialized elapsed call time, not per-path CPU or queue time. The latter
remain unsupported by this query-mix instrumentation and are reported
separately in the [four-area checkpoint](../../phase1-checkpoint-2026-10-08.md).

The [copied raw archive](raw-archive/) retains the exact commands,
environment and each block's source, wheel, model, database and resource
metadata. Its 305-file `SHA256SUMS` manifest has SHA-256
`a08cc52bf8ac707b6907d9d6779643405816ca0f684b7d946f371e867aa9902f`;
all copied bytes were verified. It remains untracked, with its manifest and
a verified local bundle recorded in the [retention receipt](../2026-10-08-raw-retention-review/README.md).
