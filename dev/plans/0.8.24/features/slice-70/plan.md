---
title: 0.8.24 Slice 70 — release integration and evidence draft plan
status: DRAFT
target_release: 0.8.24
---

# Slice 70 — release integration and owner-ready evidence

## Planning boundary

This document plans the final integration/evidence slice. It does not merge a
feature, bump a version, dispatch a workflow, approve an environment, create or
push a tag, publish a package, promote a dist-tag, or write release completion.
Every external release action remains an explicit owner gate.

Slice 70 contains no feature implementation. It begins after the upstream
feature outputs are reviewed and assembles only their approved code and
evidence. A missing upstream proof is not repaired by weakening the final gate.

## Goal and outcome

Produce one owner-reviewable 0.8.24 release candidate and evidence packet that:

- reconciles current `main`, the release branch, and each accepted feature
  commit without overwriting newer main-owned CI;
- maps every R24 requirement and draft canonical update to exact code, tests,
  target evidence, and unresolved external actions;
- proves version, package, publisher, target, and release-workflow readiness
  locally to the maximum extent possible;
- requests external or hosted execution only for facts that cannot be proven
  locally;
- presents an exact, bounded publication and post-publish-smoke sequence for
  owner authorization; and
- marks 0.8.24 complete only after all required registry-installed evidence is
  present.

## Authority and inputs

- `dev/plans/plan-0.8.24.md`, Slice 6 decisions, and completed Slice 7 record.
- Reviewed outputs/evidence from Slices 10, 20, 30, 40, 50, and 60.
- Execution-time `origin/main`; accepted feature commits verified from git.
- `dev/design/release.md`, `scripts/verify-release-gates.sh`,
  `scripts/set-version.sh`, `scripts/release/local-dry-run.sh`, release workflow,
  smoke scripts, publisher guards, and accepted release/platform ADRs.
- Memory `release-publish-gotchas.md`,
  `release-dod-requires-full-workspace-gate.md`, and
  `dont-gate-trivial-changes-on-ci.md`.

The 0.8.23 release is historical and complete. Slice 70 does not reopen or
rewrite it; later corrections are additive 0.8.24 records only when relevant.

## Scope

### In scope

- Verify upstream branch/commit provenance, review verdicts, tests, design/ADR/
  interface/docs updates, and feature evidence.
- Integrate accepted changes onto a candidate based on current main, resolving
  actual integration conflicts as part of that integration review.
- Apply the mechanical 0.8.24 version/changelog/release-note changes using the
  canonical version script after feature integration is stable.
- Build the release requirement/evidence matrix and package/publisher readiness
  inventory.
- Run proportional local release checks and identify the smallest external-only
  dry run or target execution, if any, still required.
- Prepare exact publication, registry-query, post-publish smoke, and completion
  instructions for the owner gate.

### Non-goals

- New engine, packaging, CI, WAL, target, publisher, or smoke functionality.
- Re-running the retained performance benchmark.
- Starting a full hosted workflow for administrative integration or because it
  is conventional.
- Removing or bypassing release safety checks, skipping hooks, replacing
  immutable artifacts, or granting publication authority through this plan.
- Declaring success from green source CI without registry-installed smoke.

## Slice prep — planned first phase

Create under this directory:

- `prep.md` — goals, current-main/release topology, upstream readiness inventory;
- `draft-contracts.md` — final slice-local integration/evidence requirements;
- `design.md` — integration graph and release-evidence state machine;
- `research.md` — focused primary-source registry/workflow questions;
- `evidence-matrix.md` — R24/target/package/publisher/verification status; and
- `owner-handoff.md` — exact decisions, commands/actions, failure branches, and
  post-publish completion steps.

### Prep tasks

1. Resolve current `origin/main`, release branch, and every proposed feature
   SHA. Verify ancestry, changed files, review status, and whether shared files
   overlap. Narration is not evidence.
2. Restate final requirements R24-1 through R24-7 plus accepted draft updates
   from Slices 30/40/60. Propose slice-local integration drafts:
   - **R70-DRAFT-1:** one candidate SHA contains exactly the accepted feature,
     contract, docs, and release-preparation changes based on current main;
   - **R70-DRAFT-2:** every required artifact has build/publisher/smoke evidence
     or an explicit owner-gated pending state;
   - **R70-DRAFT-3:** no release is complete until exact public versions and
     required target-native installed smokes are verified;
   - **AC70-DRAFT:** evidence matrix is complete, full local release/workspace
     gates pass, external facts are narrowly proven, and post-publish queries/
     smokes match the tagged candidate.
3. Read architecture/release design, all changed ADR/interface files, actual
   release workflow/scripts, and every upstream completion record. Write an
   exists-versus-net-new map and identify stale plans/docs.
4. Enumerate prerequisites and assign them back to the owning slice before
   integration. Slice 70 does not implement a missing target artifact or fix an
   unattributed WAL issue.

## Draft integration and release design

### Integration graph

The design records:

- base SHA and feature SHAs;
- dependency/order edges;
- shared-file collision set, especially workflows, package metadata, release
  scripts, requirements/acceptance, architecture, changelog, and docs indexes;
- merge/rebase strategy using ordinary reviewed commits;
- exact conflict-resolution rationale; and
- verification after each meaningful integration group.

Slice 10 current-main CI is integrated from main, never recreated from the
release branch. Independent features may be reviewed separately, but shared
mutable release files are serialized.

### Evidence state machine

Each requirement/artifact row is one of:

1. **MISSING** — upstream work/evidence absent; return to owner slice.
2. **IMPLEMENTED_UNVERIFIED** — code exists without required local/target proof.
3. **LOCALLY_VERIFIED** — local contract/workspace proof complete.
4. **TARGET_VERIFIED** — target-native candidate-installed evidence complete.
5. **PUBLISH_READY** — identity, publisher, environment, version, and artifact
   inputs verified; publication still unauthorized.
6. **PUBLISHED_UNSMOKED** — exact public artifact exists but installed proof is
   pending/failing.
7. **COMPLETE** — exact tagged bytes are public and every required smoke passed.

Only the owner can authorize transition into publication. The release is not
complete in any earlier state.

### Challenging aspects and research plan

- Verify current PyPI/npm/crates trusted-publisher/authentication and package
  existence semantics from official registry docs only where readiness changed.
- Verify GitHub workflow environment/OIDC/artifact claims and exact tag/ref
  behavior from official GitHub docs when the workflow design depends on them.
- Verify NVIDIA target compatibility only through the already approved Slice
  30/40 records; Slice 70 does not reopen their target design.
- Treat registry propagation separately from publish failure; use existing
  exact-version queries and bounded smoke retry policy.

### Architectural-fit review and revision

Review the candidate against `dev/architecture.md`, release design, accepted
ADRs, public interfaces, canonical needs/requirements/acceptance, and actual
code. Revise stale docs or return a feature to its owner. Do not resolve a
material contract conflict only in a release note.

## Planned execution after prep approval

### Phase 1 — upstream acceptance audit

- Verify every upstream commit and evidence record.
- Require scoped review verdicts and green checks proportional to each feature.
- Return incomplete work before integration; do not patch it incidentally.

### Phase 2 — current-main integration

- Create an isolated candidate from current `origin/main`.
- Integrate accepted commits in dependency order, resolving conflicts as part
  of the integration with recorded rationale.
- Re-run scoped checks after shared-file integrations.

### Phase 3 — release preparation

- Use `scripts/set-version.sh --workspace 0.8.24`; inspect every changed
  version/source/dependency surface.
- Add the 0.8.24 changelog/release notes and update documentation indexes.
- Verify tag/candidate assumptions, package manifests, release job graph,
  publisher identities/environments, and exact target artifact matrix.

### Phase 4 — proportional verification

- Run local lint/typecheck/test/build in latency order and the full workspace
  clippy/check release gate before a green claim.
- Run the release local dry-run and target/release contract tests applicable to
  the actual diff.
- Request one external/hosted exercise only for each remaining external-only
  claim. Do not deliberately start redundant full CI for administrative work.

### Phase 5 — owner publication gate

Present `owner-handoff.md` with:

- candidate full SHA and expected tag;
- exact public artifact/package/version matrix;
- trusted-publisher/environment readiness;
- unresolved risks and rollback-forward behavior;
- the one irreversible tag/publish action clearly identified; and
- exact post-publish queries and target smoke commands.

Without explicit owner authorization, stop at PUBLISH_READY.

### Phase 6 — post-publication completion

After separately authorized publication, verify exact public versions and run
the required registry-installed smokes from Slice 60 on the actual target
families. If a smoke fails because of propagation, distinguish it from publish
failure and rerun only the failed smoke path when appropriate. Do not republish
valid immutable artifacts.

## Verification and evidence

The candidate-level minimum includes:

- all proportional feature checks and independent review records;
- `scripts/agent-verify.sh` plus full workspace clippy/check;
- `scripts/check.sh`/long variants only where release policy and actual changes
  require them, with no claim that a short gate replaces long evidence;
- `scripts/set-version.sh --check-files` and `verify-release-gates.sh` in a safe
  local/dry-run configuration;
- actionlint and release-contract/publisher/smoke tests;
- strict docs build and document/plan/index checks;
- target candidate-installed evidence from Slices 30/40/60; and
- after publication, exact registry queries and installed-package lifecycle
  smokes tied to the tagged commit.

GitHub-hosted macOS/Windows, environment approval, OIDC, artifact transfer, and
registry publication cannot be fully emulated locally. The handoff identifies
which of those facts is actually needed and avoids multiplying hosted runs.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| Candidate omits newer main CI | Base on current main and verify ancestry before/after integration. |
| Conflict resolution changes feature intent | Record file-level rationale and rerun owning slice tests/review. |
| Version bump misses a surface | Use canonical script plus consistency checks and manifest inspection. |
| A target/publisher is assumed ready | Require exact external readiness evidence in the matrix. |
| Tag push publishes unexpectedly | Clearly label it irreversible and require separate owner authorization. |
| Partial publication occurs | Use idempotent retry/no-op; never replace valid immutable versions. |
| Smoke fails during registry propagation | Query presence, retry only smoke/dependent completion, do not recut reflexively. |

## Decisions and prerequisites for the next reviewer

Before ready status, the reviewer must approve the upstream acceptance audit,
integration order, conflict strategy, final artifact matrix, proportional check
set, and exact external-only evidence requests. Before publication, the owner
must explicitly approve the candidate SHA/tag and publication action.

Any missing feature proof returns to Slice 10/20/30/40/50/60. Slice 70 does not
become a catch-all implementation slice.

## Definition of done

Slice 70 closes only when the accepted candidate is on current main, every
requirement and artifact is traced to reviewed evidence, version/release/docs
state is consistent, full local release/workspace gates are green, the owner
authorized publication, exact registry versions exist, and all required
registry-installed target smokes pass. PUBLISH_READY alone is not release
completion.
