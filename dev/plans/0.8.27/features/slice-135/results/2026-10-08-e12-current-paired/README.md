---
title: Slice 135 current-source E01–E12 paired engine diagnostic
status: PAIRED_DIAGNOSTIC_NOT_PHASE1_CHECKPOINT
target_release: 0.8.27
---

# Current-source E01–E12 paired engine diagnostic — 2026-10-08

The [replacement frozen subset](../../e12-current-comparison-protocol-v2.json)
bound the 0.8.26 baseline `f99e002f0d2e4002f3694c9f8d4986b56089edaa`,
0.8.27 product source `8c2455b6ccf1d06d5bf87fc4278aecc631014da0`,
compatible workload binary hashes, expected outputs, and five alternating
pairs before timing. The candidate includes the edge-explanation and
importance-lookup repairs; the Rust crate tree and `Cargo.lock` are unchanged
between that source and the documentation commits following it. The
[rejected preflight](../2026-10-08-e12-current-invalid-preflight/README.md)
used a baseline binary with the wrong workload hash; it ran no timed block.

The [schedule](schedule.json) records 20 audited blocks separated by at least
20 seconds. Each query cell has 1,000 valid samples per block and supports
nearest-rank p50/p95/p99. Each lifecycle cell has 100 valid samples per block
and supports p50/p95. The [independent paired audit](paired-audit.json)
accepted all twelve cells with zero invalid blocks. It recomputed statistics
from raw attempts and checked source, binary, runner, corpus, model, ordered
outputs, persisted state, sample count, block order and resources. A second
audit of the copied archive produced byte-identical JSON. The
[negative controls](negative-controls.json) rejected an altered schedule order
and an altered ordered text result.

These are medians of five within-pair percentage changes. Positive means the
candidate was slower. The audit retains each pair value and observed range.

| Cell | p50 change | p95 change | p99 change |
| --- | ---: | ---: | ---: |
| Text | −0.29% | +1.97% | +0.32% |
| Vector stage | +9.62% | +8.83% | +12.56% |
| Hybrid | +9.94% | +9.29% | +10.18% |
| Graph expansion | +0.94% | +0.51% | +2.76% |
| Graph evidence | −0.90% | +4.48% | +20.97% |
| Fresh open | +2.59% | +0.82% | Unsupported |
| Populated open | +11.96% | +11.02% | Unsupported |
| Close | +4.12% | +4.94% | Unsupported |
| Canonical write | +1.28% | +0.61% | Unsupported |
| Projection | +1.20% | +0.03% | Unsupported |
| CPU model call | −0.90% | −0.74% | Unsupported |
| Erasure | +1.79% | +1.48% | Unsupported |

Vector-stage, hybrid and populated-open p50 increased in all five pairs.
Their p50 ranges were +4.03% to +11.43%, +5.85% to +11.00%, and +11.56% to
+13.45%, respectively. Four blocks had host-only swap-counter drift of one
to three pages, with zero measured-child swap events. In warning-free pairs,
the corresponding p50 median changes were +9.61% (four pairs), +10.11%
(four pairs), and +11.93% (three pairs). Graph-evidence p99 has a wide
−3.12% to +30.54% range; close, projection and erasure also have wide paired
ranges. Keep all valid slow samples and warnings in the audit.

The raw 440 MiB campaign is retained locally in this result directory until
the end-of-phase archive decision. Its [schedule](schedule.json) SHA-256 is
`83d83a6773e24f74c6f00ffe24799a4dc191e1ff4abf2bd081638f0a260139a5`;
the [paired audit](paired-audit.json) SHA-256 is
`16cdab0c3d15f1c11e2d5df8bdc0ef30f010b277009f30caa7fc33530f83b39a`.
The full raw archive is not yet published branch evidence.

This is a fixed 32-row CPU engine workload, not an installed-SDK or
competitor comparison. The consistent vector, hybrid and populated-open
leads require call-boundary attribution before a release-wide latency claim.
It does not close the broader Phase 1 protocol or checkpoint.
