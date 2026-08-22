---
title: CI/CD simplified redesign — single-maintainer correction
date: 2026-08-21
status: PROPOSED
desc: >
  HITL correction to `ci-cd-final-recommendation-20260821.md`: that doc's
  tiers/aggregators/required-checks/rollout-soak shape is more ceremony than
  a single-maintainer repo needs. This doc keeps that doc's diagnosis (the
  challenge catalogue, the delivery-requirements map) and restates, rather
  than inherits, the durable facts underneath its now-rejected prescription.
  Four times revised after adversarial correctness reviews (§8): round 1
  fixed wrong source paths, an invalid workflow-level `if:`, and a preflight
  design that validated the wrong runner; round 2 fixed a release-suppression
  hazard in the skip mechanism, an incomplete path→job mapping, an
  unsolvable Windows-path-isolation claim, a still-too-blunt preflight, and
  a gitleaks scan that was never actually diff-scoped; round 3 fixed the
  redesign still amounting to nearly-full-CI for ordinary changes, an
  always-on-invariant violation in the skip mechanism, a PR-head-commit bug
  in that same mechanism, four more factual table errors, and a
  premature-dependency bug in the environment-check relocation; round 4
  fixed a paths-filter exclusion that doesn't actually exclude, a
  merge-to-main marker-loss hazard, a Windows-category test-coverage
  misclassification, and a missing CI-harness-script category in the
  path→job mapping. Analysis and recommendation only; no CI config, script,
  or GitHub setting is changed by this document.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research,
  ci-cd-final-recommendation}-20260821.md; dev/steward/
  branch-protection-ruleset.json; .github/workflows/{ci,release}.yml;
  scripts/security/gitleaks-current.sh; scripts/tests/
  {test_dev_environment_tools,test_shell_lint_ci_job}.sh; scripts/{
  agent-verify,agent-test,agent-security,test-rust-workspace}.sh;
  src/python/tests/test_slice65_wal_attribution_*.py
---

# CI/CD simplified redesign — single-maintainer correction

**Status: PROPOSED.** Nothing here is implemented. This document supersedes
§2–§4 and §8 of `ci-cd-final-recommendation-20260821.md` completely. §1 of
that document (the challenge evaluation) still holds as a historical record.

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

## 1. What's cut from the prior recommendation, and why

| Cut | Why it doesn't fit a single-maintainer repo |
|---|---|
| Merge queue *(already dropped by stage-3 research)* | No concurrent human PRs to serialize |
| `gate-fast` / `gate-build` aggregator jobs | Exist only to be **required** checks |
| Re-applying branch protection as required-status-checks | This *is* the "gates/requires" being declined |
| Nightly `schedule` cron for GPU/gitleaks-full-history | A recurring commitment is its own ceremony |
| "Run clean for a few days" rollout gate | A soak period by another name |

## 2. The organizing idea

CI becomes **informational, not a gate**, and cost becomes **proportional to
what the diff touches**, extending the `dorny/paths-filter` job already in
`.github/workflows/ci.yml` (lines 113–129) rather than adding a new taxonomy
job. Two independent levers do the proportionality work:

1. **Path scoping** (§3.1/§3.4) — which *category* of source changed.
2. **`ci-lite` mode** (§3.6) — an explicit, maintainer-asserted signal that
   *this particular change*, whatever category it falls in, doesn't need the
   heavy jobs. This is what actually answers "a comment fix in a Rust file
   shouldn't run full CI" — path scoping alone cannot, because paths-filter
   sees files, not content, and a comment edit and a logic edit are the same
   path.

## 3. Concrete shape

### 3.1 Path-filter categories — corrected paths, a real `windows` category via expression (not negation), required permission

```yaml
  changes:
    outputs:
      docs_only: ...            # unchanged
      rust:          ${{ steps.filter.outputs.rust }}
      python:        ${{ steps.python.outputs.result }}
      typescript:    ${{ steps.filter.outputs.typescript }}
      windows:       ${{ steps.filter.outputs.windows }}
      release_infra: ${{ steps.filter.outputs.release_infra }}
      ci_harness:    ${{ steps.filter.outputs.ci_harness }}
      ci_mode:       ${{ steps.ci_mode.outputs.ci_mode }}
    steps:
      - uses: actions/checkout@...
        with:
          # Required for step "Determine CI mode" below to read the actual
          # authored commit, not a synthetic merge commit — see §3.6.
          ref: ${{ github.event_name == 'pull_request' && github.event.pull_request.head.sha || github.sha }}
      - name: Determine CI mode from the latest commit message
        id: ci_mode
        run: |
          # ci-lite ONLY ever applies on pull_request runs — see §3.6 Defect 4.
          if [ "${{ github.event_name }}" != "pull_request" ]; then
            echo "ci_mode=normal" >> "$GITHUB_OUTPUT"
          else
            msg="$(git log -1 --pretty=%B)"
            if [[ "$msg" == *"[ci-lite]"* ]]; then
              echo "ci_mode=lite" >> "$GITHUB_OUTPUT"
            else
              echo "ci_mode=normal" >> "$GITHUB_OUTPUT"
            fi
          fi
      - uses: dorny/paths-filter@fbd0ab8f3e69293af611ebaee6363fc25e6d187d # v4.0.1
        id: filter
        with:
          filters: |
            nonmd: [ '!**/*.md' ]
            windows:
              - 'src/python/tests/test_slice65_wal_attribution_installed.py'
            python_any: [ 'src/python/**' ]
            rust: [ 'src/rust/**', 'Cargo.toml', 'Cargo.lock' ]
            typescript: [ 'src/ts/**', 'package.json', 'package-lock.json' ]
            release_infra: [ '.github/workflows/**', 'scripts/release/**' ]
            ci_harness:
              - 'scripts/agent-verify.sh'
              - 'scripts/agent-test.sh'
              - 'scripts/agent-security.sh'
              - 'scripts/test-rust-workspace.sh'
              - 'scripts/security/**'
      - name: Derive python (python_any minus windows), without relying on paths-filter negation
        id: python
        run: |
          if [ "${{ steps.filter.outputs.python_any }}" = "true" ] && [ "${{ steps.filter.outputs.windows }}" != "true" ]; then
            echo "result=true" >> "$GITHUB_OUTPUT"
          else
            echo "result=false" >> "$GITHUB_OUTPUT"
          fi
```

Corrections in this revision:

- **The `!excluded`-pattern approach from round 3 is gone.** `dorny/paths-
  filter@v4.0.1` defaults to `predicate-quantifier: some` (OR across a
  filter's own pattern list), and under that mode a negation pattern
  doesn't reliably exclude — a Slice 65 file change would have set both
  `windows=true` and `python=true`, defeating the entire point of carving
  `windows` out and silently restoring `verify`, `security`, `wheel-size-
  gate`, and the 5-platform matrix for what should have been a
  windows-only change. Flipping the quantifier globally to `every` isn't a
  fix either — it would break `rust`, whose filter needs `src/rust/**` OR
  `Cargo.toml` OR `Cargo.lock`, not all three at once. The fix computes the
  exclusion as a plain shell/GitHub-Actions-expression boolean
  (`python_any AND NOT windows`) in a separate step, entirely outside
  paths-filter's own pattern-matching semantics — correct regardless of
  which quantifier mode is active, because it doesn't depend on one.
- **The `windows` category is narrowed to exactly one file.** Round 3's
  glob `test_slice65_wal_attribution_*.py` matched **both**
  `test_slice65_wal_attribution_installed.py` and its sibling
  `test_slice65_wal_attribution_typing.py` — but `windows-wal-attribution`
  (`ci.yml`, confirmed by reading the job body directly) invokes **only**
  `test_slice65_wal_attribution_installed.py`; `_typing.py`'s assertions
  run exclusively inside the heavy Python suite (pytest auto-collects it
  from `src/python/tests/`, and grepping the entire `windows-wal-
  attribution` job body for `_typing` returns zero matches). A change
  confined to `_typing.py` needs `verify`'s heavy Python job to actually
  execute its assertions, not the Windows job — so `_typing.py` stays
  ordinary `python`, only `_installed.py` is `windows`.
- **`scripts/tests/test_windows_wal_diagnosis_ci_job.sh` and
  `test_windows_wal_attribution_ci_job.sh` are removed from the `windows`
  category entirely.** Both are already registered directly in `agent-
  test.sh`'s `fast` tier (`run_tier_suite fast test-windows-wal-diagnosis-
  ci-job ...` / `test-windows-wal-attribution-ci-job ...`, confirmed by
  reading `scripts/agent-test.sh`), so `verify-fast` already runs them
  unconditionally on every non-doc push regardless of category. Round 3's
  inclusion was both redundant and actively wrong: a change to the
  *diagnosis* fixture would have routed through the `windows` category into
  the `windows-wal-attribution` **job condition** — the wrong job.
- **A new `ci_harness` category** covers the scripts that actually *drive*
  the heavy jobs (`scripts/agent-verify.sh`, `scripts/agent-test.sh`,
  `scripts/agent-security.sh`, `scripts/test-rust-workspace.sh`,
  `scripts/security/**` — confirmed `agent-security.sh` drives the
  `security` job by reading `ci.yml`'s own comments referencing it
  directly). Without this, a change to the harness itself — the thing that
  defines what "heavy" even means — fell through to "unclassified,
  `verify-fast` only," which is backwards: the harness change is exactly
  what needs the jobs it drives to actually run. See §3.4 for how it's
  wired into each job's condition.
- `pull-requests: read` and the note about `python/` not being a tracked
  path are unchanged from round 2 and still correct.

### 3.2 No silent "unclassified" gap

`verify-fast` keeps its current `if: needs.changes.outputs.docs_only !=
'true'` condition unchanged — runs on any non-doc change, is **not**
suppressed by `ci_mode=lite`, and is never zero-signal.

### 3.3 Windows scoping — an honest boundary, unchanged from round 2

The `os.uname()` staleness is fixed (`25efa4eb`,
`src/python/fathomdb/_coinstall.py`) — stop describing it as an open
blocker. A Windows-specific *workflow step* still cannot be path-scoped,
because every job lives in one `ci.yml` file (§7 lists the two real
options; this document still doesn't choose between them). The
*source-level* Windows scoping (§3.1's `windows` category, now correctly
narrowed) is real and specified, independent of that unresolved question.

### 3.4 Authoritative path→job mapping — corrected again

Verified directly against `.github/workflows/ci.yml`, `scripts/agent-
test.sh`, and `scripts/agent-security.sh`:

| Job | rust | python | typescript | windows | release_infra | ci_harness | `ci-lite` suppressible? |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| `verify` | ✓ | ✓ | ✓ | | | ✓ | **yes** |
| `verify-fast` | (any non-doc change — own row; see note) | | | | | | **no** — the check `ci-lite` exists to preserve |
| `rust-workspace-race-report` | ✓ | | | | | ✓ | **yes** — `bash scripts/test-rust-workspace.sh --parallel-report`, Rust-only, not part of `verify` |
| `security`, `default-embedder-tests` | ✓ | ✓ | ✓ | | | ✓ | **yes** |
| `wheel-size-gate` | ✓ | ✓ | | | ✓ | | **yes** |
| `native-artifact-runtime-validation` | ✓ | ✓ | ✓ | | ✓ | | **yes** — stays per-push, not release-only |
| `windows-wal-checkpoint-diagnosis` | ✓ | | | | | | **yes** |
| `windows-wal-attribution` | ✓ | ✓ | | ✓ | | | **yes** — condition `rust \|\| python \|\| windows` |
| `markdownlint` | (`docs_only == 'true'`-gated) | | | | | | n/a |
| `design-status` | ALWAYS-ON (see below) | | | | | | n/a |
| 14 always-on jobs (below) | | | | | | | n/a — never touched by `ci_mode` |

**`ci_harness`'s effect on job conditions:** `verify`, `verify-fast`,
`default-embedder-tests` add `|| ci_harness` (their harness is `agent-
verify.sh`/`agent-test.sh`, which drives all three); `rust-workspace-race-
report` becomes `rust || ci_harness` (its own harness script,
`test-rust-workspace.sh`, is itself part of the `ci_harness` category —
accepted as slightly broader than the strict minimum, proportionate rather
than precise, consistent with this document's stance elsewhere); `security`
becomes `rust || python || typescript || ci_harness` (covers `agent-
security.sh` and `scripts/security/**`). `verify-fast` already runs on any
non-doc change regardless, so `ci_harness` doesn't change its behavior —
it's listed for completeness, not because it alters that row.

`verify-fast`'s row note: it fires on **any** non-md change — broader than
"rust or python or typescript or release_infra or ci_harness" individually
(e.g. a genuinely unclassified script change still triggers it). Its own
row prevents the table implying it's gated the same way the heavy jobs are.

**`markdownlint` is `docs_only == 'true'`-gated. `design-status` is NOT** —
read directly (`.github/workflows/ci.yml:1208`): no `if:`, no `needs:` at
all, deliberately, because `markdownlint` is docs-only-gated and would
otherwise miss a design-doc change landing alongside code.

**The always-on set is 14 jobs**: `gitleaks` (current-tree step),
`shell-lint`, `board-currency`, `ledger-integrity`, `plan-anchors`,
`governed-surface-pin`, `pinned-override-rot`, `c1-contract-conformance`,
`transcript-hygiene`, `release-state-views`, `commission-manifest`,
`design-status`, `steward-orient`, `docs`. **None of these are touched by
`ci_mode` in any way.**

**Explicit decision on `native-artifact-runtime-validation`** (unchanged):
stays in `ci.yml`, scoped to `rust || python || typescript ||
release_infra`, not moved to release-only — real cross-platform build
signal that would otherwise surface days after the causing commit.

#### 3.4a What needs real script work before it's achievable

Splitting `verify` by language still requires a `--surface=rust|python|
typescript` selector added to `scripts/agent-verify.sh` /
`scripts/agent-test.sh` — unchanged from round 2. Until then, any of
`rust`/`python`/`typescript`/`ci_harness` triggers the full three-language
`verify` run.

### 3.5 Environment checks — minimal, per-job, no generic pre-bootstrap check

Round 3 added a generic pre-bootstrap host-capability check (`git`, `bash`,
`tar`) as its own first stage. Dropped in this revision: `git`/`bash` are
already exercised by the checkout step and the job's own shell before any
custom step runs, and `tar` is job-specific, not universal — the check
provided negligible real signal for what it cost to maintain. **Prefer
minimal per-job dependency installation/checking over a shared blanket
pre-check.**

What's kept, both unchanged from round 2/3's genuinely useful findings:

1. **Early installation of tools the workflow owns** (e.g. `rg`): move the
   existing `apt-get install ripgrep` step in `verify-fast` to before
   Python/Node/Rust setup. Installing early — not checking early for a tool
   that gets installed regardless — is what actually shortens the failure
   path.
2. **`test_dev_environment_tools.sh` stays exactly where it already runs**
   (inside `agent-test.sh`'s fast tier, after bootstrap). Reading it
   directly, it checks `.venv/bin/ruff`, `.venv/bin/pyright`, and root
   `node_modules/js-yaml` — none of which exist before `scripts/
   bootstrap.sh`/`npm ci` have run, so an earlier relocation would fail
   every job unconditionally, before bootstrap gets a chance to run. Its
   job is validating that bootstrap produced pinned-correct tool versions,
   which is only meaningful *after* bootstrap has run.

Jobs that don't call `agent-test.sh`/`agent-verify.sh` (e.g. `docs`,
`markdownlint`) don't need either of these. Platform-specific jobs still
need the Windows equivalent (`Get-Command` in `pwsh`) wherever an
equivalent workflow-owned-tool install exists.

### 3.6 `ci-lite` mode — four corrected defects across three rounds

**Defect 1 (round 3)** — would have violated the always-on invariant
`test_shell_lint_ci_job.sh` Arm B enforces. Fixed: `ci_mode` only applies
to the 8 jobs in §3.4's rightmost column, which already have `needs:
changes`; the 14 always-on jobs are never touched.

**Defect 2 (round 3)** — the `mode` output was referenced but never
declared. Fixed: `ci_mode` is declared in the `changes` job's `outputs:`
block (§3.1).

**Defect 3 (round 3)** — `git log -1` after default checkout on a
`pull_request` event reads the synthetic merge commit, not the
contributor's head commit. Fixed: checkout uses `${{
github.event.pull_request.head.sha }}` on PR events (§3.1).

**Defect 4 (this round) — the marker is lost on landing to `main`.** The
PR run reads the head commit correctly (Defect 3's fix), but the
subsequent `push`-to-`main` run — triggered by the merge itself — reads
whatever commit that push actually is, and this repository's branch
protection (`dev/steward/branch-protection-ruleset.json`)
`allowed_merge_methods` permits `merge`, `squash`, **and** `rebase`, not
one fixed strategy. A real example: `git show -s --format='%B' 728b5794`
reads

```text
Merge pull request #242 from fathomadb/fix/0.823-cuda-candidate-record-sha

fix(release): correct CUDA candidate sha
```

— GitHub's auto-generated merge message (PR number and title), which does
not preserve a `[ci-lite]` marker from the original head commit. A
strategy-aware fix (scan the pushed commit range, handle merge vs. squash
vs. rebase differently) is real engineering work disproportionate to what
this document is trying to keep simple.

**Resolved with a deliberately simple policy instead: `[ci-lite]` only ever
applies on `pull_request`-triggered runs.** Every `push`-to-`main` run
(`ci.yml`'s `push` trigger is `main`-only) always computes `ci_mode=normal`
— full, scoped-but-not-lite CI — regardless of any commit message,
regardless of merge strategy (§3.1's `ci_mode` step checks `github.
event_name` before ever reading a commit message). `ci-lite` exists for
fast iteration feedback while a change is still under review; once it lands
on `main` you want real confidence regardless of how the PR was merged.
This isn't a limitation being apologized for — it's a deliberate choice
that avoids merge-strategy-dependent commit-message parsing entirely,
consistent with this document's stance against building more machinery
than the problem needs.

**What's unchanged:**

- **Suppression scope**: exactly the 8 jobs in §3.4's rightmost column;
  `verify-fast` and all 14 always-on jobs are never suppressed.
- **Multi-commit-push ambiguity within a single PR** (before merge):
  `[ci-lite]` on the latest commit authorizes `ci-lite` treatment for the
  entire pushed diff — a deliberate choice, trusting the maintainer's
  stated intent rather than programmatically second-guessing it, the same
  trust model this design already applies to every other informational,
  non-gating check.
- **Release safety**: `ci_mode` is read and applied only inside `ci.yml`;
  `release.yml` never references it, so a `[ci-lite]`-tagged commit cannot
  suppress a release — and Defect 4's fix makes this even more robust,
  since `ci_mode` is now `normal` unconditionally on any `push` event,
  including a tag push, without even needing to read that event's commit
  message.

### 3.7 What actually moves off the always-run path

Unchanged: full-history gitleaks scanning (§4) and any future GPU/Tegra
rehearsal work move to `workflow_dispatch` plus the existing tag-triggered
`release.yml`. `native-artifact-runtime-validation` stays per-push (§3.4).

### 3.8 A fixture requirement before any YAML changes land — decided, not open

**This is now a firm precondition, not an open question** (§7 previously
listed "before or alongside" as unresolved — resolved to **before**, TDD,
no exceptions, consistent with this repo's own convention of testing CI
shape rather than trusting comments, e.g. `test_shell_lint_ci_job.sh` and
`test_dev_environment_tools_contract.sh`).

**Critically: the fixture must exercise the real `dorny/paths-filter`
matching engine** (e.g. a scratch `workflow_dispatch` run against
representative diffs, or a local harness using the actual matching library
the action uses) — **not** a structural/static YAML parse that only checks
category names line up syntactically. A structural-only fixture is exactly
the kind of check that would have missed this round's P1-1 defect (the
`!`-negation pattern *looked* correct in the YAML; it just didn't behave
correctly under `predicate-quantifier: some`), so a structural fixture does
not satisfy this requirement.

Representative diffs to cover:

- Markdown-only
- An unclassified script/config change — asserts `verify-fast` still runs,
  unaffected by `ci_mode`
- Python-only (`src/python/**`, excluding
  `test_slice65_wal_attribution_installed.py`)
- TypeScript-only (`src/ts/**`)
- Rust core (`src/rust/**`)
- **A change confined to `test_slice65_wal_attribution_installed.py`
  alone** — asserts `windows=true`, `python=false` from the classifier
  (via the derived expression, not a paths-filter negation), and that the
  job condition `rust || python || windows` fires `windows-wal-attribution`
  while `verify`/`security`/`default-embedder-tests`/`wheel-size-gate`/
  `native-artifact-runtime-validation` do not
- **A change confined to `test_slice65_wal_attribution_typing.py` alone**
  — asserts `windows=false`, `python=true`, `verify`'s heavy Python suite
  fires (it's the only thing that actually runs this file's assertions),
  and `windows-wal-attribution` does **not**
- A `ci_harness` change (e.g. `scripts/agent-verify.sh`) — asserts `verify`/
  `verify-fast`/`default-embedder-tests`/`security`/`rust-workspace-race-
  report` all fire even though no `rust`/`python`/`typescript` path matched
- A release script (`scripts/release/**`)
- `ci.yml` itself — asserts `release_infra` scope
- A mixed-surface diff touching two categories at once
- A commit carrying `[ci-lite]` on a `pull_request` run, touching
  `src/rust/**` — asserts the 8 suppressible jobs do not fire while
  `verify-fast` and the always-on set do
- **A `push`-to-`main` run simulating a merge of a `[ci-lite]`-tagged PR**
  (e.g. a synthetic merge-commit message like `728b5794`'s, with no
  marker) — asserts `ci_mode=normal` regardless, proving Defect 4's fix

## 4. Gitleaks — unchanged from round 2/3

The gitleaks job was already made non-blocking on `main` by `a755e1d8`.
`gitleaks-current.sh` scans the full tracked tree, not a diff, every push.
Commit-range scanning via `--log-opts` needs new script work; full-history
scanning moves to `workflow_dispatch` plus a real step in `release.yml`
(which has none today) and stays advisory until the existing allowlist
mismatch (`expected_records=100 observed_records=121 unknown=21`) is
reconciled. Rounds 3 and 4 both confirmed this section as already correct.

## 5. Explicitly out of scope

Unchanged: CUDA/maturin toolchain churn; `cargo hack check --each-feature`
remains unverified for this repo's actual feature combinations.

## 6. What this doesn't solve, said plainly

Unchanged from round 2: removing required checks trades a red-check-forces-
a-choice failure mode for a red-check-nobody-is-forced-to-look-at risk,
deliberately, for a single maintainer. Fine-grained scoping (§3) and
`ci-lite` (§3.6) narrow *what* runs and *why*, which makes a red result mean
more, but neither makes looking at it automatic.

## 7. Open decisions for HITL

1. **Windows job isolation** (§3.3): accept the `release_infra` coupling
   for `ci.yml`-level edits, or invest in a separate reusable workflow file?
2. **The 14 always-on jobs** (§3.4): keep unconditional, or scope some to
   `dev/plans|steward|design/**`?
3. **`--surface=` selector for `agent-verify.sh`** (§3.4a): worth building?
4. **Commit-range gitleaks scanning** (§4): worth building, or leave
   full-tree-but-fast as the every-push baseline?
5. **`[ci-lite]` marker text and its "authorizes the whole push" semantics
   within a PR** (§3.6): accept as specified, or want a narrower/different
   convention?

*(Fixture-first sequencing is no longer an open question — §3.8 resolves it
to "before," firmly.)*

## 8. Revision history

- **Round 1** fixed: wrong `src/python`/`src/ts` paths, an invalid
  workflow-level `if:` for a `[ci-lite]` idea, a shared `preflight` job
  that validated the wrong runner, an over/under-matching `windows_
  relevant` glob, an unsupportable per-language `verify`-split claim, and a
  stale gitleaks-triage framing that predated `a755e1d8`.
- **Round 2** fixed: a tag-push/release-suppression hazard in the native
  `[skip ci]` recommendation, an incomplete and internally-contradictory
  path→job mapping, an unfixable "Windows-specific workflow step"
  path-filter claim plus a stale `os.uname()` blocker, a preflight design
  that checked tools jobs didn't actually need, a gitleaks scan wrongly
  described as diff-relevant when it's full-tree, a too-tentative
  permissions claim, and a missing test-fixture requirement.
- **Round 3** fixed: the redesign still amounting to nearly-full-CI for
  ordinary changes (added `ci-lite` mode and a `windows` path category);
  an always-on-invariant violation and undeclared output in the skip
  mechanism; a wrong-commit-on-PR-events bug in the same mechanism;
  `design-status` misclassified as docs-only when it's always-on (13→14);
  `rust-workspace-race-report` wrongly merged into the 3-language `verify`
  row; `verify-fast` wrongly implied to share the other jobs' category
  semantics; a wrong classifier-output assertion in the Windows fixture;
  and a premature-dependency bug in the environment-check relocation.
- **Round 4** (this revision) fixed: the `!`-negation `windows` exclusion
  not actually working under `dorny/paths-filter`'s default
  `predicate-quantifier: some` (§3.1 — replaced with a derived boolean
  expression, verified via `python_any AND NOT windows` computed outside
  paths-filter's own matching); `[ci-lite]` being lost when a PR lands on
  `main` because the post-merge `push` run reads a GitHub-generated merge
  commit message that doesn't preserve it (§3.6 Defect 4 — resolved by
  scoping `ci-lite` to `pull_request` runs only, verified against a real
  merge commit, `728b5794`, and this repo's `merge`/`squash`/`rebase`-
  permissive branch protection); the `windows` category wrongly including
  `test_slice65_wal_attribution_typing.py` (whose assertions only run in
  the heavy Python suite, never in the Windows job — verified by grepping
  the job body directly) and both `test_windows_wal_*_ci_job.sh` fixtures
  (already covered unconditionally by `verify-fast`, verified against
  `agent-test.sh`'s fast-tier registration); and a missing `ci_harness`
  category so changes to the scripts that drive the heavy jobs
  (`agent-verify.sh`, `agent-test.sh`, `agent-security.sh`,
  `test-rust-workspace.sh`, `scripts/security/**`) actually trigger them
  instead of falling through to `verify-fast` alone. Also resolved §3.8's
  fixture-sequencing question from open to decided ("before," firmly), and
  dropped round 3's generic pre-bootstrap host-capability check as
  low-signal per P2 feedback.
- All four rounds confirmed the policy direction (§0–§2) is correct; only
  the mechanics were wrong, progressively less so each round.

## Appendix — pipeline diagram (v5, corrected)

```text
                  FathomDB CI -- simplified, single-maintainer shape (v5)
              (dev/design/ci-cd-simplified-redesign-20260821.md, PROPOSED)
                    informational only -- nothing here is a gate

  TWO INDEPENDENT SCOPING LEVERS
  ================================================================
  1. PATH CATEGORY  -- rust/python/typescript/windows/release_infra/
                        ci_harness -- §3.1/§3.4
  2. ci-lite MODE    -- maintainer asserts THIS diff doesn't need the heavy
                         jobs -- PR-ONLY now (§3.6 Defect 4), never on push

  changes JOB (corrected mechanics, round 4)
  ================================================================
  1. checkout ref: pull_request.head.sha (PR) / github.sha (push)
  2. ci_mode: if event != pull_request -> ALWAYS "normal", no commit-
     message read at all. Only on pull_request: read git log -1 --pretty=%B
     for "[ci-lite]". FIXES: a PR's [ci-lite] marker used to vanish on
     landing because the post-merge push reads a GitHub-generated merge
     commit message (real example: 728b5794 = "Merge pull request #242
     from .../fix-...\n\nfix(release): correct CUDA candidate sha" -- no
     marker, and this repo's branch protection permits merge/squash/rebase,
     so no single strategy-aware fix would cover all three). Scoping to
     PR-only sidesteps the whole class of problem instead of chasing it.
  3. dorny/paths-filter (v4.0.1, permissions: pull-requests: read REQUIRED)
     computes PURE POSITIVE filters only:
       windows:     src/python/tests/test_slice65_wal_attribution_installed.py
                     (ONLY this file -- NOT _typing.py, NOT the two
                      windows_wal_*_ci_job.sh fixtures -- see below)
       python_any:  src/python/**
       rust, typescript, release_infra, ci_harness: unchanged/new
  4. a SEPARATE step derives python = python_any AND NOT windows as a
     plain boolean expression -- NOT a paths-filter `!` negation pattern.
     FIXES: v4.0.1 defaults to predicate-quantifier=some (OR across a
     filter's patterns); under that mode a `!excluded` pattern inside the
     SAME filter list doesn't reliably exclude -- round 3's version would
     have set BOTH windows=true AND python=true for a Slice 65 file change,
     silently restoring verify/security/wheel-size-gate/the 5-platform
     matrix for what should've been windows-only. Global quantifier=every
     isn't a fix either (breaks rust's OR-of-3-paths filter). Computing the
     exclusion OUTSIDE paths-filter sidesteps the whole defect class.

  WHY windows IS NARROWED TO ONE FILE (round 4 correction)
  ================================================================
  test_slice65_wal_attribution_typing.py: assertions run ONLY in verify's
    heavy Python suite (pytest auto-collects it) -- windows-wal-attribution
    job NEVER invokes it (grepped the whole job body: 0 matches). Stays
    ordinary `python`, not `windows`.
  test_windows_wal_diagnosis_ci_job.sh / test_windows_wal_attribution_ci_
    job.sh: ALREADY run unconditionally by verify-fast (registered in
    agent-test.sh's fast tier). Including them in `windows` was redundant
    AND wrong -- would route a diagnosis-fixture change to the
    ATTRIBUTION job's condition, the wrong job. Removed from the category.

  ci_harness -- NEW category (round 4), fixes a real gap
  ================================================================
  scripts/agent-verify.sh, agent-test.sh, agent-security.sh,
  test-rust-workspace.sh, scripts/security/** -- these DRIVE the heavy
  jobs. A change to the harness itself used to fall through to
  "unclassified -> verify-fast only," backwards: the harness change is
  exactly what needs the jobs it drives to run.
    verify / verify-fast / default-embedder-tests: += || ci_harness
    rust-workspace-race-report:                     rust || ci_harness
    security:                    rust || python || typescript || ci_harness

  AUTHORITATIVE TABLE (corrected again this round)
  ======================================================================
  job                              r  p  ts win relinfra harness ci-lite?
  --------------------------------- -- -- -- --- -------- ------- --------
  verify                             y  y  y  --   --      y      YES
  verify-fast                        (any non-md change -- own row)  NO --
                                                                      preserved
  rust-workspace-race-report         y  --  --  --   --      y      YES
  security, default-embedder-tests   y  y  y  --   --      y      YES
  wheel-size-gate                    y  y  --  --   y       --     YES
  native-artifact-runtime-           y  y  y  --   y       --     YES  <- stays
    validation (5-platform matrix)                                    per-push
  windows-wal-checkpoint-diagnosis   y  --  --  --   --      --     YES
  windows-wal-attribution            y  y  --  y    --      --     YES
  design-status                      ALWAYS-ON (ci.yml:1208 -- no if:/
                                      needs: at all; NOT docs_only-gated)
  markdownlint                       (docs_only=='true' ONLY)
  14 always-on jobs                  NEVER touched by ci_mode -- adding
                                      needs: changes would violate
                                      test_shell_lint_ci_job.sh Arm B

  RESULT: a diff confined to test_slice65_wal_attribution_installed.py --
    windows=true, python=false -> ONLY windows-wal-attribution + verify-fast
    + the 14 always-on jobs run.
  RESULT: a diff confined to test_slice65_wal_attribution_typing.py --
    windows=false, python=true -> verify (heavy Python suite) runs;
    windows-wal-attribution does NOT (it never executes this file anyway).
  RESULT: [ci-lite] on a PR touching src/rust/** -- the 8 suppressible jobs
    skip; verify-fast + all 14 always-on jobs still run.
  RESULT: that same PR merges to main -- ci_mode is unconditionally
    "normal" on the push run, regardless of the merge commit's message or
    merge strategy -- full scoped CI runs on main, every time.

  ENVIRONMENT CHECKS -- minimal, no generic pre-bootstrap stage (round 4)
  ======================================================================
  DROPPED: round 3's pre-bootstrap git/bash/tar check -- checkout and the
    job's own shell already exercise git/bash; tar is job-specific, not
    universal. Low signal for its maintenance cost.
  KEPT: move verify-fast's `apt-get install ripgrep` BEFORE python/node/
    rust setup (installing early beats checking early for a tool that
    gets installed regardless).
  KEPT: test_dev_environment_tools.sh stays exactly where it runs today
    (post-bootstrap) -- it checks .venv/bin/ruff, .venv/bin/pyright,
    node_modules/js-yaml, none of which exist before bootstrap runs.

  FIXTURE REQUIREMENT -- now A FIRM PRECONDITION, not an open question
  ======================================================================
  Must exercise the REAL dorny/paths-filter matching engine (a scratch
  workflow_dispatch run, or a local harness using the real matcher) --
  NOT a structural YAML parse. A structural-only check is EXACTLY what
  would have missed this round's P1-1 defect (the `!` pattern LOOKED
  right in the YAML; it just didn't behave right under quantifier=some).

  GITLEAKS -- unchanged from round 2/3, review confirmed correct
  ======================================================================
  gitleaks-current.sh: full-tree scan every push, not diff-scoped.
  commit-range scanning via --log-opts: needs new script work.
  full-history scan -> workflow_dispatch + a real release.yml step
    (none exists today) -- advisory until allowlist mismatch
    (expected=100 observed=121 unknown=21) is reconciled.

  ------------------------------------------------------------------------------
  RESULT ON THE CHECKS TAB / PR -- INFORMATIONAL ONLY, unchanged
  ------------------------------------------------------------------------------
    red or green, every time -- nothing REQUIRED, no ruleset, no merge queue.
```
