---
title: 0.8.24 Slice 7 — accepted prework implementation plan
status: COMPLETE
target_release: 0.8.24
---

# Slice 7 — accepted prework implementation plan

## Contingency

The owner accepted the four packages below in the first-round Slice 6 decision.
The required independent read-only review passed after FIX-1, and the owner
approved this final plan on 2026-08-23. The implementation boundary remains
exactly the four packages below.

**Completion:** S7-01 through S7-04 are complete. Exact evidence and commits
are in [the Slice 7 completion record](slice-7-completion.md).

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

## Accepted scope

The [Slice 6 decision record](slice-6-hitl-decisions.md) accepts **only**
P24-01, P24-03, P24-04, and P24-05. Pyright is postponed, Dependabot remains
paused, and no archive/delete candidate exists. No feature-slice work is
included.

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

## Ordered implementation packages

### S7-01 — root Markdown security remediation

- **Owner decision:** P24-01 accepted: update root `markdownlint-cli2` and its
  lockfile to remove the audited `js-yaml` path.
- **Files/surfaces:** `package.json`, `package-lock.json`, and the shared
  completion record `slice-7-completion.md` only. No TypeScript-package
  manifest, source, workflow, or Markdown content change.
- **RED/baseline proof:** capture root `npm audit --json` showing the known
  `markdownlint-cli2 → js-yaml` advisory path and record the resolved version.
- **Change:** update only the root `markdownlint-cli2` declaration to
  `0.23.2` and its lock resolution to the reviewed `js-yaml` `5.2.2`; do not
  add overrides or unrelated package updates.
- **GREEN proof:** rerun `npm audit --json` and
  `npm ls markdownlint-cli2 js-yaml`; run `bash scripts/agent-lint-md.sh` and
  `git diff --check`. The named advisory path must be absent and the
  AST-guarded Markdown path must pass.
- **Broader check:** no hosted CI. A root dev-tool dependency change warrants
  the selected local lint/contract checks, not release, CUDA, or package smoke.
- **Stop conditions:** version 0.23.2 or `js-yaml` 5.2.2 is unavailable or
  resolves differently; a new advisory cohort, lockfile churn outside the
  named dependency tree, a Markdown semantic-neutrality failure, or changed
  tool behavior requiring broader owner choice.

### S7-02 — remove unused Prettier tooling

- **Owner decision:** P24-03 accepted: remove the unsupported root Prettier
  dependency and obsolete bootstrap wording.
- **Files/surfaces:** `package.json`, `package-lock.json`,
  `scripts/bootstrap.sh`, and the shared `slice-7-completion.md`. Retain
  historical records, the Markdown safety policy, and comments that accurately
  state Prettier is not used for Markdown.
- **RED/baseline proof:** retain the completed direct-use scan: root manifests
  install Prettier while active commands do not invoke it; the bootstrap text
  advertises it as Markdown tooling.
- **Change:** remove only the root dev dependency/lock entry and change the
  bootstrap message/comment to name the remaining Markdown tooling accurately.
- **GREEN proof:** repeat the supported-command scan; in the fresh Slice 7
  worktree perform `npm ci`; then run `bash scripts/agent-lint-md.sh` and
  `git diff --check`.
- **Broader check:** no hosted CI. Do not run Prettier or rewrite historical
  references merely because they mention it.
- **Stop conditions:** any active supported non-Markdown invocation, a clean
  install regression, or a need to delete tool configuration/history not
  covered by the accepted package.

### S7-03 — maintained public-link and release-currency correction

- **Owner decision:** P24-04 accepted: correct former-owner links and
  reader-facing stale current-release assertions on maintained surfaces.
- **Files/surfaces:** `mkdocs.yml`; `src/python/README.md`; `src/ts/README.md`;
  `src/rust/crates/{fathomdb,fathomdb-cli,fathomdb-embedder,fathomdb-embedder-api,fathomdb-engine,fathomdb-query,fathomdb-schema}/README.md`;
  and only active assertions found by the baseline in `docs/{index.md,compatibility/index.md,concepts/index.md,embedder.md,getting-started/index.md,guides/index.md,install/python.md,install/rust.md,install/typescript.md,operations/index.md,reference/index.md,reference/python-api.md,reference/typescript-api.md}`;
  `docs/reference/{cli,config,errors}.md` and
  `docs/operations/worktree-consolidation.md` under the owner-authorized
  S7-03 scope amendment;
  `dev/DOC-INDEX.md`, `dev/doc-index/plans.md`, and the shared
  `slice-7-completion.md` when required by the edited documentation.
  `docs/release-notes/{0.6.0,0.6.1,0.8.0}.md` and other historical evidence are
  intentionally excluded unless an individual line falsely presents itself as
  current guidance.
- **RED/baseline proof:** save the bounded `coreyt/fathomdb` and `0.8.21`
  scans, classify each hit active versus historical, and confirm the latest
  published release before editing. Do not pre-announce 0.8.24.
- **Change:** replace only active former-owner URLs with the canonical
  `fathomadb/fathomdb` route and update only active current-release claims to
  the actually published release.
- **GREEN proof:** rerun the bounded scans with historical exclusions; run
  `bash scripts/agent-lint-docs.sh`, `mkdocs build --strict`, and
  `git diff --check`.
- **Broader check:** documentation-only checks; no hosted CI or registry
  mutation.
- **Stop conditions:** uncertainty whether a line is historical, a link target
  that has a different canonical owner, an unresolved release version, or a
  required public contract/ADR change.

### S7-04 — active engineering navigation correction

- **Owner decision:** P24-05 accepted: reconcile active engineering navigation
  to the existing release-state lookup rule.
- **Files/surfaces:** `dev/README.md`, `dev/plans/README.md`,
  `dev/DOC-INDEX.md`, `dev/doc-index/plans.md`, and the shared
  `slice-7-completion.md` only. Do not create/edit a `release-state-*.json`
  file or historical release board.
- **RED/baseline proof:** retain the exact conflicting statements: the former
  `0.8.6–0.8.16` schedule called master, the current program schedule entry,
  and the already-defined release-state resolution rule.
- **Change:** remove or qualify stale hard-coded active-program claims so all
  four active navigation surfaces direct readers to the current program
  schedule and resolve a live release through `release-state-*.json`.
- **GREEN proof:** bounded text scan and index consistency review; run
  `npx markdownlint-cli2 dev/README.md dev/plans/README.md dev/DOC-INDEX.md dev/doc-index/plans.md`,
  `bash scripts/lint-plans-status.sh`, `bash scripts/lint-plan-anchors.sh`,
  and `git diff --check`.
- **Broader check:** documentation/index checks only; no hosted CI or
  release-state mutation.
- **Stop conditions:** a conflict with an accepted ADR or live release-state
  contract, a required history rewrite, or a scope expansion beyond the four
  named files.

The root manifests/lockfile are shared by S7-01 and S7-02; execute them
serially in that order. S7-03 and S7-04 may share one documentation commit only
if their bounded evidence remains separately recorded; they must not pull
feature documentation into Slice 7.

### Shared durable completion evidence

Create `dev/plans/0.8.24/prework/slice-7-completion.md` as the sole Slice 7
completion record. For each S7-01 through S7-04 package it must record the
owner-decision ID, baseline command and result, changed files, exact GREEN
commands and results, completion commit SHA, and any stop/blocked disposition.
Update `dev/DOC-INDEX.md` and `dev/doc-index/plans.md` for that record. The
record is evidence, not a new implementation package or a release-state board.

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
