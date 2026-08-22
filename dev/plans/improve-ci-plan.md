---
title: Improve CI — in-place proportional routing plan
date: 2026-08-22
status: PROPOSED
desc: >
  Draft implementation plan for replacing FathomDB's monolithic non-doc CI
  routing with tested, proportional, informational routing while preserving
  the proven test, packaging, security, Windows, and release job bodies.
blast_radius: >
  .github/workflows/{ci,aarch64-release-preflight,gitleaks-history,release}.yml;
  scripts/tests/ CI routing fixtures; dev/design/
  ci-cd-simplified-redesign-20260821.md
---

replace the current routing policy with the corrected new design. Do not replace
the proven job bodies wholesale.

The existing test, packaging, Windows, security, and native-artifact jobs encode
real delivery requirements. The costly defect is that they are routed as one
monolithic non-doc tier. An in-place refactor preserves those hard-won job bodies
while replacing:

- the binary docs_only classifier;
- broad heavy-job conditions;
- per-push full-history Gitleaks;
- the independent AArch64 trigger;
- and the absence of an explicit administrative fast path.

Current live state still has no rulesets and main returns “Branch not
protected,” so informational CI remains aligned with repository policy. No
required checks, merge queue, nightly schedule, aggregator gates, or soak period
should be added.

Cutover should be atomic after fixtures cover the five findings above. Also give
full-history Gitleaks a concrete dispatchable home before removing its current
step. That is an implementation prerequisite, not a soak period.

# Improve CI — draft implementation plan

## 1. Goal

Make CI cost proportional to the changed surface without weakening the fast,
informational signal or rewriting job bodies whose platform and packaging
behavior is already proven.

The implementation changes routing, trigger ownership, and secret-scan cadence.
It does not redesign tests, release publication, native packaging, Windows WAL
diagnostics, or the five-platform artifact matrix.

## 2. Binding guardrails

- CI remains informational. Do not add branch protection, required checks,
  required reviews, or aggregator jobs.
- Do not add a merge queue, scheduled/nightly execution, or a soak period.
- Preserve the existing eight scoped job bodies and all 14 independent job
  bodies. Change their selection conditions only where this plan says to.
- Preserve `release.yml` publication dependencies and post-publish smoke jobs.
  The only release-workflow addition in scope is an independent, advisory
  full-history Gitleaks job.
- Keep `verify-fast` as the non-Markdown baseline. `[ci-lite]` may suppress only
  the eight scoped heavy jobs.
- Use failing fixtures before workflow edits. A YAML parser alone is not an
  oracle for changed-file behavior.
- Keep third-party actions pinned to reviewed commit SHAs.

## 3. Requirements and acceptance signals

| ID | Requirement | Offline acceptance signal |
|---|---|---|
| CI-R1 | A Markdown-only diff never selects a scoped heavy job, including Markdown under each language tree | Routing fixture covers `src/rust/**/README.md`, `src/python/README.md`, and `src/ts/README.md` |
| CI-R2 | The installed Windows attribution control selects its Windows job without Python or ARM64 artifact work | Routing fixture reports `windows=true`, `python=false`; AArch64 workflow has no `push` trigger |
| CI-R3 | A mixed Windows plus ordinary-Python diff selects both routes | Real matcher fixture reports `windows=true`, `python=true` |
| CI-R4 | Lite mode is accepted only from a trusted same-repository PR or repository push and only from an exact `[ci-lite]` line | Marker fixture covers trusted PR, fork/untrusted PR, exact line, incidental substring, merge, rebase, squash, and direct push |
| CI-R5 | Root npm dev-tooling changes do not impersonate TypeScript SDK changes | Fixture reports `typescript=false` for root `package.json` and `package-lock.json` |
| CI-R6 | `verify-fast` runs for every non-Markdown diff and when classification fails | Job-condition fixture forces `changes=failure` and observes `verify-fast=run` |
| CI-R7 | Heavy routing follows the dependency-accurate table in the design | Per-category job-selection fixture enumerates every scoped job |
| CI-R8 | Current-tree Gitleaks remains per-push; full-history scanning is dispatchable and advisory at release time | Workflow fixture proves all three routes and proves no publish job needs the advisory job |
| CI-R9 | Linux ARM64 release evidence is retained without an independent automatic push workflow | Native ARM64 row proves the migrated unique assertions; AArch64 rehearsal is dispatch-only |
| CI-R10 | Release tags cannot be suppressed by `[ci-lite]` | Trigger fixture proves `release.yml` still runs for a tagged lite commit |

## 4. Work sequence

### 4.1 Establish the failing routing fixture

Extend the existing CI contract-fixture family under `scripts/tests/`. The test
must execute the real `dorny/paths-filter` matching semantics for path cases and
must evaluate the actual job conditions for route cases.

Add RED cases for:

- Markdown files in the Rust, Python, and TypeScript trees;
- the Windows attribution control alone and mixed with ordinary Python;
- root `package.json` and `package-lock.json`;
- each source and harness category independently;
- a failed `changes` job;
- trusted, fork, and untrusted-author same-repository PR markers;
- exact `[ci-lite]` line versus incidental substring;
- merge, rebase, squash, and direct-push landing forms;
- AArch64 automatic-push absence and migrated assertion ownership;
- current-tree, dispatchable full-history, and advisory release Gitleaks; and
- tag-push release safety.

Commit or stage the failing fixture before editing workflow behavior.

### 4.2 Replace the classifier in place

Modify the existing `changes` job in `.github/workflows/ci.yml`:

- retain `docs_only`;
- add `rust`, focused Python-minus-Windows, `typescript`, `windows`,
  `ci_workflow`, and the job-specific harness outputs;
- classify the TypeScript SDK as `src/ts/**` only; root npm manifests remain
  unclassified non-Markdown dev tooling owned by `verify-fast` and existing
  tooling checks;
- retain the focused `predicate-quantifier: every` Python matcher; and
- keep `pull-requests: read` plus `contents: read` permissions.

Do not create a second taxonomy workflow or rewrite the downstream jobs.

### 4.3 Make Markdown and classifier-failure behavior explicit

Every scoped heavy-job condition takes this form:

```yaml
if: >-
  needs.changes.outputs.ci_workflow == 'true' ||
  (
    needs.changes.outputs.docs_only != 'true' &&
    (<job-specific categories>) &&
    needs.changes.outputs.ci_mode != 'lite'
  )
```

`ci_workflow` remains the deliberate override that executes all eight scoped
jobs when their shared workflow changes.

`verify-fast` uses `always()` and runs when `changes` fails or when the diff is
non-Markdown. A classifier failure remains visibly red, runs the baseline, and
does not fan out into expensive jobs using unknown classification outputs.

### 4.4 Restrict lite mode to a real maintainer assertion

- Continue using the literal marker `[ci-lite]`, but recognize it only when it
  occupies an entire commit-message line.
- Honor it for repository `push` events.
- Honor it for a PR only when
  `github.event.pull_request.head.repo.full_name == github.repository` and the
  PR author association is `OWNER`, `MEMBER`, or `COLLABORATOR`.
- Treat fork and Dependabot-style PRs as `normal`, regardless of their commit
  message.
- Preserve the documented merge-second-parent, rebase-tip, squash-tip, and
  direct-tip landing behavior.
- Keep `ci_workflow=true` as an override that ignores lite mode.

### 4.5 Retire the independent automatic AArch64 route

Before removing its `push` trigger, compare
`.github/workflows/aarch64-release-preflight.yml` with the Linux ARM64 row of
`native-artifact-runtime-validation` and migrate its unique contract assertions:

- ABI3/interpreter compatibility evidence not already proved by the native row;
- staging the Linux ARM64 N-API binary into its platform package; and
- `npm pack --dry-run` verification of that package.

Put executable artifact assertions in the Linux ARM64 native row and structural
contract assertions in fast fixtures where execution adds no signal. Then make
`aarch64-release-preflight.yml` `workflow_dispatch`-only for targeted manual
rehearsal. Do not add a schedule.

### 4.6 Split current-tree and full-history Gitleaks

- Keep `scripts/security/gitleaks-current.sh` in the existing always-on
  `gitleaks` job.
- Remove `gitleaks-history.sh` from that per-push job only after the replacement
  routes exist.
- Add `.github/workflows/gitleaks-history.yml` with `workflow_dispatch` only,
  full checkout, the pinned installer, and the existing history script.
- Add an independent `continue-on-error: true` release job that runs the same
  history scan against the release candidate. No publisher or build job may
  `need` it.
- Keep the existing allowlist mismatch visible; do not describe history scanning
  as clean until its baseline is reconciled.

### 4.7 Apply the job-selection table without changing bodies

Change only `needs`/`if` routing for:

- `verify`;
- `rust-workspace-race-report`;
- `security`;
- `default-embedder-tests`;
- `wheel-size-gate`;
- `native-artifact-runtime-validation`;
- `windows-wal-checkpoint-diagnosis`; and
- `windows-wal-attribution`.

Do not refactor their steps, matrices, runners, timeouts, caches, build commands,
or artifact handling in this change.

## 5. Verification and atomic cutover

Run in latency order and stop on the first failure:

1. The new focused routing and marker fixtures.
2. Existing CI-shape and shell-job fixtures, including
   `scripts/tests/test_shell_lint_ci_job.sh`.
3. `actionlint` over every modified workflow.
4. Markdown, plan-status, design-status, findings, and anchor lint.
5. `./scripts/agent-verify.sh` because workflow and harness contracts changed.
6. `git diff --check` and an independent review of the final workflow diff.

Land the classifier, conditions, AArch64 trigger correction, Gitleaks replacement
route, fixtures, plan, and design update together. Do not leave an intermediate
commit on the landing branch where full-history scanning has been removed but its
dispatchable replacement does not exist.

The first resulting CI run is operational evidence, not a required gate or soak
period. Correct an implementation defect if it is red; do not delay cutover for
an arbitrary clean-duration requirement.

## 6. Explicitly out of scope

- Rewriting test, packaging, Windows, security, native-artifact, or release job
  bodies.
- Splitting the heavy verifier by language before a real `--surface=` contract
  exists.
- Branch protection, required checks, required reviews, or aggregator jobs.
- Merge queues, scheduled/nightly workflows, or rollout soak periods.
- CUDA, maturin, Candle, or GPU-toolchain redesign.
- Changing the published language, platform, architecture, or glibc support
  contract.

## 7. Expected file surface

- `.github/workflows/ci.yml`
- `.github/workflows/aarch64-release-preflight.yml`
- `.github/workflows/gitleaks-history.yml` (new)
- `.github/workflows/release.yml` (one independent advisory job only)
- `scripts/tests/<focused-ci-routing-fixture>.sh` (name chosen during TDD setup)
- any existing CI-shape fixture that owns the invariant being changed
- `dev/design/ci-cd-simplified-redesign-20260821.md`
- `dev/plans/improve-ci-plan.md`

## 8. Definition of done

- All CI-R1 through CI-R10 acceptance signals pass.
- The five adversarial findings are represented by permanent fixtures.
- A Windows-only attribution change selects no ARM64, generic Python, Rust,
  TypeScript, security, wheel, or native matrix job.
- A source-tree Markdown edit selects no scoped heavy job.
- A trusted exact-line lite change runs `verify-fast` and the 14 independent
  jobs, but none of the eight scoped jobs.
- An untrusted or incidental marker cannot activate lite mode.
- A classifier failure is red and still runs `verify-fast`.
- Full-history Gitleaks is dispatchable and release-visible before it leaves the
  per-push job.
- No release publisher depends on advisory CI.
- No ruleset, required check, merge queue, schedule, or soak period is added.
