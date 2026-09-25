---
title: FathomDB 0.8.27 release status
status: ACTIVE
---

# FathomDB 0.8.27 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.27.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.27:status-current-state -->**Next is Slice 60 (ENGINE-WRITE), PLANNED.** Completed on local `release/0.8.27` per release state: 0 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 1 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 2 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 3 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 4 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 5 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 6 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 7 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 8 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 9 (`a3e6cff6f25493096b4b4ca9b76ee560cb6f7ef1`) · 10 (`3097d191511d81a221b038ccd2e14f074dcafa6d`) · 20 (`b455bb73fb2b04c91f50e6e5dbdc16752325453b`) · 30 (`6ba3be95cd043570da1deafbe4e2f78c878d8a87`) · 40 (`fdd7fb646b0fb922b9b8fea134ef7ce7e71a5aae`) · 50 (`1f5b8614813b5a363ec5f81fcb580d48da4a4e8f`) — state-owned, not an `origin/main` claim.<!-- END GENERATED release-state:0.8.27:status-current-state -->

Prework Slices 0-9, bounded preparation Slice 10, correction-safe erasure
Slice 20, real-surface comparator Slice 30, and engine-foundation Slice 40 are
complete on `release/0.8.27`. Slice 40 preserves runtime behavior, schema,
public API, feature gates, and publication state while moving shared engine
foundations to private modules.

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.27:status-next-action -->**Commission Slice 60 (ENGINE-WRITE)** — engine write, ingest, consolidation, and actuation domains. **Remaining ladder:** 60 → 70 → 80 → 90 → 100 → 110 → 120 → 130 → 140 → 150.<!-- END GENERATED release-state:0.8.27:status-next-action --> |

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
| 20 | Correction-safe source erasure | Product complete at `b455bb73`; exact atomicity, at-rest, binding, and unchanged-fixture Memex evidence pass review. The shared Python artifact gate is corrected at `6ba3be95`. |
| 30 | Current inventory and comparison guardrails | Complete at `6ba3be95`; deterministic 13-row real-surface baseline, mutation-tested comparator, candidate-bound artifact gates, independent design/code review, and focused verification pass. |
| 40 | Engine foundation and test seams | Complete at `fdd7fb64`; private error, identity, temporal, and hook modules preserve root paths and cfg gates. Final surface comparison, 124-suite gate, strict security, workspace Clippy/check, and independent review pass. |

## Verification boundary

The final whole-work review covered the complete Slice 20/30 plans, assigned
functions, implementation, tests, baseline, records, and generated artifacts.
Committed RED/GREEN rounds closed stale Python native shadowing, debug NAPI
leakage, multiline cfg ownership, distinct-filesystem capacity, scratch
ownership, and fixture-cleanup defects. Independent design review, code review,
and focused verification passed at clean `2967593c`; the final `6ba3be95`
change only made an older packaging assertion follow the canonical wrapper.

The reviewed baseline has 13 rows and SHA-256 `06212f66…`; a clean later
capture compared equal with no metadata or row diff. The canonical gate passed
lint, typecheck, strict security 0/0/0, Rust, TypeScript, executable NAPI, and
123 of 124 registered suites. Its sole failure was that obsolete packaging
assertion. After correction, the full Python suite passed 1,533 tests with 27
documented skips and the candidate receipt validated. Slice 150 retains fresh
installed-package and platform qualification.

Slice 40's final clean candidate `fdd7fb64` produced a fresh 13-row capture
that compared equal to the immutable baseline with empty metadata and row
diffs. The canonical gate passed all 124 registered suites with no failures,
skips, or exclusions; strict security was 0/0/0; the Python native receipt was
candidate-bound; and full-workspace Clippy/check passed. The locked Slice 20
erasure matrix remained unchanged and passed 5/5.

Slice 50's final clean candidate `1f5b8614` split the lifecycle, provenance,
dependency, and erasure domains into private modules and closed nonterminal
soft-closure residue without changing public paths. Independent design/code
review and verification passed. Public and hidden structural surfaces are
equal; hidden inventory has only reviewed additive tests; the canonical gate
passed 127/127 with strict security 0/0/0; workspace Clippy/check and the
candidate-bound Python receipt passed.

## Boundaries

- This release starts at schema 34 and currently proposes no schema migration.
- Publication is unauthorized.
- Slices 30, 40, and 50 were directly authorized by the repository owner and
  are complete; Slice 60 and later slices require separate commission.
- No temporary branch or worktree was created for prework; the existing
  `release/0.8.27` worktree remains the active release workspace.
