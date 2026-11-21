---
title: Inspector 0.2.0 planning index
date: 2026-11-19
status: DRAFT
target_release: 0.2.0
desc: Adopted scope and proposed prework and feature delivery ladder
---

# Inspector 0.2.0

## Current execution

Slices0–7,9,10,20–60 and75 are complete;8 remains reserved.70 technical
qualification is retained. The owner explicitly removed the five-person study
requirement in [INS-020-LOCAL-RELEASE](release-decisions.md) (Inspector ledger
seq-1); no sessions occurred and none is reported as passed.80 reconciled current
requirements/docs and70 disposition without reopening this ruling.

[80 readiness is COMPLETE](slice-80/status.md): current docs, local CI,
compatible candidate and safe promotion are tested and independently verified.
[90 local release](slice-90/status.md) created the final local distribution at
`dist/inspector/0.2.0/`; no registry publication occurred.

The owner commissioned [80 release readiness](slice-80/plan.md), then
[90 local release](slice-90/plan.md).80 establishes tested documentation, local CI,
packaging and distribution methods;90 releases exact qualified Inspector0.2.0
artifacts into ignored `dist/inspector/0.2.0/`. No registry upload, release tag,
remote push or Engine publication is authorized. The existing dedicated branch/
worktree `inspector/dev/0.2.0` is reused sequentially.90 followed80's verified COMPLETE handoff and material review remediation.

## Original planning baseline — before the later execution commission

The [scope](scope.md) is adopted. The [release plan](plan.md),
[shared execution workflow](slice-workflow.md) and all plans/designs below are
DRAFT. Writing these documents performs none of the planned slices. No release
checkout, environment change, upgrade, cleanup or product implementation is
claimed by this planning work. Historical port evidence keeps its original limits.
The [planning review](planning-review.md) records independent draft review,
provenance, scoped checks and unresolved execution prerequisites.

Each slice has its own Sol-high-authored draft plan and high-level design.
Future execution assigns one separate direct agent per slice, with direct
Sol-high design/code review and Terra verification as applicable. The filled
execution prompt appears in each plan. Prework's no-action boundaries, the
reserved placeholder and actual Slice 9 human gate override its generic wording.

| Slice | Scope | Drafts |
| --- | --- | --- |
| 0 | Environment, project infrastructure and release-checkout decisions | [Plan](slice-0/plan.md) · [Design](slice-0/design.md) |
| 1 | Library, Dependabot and pin review | [Plan](slice-1/plan.md) · [Design](slice-1/design.md) |
| 2 | Repo-wide cruft disposition proposals | [Plan](slice-2/plan.md) · [Design](slice-2/design.md) |
| 3 | Local draft needs, requirements, AC and architecture allocation | [Plan](slice-3/plan.md) · [Design](slice-3/design.md) |
| 4 | Architecture and high-level code alignment review | [Plan](slice-4/plan.md) · [Design](slice-4/design.md) |
| 5 | Test adequacy, traceability and critical paths | [Plan](slice-5/plan.md) · [Design](slice-5/design.md) |
| 6 | Stale user/developer/contributor documentation evidence | [Plan](slice-6/plan.md) · [Design](slice-6/design.md) |
| 7 | Build, preflight, verification and CI/CD failure evidence | [Plan](slice-7/plan.md) · [Design](slice-7/design.md) |
| 8 | Reserved placeholder | [Plan](slice-8/plan.md) · [Design](slice-8/design.md) |
| 9 | Consolidation, scoring, interactive human decisions and reviewed Slice 10 | [Plan](slice-9/plan.md) · [Design](slice-9/design.md) |
| 10 | Conditional implementation of the individually approved prework set | [Plan](slice-10/plan.md) · [Design](slice-10/design.md) |
| 20 | Coordinated records and representation navigation | [Plan](slice-20/plan.md) · [Design](slice-20/design.md) |
| 30 | Indexed text and FTS/BM25 evidence | [Plan](slice-30/plan.md) · [Design](slice-30/design.md) |
| 40 | Vectors, cached CPU semantic/hybrid retrieval and Find similar | [Plan](slice-40/plan.md) · [Design](slice-40/design.md) |
| 50 | Interactive stored graph, provenance, dependencies and history | [Plan](slice-50/plan.md) · [Design](slice-50/design.md) |
| 60 | Two bounded observations and coordinated person workflow | [Plan](slice-60/plan.md) · [Design](slice-60/design.md) |
| 70 | Installed technical qualification; study removed by owner decision | [Plan](slice-70/plan.md) · [Design](slice-70/design.md) |
| 75 | Seven expert UX findings and updated installed candidate | [Plan](slice-75/plan.md) · [Design](slice-75/design.md) |
| 80 | Documentation, locally tested CI, package/distribution readiness | [Plan](slice-80/plan.md) · [Design](slice-80/design.md) |
| 90 | Perform local0.2.0 release to ignored dist | [Plan](slice-90/plan.md) · [Design](slice-90/design.md) |

Feature slices begin at 20; 10 is reserved for the eventual Slice 9 decisions.
The [single existing verification matrix](../0.1/verification-plan.md) remains
canonical; the master plan maps historical owners to this current ladder.
Each feature must be complete locally; 70 adds installed and integrated proof.

Proposed execution branch: `inspector/dev/0.2.0`, based on the approved final
HEAD of `llm/0.8.29-inspector-additional`, preserving the unchanged selected
0.8.29 Engine ancestry and all Inspector port/scope/planning commits. The
[owner decision](slice-0/checkout-decision.md) requires future Slice 0 to create
and enter that release branch/worktree. Reports for 0–9 then use it sequentially.
Other environment/infrastructure changes remain proposals for actual 9 disposition
and conditional 10 implementation. No branch/worktree creation is performed now.
