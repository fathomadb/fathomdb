---
title: Slice 135 exact-candidate installed TypeScript S01 paired comparison
status: AUDITED_SUBSET_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Installed TypeScript S01 paired comparison — 2026-10-08

The [frozen S01 protocol](../../s01-ts-224e-comparison-protocol.json), SHA-256
`18818cf596a3747806c9b316eeecf80fb31cac1913f975a039a7100d8fb2896c`,
preceded candidate timing. It binds baseline source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` and exact candidate source
`224e44c593c13d86ece648adabe445723db04070`, rebuilt npm archive bytes,
the CPU default model, 32/256-row corpora, 1,000 warm materialized calls per
text/vector/hybrid cell and five alternating pairs per size. This is an
installed TypeScript async-call boundary, excluding setup and model load.

The [independent audit](independent-audit.json) accepted all **20/20 blocks**
and **60,120/60,120** materialized call attempts. It rechecked source and
archive hashes, JS/native archive members, raw semantics, nearest-rank
percentiles, resources, block order and the minimum idle gap. The
[negative controls](negative-controls.json) rejected changed order, protocol
identity, block validity and a wrong text ID after its enclosing receipt
hashes were recomputed. Protocol, audit, negative-control and run-order
SHA-256 values are respectively `18818cf596a3747806c9b316eeecf80fb31cac1913f975a039a7100d8fb2896c`,
`66563aaf52a0d7fb431f443d3e07c3fed7ed1c29f0a7283ba56196e3e5d305ec`,
`2426f1e6b4eb6a7c90302eb27edc3f1cee1dfbc94108807af03794957d719eaa`
and `8d56d3526b47ea2f3c397e74958cc0788811f96332cd787856ee31a62b4c804f`.

The table reports the median of five within-pair percentile deltas; positive
means slower candidate calls. The audit retains all five deltas and ranges.

| Rows | Cell | p50 | p95 | p99 | Warning-free pairs |
| --- | --- | ---: | ---: | ---: | ---: |
| 32 | Text | +3.6285% | +1.2342% | +2.7767% | 4/5 |
| 32 | Vector | +2.3715% | +4.8731% | +9.7323% | 4/5 |
| 32 | Hybrid | +0.0504% | +1.4726% | −1.2049% | 4/5 |
| 256 | Text | +0.6701% | −2.2116% | −7.1157% | 2/5 |
| 256 | Vector | −0.7761% | −3.4480% | −4.5050% | 2/5 |
| 256 | Hybrid | +0.7992% | +3.5650% | +5.9862% | 2/5 |

At 32 rows, vector p50 increased in all five pairs (+0.1049% to +4.4323%).
At 256 rows it decreased in all five pairs (−2.3320% to −0.2977%), but
only two pairs are warning-free. Six blocks had host-only swap-counter drift;
no measured child swapped. The 256-row warning-free sample is too narrow for
a separate conclusion. These are descriptive workload-specific results, not
an equivalence claim or release verdict. The engine-only E01–E12 vector-stage
and hybrid increases use different work and timing boundaries.

The [copied raw archive](raw-archive/) retains each command, raw call,
environment and resource record, npm archives, runner bytes and run order.
Its 271-file `SHA256SUMS` manifest has SHA-256
`e2e797430b0db39fb758ecae9a855ca503d423c29383f3af936aacd00d74f044`;
all copied bytes were verified. The raw archive remains untracked pending
end-of-phase retention; the audit and negative controls are tracked.
This subset does not close S02, the broader operation mix, robustness or the
four-area Phase 1 checkpoint.
