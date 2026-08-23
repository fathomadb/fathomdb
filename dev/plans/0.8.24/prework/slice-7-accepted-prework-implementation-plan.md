---
title: 0.8.24 Slice 7 — accepted prework implementation plan
status: DRAFT-CONTINGENT
target_release: 0.8.24
---

# Slice 7 — accepted prework implementation plan

## Contingency

This is a planning shell, not authorization to implement. Slice 6 must replace
the candidate register with the owner's accepted prework, complete the required
independent read-only review/FIX-n process, and record final owner approval
before Slice 7 starts.

## Purpose

Implement only the bounded, version-preparation work identified by Slices 0–6
and accepted in Slice 6, with one writer, explicit file scope, proportional
verification, and durable evidence. Feature planning and implementation remain
entirely in Slices 10–70.

## Required inputs and start gate

Slice 7 may start only when all of the following are true:

1. Slices 0–5 are complete and Slice 6 has a complete proposal register.
2. Every included work package has an explicit final owner decision of
   **accepted** and one primary destination of Slice 7.
   A package whose primary destination is a feature slice is excluded even if
   it appears preparatory or low-risk.
3. The independent read-only review is pass, or all accepted review findings
   have been closed within the maximum two FIX-n cycles and the owner has
   expressly accepted any remaining risk.
4. This file identifies exact files, verification, dependencies, and stop
   conditions for every included package.
5. The writer operates in a fresh isolated worktree from the current authorized
   integration base; the shared main checkout is not an editing surface.

If any gate is false, Slice 7 remains blocked and makes no changes.

## Current candidate register — not authorized

The candidates below are inputs to Slice 6 only. Presence in this table does
not mean accepted, and Slice 6 must remove every non-accepted row from the
implementation scope.

| Candidate | Source | Possible bounded proof |
| --- | --- | --- |
| Update root `markdownlint-cli2` and lockfile to remediate the audited `js-yaml` path | Slice 1 | Root audit plus the repository's guarded Markdown checks |
| Review Pyright 1.1.410 → 1.1.411 | Slice 1 | Version guard and Python typecheck; postpone if output changes are not understood |
| Remove the unused Prettier dependency and obsolete bootstrap wording | Slices 1–2 | Direct-use scan, clean root install, and guarded Markdown tooling checks |
| Correct maintained former-owner repository URLs | Slice 2 | Bounded maintained-surface scan and documentation link/build checks |
| Correct stale reader-facing “current release” assertions | Slice 2 | Bounded current-release scan against actually published registry state |
| Reconcile active engineering navigation | Slice 2 | Navigation/index consistency and release-state lookup checks |
| Change Dependabot's paused posture | Slice 1 | Owner policy decision and configuration-specific check; no implied acceptance |

Slice 2 found no valid archive/delete candidate. No file deletion or archival
belongs in Slice 7 unless Slice 6 adds a concrete owner-accepted proposal that
satisfies R24-15/AC24-15 draft safeguards.

## Scope rules

- Implement every accepted Slice 7 package and nothing else.
- Preserve rejected, postponed, needs-clarification, and feature-slice rows
  without modification.
- Do not draft, make ready, implement, or provide a substitute implementation
  for a Slice 10–70 feature. Each feature slice owns its own
  draft-to-ready planning, design review, implementation, and evidence.
- A Slice 7 package may prepare the repository generally, but it may not
  decide a feature's product shape, create its feature-specific code/test/
  workflow/artifact path, or satisfy that feature's acceptance evidence.
- Keep related low-risk documentation/tooling repairs in one bounded change
  set when their file scopes and verification are compatible; do not create a
  hosted CI cycle per administrative edit.
- Use TDD for code behavior. For tooling/configuration remediation, first
  retain the failing audit/scan/contract evidence, then apply the minimum
  change and prove that same condition green.
- Update `dev/DOC-INDEX.md` or other navigation only when an accepted document
  change requires it.

## Non-goals

- No Tegra package identity, Windows SDK/executor, performance-engine, WAL,
  artifact-smoke, publisher, or release-integration feature work; those belong
  to Slices 10–70 and must first receive their own draft-to-ready plan.
- No tag, publication, registry/environment/secret mutation, runner setup,
  workflow dispatch, branch-protection change, or release-state creation unless
  separately and explicitly included by an owner decision.
- No broad dependency sweep, major runtime/library migration, bulk documentation
  rewrite, repository prune, or historical evidence rewrite.
- No intentional hosted full CI run solely to validate administrative
  integration. Existing automatic behavior, if any, is recorded rather than
  treated as an extra gate.

## Implementation-plan form

Slice 6 replaces this section with one ordered block per accepted work package:

### S7-XX — `<accepted work package>`

- **Owner decision:** link and exact accepted wording.
- **Purpose:** observable problem being corrected.
- **Files/surfaces:** exhaustive edit set and intentionally excluded neighbors.
- **Preconditions:** required evidence, versions, and branch/base state.
- **RED/baseline proof:** failing test, audit, scan, or reproducible stale-state
  check that demonstrates the problem.
- **Change:** smallest approved implementation.
- **GREEN proof:** exact targeted commands and expected end state.
- **Broader check:** only the repository verification justified by the actual
  blast radius; explain any omission or environment limitation.
- **Stop conditions:** unexpected public/ADR/runtime impact, new dependency
  cohort, unavailable prerequisite, or scope beyond the owner decision.
- **Evidence:** files/logs/commit recorded at close.

Sequence shared-file packages serially. Separate a package only when it has a
different dependency, risk boundary, or required executor—not merely to create
an additional review or CI event.

## Baseline execution sequence

1. Revalidate branch/worktree/base and confirm no unrelated user changes are
   present in the Slice 7 worktree.
2. Re-read the final Slice 6 decisions and verify the exact accepted package
   list against this file.
3. For each package in dependency order, capture its RED/baseline evidence,
   make the minimum change, and run its targeted GREEN proof.
4. Run the combined local verification selected by the final plan. If source,
   build, or cross-cutting dependency behavior changes, use the normal
   repository verification appropriate to that blast radius; documentation-only
   work uses documentation checks.
5. Run scoped Markdown lint and `git diff --check`; inspect the complete diff
   for rejected/postponed scope.
6. Commit the accepted implementation in the isolated branch. Do not push,
   merge, dispatch CI, tag, or publish without separate direction.
7. Record completion evidence and any accepted item that remains blocked.

## Deliverables

- The exact owner-accepted maintenance changes and their targeted tests/checks.
- A Slice 7 completion record mapping each accepted decision to files, commit,
  verification, and final state.
- Necessary plan/index updates caused by the accepted changes, and no feature
  implementation.

## Completion criteria

Slice 7 is complete when:

- every accepted package is implemented and verified or explicitly returned
  to the owner as blocked;
- no rejected, postponed, feature, or unruled work appears in the diff;
- every change has its declared baseline/RED and GREEN evidence;
- public/shared-surface consequences, if any, have their required same-change
  contract documentation;
- the worktree is clean after commit and the completion record names the exact
  SHA; and
- no publication, tag, workflow dispatch, or unapproved external mutation has
  occurred.
