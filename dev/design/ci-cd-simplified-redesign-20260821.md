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
  Revised after an adversarial correctness review found several concrete
  implementation errors in the first draft (wrong source paths, an invalid
  workflow-level `if:`, a preflight design that can't validate what it
  claims to, and stale facts about gitleaks' current blocking status) — see
  §7. Analysis and recommendation only; no CI config, script, or GitHub
  setting is changed by this document.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research,
  ci-cd-final-recommendation}-20260821.md; .github/workflows/{ci,
  release}.yml; scripts/security/gitleaks-history.sh;
  scripts/agent-verify.sh
---

# CI/CD simplified redesign — single-maintainer correction

**Status: PROPOSED.** Nothing here is implemented. This document supersedes
§2–§4 and §8 of `ci-cd-final-recommendation-20260821.md` (the trigger/tier
structure, the required-check set, the nightly-schedule commitment, and the
soak-before-re-protecting rollout) **completely** — not just in shape, but
in every specific fact those sections asserted, several of which (a required
five-check ruleset, a nightly-scan commitment, "why required checks will
stay trustworthy") no longer apply once nothing is required. §1 of that
document (the challenge evaluation) still holds as a historical record. §5–§7
are restated here in corrected, current form (§4, §6) rather than inherited
by reference, because two of their specifics were already stale by the time
of this revision — see §4.

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

| Cut from the prior recommendation | Why it doesn't fit a single-maintainer repo |
|---|---|
| Merge queue *(already dropped by stage-3 research)* | No concurrent human PRs to serialize |
| `gate-fast` / `gate-build` aggregator jobs | Exist only to be **required** checks; nothing needs a stable aggregate name if nothing is required |
| Re-applying branch protection with a required-status-checks ruleset | This *is* the "gates/requires" the HITL is declining |
| Nightly `schedule` cron for GPU/gitleaks-full-history rehearsal | A recurring commitment is its own kind of ceremony |
| "Run clean for a few days" rollout gate | A soak period by another name |
| Full 5-platform matrix "once per push to main" as a distinct new trigger | Redundant once nothing requires it — see §3.5 |

## 2. The organizing idea

CI becomes **informational, not a gate**, and its cost becomes **proportional
to what the diff touches**, using the mechanism the repo already has —
`dorny/paths-filter` in the `changes` job (`.github/workflows/ci.yml` lines
113–129) — extended from one boolean to several, rather than a new
"taxonomy classifier" job. But path-scoping has a real ceiling: it can only
see which *files* changed, not what changed *inside* them, and today's
`verify` job (the single most expensive one) has no per-language entry
point at all. §3.3 and §3.6 are honest about exactly where that ceiling is,
per the correctness review.

Today, `changes` computes one thing (`docs_only`, via a `nonmd` filter), and
every heavy job shares `if: needs.changes.outputs.docs_only != 'true'` — so
editing one Windows diagnostic script runs literally everything else too,
and a comment fix in a `.rs` file runs the full 60-minute `verify` job
because it isn't a `.md` file.

## 3. Concrete shape

### 3.1 Extend the path-filter categories — with the repo's real layout

The first draft of this doc used `python/**` and `typescript/**` as filter
roots. Those are wrong: the actual Python SDK source is `src/python/**`
(`python/` at repo root is a build/example output directory, not source),
and the actual TypeScript/napi source is `src/ts/**`, which has its own
`package.json`/`package-lock.json` distinct from the repo-root ones.
Corrected:

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

`dorny/paths-filter`'s README documents that its default PR-diff mode can
fall back to the GitHub REST API depending on checkout depth/event type,
which needs `pull-requests: read`; `ci.yml` currently grants only
`contents: read` (line 13). The existing `changes` job has apparently
worked in production without that permission, which suggests it's staying
in local-git-diff mode given the current checkout — but that should be
**verified on a real PR, not assumed**, and `pull-requests: read` is cheap
enough to add defensively regardless (least-privilege cost is negligible;
the failure mode it prevents — paths-filter silently degrading — is not).

There is no `windows`-specific filter category in this list. §3.3 explains
why, concretely, rather than proposing one that would misbehave.

### 3.2 No silent "unclassified" gap

The prior draft implied any job not covered by an explicit path category
could receive **no** verification at all. That's not actually true of the
mechanism already in place, but it's worth stating explicitly rather than
leaving it implicit: `verify-fast` (the cheap, ~1–2 minute lint/typecheck/
security suite) keeps its current `if: needs.changes.outputs.docs_only !=
'true'` condition unchanged — it already runs on **any** non-doc change,
regardless of which specific path category matched. Only the *heavier*,
per-surface jobs (§3.4) get narrowed further. A change to, say,
`scripts/**` or a config file with no specific category still gets
`verify-fast` at minimum; it is never zero-signal.

### 3.3 Windows scoping — what path-filtering can and can't do here

`windows-wal-checkpoint-diagnosis` (`ci.yml` line 459) runs:

```text
cargo test -p fathomdb-engine --features operator --test erasure_completeness \
  erasure_busy_cross_process_windows_yields_typed_diagnostic
```

— a Windows-specific test *inside* `erasure_completeness.rs`, a generically
named file in core `fathomdb-engine`. No path glob (`**/*windows*`,
`**/*wal*`, or anything else) can distinguish a change to that specific test
from any other change to that file, because the Windows-specificity is
inside the file's content, not its path. A glob broad enough to catch it
would also match unrelated Rust files that happen to contain "wal" (e.g.
write-ahead-log code generally) and would match Windows-related *design
docs* under `dev/design/**/*windows*.md`, breaking the cheap-docs promise
in the other direction — exactly the "too broad and too narrow at once"
finding from the correctness review.

The honest scoping: this job stays under the `rust` category (§3.1) like
the rest of `fathomdb-engine` — which is *correct*, not a compromise, since
its test genuinely depends on core Rust code. What §3.1's filters *can*
scope precisely is changes to files that are actually, structurally
Windows-only: the workflow YAML step itself, or a dedicated
Windows-only script/module if one exists outside shared source (none
currently does for this job). The user's "one Windows-specific item"
example is best satisfied by keeping Windows-only *tooling* (scripts,
workflow steps) in Windows-only paths going forward, not by trying to infer
platform-specificity from file content after the fact — that's a repo
organization habit, not a CI mechanism.

### 3.4 What's immediately actionable vs. what needs a script change

The correctness review's sharpest finding: `verify` (the 60-minute heavy
suite) invokes `agent-verify.sh --tier=heavy`, which runs Rust, Python, and
TypeScript **together** — there is no existing way to run just the Python
suite. Claiming `python`-only changes would run only "verify's Python
suite" was not describing anything that exists.

**Immediately actionable (CI-YAML-only, no script changes):**

- `wheel-size-gate`, `native-artifact-runtime-validation` — already
  independent jobs; gate on `release_infra` or `rust` as appropriate.
- `windows-wal-checkpoint-diagnosis` / `windows-wal-attribution` — gate on
  `rust` per §3.3 (same condition as today's `docs_only != 'true'`, just
  narrower than "any non-doc change"). These should also stop running
  entirely until their `os.uname()` bug (challenge catalogue §1.3) is
  fixed — a broken diagnostic gains nothing from being scoped instead of
  removed.
- `verify-fast`, `verify`, `rust-workspace-race-report`, `security`,
  `default-embedder-tests` — stay on today's `docs_only != 'true'`
  condition (i.e. any `rust`/`python`/`typescript` category, treated as one
  bucket) because `verify` cannot currently be split further. This is
  **coarser than the pitch in the first draft** — a Python-only change
  still pays for the full Rust+Python+TS suite — and should be described
  that way, not oversold.

**Needs real script work first (out of scope for a docs/CI-YAML change):**

- Add a `--surface=rust|python|typescript` selector to
  `scripts/agent-verify.sh` / `scripts/agent-test.sh` so `verify` can
  actually run only the touched language's suite. Until that lands,
  per-language scoping of the biggest cost item in the pipeline is a stated
  future improvement, not something this doc can deliver on its own.

### 3.5 Fast environment preflight — per job, not a shared job

The first draft proposed one `preflight` job that other jobs `needs:`. That
doesn't work: GitHub-hosted jobs each get a fresh, independent VM (runner
images are also updated on a rolling weekly basis per GitHub's own docs), so
one job's tool-presence check proves nothing about a different job's
runner — including a different OS entirely (Windows/macOS jobs vs. the
Ubuntu preflight VM). Worse, a `needs: preflight` edge means a preflight
failure skips every other job's *real* diagnostics, hiding the actual
failure behind an unrelated one.

Corrected: put the tool-presence check as the **first step of each job that
needs it**, on that job's own runner, before any toolchain setup:

```yaml
      - uses: actions/checkout@...
      - name: Verify required tools are present
        shell: bash              # or pwsh on windows-latest, with Get-Command
        run: |
          set -euo pipefail
          for tool in rg jq git curl; do
            command -v "$tool" >/dev/null || { echo "::error::missing required tool: $tool"; exit 1; }
          done
```

On `windows-latest` runners, the equivalent step uses `Get-Command` in
`pwsh`, not `command -v`. This adds ~2–5 seconds per job rather than one
shared ~30-second job, and a missing-tool failure now reports standing
alone, on the runner that actually has the problem, before any expensive
setup — which is what "caught in the first 30 seconds" actually requires.

### 3.6 Full skip: use GitHub's native mechanism, not a custom one

The first draft proposed checking `github.event.head_commit.message` for a
`[ci-lite]` trailer in a workflow-level `if:`. Neither part of that works:
GitHub Actions doesn't support a workflow-level `if:` (only
`jobs.<job_id>.if` and step-level conditions), and
`github.event.head_commit` belongs to the `push` event payload, not `pull_
request` — a PR-triggered run wouldn't see it at all, and even on `push`,
checking only the head commit lets an earlier substantive commit in the
same push slip through unexamined.

Since nothing in this design is required, there is no need for a bespoke
partial-skip mechanism: GitHub's native `[skip ci]` / `[ci skip]` commit-
message markers (and the `skip-checks: true` pull-request/commit-status
equivalent) already skip an entire workflow run, are documented, and are
safe to rely on precisely *because* nothing here gates anything — a skipped
run was never going to block anything anyway. Use that for "this commit
needs no CI at all." For "run only the cheap stuff," §3.1–§3.4's automatic
path-scoping is the mechanism — there's no separate manual "lite mode" to
build or maintain.

### 3.7 What moves off the always-run path, and how

The full 5-platform matrix and any GPU/Tegra work never need a distinct
"once per push to main" trigger layer, because in this design nothing forces
them to run on every push in the first place — `release_infra`/`rust`
scoping already limits when the relevant jobs fire, and full cross-platform
validation belongs at `workflow_dispatch` (run on demand) and at the
existing tag-triggered `release.yml`, not as new standing infrastructure.

## 4. Gitleaks — corrected to current reality

The first draft's framing ("57 pre-existing findings block every PR, need a
one-time triage before moving off the blocking path") is **stale**. As of
this revision:

- `gitleaks` (`ci.yml` line 26) was already made non-blocking on `main`
  by commit `a755e1d8` ("ci: make Gitleaks report-only", 2026-08-20) —
  `continue-on-error: true` at the job level. It no longer blocks
  anything, on `main`, today. Challenge catalogue §1.1 is therefore
  already partially resolved independent of this redesign.
- It still **runs unconditionally on every push** (no `if:` at all — not
  even gated on `docs_only`), and both steps ("Scan current tracked tree"
  and "Scan reachable Git history" — `scripts/security/gitleaks-current.sh`
  and `scripts/security/gitleaks-history.sh`) run together in the same job
  every time. That's a real, current cost/volume issue, just not a
  blocking one.
- `scripts/security/gitleaks-history.sh` already implements an
  allowlist/reconciliation mechanism ("safe report") distinct from a raw
  finding count — re-running it during this revision showed
  `expected_records=100 observed_records=121 unknown=21`, i.e. the
  allowlist and the actual history are currently **out of sync**, not
  simply "57 untriaged findings" as the earlier catalogue stated. The exact
  current count and classification needs the maintainer to look at this
  script's own output, not this doc's summary of a snapshot from several
  days prior.

**Revised recommendation:** split the job's two steps by trigger instead of
running them together every time — "Scan current tracked tree" (fast,
diff-relevant) stays on every push; "Scan reachable Git history" moves to
`workflow_dispatch` plus (once actually wired — `release.yml` currently has
**no** gitleaks step at all, contrary to the assumption in the prior
recommendation doc) a real step in the release workflow. `gitleaks` itself
supports commit-range scanning via `--log-opts`, which is the natural way
to keep "current tracked tree" cheap without needing the full-history
machinery on every push. None of this is a security-blocking precondition
anymore, since the job is already report-only — it's a cost reduction and
an accuracy fix (reconciling the 21-record mismatch) that stands on its own,
whenever the maintainer wants to do it.

## 5. Explicitly out of scope

CUDA/maturin toolchain churn (challenge catalogue §1.2/§1.7) stays out of
scope — real release-engineering defects, not a CI-triggering problem.
`cargo hack check --each-feature` as a GPU-compile proxy (carried from
stage-3 research) is a reasonable idea but is **unverified for this repo's
actual CUDA/Metal/ONNX feature combinations** — it should be spiked once,
manually, before being relied on, not assumed to work.

## 6. What this doesn't solve, said plainly

Removing required checks removes the 2026-08-20 failure mode (a check
staying red for days, forcing a choice between living with it or deleting
the gate) by removing the gate, not by fixing the checks — a deliberate
trade for a single maintainer, and the risk is symmetric either way: a red
check nobody is forced to look at can go unnoticed longer than a required
one would. Fine-grained scoping (§3.1–§3.4) partly offsets this by making a
red result mean something more specific, but it is still on the maintainer
to look, not on GitHub to force it — no CI mechanism substitutes for that.

## 7. Revision note

This document was revised after an adversarial correctness review
identified eight concrete implementation errors in the first draft (wrong
`src/python`/`src/ts` paths; an invalid workflow-level `if:` for the
`[ci-lite]` idea; a `preflight` job design that can't validate a different
job's runner; an over/under-matching `windows_relevant` glob; an
unsupportable claim that `verify` could be split per-language today; a
stale gitleaks-triage framing that predated `a755e1d8`; a wrong claim that
`release.yml` already runs gitleaks; and an "any branch" push-trigger claim
where `ci.yml` actually restricts `push` to `main`). Every finding was
checked against the current repo before this revision, not taken on faith —
§3.1–§3.7 and §4 above are the corrected result. §0–§2 and §5–§6 (the
policy direction itself — informational CI, no gates, no soak) are
unchanged, because the review confirmed that direction was right; only the
mechanics were wrong.

## Appendix — pipeline diagram (v2, corrected)

```text
                  FathomDB CI -- simplified, single-maintainer shape (v2)
              (dev/design/ci-cd-simplified-redesign-20260821.md, PROPOSED)
                    informational only -- nothing here is a gate

  TRIGGERS (as ci.yml actually defines them today -- unchanged)
  ================================================================
  +----------------------+          +----------------------+
  | push: main only        |          | pull_request: any     |
  | (ci.yml `on.push.       |          | branch                 |
  |  branches: [main]`)     |          +-----------+-----------+
  +-----------+-----------+                      |
              |                                    |
              +---------------------+--------------+
                                     v
  full skip: commit carries [skip ci] / [ci skip] -- GitHub's OWN native
  mechanism, not a custom marker. Safe here because nothing is required,
  so a skipped run was never going to block anything anyway.
                                     |
                                     v
  +----------------------------------------------------------------+
  | changes (dorny/paths-filter -- EXTENDS the existing job)          |
  |   docs_only unchanged; adds, with CORRECTED real paths:            |
  |     rust           src/rust/**, Cargo.toml, Cargo.lock             |
  |     python          src/python/**            (NOT top-level        |
  |     typescript       src/ts/**, its own       python/ -- that's    |
  |                       package.json/-lock      build/example output)|
  |     release_infra   .github/workflows/**, scripts/release/**       |
  |   permissions gains `pull-requests: read` (defensive, verify on a  |
  |   real PR -- paths-filter's PR-diff mode can need the REST API)    |
  +----------------------------------------------------------------+
                                     |
              +----------------------+----------------------+
              |                                               |
              v                                               v
  +---------------------------+                 +--------------------------------+
  | EVERY job: tool-presence    |                 | verify-fast -- unchanged        |
  | check as ITS OWN first step,|                 |   condition: docs_only!='true'  |
  | on ITS OWN runner (Ubuntu/  |                 |   i.e. runs on ANY non-doc      |
  | Windows/macOS each get a    |                 |   change -- the baseline; NEVER |
  | fresh VM -- one shared      |                 |   zero-signal even for an       |
  | "preflight" job proves      |                 |   unclassified path             |
  | nothing about another job's |                 +--------------------------------+
  | runner)                     |
  |   bash: command -v rg jq    |
  |         git curl            |
  |   pwsh (windows-latest):    |
  |         Get-Command          |
  |   ~2-5s per job, fails loud, |
  |   BEFORE toolchain setup     |
  +---------------------------+

  IMMEDIATELY ACTIONABLE (CI-YAML only)              NEEDS SCRIPT WORK FIRST
  ======================================              ==========================
  wheel-size-gate                  -> release_infra   verify (60-min heavy suite)
  native-artifact-runtime-         -> release_infra    runs Rust+Python+TS
    validation                                          TOGETHER via
  windows-wal-checkpoint-          -> rust (NOT a       agent-verify.sh --tier=heavy.
    diagnosis / -attribution          windows_*         No per-language selector
    (once os.uname() bug fixed;       filter -- its     exists today. Splitting it
    broken diagnostics run for        test lives in     needs a --surface=rust|
    NOBODY until then)                erasure_          python|typescript flag
                                       completeness.rs,  added to agent-verify.sh /
                                       a generic Rust    agent-test.sh FIRST.
                                       file -- no path   Until then: python-only
                                       glob can see a    and typescript-only changes
                                       #[cfg(windows)]   still pay for the full
                                       block inside it)  Rust+Python+TS run, same as
                                                          today's docs_only split --
                                                          coarser than first hoped,
                                                          stated honestly.

  ------------------------------------------------------------------------------
  RESULT ON THE CHECKS TAB / PR -- INFORMATIONAL ONLY
  ------------------------------------------------------------------------------
    red or green, every time -- but nothing is REQUIRED.
    no branch-protection ruleset re-applied. no merge queue. no "requires."
    maintainer reads the Checks tab and decides -- GitHub enforces nothing.

  ------------------------------------------------------------------------------
  GITLEAKS -- split by cost, not blocking status (already report-only since
  commit a755e1d8, 2026-08-20 -- this is a COST fix, not a security-gate fix)
  ------------------------------------------------------------------------------
    "Scan current tracked tree"  -> stays on every push, cheap, diff-relevant
    "Scan reachable Git history" -> moves to workflow_dispatch + a REAL step
                                     added to release.yml (which currently has
                                     NO gitleaks step at all)
    the allowlist/reconciliation script (gitleaks-history.sh) currently shows
    expected_records=100 observed_records=121 unknown=21 -- maintainer's own
    look at that output is the source of truth, not any doc's finding count.

  ------------------------------------------------------------------------------
  RELEASE  (release.yml -- unchanged in kind; full 5-platform matrix and GPU/
  Tegra work live here or behind workflow_dispatch, never as new standing
  nightly/per-push infrastructure)
  ------------------------------------------------------------------------------
  +----------------------+          +--------------------------------------+
  | tag push (v0.x.y)       |------> | crates.io * PyPI (5 wheel variants) * |
  | or workflow_dispatch     |        | npm (6 packages) * gitleaks full-      |
  +----------------------+         | history * GPU/Tegra rehearsal          |
                                     +--------------------------------------+
```
