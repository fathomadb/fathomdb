---
title: Slice 135 installed Python S02 integrated-candidate paired timing
---

# Installed Python S02 integrated-candidate paired timing

The [frozen integrated S02 subset](../../s02-python-integrated-comparison-protocol.json)
keeps the earlier whole-sequence workload and five alternating pairs, changing
only the candidate identity to the post-Slice-132 source
`cdf253cd223a82e954591db532397a3d78a2027a` and its installed wheel
SHA-256 `c33987023754fee2d85f887bfbbd17c68889f287603d3df06844d06a240a60a0`.
That source includes the embedder-close and Tegra install-route landings.
The baseline remains source `f99e002f0d2e4002f3694c9f8d4986b56089edaa`
and wheel SHA-256
`7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`.
The candidate source was checked out cleanly at its exact commit for the run.
Package version strings are not used as source identity.

Each of ten blocks used one warmup and 20 measured fresh installed-Python
processes and real SQLite databases. The client timer covers default-model
open, governed write, projection readiness, text/vector/hybrid and
graph/evidence retrieval, erasure, close, reopen and reopened close through
materialized output. Independent SQLite/state verification follows the timer.
All 100 measured samples per version passed. The
[paired audit](independent-audit.json) independently recomputed artifact and
runner identity, semantic and reopened state, raw sample hashes, child
resources and latency distributions. The [order audit](order-audit.json)
recomputed that result and checked the actual version order, idle gaps,
commands and summary hashes. Its output was byte-identical after this receipt
was relocated. The paired auditor rejected a surviving-erased-source negative
control with the expected semantic reason.

| Whole sequence | 0.8.26 baseline | Integrated 0.8.27 candidate | Observed change |
| --- | ---: | ---: | ---: |
| p50, 100 samples | 5,414.516 ms | 5,441.326 ms | +0.495% |
| p95, 100 samples | 5,510.922 ms | 5,561.827 ms | +0.924% |
| Observed min–max | 5,331.002–5,655.674 ms | 5,340.561–5,967.300 ms | Descriptive |
| Child peak RSS range | 447,124–449,816 KiB | 313,012–315,512 KiB | Lower on candidate |

Pair p50 deltas were −0.032%, +0.920%, +0.518%, +0.092% and +0.369%; their
median is +0.369% and range is −0.032% to +0.920%. Pair p95 deltas span
−0.930% to +2.911%. All five pairs contain host swap-counter movement, so
there is no warning-free pair for a sensitivity comparison. Measured child
swap events were zero; one candidate child had a major fault. These warnings
and the five-pair sample size prevent a significance or equivalence claim.
The pooled system result is **slower as observed**, but its release
disposition remains open. It does not establish that 0.8.27 is at least as
fast as 0.8.26.

The [stage diagnostic](stage-diagnostic.json) shows a candidate median increase
of 31.3 ms in reopen, 7.7 ms in close and 7.3 ms in reopened close, while drain
decreased 7.7 ms. Stage percentiles are marginal and do not add to the
whole-sequence percentile; this is a lead for investigation, not a causal
decomposition. The separate [S02-L lifecycle comparison](../2026-10-07-python-s02-lifecycle-paired-current/README.md)
measured close p50 +141.2% with intended memory release at close. Together,
these results require a system-level performance and robustness disposition.

This installed Python S02 subset does not close contention, TypeScript/Rust
S02, S01/S03/C01, Pareto coverage, robustness or logic/exception matrices, or
the broader Phase 1 protocol. p99 is unsupported by the frozen sample count.
