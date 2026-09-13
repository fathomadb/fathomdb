---
title: FathomDB 0.8.26 feature slices
status: DRAFT
---

# FathomDB 0.8.26 feature slices

Feature slices begin only after Slice 9 closes. They are dependency-linear and
numbered by tens so evidence or repair slices can be inserted without
renumbering the release.

| Slice | Capability | Priority |
| ---: | --- | --- |
| 10 | Frozen explanation repair, retrieval guide, installed-artifact witness | P0 |
| 15 | Graph-evidence performance and erasure-linearization spike | P1 feasibility |
| 20 | Exact graph-target and terminal-edge evidence by immutable revision | P1 |
| 30 | Distributable read-only operator integrity inspection | P1 |
| 35 | Breaking changed-in-place V1 actuation, fresh-database, receipt, and performance spike | P2 feasibility |
| 40 | V1 atomic derived-edge actuation and fresh-database boundary | P2 |
| 50 | Integrated package, platform, cross-SDK, and release verification | P0–P2 |

Each implementation slice and approved spike contains a draft execution plan
and draft design note. Slice 8 may narrow or postpone them; it must not
silently broaden them. Slice 15 and Slice 35 were inserted by Slice 8 evidence
after the initial mod-10 ladder was drafted. Slice 35 has a draft plan/design
applying ruled D26-03 through D26-05. Slice 15 has an authorized
decision-support plan/design; D26-01 remains open until its results return to
HITL.
