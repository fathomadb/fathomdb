---
title: FathomDB 0.8.27 release status
status: ACTIVE
---

# FathomDB 0.8.27 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.27.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.27:status-current-state -->**Next is Slice 40 (ENGINE-FOUNDATION), PLANNED.** Completed on local `release/0.8.27` per release state: 0 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 1 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 2 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 3 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 4 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 5 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 6 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 7 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 8 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 9 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 10 (`3097d191511d81a221b038ccd2e14f074dcafa6d`) · 20 (`3943cb64dc2d1b99ef9fc4ec2131dca59a71b337`) · 30 (`58bc8eb4836f6f0f52b40b21befaea87a490b9fa`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.27:status-current-state -->

Prework Slices 0-9, bounded preparation Slice 10, correction-safe erasure
Slice 20, and real-surface comparator Slice 30 are complete on
`release/0.8.27`. Slice 30 adds repository tooling and evidence only: no
runtime behavior, schema, public API, tag, registry, or publication state.

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.27:status-next-action -->**Commission Slice 40 (ENGINE-FOUNDATION)** — engine foundation and test seams. **Remaining ladder:** 40 → 50 → 60 → 70 → 80 → 90 → 100 → 110 → 120 → 130 → 140 → 150.<!-- END GENERATED release-state:0.8.27:status-next-action --> |

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
| 30 | Current inventory and comparison guardrails | Complete at `b102bceb`; deterministic 12-row real-surface baseline, mutation-tested comparator, independent code rereview, and independent reverification pass. |

## Verification boundary

Slice 30 records committed RED/GREEN chronology, independent design-review
PASS, final independent code-review PASS after closing three P1 and three P2
findings, and independent reverification PASS at `b102bceb`. Two Node 25.9.0
captures were byte-identical; a later source SHA compared equal with no surface
diff. Comparator hardening, SDK parity 10/10, Rust surfaces 4/4 default and 5/5
operator, Python surfaces 20/20, and TypeScript surfaces 20/20 passed. The
canonical gate's lint/typechecks, strict security 0/0/0, Rust, TypeScript, and
118 registered harness suites passed; its Python outcome and exact environment
qualification limits remain recorded in the slice status. Slice 150 retains
fresh installed-package and platform qualification.

## Boundaries

- This release starts at schema 34 and currently proposes no schema migration.
- Publication is unauthorized.
- Slice 30 was directly authorized by the repository owner and is complete;
  Slice 40 and later slices require separate commission.
- No temporary branch or worktree was created for prework; the existing
  `release/0.8.27` worktree remains the active release workspace.
