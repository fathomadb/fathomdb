---
title: Slice 135 baseline-only installed TypeScript S01 noise pilot
status: VALID_BASELINE_NOISE_NO_CANDIDATE_VERDICT
target_release: 0.8.27
---

# Installed TypeScript S01 baseline noise pilot

The exact 0.8.26 source was
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`. Node `v25.9.0` and npm
`11.12.1` built release-mode N-API and TypeScript archives locally. The
[main package](fathomdb-0.8.26.tgz) SHA-256 was
`90363762405041b11e6dd654da9cb6347695399eb37b81c5c09fde91296ac336`;
the [Linux x64 GNU native package](fathomdb-linux-x64-gnu-0.8.26.tgz)
SHA-256 was
`b59b1862b11b8ed3edcdc2e5ee86f9db142268bb5e14571054f6245718fc28f6`.
The installed native binary SHA-256 was
`4a3aef2833060cde858faad689e4c734de31dc70e9dab13a00a6594f694c60ca`.
Both archives are retained, and the auditor checks their member bytes against
the measured installed module and native binary.

The first block ran directly through the retained wrapper; the
[driver](noise-driver.sh) ran the remaining nine blocks serially, for five
blocks at each of 32 and 256 rows. Idle intervals separated subsequent
blocks. Each block has one
session-first call, one warm-up and 100 warm calls for text, vector-bearing
and hybrid queries through an **installed package**. The corpus bytes match
the Python S01 fixture hashes. Timing starts at the async SDK method call
and ends at its materialized result; corpus setup, model loading and result
validation are outside each timer. The session-first observation follows a
database reopen, though OS and model caches may be warm. The blocks used the
same [workload runner](32-01-baseline/runner.mjs), SHA-256
`45639bd5f4a345db7aa08ade548d1d3ac17cb9fd225c95408af3709a4336841d`.
The exact pilot [block wrapper](block-runner.py) is retained as a snapshot.

The [independent audit](independent-audit.json) verified **10 valid blocks
and 3,060 materialized call attempts**, recomputed every basic result check
and supported percentile, checked source and archive identities, inspected
the environment/resource records, and reproduced the committed audit bytes.
All ten measured child processes recorded zero swap events and zero major
faults. Four blocks recorded small **host** swap counter changes; those
warnings remain in the receipts. No valid slow call was removed. The
auditor's separate unit controls reject a wrong seeded text ID, a false
percentile and a wrong native identity.

Earlier ten-warm-call [baseline](baseline-exploratory-32.json) and
[candidate](candidate-exploratory-32.json) feasibility probes are retained
outside the audited noise set. They showed the same seeded IDs and branch
shapes through both installed packages, but have no environment/resource
qualification, no p99 and no paired performance status. Their stderr logs
were empty. The candidate [main](candidate-fathomdb-0.8.26.tgz) and
[native](candidate-fathomdb-linux-x64-gnu-0.8.26.tgz) archives are retained;
their SHA-256 digests are respectively
`d118b8d299e647ce3804afe1989fe8dc9fcc459a060f094897d51439f9b9c2dd`
and `c4e1a0d93e8fc11c19ffef68c5ff417ebc27a879890bdce7c4df5c2bc01f50b9`.
The source-bound candidate native SHA-256 in its probe is
`cb6402660e90f1dcc7d21fbecda7d44c50305ec62c355d6b71a5fd4aa4606a9a`.

| Rows | Query | Warm p95 range across five blocks | Range as percent of median p95 |
| --- | --- | ---: | ---: |
| 32 | Text | 0.380–0.532 ms | 34.059% |
| 32 | Vector-bearing | 10.680–11.740 ms | 9.749% |
| 32 | Hybrid | 9.518–10.068 ms | 5.610% |
| 256 | Text | 0.368–0.466 ms | 26.312% |
| 256 | Vector-bearing | 11.929–13.323 ms | 10.790% |
| 256 | Hybrid | 10.577–12.057 ms | 13.501% |

This is a baseline-only **noise and functional qualification**. One hundred
warm calls per block do not support p99, and these data do not establish a
candidate latency change. The 32-row text tail is especially variable; the
paired protocol must use longer blocks, alternating order and report the
full paired spread. No installed Rust or TypeScript S02 path is closed here.
