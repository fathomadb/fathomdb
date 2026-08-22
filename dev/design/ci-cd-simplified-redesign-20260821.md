---
title: CI/CD simplified redesign — single-maintainer correction
date: 2026-08-21
status: PROPOSED
desc: >
  HITL correction to `ci-cd-final-recommendation-20260821.md`: keep CI
  informational and make its cost proportional to the change. Revised seven
  times after adversarial correctness review; this revision closes the
  independent AArch64 trigger, source-tree Markdown, ci-lite trust-boundary,
  root npm classification, and classifier-failure gaps.
  Analysis and recommendation only; no workflow, script, or repository setting
  is changed by this document.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research,
  ci-cd-final-recommendation}-20260821.md; .github/workflows/
  {ci,release,aarch64-release-preflight}.yml; package.json; package-lock.json;
  scripts/security/gitleaks-current.sh; scripts/tests/
  {test_dev_environment_tools,test_shell_lint_ci_job}.sh; scripts/
  {agent-verify,agent-test,agent-security,test-rust-workspace}.sh;
  scripts/lib/{agent-output,agent-suite-run,agent-python-env}.sh;
  src/python/tests/test_slice65_wal_attribution_*.py
---

# CI/CD simplified redesign — single-maintainer correction

**Status: PROPOSED.** Nothing here is implemented. This document supersedes
§2–§4 and §8 of `ci-cd-final-recommendation-20260821.md` completely. §1 of
that document remains a historical challenge evaluation.

## 0. The correction, verbatim

> "The proposed CI needs to be simplified. note this is a single maintainer
> repo. also note that having required soak periods, etc. is not what I am
> trying to achieve. Adding gates, 'requires', etc. is not what I am trying
> to achieve. If a code change does something administrative (correct a
> misspelling in a comment) a full CI should not be needed. If one item in
> Windows-specific is being changed, we should not need the CI to do the
> full run. CI environment checks (e.g. a missing tool like rg, which is
> then needed by CI script) should be rare, and caught in the first 30
> second."

## 1. What is cut

| Cut | Why it does not fit this repository |
|---|---|
| Merge queue | There is one maintainer, not a queue of competing PRs |
| Required-check aggregator jobs | They exist to support required checks |
| Required status checks / required PR reviews | The HITL explicitly declined gates and “requires” |
| Nightly GPU or full-history schedules | A recurring schedule is ceremony and cost |
| Rollout soak periods | “Run clean for N days” is a gate under another name |

CI remains **informational**. Red is useful evidence, not a merge lock.

The branch-protection history must not be confused with current state. The
snapshot at `dev/steward/branch-protection-ruleset.json` describes a deleted
ruleset and is not evidence of current protection. The live checks recorded in
`ci-challenges-review-20260821.md` found rulesets `[]` and legacy `main`
protection returning HTTP 404. Repository-level merge settings independently
allow merge commits, squash merges, and rebases; those settings are not branch
protection.

## 2. The organizing idea

Two independent levers make cost proportional:

1. **Path categories** select jobs whose source or harness dependency changed.
2. **`[ci-lite]`** is a trusted-maintainer assertion that this particular diff
   is administrative and does not need the eight expensive, change-scoped
   jobs.

Path matching cannot distinguish a typo in a Rust comment from a logic change
in the same file. `ci-lite` supplies that missing intent. It never suppresses
`verify-fast` or any of the 14 independent jobs.

## 3. Concrete shape

### 3.1 Classifier

Extend the existing `changes` job. The following is implementation-shaped
pseudocode: action SHAs remain pinned as in the live workflow, and the final
fixture in §3.8 must prove the exact YAML before it lands.

```yaml
permissions:
  contents: read
  pull-requests: read

jobs:
  changes:
    outputs:
      docs_only:              ${{ steps.filter.outputs.nonmd == 'false' }}
      rust:                   ${{ steps.filter.outputs.rust }}
      python:                 ${{ steps.python_non_windows.outputs.python }}
      typescript:             ${{ steps.filter.outputs.typescript }}
      windows:                ${{ steps.filter.outputs.windows }}
      ci_workflow:            ${{ steps.filter.outputs.ci_workflow }}
      verify_harness:         ${{ steps.filter.outputs.verify_harness }}
      rust_test_harness:      ${{ steps.filter.outputs.rust_test_harness }}
      security_harness:       ${{ steps.filter.outputs.security_harness }}
      native_artifact_harness: ${{ steps.filter.outputs.native_artifact_harness }}
      ci_mode:                ${{ steps.ci_mode.outputs.ci_mode }}
    steps:
      - uses: actions/checkout@...
        with:
          fetch-depth: 0
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}

      - name: Determine CI mode
        id: ci_mode
        shell: bash
        env:
          EVENT_NAME: ${{ github.event_name }}
          PR_AUTHOR_ASSOCIATION: ${{ github.event.pull_request.author_association }}
          PR_HEAD_SHA: ${{ github.event.pull_request.head.sha }}
          PR_HEAD_REPOSITORY: ${{ github.event.pull_request.head.repo.full_name }}
          PUSH_SHA: ${{ github.sha }}
          REPOSITORY: ${{ github.repository }}
        run: |
          set -euo pipefail
          candidate="$PUSH_SHA"
          trusted_source=true
          if [ "$EVENT_NAME" = "pull_request" ]; then
            candidate="$PR_HEAD_SHA"
            if [ "$PR_HEAD_REPOSITORY" != "$REPOSITORY" ]; then
              trusted_source=false
            fi
            case "$PR_AUTHOR_ASSOCIATION" in
              OWNER|MEMBER|COLLABORATOR) ;;
              *) trusted_source=false ;;
            esac
          elif [ "$EVENT_NAME" = "push" ]; then
            read -r _first_parent second_parent _rest < <(
              git show -s --format='%P' "$PUSH_SHA"
            )
            if [ -n "${second_parent:-}" ]; then
              candidate="$second_parent"
            fi
          fi
          message="$(git show -s --format='%B' "$candidate")"
          if [ "$trusted_source" = true ] && grep -Fxq -- '[ci-lite]' <<<"$message"; then
            echo "ci_mode=lite" >> "$GITHUB_OUTPUT"
          else
            echo "ci_mode=normal" >> "$GITHUB_OUTPUT"
          fi

      - uses: dorny/paths-filter@fbd0ab8f3e69293af611ebaee6363fc25e6d187d # v4.0.1
        id: filter
        with:
          filters: |
            nonmd: [ '!**/*.md' ]
            windows:
              - 'src/python/tests/test_slice65_wal_attribution_installed.py'
            rust: [ 'src/rust/**', 'Cargo.toml', 'Cargo.lock' ]
            typescript: [ 'src/ts/**' ]
            ci_workflow:
              - '.github/workflows/ci.yml'
            verify_harness:
              - 'scripts/agent-verify.sh'
              - 'scripts/agent-test.sh'
              - 'scripts/lib/agent-output.sh'
              - 'scripts/lib/agent-suite-run.sh'
              - 'scripts/lib/agent-python-env.sh'
              - 'scripts/bootstrap.sh'
            rust_test_harness:
              - 'scripts/test-rust-workspace.sh'
            security_harness:
              - 'scripts/agent-security.sh'
              - 'scripts/bootstrap.sh'
              - 'scripts/security/check-no-listen.sh'
              - 'scripts/security/check-netns-deny-egress.sh'
              - 'scripts/security/check-netns-deny-egress-catch.sh'
              - 'scripts/security/lib-egress-allowlist.sh'
              - 'scripts/security/lib-gate-policy.sh'
              - 'scripts/security/lib-gate-policy.test.sh'
              - 'scripts/security/ast_scan.py'
              - 'scripts/security/check-removal-changelog.sh'
              - 'scripts/security/check_removal_changelog.py'
            native_artifact_harness:
              - 'scripts/release/smoke/smoke-local-native-artifacts.sh'
              - 'scripts/release/smoke/smoke-local-native-artifacts.ps1'

      - uses: dorny/paths-filter@fbd0ab8f3e69293af611ebaee6363fc25e6d187d # v4.0.1
        id: python_non_windows
        with:
          predicate-quantifier: every
          filters: |
            python:
              - 'src/python/**'
              - '!src/python/tests/test_slice65_wal_attribution_installed.py'
```

The second invocation is deliberate. With `predicate-quantifier: every`, a
file contributes to `python` only when it matches the positive Python glob and
does not match the negative Windows-control path. The action still reports a
filter true when **any changed file** satisfies that filter. Therefore:

- the installed Windows control alone gives `windows=true`, `python=false`;
- an ordinary Python file alone gives `windows=false`, `python=true`; and
- a mixed diff containing both gives `windows=true`, `python=true`.

The rejected `python_any AND NOT windows` expression operated on whole-diff
booleans and incorrectly made the third case `python=false`. A global
`predicate-quantifier: every` is also wrong because the Rust category needs OR
semantics across `src/rust/**`, `Cargo.toml`, and `Cargo.lock`. Only the focused
Python-minus-Windows invocation uses `every`.

This focused use matches the action's documented semantics:
[`every` includes a file only when it matches every pattern](https://github.com/dorny/paths-filter#usage),
while the filter output is true when any changed file satisfies the filter.

`pull-requests: read` is required because `paths-filter` uses the pull-request
files API on PR events. The repository has no tracked top-level `python/`
source tree; `src/python/**` is the real surface.

Root `package.json` and `package-lock.json` are deliberately not TypeScript SDK
inputs. The root package identifies itself as private repository Markdown/dev
tooling and points to `src/ts/` as the actual binding. A root npm-tooling change
is an unclassified non-Markdown change: `verify-fast` plus the existing tooling
checks own it, without invoking embedder or native-artifact matrices.

### 3.2 No silent unclassified gap

`verify-fast` remains the baseline, but its condition becomes failure-aware:

```yaml
if: >-
  always() &&
  (
    needs.changes.result != 'success' ||
    needs.changes.outputs.docs_only != 'true'
  )
```

It runs for every non-Markdown change, including unclassified scripts and
configuration. It also runs when checkout, Git, the pull-request files API, the
marker parser, or either matcher makes `changes` fail. The failed classifier
remains visibly red; the eight expensive jobs do not fan out from unknown
outputs. `verify-fast` is never suppressed by `ci-lite`.

GitHub otherwise skips a job when one of its `needs` dependencies fails or is
skipped. The explicit `always()` is therefore part of the baseline guarantee,
not optional defensive syntax. See GitHub's
[workflow syntax for `needs`](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#jobsjob_idneeds).

### 3.3 Windows source boundary

The old `os.uname()` defect is already fixed by `25efa4eb`; it is not an open
blocker. The `windows` category contains exactly
`test_slice65_wal_attribution_installed.py`, the file directly executed by the
Windows attribution job.

`test_slice65_wal_attribution_typing.py` remains ordinary Python because its
assertions run in the heavy Python suite. The two
`scripts/tests/test_windows_wal_*_ci_job.sh` shape fixtures remain fast-only;
`agent-test.sh` already registers them in its fast tier. Routing those fixtures
to a Windows runtime job would be redundant and, for the diagnosis fixture,
would select the wrong Windows job.

### 3.4 Dependency-accurate path-to-job mapping

The former `ci_harness` boolean was both overbroad and incomplete. For example,
`scripts/security/**` made unrelated heavy suites run, while the three libraries
sourced directly by `agent-test.sh` were absent. Use these job-specific
conditions instead:

| Job | Run when this category is true, before Markdown/lite guards |
|---|---|
| `verify` | `rust \|\| python \|\| typescript \|\| verify_harness \|\| rust_test_harness \|\| ci_workflow` |
| `verify-fast` | Any non-Markdown change or classifier failure; never lite-suppressed |
| `rust-workspace-race-report` | `rust \|\| rust_test_harness \|\| ci_workflow` |
| `security` | `rust \|\| python \|\| typescript \|\| security_harness \|\| ci_workflow` |
| `default-embedder-tests` | `rust \|\| python \|\| typescript \|\| ci_workflow` |
| `wheel-size-gate` | `rust \|\| python \|\| ci_workflow` |
| `native-artifact-runtime-validation` | `rust \|\| python \|\| typescript \|\| native_artifact_harness \|\| ci_workflow` |
| `windows-wal-checkpoint-diagnosis` | `rust \|\| ci_workflow` |
| `windows-wal-attribution` | `rust \|\| python \|\| windows \|\| ci_workflow` |
| `markdownlint` | `docs_only == 'true'`; unchanged |
| `design-status` | Always-on; unchanged |
| Remaining 13 always-on jobs | Always-on; unchanged |

Every category-selected scoped job also requires `docs_only != 'true'`.
Language globs intentionally own their entire source trees, which include
README files; the shared condition guard, not eight duplicated matcher
exclusions, preserves the Markdown fast path. `ci_workflow=true` remains the
sole override because changing the shared workflow must exercise all eight jobs.

The 14 always-on jobs are `gitleaks`, `shell-lint`, `board-currency`,
`ledger-integrity`, `plan-anchors`, `governed-surface-pin`,
`pinned-override-rot`, `c1-contract-conformance`, `transcript-hygiene`,
`release-state-views`, `commission-manifest`, `design-status`,
`steward-orient`, and `docs`. They acquire neither `needs: changes` nor a new
`if:`. This preserves the invariant enforced by
`scripts/tests/test_shell_lint_ci_job.sh`.

The categories follow actual call edges:

- `verify_harness` contains `agent-verify.sh`, `agent-test.sh`, the three
  libraries sourced by `agent-test.sh`, and `bootstrap.sh`. It selects
  `verify`, not security, the race report, or default-embedder tests.
- `rust_test_harness` contains `test-rust-workspace.sh`. Both `verify`'s heavy
  Rust suite and the race-report job invoke it, so both run.
- `security_harness` enumerates `agent-security.sh`, `bootstrap.sh`, and the
  exact scripts it executes or sources. It selects the dedicated security job.
  Gitleaks files sharing the `scripts/security/` directory are intentionally
  absent because this job does not execute them. `verify-fast` already exercises
  the ordinary security path for any non-Markdown change.
- `default-embedder-tests` has no harness category: it invokes Cargo and pytest
  directly, then builds and executes the TypeScript test tree; it does **not**
  call `agent-test.sh`. Rust, Python, TypeScript, and `ci_workflow` are its real
  inputs.
- `native_artifact_harness` names only the two smoke scripts that the native
  artifact job invokes. An unrelated `scripts/release/**` edit gets
  `verify-fast` and its release-contract fixtures, not two five-platform
  matrices.

This is dependency-accurate at the job-driver boundary. It does not attempt to
duplicate every Rust/Python/TypeScript test path into harness globs; the language
categories own those source and test trees.

#### 3.4a `ci.yml` edits — coupling chosen and resolved

This design chooses the simple option: a change to
`.github/workflows/ci.yml` sets `ci_workflow=true` and runs all eight
change-scoped jobs. That is intentionally broader than ordinary source routing,
because the modified job must not be skipped by the classifier it defines.
`ci_workflow` also overrides `ci-lite`; a comment-only edit to `ci.yml` is the
small accepted exception to lite proportionality.

This resolves the earlier open Windows-isolation question. Windows jobs stay in
`ci.yml`; no reusable-workflow split is proposed. `.github/workflows/release.yml`
and general `scripts/release/**` changes are separate from `ci_workflow` and do
not accidentally invoke every CI job. Exact native smoke-script changes are
routed by `native_artifact_harness`.

#### 3.4b Independent AArch64 preflight — automatic trigger retired

`.github/workflows/aarch64-release-preflight.yml` currently bypasses this
classifier. Its `push.paths` includes every Rust, Python, and TypeScript path,
so the Windows-only attribution control launches a Linux ARM64 artifact build;
it also ignores `ci-lite`, runs on matching feature-branch pushes, and runs on
tag pushes because GitHub does not evaluate path filters for tags. Recent runs
took about six minutes and included duplicate same-SHA branch/tag executions.

The redesign makes that workflow `workflow_dispatch`-only. The code-grounded
comparison must distinguish existing ownership from genuinely missing evidence:

- `smoke-local-native-artifacts.sh` already stages the ARM64 N-API binary into
  its platform package, creates the actual package tarball with `npm pack`,
  installs it offline, and runtime-smokes it. That is stronger than duplicating
  the old workflow's `npm pack --dry-run`; do not add duplicate staging or pack
  steps.
- The remaining comparison is ABI3/interpreter evidence. The preflight passes
  Python 3.10, 3.11, and 3.12 to maturin, while the proportional native row
  builds and installs with Python 3.11. Before retiring the push trigger, add
  only the missing executable or structural ABI3 assertion identified by the
  fixture; do not assume the three-interpreter build proves three runtime
  installs.

The existing native row already builds and runtime-smokes both the ARM64 Python
wheel and N-API artifact on `ubuntu-24.04-arm`; it is the proportional automatic
owner. The separate workflow remains available for a targeted manual rehearsal,
with no schedule. Trigger retirement and any genuinely missing ABI3 assertion
land atomically.

GitHub documents that `paths` filters are not evaluated for tag pushes in its
[workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax#onpushpull_requestpull_request_targetpathspaths-ignore).
The observed duplicate route is retained as evidence in
[run 32514924075](https://github.com/fathomadb/fathomdb/actions/runs/32514924075).

#### 3.4c Future language splitting

Splitting `verify` by language still requires a real
`--surface=rust|python|typescript` selector in `agent-verify.sh` and
`agent-test.sh`. Until that exists, any selected `verify` run executes its full
heavy three-language tier. This document does not pretend path conditions can
split a monolithic script internally.

### 3.5 Environment failures — early only where useful

Do not add a shared preflight checking tools that a job never uses.

1. Move `verify-fast`'s existing ripgrep installation ahead of Python, Node,
   and Rust setup. Installing a workflow-owned prerequisite early catches an
   install failure before expensive setup.
2. Keep `test_dev_environment_tools.sh` after bootstrap. It checks checkout
   `.venv` tools and root `node_modules/js-yaml`, which cannot exist before
   bootstrap/npm installation.
3. Add equivalent early installation or `Get-Command` checks only to a Windows
   job that actually needs the tool.

There is no generic `git`/`bash`/`tar` preflight: checkout and the active shell
already exercise the first two, while `tar` is job-specific.

### 3.6 `ci-lite` across PRs and `main`

Use a standalone line exactly equal to `[ci-lite]`, not a substring and not
GitHub's native `[skip ci]`. Exact-line matching prevents a commit such as
“document `[ci-lite]` behavior” from accidentally selecting lite mode. Native
skip syntax can suppress whole workflows; this design must never suppress
tag-triggered `release.yml`.

The marker is trusted only on repository `push` events and on pull requests
whose head repository equals `github.repository` and whose author association is
`OWNER`, `MEMBER`, or `COLLABORATOR`. A fork, Dependabot-style, or otherwise
untrusted PR always gets `ci_mode=normal`, even if its head message contains the
exact marker. The same-repository-plus-association conjunction is the bounded
trust surface intended by “maintainer assertion.”

The marker is read from one well-defined candidate commit:

| Event / landing method | Candidate commit whose message is read |
|---|---|
| Trusted same-repository pull request | `github.event.pull_request.head.sha`, not the synthetic merge ref |
| Fork or untrusted-author pull request | No candidate is trusted; always `normal` |
| Merge commit pushed to `main` | The pushed merge commit's second parent (the PR head) |
| Rebase merge pushed to `main` | The pushed tip commit; rebase preserves its message |
| Squash merge pushed to `main` | The resulting squash commit; the maintainer must retain/add `[ci-lite]` in the squash message |
| Direct push to `main` | The pushed tip commit |

Thus a trusted lite PR merged with a merge commit or rebase remains lite on the
subsequent `main` run. A squash can remain lite, but only when its resulting
commit message retains the standalone marker line; the mechanism cannot recover
a marker that GitHub or the maintainer discarded. A direct push can be lite. For
a multi-commit direct push, the tip marker authorizes lite treatment for the
entire pushed diff. These are explicit trust semantics for a single maintainer,
not an inference about whether the diff is truly administrative.

The mechanism avoids duplicate heavy work for lite PRs when the landing method
preserves the marker as specified. Normal PRs still run scoped heavy jobs on the
PR and again after landing; eliminating that duplication across merge, squash,
rebase, and direct pushes would require API-backed association/state that this
simplified design deliberately does not add.

`ci_mode` affects only the eight change-scoped jobs in §3.4. Their condition has
this shape:

```yaml
if: >-
  needs.changes.outputs.ci_workflow == 'true' ||
  (
    needs.changes.outputs.docs_only != 'true' &&
    (<job-specific categories from §3.4>) &&
    needs.changes.outputs.ci_mode != 'lite'
  )
```

`verify-fast` and the 14 always-on jobs are never lite-suppressed. Source-tree
Markdown does not run a scoped job merely because it also matches a language
glob. A `ci.yml` change always runs the eight jobs even if its commit message
contains `[ci-lite]`.

Release safety is structural: marker detection and job conditions exist only
in `ci.yml`, whose push trigger is `main`; `release.yml` neither reads
`ci_mode` nor uses native skip syntax. Tag-triggered publishing therefore still
runs even when the tagged commit message contains `[ci-lite]`.

### 3.7 Work moved off the always-run path

Full-history Gitleaks gets a dedicated
`.github/workflows/gitleaks-history.yml` with `workflow_dispatch` only, plus an
independent advisory job during release. Future GPU/Tegra rehearsals likewise
remain explicitly dispatched or release-time work, not a nightly cron.
`native-artifact-runtime-validation` remains change-scoped CI because it is the
pre-release cross-platform artifact signal.

### 3.8 Fixture-first implementation requirement

Before editing workflow conditions, pass a blocking matcher-oracle checkpoint:
the fixture must demonstrate that it executes code from the exact pinned
`dorny/paths-filter@fbd0ab8f3e69293af611ebaee6363fc25e6d187d` implementation.
Record the adapter, commit pin, and invocation in the fixture header. A
hand-written glob evaluator, a YAML structure parser, root `picomatch` used as a
proxy, or copied expected booleans does not pass this checkpoint. If the pinned
action cannot be executed offline, stop and choose a verifiable exact-source
adapter—such as checksum-pinned vendored action source or distribution invoked
through its real matcher entry point—before changing `ci.yml`; do not weaken the
oracle to keep moving.

After that checkpoint, add the failing routing fixture and observe the required
RED cases against the current workflow. Commit or stage the fixture before
editing workflow behavior. The fixture then becomes green with the workflow
change. This TDD ordering is a firm precondition.

The fixture must cover:

- Markdown-only and unclassified non-Markdown changes;
- a Markdown file under each of `src/rust/**`, `src/python/**`, and `src/ts/**`,
  proving no scoped heavy route fires;
- Rust-, ordinary-Python-, and TypeScript-only diffs;
- root `package.json` and `package-lock.json`, proving `typescript=false` and no
  scoped TypeScript route;
- the installed Windows control alone: `windows=true`, `python=false`;
- the typing test alone: `windows=false`, `python=true`;
- **the installed Windows control plus an ordinary Python file:**
  `windows=true`, `python=true`, and both relevant routes fire;
- each job-specific harness independently, proving that a security-only script
  change does not run `verify`, race-report, or default-embedder jobs;
- `ci.yml` itself, proving all eight scoped jobs run even with `[ci-lite]`;
- a general release script versus each native artifact smoke script;
- a trusted same-repository PR marker, merge-commit second-parent marker,
  rebased-tip marker, squash-tip marker, and direct-push-tip marker;
- a fork PR and an untrusted-author same-repository PR with an exact marker, plus
  a trusted PR with only an incidental marker substring, proving all produce
  `normal`;
- a squash result with no marker, proving it safely produces `normal`;
- lite mode, proving only the eight scoped jobs skip while `verify-fast` and all
  14 independent jobs remain unchanged;
- a tagged commit containing `[ci-lite]`, proving `release.yml` still runs;
- a failed `changes` job, proving `verify-fast` runs while the eight scoped jobs
  do not fan out from unknown outputs;
- the AArch64 workflow, proving it is dispatch-only and that its unique
  assertions have another tested owner; and
- current-tree, dispatchable full-history, and advisory release Gitleaks routes.

The workflow fixture must also pass `actionlint`. A test that only checks
category names would have missed both the old negation bug and the later mixed-
diff boolean bug.

## 4. Gitleaks

`gitleaks-current.sh` scans the full tracked tree, not a diff, on every push and
stays in the existing independent `gitleaks` job. Commit-range scanning through
Gitleaks `--log-opts` requires new script work.

Before removing `gitleaks-history.sh` from the per-push job, create
`.github/workflows/gitleaks-history.yml` with `workflow_dispatch`, a full-history
checkout, the pinned installer, and the existing history script. Also add an
independent `continue-on-error: true` release job that scans the selected release
candidate. No build or publisher job may `need` that advisory job. The dispatch
workflow and release-visible route land atomically with removal from per-push CI.

The history scan stays advisory until the existing allowlist mismatch
(`expected_records=100 observed_records=121 unknown=21`) is reconciled; moving it
does not convert a known noisy baseline into a green claim.

## 5. Explicitly out of scope

- CUDA/maturin toolchain churn;
- claiming `cargo hack check --each-feature` is valid without testing this
  repository's actual feature combinations; and
- required checks, merge queues, soak periods, or nightly ceremony.

## 6. Tradeoff stated plainly

Informational CI trades “a red check forces a choice” for “the maintainer must
look at useful red results.” That is deliberate. Narrow routing and a fast,
never-suppressed baseline make each result cheaper and more meaningful; they do
not turn it into a gate.

## 7. Deferred options, not implementation decisions

The implementation plan resolves this redesign's choices: keep all 14
independent jobs unconditional, do not build `--surface=`, and retain the fast
full-tree Gitleaks baseline. Scoping governance jobs, splitting the heavy
verifier, and adding commit-range Gitleaks remain possible future work, but none
is a prerequisite or an authorized expansion of this implementation.

The `[ci-lite]` marker shape and trust boundary are resolved in §3.6. Windows
workflow routing, the independent AArch64 trigger, and fixture sequencing are
also no longer open: §3.4a chooses `ci.yml` coupling, §3.4b makes the redundant
preflight dispatch-only after evidence migration, and §3.8 requires
fixture-first implementation.

## 8. Revision history

- **Round 1:** corrected source paths, invalid workflow-level `if:`, a
  wrong-runner shared preflight, Windows path claims, language splitting, and
  stale Gitleaks framing.
- **Round 2:** removed native skip syntax that could suppress releases; completed
  path/job mapping; made Windows workflow isolation and Gitleaks scan behavior
  honest; added permissions and fixtures.
- **Round 3:** added proportional `ci-lite`/Windows routing; preserved always-on
  job independence; fixed PR-head checkout, output declaration, table facts,
  and premature environment checks.
- **Round 4:** corrected `paths-filter` negation semantics, narrowed the Windows
  source category, kept CI-shape fixtures fast-only, added initial harness
  routing, and made fixture-first sequencing mandatory.
- **Round 5:** replaced the mixed-diff-invalid whole-diff Python boolean with a
  focused `predicate-quantifier: every` matcher; propagated lite intent through
  merge/rebase and explicitly specified squash/direct behavior; replaced the
  shared harness bucket with dependency-accurate categories; made `ci.yml`
  changes run every scoped job while separating release scripts; and
  distinguished the deleted protection snapshot from independent repository
  merge settings.
- **Round 6:** added the missing TypeScript route for `default-embedder-tests`,
  which compiles and executes tests sourced from `src/ts/**` directly.
- **Round 7:** specified retirement of the classifier-bypassing AArch64 push
  route after its unique evidence moves; guarded language-tree Markdown from
  scoped jobs; restricted lite mode to exact-line markers from trusted
  repository refs; separated root npm dev tooling from the TypeScript SDK; made
  `verify-fast` survive classifier failure; and specified concrete
  dispatch/release homes for full-history Gitleaks.

## Appendix — pipeline diagram (v7)

```text
             FathomDB CI — informational, proportional, single-maintainer

 changed files
      |
      v
 +------------------------- changes --------------------------+
 | positive filters: rust / typescript / windows              |
 | focused every-filter: python excluding installed control   |
 | job drivers: verify / rust-test / security / native smoke  |
 | self-change: ci_workflow                                   |
 | mode: trusted exact-line marker; fork PRs always normal    |
 +----------------------------+-------------------------------+
                              |
             +----------------+----------------+
             |                                 |
             v                                 v
  never suppressed                    eight scoped heavy jobs
  ----------------                    ------------------------
  verify-fast (non-md/failure)          selected by source/driver
  14 independent jobs                  skipped by [ci-lite]
  markdownlint (docs-only)              docs_only guard on every route

 windows control alone -> windows=true, python=false
 windows + ordinary Python -> windows=true, python=true
 security harness -> security only (+ invariant fast/independent jobs)
 ci.yml change -> all eight scoped jobs; no unsafe self-classification gap
 classifier failure -> red + verify-fast; no unknown heavy fan-out
 AArch64 preflight -> workflow_dispatch only after evidence migration

 current-tree Gitleaks -> per push | history -> dispatch + release advisory
 [ci-lite] is not native [skip ci] and never controls release.yml
 tag push -> release workflow still runs

 no required checks | no merge queue | no soak | no nightly ceremony
```
