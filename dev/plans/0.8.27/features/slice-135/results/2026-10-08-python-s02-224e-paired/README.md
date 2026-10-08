---
title: Slice 135 installed Python S02 paired comparison at source 224e44c59
status: AUDITED_DIAGNOSTIC_IN_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Installed Python S02 paired refresh at source 224e44c59

The [frozen protocol](../../s02-python-224e-comparison-protocol.json), SHA-256
`095195b28220a07b92b0317c917b1848575b3fb6e5e65a521b6739a04e8c682e`,
preceded candidate timing. It preserves the qualified S02 whole-sequence
fixture, timing boundary, 20 measured sequences per block and five alternating
pairs. The baseline is exact 0.8.26 source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`; the rebuilt installed
candidate wheel is from `224e44c593c13d86ece648adabe445723db04070`,
with SHA-256 `ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a`.

All ten blocks completed with 100 valid fresh-process whole sequences per
version. The [independent paired audit](independent-audit.json) recomputed
materialized results, reopened SQLite state, resources and percentiles from
raw samples. The [independent order audit](order-audit.json) checked protocol
bytes, commands, alternation and idle intervals. The [negative
controls](negative-controls.json) rejected a visible erased source after
reopen and a reordered block; the paired auditor also rejected its own
tampered reopened-state fixture. No measured child swapped or incurred a
major fault. Three pairs have host-only swap-counter warnings; two are
warning-free.

| Whole-sequence measure | 0.8.26 baseline | 0.8.27 candidate | Change |
| --- | ---: | ---: | ---: |
| Pooled p50 | 5,304.311 ms | 5,319.525 ms | +0.287% |
| Pooled p95 | 5,364.560 ms | 5,375.594 ms | +0.206% |

The median five-pair p50 change is **+0.329%**, with observed range
+0.067% to +0.639%. The two warning-free pair-p50 changes are +0.067% and
+0.419%. Median stage times show close increasing by 7.717 ms,
reopened-close by 7.546 ms and reopened open by 19.563 ms. These stage
medians are attribution leads, not additive parts of the pooled p50 change.
Candidate peak RSS spans 312,992–316,052 KiB versus 446,624–449,464 KiB
on baseline. The memory release at close is an intended lifecycle change;
its latency/resource tradeoff remains a checkpoint disposition. Five pairs
do not support an equivalence or statistical significance claim.

The copied raw archive (`raw-archive/`) is untracked; its verified local
bundle and copied manifest are recorded in the
[retention receipt](../2026-10-08-raw-retention-review/README.md).
Its 1,167-file `SHA256SUMS` manifest has SHA-256
`8c3a4a2cb2395560465fee43d2de675816297c648286c98ec5f15398d3a8d5e4`;
the copied archive passed `sha256sum -c`. The paired and order audit hashes
are `13d50229a7f3a20dd58d9da1bf18a399423b52e74ea11e43f61c4098d879d4c3`
and `8ea5b7ade9236e6aa6efc8c76442ee51fbf671b4b41b479e39aef58bb718fbf3`.
This Python S02 result is an exact-source diagnostic incorporated into the
[four-area Phase 1 checkpoint](../../phase1-checkpoint-2026-10-08.md).
Other SDK and workload boundaries are dispositioned there.
