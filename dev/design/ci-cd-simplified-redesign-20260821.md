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
  Twice revised after adversarial correctness reviews (§8) — round 1 fixed
  wrong source paths, an invalid workflow-level `if:`, and a preflight
  design that validated the wrong runner; round 2 fixed a release-suppression
  hazard in the skip mechanism, an incomplete path→job mapping, an
  unsolvable Windows-path-isolation claim, a still-too-blunt preflight, and
  a gitleaks scan that was never actually diff-scoped. Analysis and
  recommendation only; no CI config, script, or GitHub setting is changed by
  this document.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research,
  ci-cd-final-recommendation}-20260821.md; .github/workflows/{ci,
  release}.yml; scripts/security/gitleaks-current.sh;
  scripts/tests/test_dev_environment_tools.sh; scripts/agent-verify.sh
---

# CI/CD simplified redesign — single-maintainer correction

**Status: PROPOSED.** Nothing here is implemented. This document supersedes
§2–§4 and §8 of `ci-cd-final-recommendation-20260821.md` completely — not
just in shape but in every specific fact those sections asserted. §1 of
that document (the challenge evaluation) still holds as a historical
record.

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
`.github/workflows/ci.yml` (lines 113–129) rather than adding a new
taxonomy job. Path-scoping has a real ceiling — it can only see which
*files* changed, not what changed *inside* them, and several existing jobs
depend on more than one source surface at once. §3 below is an authoritative
mapping grounded in each job's actual script invocations, not an aspiration.

## 3. Concrete shape

### 3.1 Path-filter categories — corrected paths, required permission

```yaml
  changes:
    outputs:
      docs_only: ...            # unchanged
      rust:       ${{ steps.filter.outputs.rust }}
      python:     ${{ steps.filter.outputs.python }}
      typescript: ${{ steps.filter.outputs.typescript }}
      release_infra: ${{ steps.filter.outputs.release_infra }}
    steps:
      - uses: dorny/paths-filter@...
        with:
          filters: |
            nonmd: [ '!**/*.md' ]
            rust: [ 'src/rust/**', 'Cargo.toml', 'Cargo.lock' ]
            python: [ 'src/python/**' ]
            typescript: [ 'src/ts/**', 'package.json', 'package-lock.json' ]
            release_infra: [ '.github/workflows/**', 'scripts/release/**' ]
```

Two corrections from the first draft:

- The repo-root `python/` directory referenced there **does not exist as a
  tracked path at all** — `.gitignore` line 96 excludes it
  (`/python/` covers `python/build`, `python/examples`); it's local,
  untracked build/example output in some checkouts and irrelevant to
  path-filter design either way. `src/python/**` is the only real Python
  source root.
- `dorny/paths-filter`'s default PR-diff mode uses the GitHub REST API and
  its README documents `pull-requests: read` as a requirement, not an
  optional fallback. `permissions: pull-requests: read` is **required** in
  `ci.yml`, added alongside the existing `contents: read` — not a "verify
  later" precaution.

### 3.2 No silent "unclassified" gap

`verify-fast` (the cheap, ~1–2 minute lint/typecheck/security suite) keeps
its current `if: needs.changes.outputs.docs_only != 'true'` condition
unchanged — it runs on **any** non-doc change regardless of category. Only
the heavier, per-surface jobs (§3.4) get narrowed further. Nothing is ever
zero-signal.

### 3.3 Windows scoping — an honest boundary, and what actually fixes it

**The `os.uname()` staleness in the challenge catalogue is fixed.** Commit
`25efa4eb` ("fix(python): support Windows platform detection",
2026-08-19), an ancestor of this branch, patched
`src/python/fathomdb/_coinstall.py` (a Windows platform-detection guard
exercised at package import time) and added
`test_platform_probe_is_silent_when_windows_lacks_os_uname` in
`src/python/tests/test_coinstallation_guard.py`. This is very likely the
root cause of the original `windows-wal-attribution` `AttributeError` — the
job installs and imports the Python wheel before running its tests, so an
import-time crash there would surface exactly as that job's failure. Treat
it as fixed; confirm on a real Windows CI run rather than assuming further,
but stop describing it as an open "broken diagnostic" blocker.

**What path-filtering genuinely cannot do here, corrected from the first
draft:** the first draft claimed a Windows-only *workflow YAML step* could
be scoped by path. It can't — every job in `ci.yml`, Windows-specific or
not, lives in the same single file, so path-filtering only ever sees "did
`.github/workflows/ci.yml` change," which is `release_infra` regardless of
which job's step was touched. There is no partial-file granularity in
`dorny/paths-filter`.

**The two real options, stated plainly, neither of them a docs-only fix:**

1. **Structural isolation** — move the two Windows jobs into a separate
   reusable workflow (e.g. `.github/workflows/ci-windows.yml`, called via
   `workflow_call` from `ci.yml`), so a change to *that* file path-filters
   distinctly from a change elsewhere in `ci.yml`. Real refactoring work,
   not a filter tweak.
2. **Accept the coupling** — leave both jobs in `ci.yml` and accept that
   editing any part of that file is `release_infra` scope, same as editing
   any other workflow job's YAML. This is consistent with how every other
   job in the same file is already treated and requires no new work.

This document does not choose between them (§9). What §3.4's table
captures is the *source-dependency* scoping (rust vs. python vs. neither),
which is real and doesn't depend on either option above.

### 3.4 Authoritative path→job mapping

Built from each job's actual script invocations (`ci.yml`), not inferred:

| Job | rust | python | typescript | release_infra | Notes |
|---|:-:|:-:|:-:|:-:|---|
| `verify`, `verify-fast`, `rust-workspace-race-report`, `security`, `default-embedder-tests` | ✓ | ✓ | ✓ | | `agent-verify.sh --tier=heavy/fast` runs all three languages together — cannot split further without script work (§3.4a) |
| `wheel-size-gate` | ✓ | ✓ | | ✓ | builds/measures the Python wheel from Rust source, `working-directory: src/python` |
| `native-artifact-runtime-validation` | ✓ | ✓ | ✓ | ✓ | the 5-platform artifact matrix (linux-x64/arm64-gnu, darwin-x64/arm64, win32-x64-msvc) — see explicit decision below |
| `windows-wal-checkpoint-diagnosis` | ✓ | | | | pure `cargo test -p fathomdb-engine`, no Python/TS involvement |
| `windows-wal-attribution` | ✓ | ✓ | | | builds+installs the Python wheel, runs `src/python/tests/test_slice65_wal_attribution_installed.py` against `src/python/pyproject.toml` |
| `markdownlint`, `design-status` | | | | | fire only when `docs_only == 'true'` (opposite condition — docs-only pushes) |
| `gitleaks` (current-tree step), `shell-lint`, `board-currency`, `ledger-integrity`, `plan-anchors`, `governed-surface-pin`, `pinned-override-rot`, `c1-contract-conformance`, `transcript-hygiene`, `release-state-views`, `commission-manifest`, `steward-orient`, `docs` | | | | | 13 jobs, always-on today by deliberate design (no `if:` at all) — unaffected by this doc's scoping; §9 flags whether that should change |

**Explicit decision on `native-artifact-runtime-validation`** (the first
draft left §3.4 and §3.7 contradicting each other on this): it **stays in
`ci.yml`, scoped to `rust || python || typescript || release_infra`**, same
as today's broad condition — it does **not** move to release/manual-only.
Reasoning: it's real cross-platform build correctness signal, not ceremony;
moving it to release-time-only would mean a native-build break surfaces
days or weeks after the commit that caused it instead of on that commit,
which is a worse debugging experience for a solo maintainer, not a better
one. What changes is *only* that it stops running on changes that touch
none of the four categories (e.g. a pure `dev/design/**` doc-adjacent
change that isn't `.md`), not that it stops running on ordinary code
changes.

#### 3.4a What needs real script work before it's achievable

Splitting `verify` by language requires a `--surface=rust|python|
typescript` selector added to `scripts/agent-verify.sh` /
`scripts/agent-test.sh` first — until then, any of `rust`/`python`/
`typescript` triggers the same full three-language run, which is coarser
than the per-language pitch in earlier drafts and should be described that
way, not oversold.

### 3.5 Environment checks — reuse the existing mechanism, scoped per job

The repo already has a purpose-built diagnostic for exactly this:
`scripts/tests/test_dev_environment_tools.sh` (0.8.23 Slice 80.3, its own
header states the goal directly: "one legible diagnostic for a bootstrapped
developer environment, instead of the pinned-tool gap surfacing as N
scattered, differently-worded failures"), backed by its own contract fixture
`scripts/tests/test_dev_environment_tools_contract.sh`. It checks pinned
versions of `shellcheck`, `gitleaks`, and `actionlint` — not `rg`/`jq`/
`curl` generically, and the first draft's blanket four-tool check was
inventing requirements several jobs don't actually have (`jq`/`curl` are
used by `scripts/release/**` scripts and `shell-lint`'s
`agent-lint-shell.sh`; `rg` by `agent-test.sh`-driven suites — not
uniformly by every job).

The real gap isn't the check's existence, it's **where it runs**: `agent-
test.sh` registers it inside the `fast` tier's suite list
(`scripts/agent-test.sh:439-440`), which only executes after `verify-fast`
has already done checkout, per-session `TMPDIR` setup, Python setup, Node
setup, Rust toolchain install, `Swatinem/rust-cache`, an unconditional
`apt-get install ripgrep`, and `scripts/bootstrap.sh` — several minutes of
setup, not 30 seconds, and by then `rg` has already been silently installed
regardless of whether its absence would have been a real signal.

**Corrected recommendation, per job, not a shared blanket:**

- Move `test_dev_environment_tools.sh` (or the subset relevant to that
  job's own scripts) to literally the first step after checkout, before
  any language toolchain setup — it already exists and is already tested;
  it needs relocating, not reinventing.
- For `verify-fast` specifically: move the existing `apt-get install
  ripgrep` step to before Python/Node/Rust setup rather than adding a
  redundant presence-check for a tool the job already unconditionally
  installs — testing for a tool immediately followed by installing it
  regardless doesn't produce an earlier failure, it just adds a check that
  can never fire.
- Jobs that don't call `agent-test.sh`/`agent-verify.sh` at all (e.g.
  `docs`, `markdownlint`) don't need this step — their single tool (mkdocs,
  markdownlint) is already installed by their own one setup line.
- Platform-specific jobs (`windows-wal-*`, the Windows leg of
  `native-artifact-runtime-validation`) need the Windows equivalent
  (`Get-Command` in `pwsh`), since `command -v` isn't meaningful there.

### 3.6 Skipping CI entirely — not via GitHub's native marker

The first draft recommended GitHub's native `[skip ci]` commit-message
marker as unconditionally safe because "nothing here is required." That
reasoning has a hole: `[skip ci]` applies to any workflow triggered by a
`push` event, and `release.yml` publishes on `push: tags: - 'v*'` — a tag
push **is** a push event. If a commit that was ever authored with
`[skip ci]` in its message is later tagged for release, the release
workflow triggered by that tag push can also be suppressed by the same
marker. This is a plausible, real hazard for a publish pipeline and is too
dangerous to rely on without first proving it doesn't happen (e.g. a
disposable-tag fixture test) — which this document doesn't have, so it
does not recommend native skipping.

**Corrected mechanism, scoped only to `ci.yml`'s own jobs, and never
touching `release.yml` at all:** compute a `ci_mode` output in the
`changes` job by reading the actual latest commit message via git — not
`github.event.head_commit`, which only exists on `push` events and is
absent on `pull_request` (the exact defect the previous round's review
caught):

```yaml
      - name: Determine CI mode from the latest commit message
        id: mode
        run: |
          msg="$(git log -1 --pretty=%B)"
          if [[ "$msg" == *"[ci-skip]"* ]]; then
            echo "mode=skip" >> "$GITHUB_OUTPUT"
          else
            echo "mode=normal" >> "$GITHUB_OUTPUT"
          fi
```

Every job in `ci.yml` (never `release.yml`) adds
`&& needs.changes.outputs.mode != 'skip'` to its existing `if:`. Because
this marker is never referenced anywhere in `release.yml`, tagging a commit
that used it cannot suppress a release — the hazard this section exists to
avoid is structurally impossible here, not just unlikely.

### 3.7 What actually moves off the always-run path

Only genuinely *new* scheduled/on-demand work belongs here — not
`native-artifact-runtime-validation`, which stays per-push per §3.4. What
does move: full-history gitleaks scanning (§4) and any future GPU/Tegra
rehearsal work, both to `workflow_dispatch` plus the existing tag-triggered
`release.yml`, never a new nightly cron.

### 3.8 A fixture requirement before any YAML changes land

The path→job classification in §3.4 is load-bearing — if it's wrong, jobs
silently under- or over-run, exactly the class of defect both correctness
review rounds caught in this document itself. Before changing `ci.yml`, add
a fixture (in the style of the repo's existing `test_shell_lint_ci_job.sh`
tier-totality fixture and `test_dev_environment_tools_contract.sh`) that
asserts representative diffs select exactly the intended job set:

- Markdown-only
- An unclassified script/config change (e.g. a file outside all four
  categories) — asserts `verify-fast` still runs, nothing silently skips
- Python-only (`src/python/**`)
- TypeScript-only (`src/ts/**`)
- Rust core (`src/rust/**`)
- The Windows attribution Python test path specifically
  (`src/python/tests/test_slice65_wal_attribution_installed.py`) — asserts
  both `rust` and `python` categories fire per §3.4's row
- A release script (`scripts/release/**`)
- `ci.yml` itself — asserts `release_infra` scope, per §3.3's accepted
  coupling
- A mixed-surface diff touching two categories at once

## 4. Gitleaks — corrected scanning model, not just corrected blocking status

The gitleaks job (`ci.yml` line 26) was already made non-blocking on `main`
by commit `a755e1d8` ("ci: make Gitleaks report-only", 2026-08-20,
`continue-on-error: true`), so challenge catalogue §1.1 is already
partially resolved independent of this redesign. But `scripts/security/
gitleaks-current.sh` — read directly, not assumed — does **not** scan a
diff or commit range: it runs `git ls-files -z | tar ... | gitleaks dir`,
archiving and scanning **every tracked file in the working tree**, every
push, regardless of what changed. Calling this "diff-relevant" (as an
earlier draft did) was wrong; it is full-tree, just fast because gitleaks
itself is fast on this repo's size, not because it's scoped.

**Corrected design, and each piece labeled honestly:**

- **Needs a new script capability** (not a YAML change): a genuine
  commit-range scan using `gitleaks`'s documented `--log-opts` support —
  push events scan `${{ github.event.before }}..${{ github.sha }}`, PR
  events scan the base/merge-base through head. This doesn't exist yet in
  `scripts/security/gitleaks-current.sh` or as a separate script.
- **Full-history scanning** (`scripts/security/gitleaks-history.sh`) moves
  to `workflow_dispatch` plus a real step in `release.yml` (which currently
  has **no** gitleaks step at all — the prior recommendation's assumption
  otherwise was wrong). It stays **advisory** (report-only, never in the
  publish dependency chain) at least until the existing allowlist/
  reconciliation mismatch is resolved — re-running `gitleaks-history.sh`
  during this revision showed `expected_records=100 observed_records=121
  unknown=21`, i.e. the allowlist and the actual history are currently out
  of sync. That reconciliation is the maintainer's own next step, informed
  by that script's own output, not by any finding count in this document.

## 5. Explicitly out of scope

CUDA/maturin toolchain churn stays out of scope — real release-engineering
defects, not a CI-triggering problem. `cargo hack check --each-feature` as
a GPU-compile proxy (carried from stage-3 research) remains unverified for
this repo's actual CUDA/Metal/ONNX feature combinations and should be
spiked once, manually, before being relied on.

## 6. What this doesn't solve, said plainly

Removing required checks removes the 2026-08-20 failure mode (a check
staying red for days, forcing a choice between living with it or deleting
the gate) by removing the gate, not by fixing the checks — deliberate for a
single maintainer, with a symmetric risk: a red check nobody is forced to
look at can go unnoticed longer than a required one would. Fine-grained
scoping (§3) partly offsets this by making a red result mean something more
specific, but it's still on the maintainer to look.

## 7. Open decisions for HITL

1. **Windows job isolation** (§3.3): accept the `release_infra` coupling
   as-is, or invest in splitting Windows jobs into a separate reusable
   workflow file for true path isolation? Neither is free; this document
   doesn't choose.
2. **The 13 always-on jobs** (§3.4's last row): keep unconditional (cheap,
   deliberate, pre-existing design), or scope some to
   `dev/plans|steward|design/**`?
3. **`--surface=` selector for `agent-verify.sh`** (§3.4a): worth the
   script investment to get real per-language `verify` scoping, or accept
   the coarser "any language touched" behavior indefinitely?
4. **Commit-range gitleaks scanning** (§4): worth building, or is the
   current full-tree-but-fast scan acceptable to leave as the every-push
   baseline while only full-history moves off-path?
5. **Fixture-first sequencing** (§3.8): build the classifier fixture before
   or alongside the `ci.yml` changes, rather than after?

## 8. Revision history

- **Round 1** fixed: wrong `src/python`/`src/ts` paths, an invalid
  workflow-level `if:` for a `[ci-lite]` idea, a shared `preflight` job
  that validated the wrong runner, an over/under-matching `windows_
  relevant` glob, an unsupportable per-language `verify`-split claim, and a
  stale gitleaks-triage framing that predated `a755e1d8`.
- **Round 2** (this revision) fixed: a tag-push/release-suppression hazard
  in the native `[skip ci]` recommendation (§3.6), an incomplete and
  internally-contradictory path→job mapping (§3.4), an unfixable
  "Windows-specific workflow step" path-filter claim plus a stale
  `os.uname()` blocker (§3.3 — actually fixed by `25efa4eb`), a preflight
  design that checked tools jobs didn't actually need instead of reusing
  the repo's existing `test_dev_environment_tools.sh` (§3.5), a gitleaks
  scan wrongly described as diff-relevant when it's full-tree (§4), a
  too-tentative permissions claim for `dorny/paths-filter` (§3.1), and a
  missing test-fixture requirement for the classifier itself (§3.8).
- Both rounds confirmed the policy direction (§0–§2) is correct; only the
  mechanics were wrong.

## Appendix — pipeline diagram (v3, corrected)

```text
                  FathomDB CI -- simplified, single-maintainer shape (v3)
              (dev/design/ci-cd-simplified-redesign-20260821.md, PROPOSED)
                    informational only -- nothing here is a gate

  TRIGGERS                                         SKIP MECHANISM (corrected)
  ================================================================
  push: main only     pull_request: any branch      NOT github-native [skip ci] --
  (ci.yml today)       (ci.yml today)                 a tag push is ALSO a push event,
                                                        so native skip could suppress
                                                        release.yml. Instead: a `ci_mode`
                                                        output in `changes`, read via
                                                        `git log -1 --pretty=%B` for
                                                        "[ci-skip]" -- applied ONLY to
                                                        ci.yml's own jobs' `if:`, NEVER
                                                        referenced by release.yml. The
                                                        hazard is structurally impossible,
                                                        not just unlikely.
                                     |
                                     v
  +----------------------------------------------------------------+
  | changes (dorny/paths-filter -- EXTENDS the existing job)          |
  |   permissions: pull-requests: read  -- REQUIRED (paths-filter's    |
  |   PR-diff mode uses the REST API by default), not optional         |
  |                                                                     |
  |   docs_only unchanged; adds, with CORRECTED real paths:            |
  |     rust           src/rust/**, Cargo.toml, Cargo.lock             |
  |     python          src/python/**   (root python/ is NOT tracked -- |
  |     typescript       src/ts/**       .gitignore excludes it, it's  |
  |                                       local build/example output   |
  |     release_infra   .github/workflows/**, scripts/release/**       |
  +----------------------------------------------------------------+
                                     |
                                     v
  AUTHORITATIVE PATH -> JOB TRUTH TABLE (grounded in real script calls)
  ======================================================================
  job                                  rust python typescript release_infra
  ------------------------------------ ---- ------ ---------- --------------
  verify / verify-fast / rust-         yes  yes    yes         --   <- runs
    workspace-race-report / security /                              ALL 3 langs
    default-embedder-tests                                          together;
                                                                      can't split
                                                                      w/o script
                                                                      work (3.4a)
  wheel-size-gate                       yes  yes    --          yes
  native-artifact-runtime-validation    yes  yes    yes         yes  <- STAYS
    (the 5-platform artifact matrix)                                 per-push,
                                                                       explicit
                                                                       decision,
                                                                       NOT moved
                                                                       to release-
                                                                       only (real
                                                                       cross-plat
                                                                       signal)
  windows-wal-checkpoint-diagnosis      yes  --     --          --   pure
                                                                       `cargo test`
  windows-wal-attribution               yes  yes    --          --   builds +
                                                                       installs the
                                                                       wheel, runs
                                                                       a Python test
  markdownlint, design-status           (docs_only=='true' ONLY -- opposite cond.)
  gitleaks(current-tree), shell-lint,   ALWAYS-ON, no `if:` at all -- 13 jobs,
  board-currency, ledger-integrity,     pre-existing deliberate design, NOT
  plan-anchors, governed-surface-pin,   changed by this redesign (open Q for HITL)
  pinned-override-rot, c1-contract-
  conformance, transcript-hygiene,
  release-state-views, commission-
  manifest, steward-orient, docs

  WINDOWS SCOPING -- an honest boundary, not a false promise
  ======================================================================
  os.uname() bug: FIXED (commit 25efa4eb, src/python/fathomdb/_coinstall.py)
    -- "run for nobody until fixed" language from earlier drafts is now stale.

  "one Windows-specific workflow STEP" cannot be path-scoped: ALL jobs live
  in the ONE file .github/workflows/ci.yml -- paths-filter sees the file,
  not the edited block inside it. Two real options, neither free:
    1. structural isolation -- split Windows jobs into a separate reusable
       workflow (ci-windows.yml via workflow_call) -- real refactor
    2. accept the coupling -- editing ci.yml = release_infra scope, same
       as any other job's YAML edit
  This doc does not choose (open HITL decision). The rust/python source-
  dependency scoping above (windows-wal-* rows) is real either way.

  ENVIRONMENT CHECKS -- reuse what already exists, scoped per job
  ======================================================================
  scripts/tests/test_dev_environment_tools.sh (0.8.23 Slice 80.3) ALREADY
  does this -- checks pinned shellcheck/gitleaks/actionlint versions, has
  its own contract fixture. Gap is WHERE it runs: buried in agent-test.sh's
  fast-tier suite list, AFTER checkout+py+node+rust+apt-get-installed-rg+
  bootstrap.sh -- minutes in, not 30 seconds.
    fix: move it (or the relevant subset) to literally the first step
         after checkout, before language toolchain setup
    fix: verify-fast's existing `apt-get install ripgrep` step moves BEFORE
         python/node/rust setup, instead of adding a redundant presence-
         check for a tool the job unconditionally installs anyway
    NOT a blanket rg/jq/git/curl check on every job -- jq/curl are used by
    scripts/release/** and shell-lint specifically, not universally
    windows-latest jobs use `Get-Command` in pwsh, not `command -v`

  GITLEAKS -- corrected scanning model, not just corrected blocking status
  ======================================================================
  gitleaks-current.sh: archives + scans EVERY TRACKED FILE, every push --
    NOT diff-scoped today despite earlier "diff-relevant" framing. Fast
    because the repo is fast to scan, not because it's scoped.
  NEEDS SCRIPT WORK: a real commit-range scan via gitleaks --log-opts
    (push: github.event.before..github.sha; PR: base/merge-base..head)
    -- does not exist yet.
  full-history scan -> workflow_dispatch + a REAL step added to release.yml
    (which today has NONE) -- stays ADVISORY (never in the publish
    dependency chain) until the existing allowlist mismatch is reconciled:
    gitleaks-history.sh currently reports
    expected_records=100 observed_records=121 unknown=21.

  ------------------------------------------------------------------------------
  RESULT ON THE CHECKS TAB / PR -- INFORMATIONAL ONLY, same as before
  ------------------------------------------------------------------------------
    red or green, every time -- nothing REQUIRED, no ruleset, no merge queue.
```
