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
  Three times revised after adversarial correctness reviews (§8): round 1
  fixed wrong source paths, an invalid workflow-level `if:`, and a preflight
  design that validated the wrong runner; round 2 fixed a release-suppression
  hazard in the skip mechanism, an incomplete path→job mapping, an
  unsolvable Windows-path-isolation claim, a still-too-blunt preflight, and
  a gitleaks scan that was never actually diff-scoped; round 3 fixed the
  redesign still amounting to nearly-full-CI for ordinary changes, an
  always-on-invariant violation in the skip mechanism, a PR-head-commit bug
  in that same mechanism, four more factual table errors, and a
  premature-dependency bug in the environment-check relocation. Analysis and
  recommendation only; no CI config, script, or GitHub setting is changed by
  this document.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research,
  ci-cd-final-recommendation}-20260821.md; .github/workflows/{ci,
  release}.yml; scripts/security/gitleaks-current.sh;
  scripts/tests/{test_dev_environment_tools,test_shell_lint_ci_job}.sh;
  scripts/agent-verify.sh; src/python/tests/test_slice65_wal_attribution_*.py
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
job. Two independent levers do the proportionality work, and round 3 exists
because the first two rounds only really delivered the first lever:

1. **Path scoping** (§3.1/§3.4) — which *category* of source changed.
2. **`ci-lite` mode** (§3.6) — an explicit, maintainer-asserted signal that
   *this particular change*, whatever category it falls in, doesn't need the
   heavy jobs. This is what actually answers "a comment fix in a Rust file
   shouldn't run full CI" — path scoping alone cannot, because paths-filter
   sees files, not content, and a comment edit and a logic edit are the same
   path.

## 3. Concrete shape

### 3.1 Path-filter categories — corrected paths, a real `windows` category, required permission

```yaml
  changes:
    outputs:
      docs_only: ...            # unchanged
      rust:          ${{ steps.filter.outputs.rust }}
      python:        ${{ steps.filter.outputs.python }}
      typescript:    ${{ steps.filter.outputs.typescript }}
      windows:       ${{ steps.filter.outputs.windows }}
      release_infra: ${{ steps.filter.outputs.release_infra }}
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
          msg="$(git log -1 --pretty=%B)"
          if [[ "$msg" == *"[ci-lite]"* ]]; then
            echo "ci_mode=lite" >> "$GITHUB_OUTPUT"
          else
            echo "ci_mode=normal" >> "$GITHUB_OUTPUT"
          fi
      - uses: dorny/paths-filter@...
        with:
          filters: |
            nonmd: [ '!**/*.md' ]
            windows:
              - 'src/python/tests/test_slice65_wal_attribution_*.py'
              - 'scripts/tests/test_windows_wal_*.sh'
            python:
              - 'src/python/**'
              - '!src/python/tests/test_slice65_wal_attribution_*.py'
            rust: [ 'src/rust/**', 'Cargo.toml', 'Cargo.lock' ]
            typescript: [ 'src/ts/**', 'package.json', 'package-lock.json' ]
            release_infra: [ '.github/workflows/**', 'scripts/release/**' ]
```

Corrections in this revision, on top of rounds 1–2:

- **A real `windows` category now exists**, and it's grounded, not invented:
  `src/python/tests/test_slice65_wal_attribution_installed.py`'s own
  docstring is "Installed-wheel controls for Slice 65 **Windows** WAL
  attribution" — it and its sibling
  `test_slice65_wal_attribution_typing.py`, plus their CI-job fixtures
  `scripts/tests/test_windows_wal_diagnosis_ci_job.sh` and
  `test_windows_wal_attribution_ci_job.sh`, are genuinely, entirely
  Windows-specific files, unlike `erasure_completeness.rs` (§3.3, still
  unfixable — that finding stands). The `python` filter **excludes** the
  slice65 files with a `!` pattern so they set `windows=true` /
  `python=false` on their own — a change confined to them no longer looks
  like "any Python change" to the heavy jobs.
- `changes` now also computes `ci_mode` (§3.6) and checks out the real head
  commit on `pull_request` events specifically so that computation is
  correct — both new; the previous round's YAML sketch declared neither the
  `mode` output nor a correct ref, so it wouldn't have worked as written.
- The `pull-requests: read` permission requirement and the note about
  `python/` not being a tracked path (`.gitignore` line 96) are unchanged
  from round 2 and still correct.

### 3.2 No silent "unclassified" gap

`verify-fast` keeps its current `if: needs.changes.outputs.docs_only !=
'true'` condition unchanged — runs on any non-doc change, is **not**
suppressed by `ci_mode=lite` (§3.6 is explicit about this), and is never
zero-signal.

### 3.3 Windows scoping — an honest boundary, unchanged from round 2

The `os.uname()` staleness is fixed (`25efa4eb`,
`src/python/fathomdb/_coinstall.py`) — stop describing it as an open
blocker. A Windows-specific *workflow step* still cannot be path-scoped,
because every job lives in one `ci.yml` file (§7 lists the two real
options; this document still doesn't choose between them). What's new in
this revision is that the *source-level* Windows scoping described as
"real either way" in round 2 is now actually specified (§3.1's `windows`
category), not just asserted as theoretically possible.

### 3.4 Authoritative path→job mapping — corrected

Verified directly against `.github/workflows/ci.yml`, fixing four factual
errors the round-3 review caught:

| Job | rust | python | typescript | windows | release_infra | `ci-lite` suppressible? |
|---|:-:|:-:|:-:|:-:|:-:|:-:|
| `verify` | ✓ | ✓ | ✓ | | | **yes** |
| `verify-fast` | (any non-doc change — broader than the other columns; see note below) | | | | | **no** — this is the cheap check `ci-lite` exists to preserve |
| `rust-workspace-race-report` | ✓ | | | | | **yes** — invokes `bash scripts/test-rust-workspace.sh --parallel-report` directly; it is **not** part of the 3-language `agent-verify.sh` run and was wrongly merged into that row in round 2 |
| `security`, `default-embedder-tests` | ✓ | ✓ | ✓ | | | **yes** |
| `wheel-size-gate` | ✓ | ✓ | | | ✓ | **yes** |
| `native-artifact-runtime-validation` | ✓ | ✓ | ✓ | | ✓ | **yes** — stays per-push (explicit decision below), not release-only |
| `windows-wal-checkpoint-diagnosis` | ✓ | | | | | **yes** |
| `windows-wal-attribution` | ✓ | ✓ | | ✓ | | **yes** — condition becomes `rust \|\| python \|\| windows` so a change confined to the slice65 files still triggers it even though `python` alone is now false for them |
| `markdownlint`, `design-status` | | | | | | n/a |
| 14 always-on jobs (below) | | | | | | n/a — never touched by `ci_mode` |

`verify-fast`'s row note: it shares no category semantics with the others —
it fires on **any** non-md change, which is broader than "rust or python or
typescript or release_infra" individually (e.g. an unclassified script
change still triggers it). Giving it its own row prevents the table from
implying it's gated the same way the heavy jobs are.

**`markdownlint` is `docs_only == 'true'`-gated** (fires only on docs-only
pushes) — unchanged from round 2. **`design-status` is NOT** — round 2
wrongly grouped it with `markdownlint`. Read directly
(`.github/workflows/ci.yml:1208`): `design-status` carries no `if:` and no
`needs:` at all, and its own comment explains why: it's deliberately its
own always-on job specifically *because* `markdownlint` is docs-only-gated
and would otherwise miss a design-doc change landing alongside code. It
belongs in the always-on set below.

**The always-on set is 14 jobs, not 13**: `gitleaks` (current-tree step),
`shell-lint`, `board-currency`, `ledger-integrity`, `plan-anchors`,
`governed-surface-pin`, `pinned-override-rot`, `c1-contract-conformance`,
`transcript-hygiene`, `release-state-views`, `commission-manifest`,
`design-status`, `steward-orient`, `docs`. **None of these are touched by
`ci_mode` in any way** — see §3.6's explicit scoping fix.

**Explicit decision on `native-artifact-runtime-validation`** (unchanged
from round 2): stays in `ci.yml`, scoped to `rust || python || typescript ||
release_infra`, not moved to release-only — real cross-platform build
signal, and moving it to release-time-only would surface a native-build
break days after the causing commit instead of on it.

#### 3.4a What needs real script work before it's achievable

Splitting `verify` by language still requires a `--surface=rust|python|
typescript` selector added to `scripts/agent-verify.sh` /
`scripts/agent-test.sh` — unchanged from round 2. Until then, any of
`rust`/`python`/`typescript` triggers the full three-language `verify` run.

### 3.5 Environment checks — three stages, not one relocation

Round 2 recommended moving `scripts/tests/test_dev_environment_tools.sh` to
literally the first step after checkout. That's wrong for this specific
script: reading it directly
(`scripts/tests/test_dev_environment_tools.sh:72` onward), it checks
`.venv/bin/ruff`, `.venv/bin/pyright` (versions sourced from
`src/python/pyproject.toml`), and root `node_modules/js-yaml` — none of
which exist until `scripts/bootstrap.sh` and `npm ci` have actually run. A
first-step relocation would make it fail every single job, unconditionally,
before bootstrap ever gets a chance to run — a worse failure mode than the
one it's meant to catch, not a fix.

**Corrected: three stages, not one relocation.**

1. **Pre-bootstrap host-capability check, first step after checkout.**
   Only tools genuinely present on a bare runner image before any install —
   `git`, `bash`, `tar` — the kind of thing whose *absence* would mean the
   runner image itself is broken, not that a workflow-owned tool hasn't
   been installed yet. This is new, minimal, and intentionally checks
   almost nothing, because almost nothing is guaranteed pre-bootstrap.
2. **Early installation of tools the workflow owns**, e.g. `rg`: move the
   existing `apt-get install ripgrep` step in `verify-fast` to before
   Python/Node/Rust setup — this part of round 2's recommendation was
   correct and is unchanged. Installing early, not "checking early for a
   tool that gets installed regardless," is what actually shortens the
   failure path.
3. **`test_dev_environment_tools.sh` stays exactly where it already runs**
   (inside `agent-test.sh`'s fast tier, after bootstrap) — round 2's
   relocation recommendation for this specific script is retracted, not
   softened. Its job is validating that bootstrap produced pinned-correct
   tool versions, which is only meaningful *after* bootstrap has run.

Jobs that don't call `agent-test.sh`/`agent-verify.sh` (e.g. `docs`,
`markdownlint`) still don't need any of this — unchanged from round 2.
Platform-specific jobs still need the Windows equivalent (`Get-Command` in
`pwsh`) for stage 1 — unchanged.

### 3.6 `ci-lite` mode — corrected scope, corrected commit-reading, explicit multi-commit decision

Round 2's mechanism had three real defects, all confirmed by reading the
actual repo:

**Defect 1 — it would have violated an enforced invariant.**
`scripts/tests/test_shell_lint_ci_job.sh` Arm B (`:98`) fails the build if
the `shell-lint` job's block contains *any* `if:` or `needs:` — "the
shell-lint job must be ALWAYS-ON (no if:/needs: gate)." Several of the 14
always-on jobs carry the same deliberate invariant in their own comments.
Round 2's "every job in `ci.yml` adds `&& needs.changes.outputs.mode !=
'skip'`" would have required adding `needs: changes` to jobs that must
never have it — self-contradicting the doc's own §3.4 classification of
them as untouched.

**Defect 2 — the YAML didn't expose what it claimed to.** The `changes`
job's `outputs:` block never declared a `mode` output, so
`steps.mode.outputs.mode` (as referenced by consumers) wouldn't have
resolved to anything.

**Defect 3 — wrong commit on PR events.** `git log -1` after a default
`actions/checkout` on a `pull_request` event reads the synthetic merge
commit GitHub creates for that event, not the contributor's actual head
commit — so a `[ci-skip]`-style marker in the real latest commit would
never be seen. `actions/checkout`'s own README documents the fix: checkout
`${{ github.event.pull_request.head.sha }}` explicitly for PR events (§3.1
now does this).

**Corrected design:**

- **Renamed to `ci-lite`, and scoped to *suppressing the heavy jobs only*,
  not skipping everything.** This directly answers the motivating example
  ("a comment fix in a Rust file shouldn't run full CI") in a way a binary
  full-skip doesn't: `verify-fast` and all 14 always-on jobs keep running
  regardless — real, always-on signal is never silenced by a commit-message
  marker — while exactly the eight jobs marked "yes" in §3.4's rightmost
  column (`verify`, `rust-workspace-race-report`, `security`,
  `default-embedder-tests`, `wheel-size-gate`,
  `native-artifact-runtime-validation`, `windows-wal-checkpoint-diagnosis`,
  `windows-wal-attribution`) add
  `&& needs.changes.outputs.ci_mode != 'lite'` to their existing
  category-based `if:` condition. Those eight jobs already have `needs:
  changes` today, so this adds a condition, not a new dependency edge — no
  always-on invariant is touched.
- **`mode` renamed `ci_mode` throughout, and actually declared** in the
  `changes` job's `outputs:` block (§3.1).
- **Multi-commit-push ambiguity, decided explicitly rather than defended
  against:** `[ci-lite]` on the latest commit is treated as authorizing
  `ci-lite` treatment for the *entire* pushed diff, not just that one
  commit. This is a deliberate choice, consistent with this document's
  existing framing elsewhere (§0, §6) that the maintainer's stated intent
  is trusted rather than programmatically second-guessed — the same trust
  model already applies to every other informational, non-gating check in
  this design.
- **The tag-push/release-suppression reasoning from round 2 is unchanged
  and still correct:** `ci_mode` is read and applied only inside `ci.yml`;
  `release.yml` never references it, so tagging a `[ci-lite]` commit cannot
  suppress a release.

### 3.7 What actually moves off the always-run path

Unchanged from round 2: full-history gitleaks scanning (§4) and any future
GPU/Tegra rehearsal work move to `workflow_dispatch` plus the existing
tag-triggered `release.yml`. `native-artifact-runtime-validation` stays
per-push (§3.4).

### 3.8 A fixture requirement before any YAML changes land

Updated from round 2 with two corrections plus one addition:

- Markdown-only
- An unclassified script/config change — asserts `verify-fast` still runs
  and is unaffected by `ci_mode`
- Python-only (`src/python/**`, excluding the slice65 files)
- TypeScript-only (`src/ts/**`)
- Rust core (`src/rust/**`)
- **A change confined to `src/python/tests/test_slice65_wal_attribution_
  installed.py` alone** — asserts `windows=true` and `python=false` from
  the classifier (not "both true," round 2's error), and that the
  **job condition** `rust || python || windows` evaluates true for
  `windows-wal-attribution` while `verify`/`security`/`default-embedder-
  tests`/`wheel-size-gate`/`native-artifact-runtime-validation` (none of
  which include `windows` in their condition) do **not** fire — this is
  the fixture that actually proves the motivating "one Windows-specific
  item" example works
- A release script (`scripts/release/**`)
- `ci.yml` itself — asserts `release_infra` scope
- A mixed-surface diff touching two categories at once
- **New: a commit carrying `[ci-lite]` that also touches `src/rust/**`** —
  asserts the eight suppressible jobs (§3.4) do not fire while
  `verify-fast` and the always-on set do

## 4. Gitleaks — unchanged from round 2

The gitleaks job was already made non-blocking on `main` by `a755e1d8`.
`gitleaks-current.sh` scans the full tracked tree, not a diff, every push.
Commit-range scanning via `--log-opts` needs new script work; full-history
scanning moves to `workflow_dispatch` plus a real step in `release.yml`
(which has none today) and stays advisory until the existing allowlist
mismatch (`expected_records=100 observed_records=121 unknown=21`) is
reconciled. None of this changed in round 3; the review confirmed it as
already correct.

## 5. Explicitly out of scope

Unchanged: CUDA/maturin toolchain churn; `cargo hack check --each-feature`
remains unverified for this repo's actual feature combinations.

## 6. What this doesn't solve, said plainly

Unchanged from round 2: removing required checks trades a red-check-forces-
a-choice failure mode for a red-check-nobody-is-forced-to-look-at risk,
deliberately, for a single maintainer. Fine-grained scoping (§3) and now
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
5. **Fixture-first sequencing** (§3.8): before or alongside the `ci.yml`
   changes?
6. **`[ci-lite]` marker text and its "authorizes the whole push" semantics**
   (§3.6): accept as specified, or want a narrower/different convention?

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
- **Round 3** (this revision) fixed: the redesign still amounting to
  nearly-full-CI for ordinary changes (added `ci-lite` mode as a second,
  independent scoping lever, and a real `windows` path category carved out
  of `python` — §3.1/§3.6); an always-on-invariant violation in the skip
  mechanism (`test_shell_lint_ci_job.sh` Arm B — §3.6 Defect 1); a
  PR-head-commit bug in the same mechanism (§3.6 Defect 3, fixed via
  `actions/checkout`'s documented `ref:` recipe); an undeclared `mode`
  output (§3.6 Defect 2); `design-status` wrongly classified as
  docs-only-gated when it's always-on, changing the always-on count from
  13 to 14 (§3.4); `rust-workspace-race-report` wrongly merged into the
  3-language `verify` row when it's Rust-only (§3.4); `verify-fast` wrongly
  implied to share the other jobs' category semantics (§3.4); the
  Windows-attribution fixture asserting the wrong classifier outputs
  (§3.8); and a premature-dependency bug in the environment-check
  relocation, corrected into three separate stages (§3.5).
- All three rounds confirmed the policy direction (§0–§2) is correct; only
  the mechanics were wrong, progressively less so each round.

## Appendix — pipeline diagram (v4, corrected)

```text
                  FathomDB CI -- simplified, single-maintainer shape (v4)
              (dev/design/ci-cd-simplified-redesign-20260821.md, PROPOSED)
                    informational only -- nothing here is a gate

  TWO INDEPENDENT SCOPING LEVERS
  ================================================================
  1. PATH CATEGORY  -- which surface changed (rust/python/typescript/
                        windows/release_infra) -- §3.1/§3.4
  2. ci-lite MODE    -- maintainer asserts THIS diff, whatever its category,
                        doesn't need the heavy jobs -- §3.6
  Path scoping alone cannot answer "a Rust comment fix shouldn't run full
  CI" (paths-filter sees files, not content) -- that's what ci-lite is for.

  TRIGGERS + CI-LITE MARKER (corrected mechanics)
  ================================================================
  push: main only      pull_request: any branch
  (ci.yml today)         (ci.yml today)
         |                      |
         +----------+-----------+
                     v
  +----------------------------------------------------------------+
  | changes job                                                       |
  |  1. checkout with ref: pull_request.head.sha (PR) / github.sha    |
  |     (push) -- REQUIRED for step 2 to read the real commit, not     |
  |     the synthetic PR merge commit (actions/checkout's documented  |
  |     recipe; round-2's `git log -1` on default checkout was wrong  |
  |     on PR events)                                                  |
  |  2. `git log -1 --pretty=%B` -> ci_mode=lite if "[ci-lite]" found   |
  |     -- DECLARED in outputs: this time (round 2 forgot to)          |
  |     -- marker on the LATEST commit authorizes ci-lite treatment    |
  |        for the WHOLE pushed diff (explicit decision, not defended  |
  |        against programmatically)                                   |
  |  3. dorny/paths-filter (permissions: pull-requests: read REQUIRED) |
  |     rust / python / typescript / windows / release_infra          |
  |     `windows` is a REAL carve-out of `python`:                     |
  |       src/python/tests/test_slice65_wal_attribution_*.py           |
  |       scripts/tests/test_windows_wal_*.sh                          |
  |     (python filter EXCLUDES these with a `!` pattern)              |
  +----------------------------------------------------------------+
                     |
                     v
  AUTHORITATIVE TABLE -- corrected: 4 factual errors fixed this round
  ======================================================================
  job                              r  p  ts windows relinfra ci-lite?
  --------------------------------- -- -- -- ------- -------- --------
  verify                             y  y  y   --      --      YES
  verify-fast                        (any non-md change -- own row,     NO --
                                       broader than the category cols)  preserved
  rust-workspace-race-report         y  --  --   --      --      YES  <- was
                                                                        wrongly
                                                                        merged
                                                                        into
                                                                        verify's
                                                                        row in
                                                                        round 2;
                                                                        it's
                                                                        test-rust-
                                                                        workspace.sh,
                                                                        rust-only
  security, default-embedder-tests   y  y  y   --      --      YES
  wheel-size-gate                    y  y  --   --      y       YES
  native-artifact-runtime-           y  y  y   --      y       YES   <- STAYS
    validation (5-platform matrix)                                    per-push,
                                                                        explicit
                                                                        decision
  windows-wal-checkpoint-diagnosis   y  --  --   --      --      YES
  windows-wal-attribution            y  y  --   y       --      YES  <- now
                                                                        rust||
                                                                        python||
                                                                        windows;
                                                                        still
                                                                        fires on
                                                                        a
                                                                        windows-
                                                                        only diff
  markdownlint                       (docs_only=='true' ONLY)          n/a
  design-status                      ALWAYS-ON -- was WRONGLY put in    n/a
                                      the docs_only row in round 2;
                                      ci.yml:1208 has NO if:/needs:
  14 always-on jobs (gitleaks-       NEVER touched by ci_mode -- adding  n/a
    current, shell-lint, board-      `needs: changes` to these would
    currency, ledger-integrity,      violate test_shell_lint_ci_job.sh
    plan-anchors, governed-          Arm B, which HARD-FAILS the build
    surface-pin, pinned-override-    if shell-lint's block contains ANY
    rot, c1-contract-conformance,    if:/needs: -- round 2's "every job
    transcript-hygiene, release-     adds ci_mode" wording would have
    state-views, commission-         broken this invariant
    manifest, design-status,
    steward-orient, docs)
                                      (13 -> 14: design-status moved here)

  RESULT: a diff confined to test_slice65_wal_attribution_installed.py --
    windows=true, python=false -> ONLY windows-wal-attribution +
    verify-fast + the 14 always-on jobs run. No verify, no security, no
    default-embedder-tests, no wheel-size-gate, no 5-platform matrix.
    THIS is what answers the "one Windows-specific item" requirement.

  RESULT: a [ci-lite]-tagged Rust comment-fix commit -- the 8 heavy jobs
    above (marked YES) are suppressed. verify-fast + all 14 always-on
    jobs still run. THIS is what answers the "administrative change
    shouldn't need full CI" requirement -- path scoping alone couldn't.

  ENVIRONMENT CHECKS -- corrected: THREE stages, not one relocation
  ======================================================================
  round 2 said "move test_dev_environment_tools.sh to the first step" --
  WRONG for this script: it checks .venv/bin/ruff, .venv/bin/pyright,
  root node_modules/js-yaml -- NONE exist before bootstrap/npm ci run.
  A first-step relocation would fail EVERY job, always, before bootstrap
  gets a chance -- worse than the problem it was meant to solve.
    stage 1 (first step, pre-bootstrap): git/bash/tar presence only --
             tools genuinely on a bare runner image, nothing from
             .venv or node_modules
    stage 2 (early, workflow-owned installs): move verify-fast's existing
             `apt-get install ripgrep` BEFORE python/node/rust setup
             (round 2's correct point, kept)
    stage 3 (unchanged location, post-bootstrap): test_dev_environment_
             tools.sh stays exactly where it runs today -- its own job
             is validating bootstrap's output, which requires bootstrap
             to have already run

  GITLEAKS -- unchanged from round 2, review confirmed correct
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
