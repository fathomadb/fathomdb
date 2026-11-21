---
title: Inspector engineering documentation
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.2.0
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

Inspector versions are independent of database release numbering. Inspector 0.2.0
implements all record, lexical, vector, graph/history and two-observation views
on selected Engine commit `6edc1155f039f87cd0a37660cb27ae4373bbce71`, schema 37.
The [current ladder](plans/0.2.0/README.md) records completed20–60,70 technical
qualification and75 UX remediation. [80 readiness](plans/0.2.0/slice-80/plan.md)
completed docs/local CI/package methods;[90](plans/0.2.0/slice-90/status.md)
released locally to ignored `dist/inspector/0.2.0/`.
The [owner ruling](plans/0.2.0/release-decisions.md) removes the unperformed
five-person study. Public registry publication and broader platforms are future
work. See [release instructions](../../src/inspector/tools/README.md#local-release-readiness).

Engine startup is permitted when required for the adopted scope. The
[attachment/effect contract](requirements.md#engine-startup-allowance-2026-10-29)
distinguishes Inspector inspection operations from normal Engine lifecycle and
worker effects; immutable byte preservation is not the baseline guarantee.
