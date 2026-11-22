---
title: Inspector engineering documentation
date: 2026-11-21
product: FathomDB Inspector
target_release: 0.2.1
desc: Entry map and ownership for the Inspector product
status: ACTIVE
---

# Inspector engineering documentation

Inspector is a FathomDB-native, local browser GUI for canonical records,
FTS/BM25, vectors, and graphs. Documentation and operation support user
understanding; the tool presents facts and coordinated controls.

## Layout and current work

| Path | Purpose |
| --- | --- |
| [needs.md](needs.md) | User outcomes and research |
| [requirements.md](requirements.md) | INS requirements and carried Need/Should scope |
| [acceptance.md](acceptance.md) | Stable criteria, fixtures, and action signals |
| [design/high-level-design.md](design/high-level-design.md) | Current architecture, contracts and historical capability inventory |
| [plans/plan-0.1.md](plans/plan-0.1.md) | Legacy allocation and historical planning baseline |
| [plans/0.2.0/scope.md](plans/0.2.0/scope.md) | Owner-adopted 0.2.0 release scope and carried obligations |
| [plans/0.2.0/README.md](plans/0.2.0/README.md) | Completed 0.2.0 ladder, release plan, workflow and all slice plan/design links |
| [plans/0.2.1/README.md](plans/0.2.1/README.md) | Inspector 0.2.1 planning index for FathomDB 0.8.30 support |
| [plans/0.2.1/scope.md](plans/0.2.1/scope.md) | Adopted 0.2.1 scope and ruled owner decisions (D1–D4) |
| [plans/0.2.1/plan.md](plans/0.2.1/plan.md) | Active four-slice 0.2.1 delivery plan (Slices 10, 20, 30, 40) |
| [plans/0.1/](plans/0.1/README.md) | Legacy slice paths; unfinished delivery carried to 0.2.0 |
| [Shared verification plan](plans/0.1/verification-plan.md) | Slice 15 fixture/oracle/owner baseline; planned tests, not runtime passes |
| [ROADMAP.md](ROADMAP.md) | Product roadmap and work outside 0.2.0 |
| [BACKLOG.md](BACKLOG.md) | Canonical Inspector worklist, survey examples and unscheduled opportunities |
| [research/compatibility-architectures.md](research/compatibility-architectures.md) | Real compatibility architectures and source examples |
| [research/inspector-tool-survey-and-parity.md](research/inspector-tool-survey-and-parity.md) | Official GUI survey, current/planned task coverage and interaction recommendations |
| [research/compatibility-evolution-stress-test.md](research/compatibility-evolution-stress-test.md) | Stress test against hypothetical database 0.9/1.0 changes |
| [reviews/](reviews/README.md) | Independent reviews and prioritization |
| [0.2.0 full assessment](reviews/assessment-0.2.0.md) | Source- and evidence-grounded assessment of released Inspector 0.2.0 |
| [0.8.30 compatibility assessment](reviews/compatibility-0.8.30-assessment.md) | Required integration and qualification work; no port performed |
| [Public documentation](../../docs/inspector/index.md) | Product-facing concepts, views, limits, and roadmap |
| [Source home](../../src/inspector/README.md) | Inspector application source boundary |

This mirrors FathomDB's engineering conventions: needs, requirements,
acceptance, subsystem design, release plans, and roadmap. Public usage prose
belongs under `docs/inspector/`; application code belongs under `src/inspector/`.

Inspector versions are independent of database release numbering. Inspector
`0.2.1` upgrades the locally released `0.2.0` product onto delivered FathomDB
`0.8.30` (`release/0.8.30`, `72fe93634321616214352a7e07066ce0f93bcf4c`, schema
37) and native Ubuntu 24.04 (`glibc 2.39`) delivery, qualified against closed
databases from both published FathomDB `0.8.29` and published FathomDB `0.8.30`.
The [0.2.1 delivery plan](plans/0.2.1/README.md) records completed Slices 10–40
and atomic local promotion to ignored `dist/inspector/0.2.1/` (`dist/inspector/0.2.0/`
is preserved separately). The [owner ruling](plans/0.2.0/release-decisions.md)
(`INS-020-LOCAL-RELEASE`, carried forward by 0.2.1 D2 = A) keeps delivery
local-only without a five-person UX study. Public registry publication and
broader platforms remain future work. See
[release instructions](../../src/inspector/tools/README.md#local-release-readiness).

Engine startup is permitted when required for the adopted scope. The
[attachment/effect contract](requirements.md#engine-startup-allowance-2026-10-29)
distinguishes Inspector inspection operations from normal Engine lifecycle and
worker effects; immutable byte preservation is not the baseline guarantee.
