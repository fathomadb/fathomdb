---
title: Slice 135 E01–E12 adapter baseline noise pilot
status: BASELINE_ONLY_PILOT_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# E01–E12 executable adapter and baseline noise pilot

The [adapter](../../../../../../../scripts/slice135_e12_adapter.py),
[workload](../../../../../../../scripts/slice135_e12_workload.rs) and
[independent audit](../../../../../../../scripts/slice135_e12_audit.py) replace
the inherited seven-sample count-only feasibility with exact state and output
checks. The [expected checks](../../e12-expected-checks.json) were pinned
before timing. The real-database workload binds the exact source SHA, Cargo
lock, binary, runner bundle, corpus and CPU model assets. Each retained raw
attempt includes ordered query IDs and branches or lifecycle/graph/evidence/
erasure state. Environment start/end snapshots and the GNU Time child report
are retained. The independent audit recomputes nearest-rank percentiles and
rejects source, artifact, state, sample-count and output mutations.

The exact 0.8.26 source is `f99e002f0d2e4002f3694c9f8d4986b56089edaa`.
Five separate baseline blocks used the same binary SHA-256
`2c819cb22285a88ad444b128e009cab50af0801351c0534652135970f6e9e7b1`.
Each block had one untimed warm-up plus 100 valid text and 100 valid
close/reopen observations. The blocks were serial, unprofiled and used a
32-row seed on the same host. All five independent audits passed with zero
invalid attempts. Block 2 saw two pages of host swap-counter drift; the
measured child had zero swap events, so this is a warning under the draft
protocol's host-only rule. Other blocks had no host swap drift.

| Block | Elapsed s | Text p50 ns | Text p95 ns | Close p50 ns | Close p95 ns |
| --- | ---: | ---: | ---: | ---: | ---: |
| 1 | 7.09 | 208,505 | 257,788 | 4,104,237 | 4,602,099 |
| 2 | 7.04 | 203,826 | 264,911 | 4,116,690 | 4,488,675 |
| 3 | 7.03 | 209,937 | 262,096 | 4,126,068 | 4,501,017 |
| 4 | 7.05 | 194,809 | 256,636 | 4,160,834 | 4,536,154 |
| 5 | 7.11 | 224,976 | 261,666 | 4,115,689 | 4,496,910 |

Across blocks, text p50 spread is 14.47% of the median and text p95 spread
is 3.16%; close p50 and p95 spreads are 1.37% and 2.52%. The text p50
sequence fluctuates rather than drifting monotonically. The pilot's 100
samples per block support p50/p95, not p99. A proposed final E01–E12
comparison protocol would use at least 1,000 valid samples per query block,
at least 100 per lifecycle block, five or more alternating source-version
pairs, nearest-rank p50/p95/p99 only where supported, and descriptive paired
deltas with the per-pair range. Freeze that protocol before examining current
candidate timing; this pilot alone does not freeze it.

[Baseline full-path smoke](baseline-all/) passed all twelve cells at 100
observations each, as did a [candidate functional run](candidate-all/) on
`d22fa9eefa0e75e9acdb9cb3d8a7bbdd13bdbca0`. The latter source was
superseded by a subsequent engine fix, so neither full-path run is a qualified
candidate latency comparison. Both raw files and independent audits are
retained to demonstrate executable state and result checks.

Each `block-N/` folder retains `raw.json`, `protocol.json`, `summary.json`,
`audit.json`, build provenance, the command and child logs. `shared/`
contains the exact compressed binaries, runner bundles, corpus and build
manifest snapshots/locks, plus the exact adapter/workload source bytes named by each
runner bundle. The current workload was subsequently formatted; its measured
bytes are retained in `shared/workload.rs`. The `binary.gz`, `runner` and `corpus` links in each block
resolve into `shared/`. The model bytes remain in the pinned local model
cache; each protocol hashes the three asset files. To re-audit a block, run:

```sh
python3 scripts/slice135_e12_audit.py \
  --receipt dev/plans/0.8.27/features/slice-135/results/2026-10-07-e12-adapter-pilot/block-1 \
  --checkout /home/coreyt/projects/fathomdb-worktrees/release-0.8.26-slice-135-baseline \
  --expectations dev/plans/0.8.27/features/slice-135/e12-expected-checks.json \
  --model-dir /home/coreyt/.cache/huggingface/hub/models--BAAI--bge-small-en-v1.5/snapshots/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a \
  --output /tmp/slice135-e12-audit.json
```

The candidate receipt also needs `--source-ref d22fa9eefa0e75e9acdb9cb3d8a7bbdd13bdbca0`
when auditing from a later checkout with unchanged engine source. The model
path and baseline checkout must be supplied on another host. Neither this
pilot nor the one-block full-path runs close the four-area Phase 1 checkpoint.
