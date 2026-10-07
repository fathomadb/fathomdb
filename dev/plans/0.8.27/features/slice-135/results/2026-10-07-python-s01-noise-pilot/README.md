---
title: Slice 135 installed Python S01 baseline-only noise pilot
status: BASELINE_ONLY_NO_CANDIDATE_VERDICT
target_release: 0.8.27
---

# Installed Python S01 baseline-only noise pilot

The [block runner](../../../../../../../scripts/slice135_python_s01_block.py)
ran the same installed 0.8.26 wheel and real-database S01 query workload in
five separate blocks at each of 32 and 256 rows. The exact baseline source is
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`, the wheel SHA-256 is
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`,
and the retained workload runner SHA-256 is
`91574e04c212c27c57cad23aa0b8ddb16b091966d2109b38cca4f9d50734c17b`.
This is a baseline noise estimate, **not** a 0.8.27 speed comparison.

Each block used a fresh real database, wrote two known anchors plus neutral
filler, configured the searchable vector projection, drained it to `ready`,
checked both anchors, then made one session-first call, one excluded warmup and
100 warm timed calls for each of text, vector-bearing and hybrid query shapes.
The timer spans the installed Python method call through materialized results;
corpus setup, model loading and semantic checks are outside that timer. The
session-first call follows an engine reopen but does not imply cold OS or model
caches. Whole-worker GNU Time resources include setup and all calls.

The five blocks at each size were serial, with about 20–50 seconds between
starts. The [independent audit](independent-audit.json), implemented separately
in [the audit script](../../../../../../../scripts/slice135_python_s01_audit.py),
re-read the raw results, checked the seeded IDs and branches, recounted all
3,060 attempts, verified hashes and environment snapshots, and recomputed
nearest-rank p50/p95 from the 100 warm samples per cell. It rejected a
deliberately changed text result ID despite a retained `semantic_ok: true`.
The [hash manifest](SHA256SUMS) covers every retained raw and receipt file.

All ten blocks passed the declared invalidators. Block `256-04` recorded one
page of **host** swap drift and zero **child** swap events; the warning is
retained in both its receipt and the audit. The failed
[launcher attempt](invalid-venv-symlink/attempt.json) is retained separately:
the wrapper had dereferenced the virtual-environment Python symlink, so the
worker could not import the installed package. A failing regression test led
to the launcher fix before the ten accepted blocks.

## Warm-query observations

All values below are milliseconds, `p50/p95` by nearest rank. Each line is one
independent block with 100 warm observations per query shape.

| Rows | Block | Text p50/p95 | Vector p50/p95 | Hybrid p50/p95 |
| --- | --- | ---: | ---: | ---: |
| 32 | 01 | 0.092 / 0.346 | 10.297 / 11.338 | 8.959 / 9.724 |
| 32 | 02 | 0.089 / 0.424 | 10.159 / 10.941 | 8.992 / 9.602 |
| 32 | 03 | 0.082 / 0.326 | 10.197 / 10.980 | 8.978 / 9.665 |
| 32 | 04 | 0.088 / 0.339 | 10.453 / 11.256 | 9.036 / 9.599 |
| 32 | 05 | 0.086 / 0.328 | 10.607 / 11.396 | 8.998 / 9.762 |
| 256 | 01 | 0.086 / 0.339 | 11.523 / 12.562 | 9.796 / 10.583 |
| 256 | 02 | 0.084 / 0.328 | 11.462 / 12.496 | 10.195 / 11.259 |
| 256 | 03 | 0.090 / 0.338 | 11.393 / 12.282 | 10.076 / 10.764 |
| 256 | 04 | 0.087 / 0.336 | 11.130 / 11.911 | 10.361 / 11.384 |
| 256 | 05 | 0.084 / 0.324 | 11.240 / 12.185 | 10.042 / 10.831 |

The range of block p50 values divided by their median was 11.3%, 4.35% and
0.85% for text, vector and hybrid at 32 rows; 6.56%, 3.45% and 5.61% at 256
rows. The corresponding p95 spreads were 28.75%, 4.04% and 1.69% at 32 rows;
4.22%, 5.30% and 7.40% at 256 rows. Thus a single block, especially the
32-row text p95, would be a weak regression claim. The 100-sample pilot does
not qualify p99. Session-first values remain individual diagnostics, without
a cold-cache percentile.

The [frozen S01 paired rule](../../s01-python-comparison-protocol.json) uses
1,000 warm samples per query cell and five alternating version pairs at both
sizes. This pilot supports that feasible sample budget but cannot guarantee
the candidate has the same noise. The protocol pins the rule and runner
identity before candidate timing and requires every pair and its range to be
reported. The broader Phase 1 protocol and full functional exercise gate
remain open.

Example block command, with the same inputs except `--rows` and output path
for each block:

```sh
.venv/bin/python scripts/slice135_python_s01_block.py \
  --checkout /home/coreyt/projects/fathomdb-worktrees/release-0.8.26-slice-135-baseline \
  --source-sha f99e002f0d2e4002f3694c9f8d4986b56089edaa \
  --wheel /tmp/slice135-python-wheel-baseline-f99e002/fathomdb-0.8.26-cp310-abi3-manylinux_2_39_x86_64.whl \
  --wheel-sha256 7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282 \
  --venv-python /tmp/slice135-python-venv-baseline-f99e002/bin/python \
  --rows 32 --samples 100 --output-dir /tmp/slice135-s01-block-32-02
```
