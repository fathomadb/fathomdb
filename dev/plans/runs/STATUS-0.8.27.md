---
title: FathomDB 0.8.27 release status
status: ACTIVE
---

# FathomDB 0.8.27 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.27.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.27:status-current-state -->**Next is Slice 30 (COMPARATOR), PLANNED.** Completed on local `release/0.8.27` per release state: 0 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 1 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 2 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 3 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 4 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 5 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 6 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 7 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 8 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 9 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 10 (`3097d191511d81a221b038ccd2e14f074dcafa6d`) · 20 (`3943cb64dc2d1b99ef9fc4ec2131dca59a71b337`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.27:status-current-state -->

Prework Slices 0-9, bounded preparation Slice 10, and correction-safe erasure
Slice 20 are complete on `release/0.8.27`. Slice 20 changes product behavior
only by restoring the existing erasure contract after correction; it adds no
schema, public API, report field, tag, registry, or publication state.

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.27:status-next-action -->**Commission Slice 30 (COMPARATOR)** — current inventory and real-surface comparison guardrails. **Remaining ladder:** 30 → 40 → 50 → 60 → 70 → 80 → 90 → 100 → 110 → 120 → 130 → 140 → 150.<!-- END GENERATED release-state:0.8.27:status-next-action --> |

## Open decisions

There is <!-- BEGIN GENERATED release-state:0.8.27:status-live-open-count -->ONE<!-- END GENERATED release-state:0.8.27:status-live-open-count --> live open decision:

- authorize tagging/publication only after the complete ladder and release
  qualification pass.

## Completed release-branch ladder

| Slice | Scope | Status and evidence |
| ---: | --- | --- |
| 0 | Environment and infrastructure | Complete at `a3e6cff6`; existing worktree/base and local/unavailable capabilities recorded. |
| 1 | Dependencies and pins | Complete at `a3e6cff6`; updates retained, postponed, or allocated with evidence limitations explicit. |
| 2 | Repository/documentation cruft | Complete at `a3e6cff6`; disposition inventory only, no deletion. |
| 3 | Needs, requirements, acceptance, allocation | Complete at `a3e6cff6`; release-local contracts, global acceptance unchanged. |
| 4 | Architecture/code alignment | Complete at `a3e6cff6`; code-grounded module and ownership map. |
| 5 | Verification adequacy | Complete at `a3e6cff6`; deep existing evidence and missing non-vacuous oracles allocated. |
| 6 | Stale-documentation evidence | Complete at `a3e6cff6`; exact current/historical correction set allocated. |
| 7 | Build and delivery evidence | Complete at `a3e6cff6`; open recurring causes separated from closed regressions. |
| 8 | Reserved | Complete at `a3e6cff6`; intentionally empty. |
| 9 | Proposal review and closeout | Planned at `a3e6cff6`, closed at `73c53ffd`; every proposal ruled, Slice 10 inputs design-reviewed, and verification passed. |
| 10 | Bounded repository preparation | Complete at `3097d191`; current truth, dependency security, Action comments, digest authority, and Slice 30 prerequisites pass review. |
| 20 | Correction-safe source erasure | Complete at `3943cb64`; exact atomicity, at-rest, binding, and unchanged-fixture Memex evidence pass independent review and verification. |

## Verification boundary

Slice 20 records committed RED/GREEN chronology, independent design-review
PASS, final independent code-review PASS after one evidence-repair cycle, 52
focused Rust tests, 8 Python tests, 8 TypeScript tests, and the frozen Memex
consumer oracle. At exact candidate `3943cb64`, the canonical gate passed all
119 registered suites with no skips or exclusions, strict security 0/0/0,
full-workspace Clippy with warnings denied, and full-workspace Cargo check.
Slice 30 retains comparator requalification and baseline capture; Slice 150
retains package/platform qualification.

## Boundaries

- This release starts at schema 34 and currently proposes no schema migration.
- Publication is unauthorized.
- Slice 20 was authorized at `seq-292` and is complete; Slice 30 and later
  slices require separate commission.
- No temporary branch or worktree was created for prework; the existing
  `release/0.8.27` worktree remains the active release workspace.
