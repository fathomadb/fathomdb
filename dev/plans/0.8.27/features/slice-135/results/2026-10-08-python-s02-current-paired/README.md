---
title: Slice 135 current-wheel installed Python S02 paired diagnostic
status: AUDITED_HISTORICAL_AFTER_HYDRATION_REPAIR
target_release: 0.8.27
---

# Current-wheel installed Python S02 paired diagnostic — 2026-10-08

The [frozen protocol](../../s02-python-current-comparison-protocol.json)
binds baseline source `f99e002f0d2e4002f3694c9f8d4986b56089edaa` and
wheel `7c11803d9629550be60a76e1549c4c0bb42d23978d9fc901cf5df3c463611282`
to candidate source `d465cd56d2e863f900ea9a9da8bc372ca6a077c5` and
wheel `33d37be7c80183b063788889954da698ea959997fc13b61e356006a3c26c8894`.
It retains the controlled baseline-only noise pilot, the established S02
open/write/project/retrieve/graph/evidence/erase/close/reopen sequence, 20
measured fresh-process sequences per block and five alternating pairs. The
timing boundary ends at the materialized reopened close; independent SQLite
verification is outside it.

All ten timed blocks completed. The [independent paired audit](independent-paired-audit.json)
accepted 100 measured sequences per version, recomputed raw state, percentiles
and resources, and rejected a wrong reopened-state control. The
[independent order audit](independent-order-audit.json) checked exact protocol
and runner bytes, schedule, idle intervals and command identities. Both audits
on the copied local archive were byte-identical to the original audits.
Additional [negative controls](negative-controls.json) rejected a reordered
block and a changed reopened-state assertion.

Whole-sequence pooled p50 was 5,319.796 ms on baseline and 5,326.660 ms on
candidate (+0.129%). Pooled p95 was 5,374.291 ms and 5,390.417 ms
(+0.300%). The median of five within-pair p50 deltas was +0.239%, ranging
from -0.053% to +0.247%. Four pairs have host-only paging warnings; their
measured children have zero swap events and zero major faults. The one
warning-free pair has p50 +0.240%. Peak child RSS ranged 446,800–448,652 KiB
on baseline and 313,536–316,336 KiB on candidate. These observations are
diagnostic; five pairs and one clean pair do not establish equivalence or a
release-wide latency claim.

The subsequent [vector-hydration row-error repair](../2026-10-08-vector-hydration-repair/README.md)
changes engine source after this installed wheel was built. This complete
paired run remains evidence for its exact `d465cd56d` source and wheel, and
the affected installed S02 cell must be refreshed before a final-candidate
checkpoint claim. The raw archive is local and untracked pending end-of-phase
retention.
