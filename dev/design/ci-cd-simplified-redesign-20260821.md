---
title: CI/CD simplified redesign — single-maintainer correction
date: 2026-08-21
status: PROPOSED
desc: >
  HITL correction to `ci-cd-final-recommendation-20260821.md`: that doc's
  tiers/aggregators/required-checks/rollout-soak shape is more ceremony than
  a single-maintainer repo needs. This doc keeps that doc's diagnosis (the
  challenge catalogue, the delivery-requirements map) but replaces its
  prescription with a much smaller one: informational (non-gating) CI,
  fine-grained path scoping in place of the existing single `docs_only`
  flag, a fast environment preflight, and heavy/fragile work moved to
  on-demand rather than either "every push" or a new nightly commitment.
  Analysis and recommendation only; no CI config, script, or GitHub setting
  is changed by this document.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research,
  ci-cd-final-recommendation}-20260821.md; .github/workflows/ci.yml
---

# CI/CD simplified redesign — single-maintainer correction

**Status: PROPOSED.** Nothing here is implemented. This document supersedes
§2–§4 and §8 of `ci-cd-final-recommendation-20260821.md` (the trigger/tier
structure, the required-check set, the nightly-schedule commitment, and the
soak-before-re-protecting rollout). It does **not** supersede that doc's §1
(challenge evaluation) or §5–§7 (gitleaks precondition, stale-doc flags,
honesty about what a redesign can't mechanically guarantee) — those still
hold and are cited here by reference rather than repeated.

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

The prior doc's §2–§3 built exactly the kind of process the HITL is
rejecting here: two new required aggregator checks (`gate-fast`,
`gate-build`), a restored 5-name required-check branch-protection ruleset,
and a rollout sequence that explicitly waited for the new tiers to "run
clean for a few real days" before re-gating `main` (§8.5) — a soak period in
substance even though that doc didn't use the word. That shape is wrong for
this repo. This doc replaces it.

## 1. What's being cut, and why

| Cut from the prior recommendation | Why it doesn't fit a single-maintainer repo |
|---|---|
| Merge queue *(already dropped by stage-3 research, doc 4 §1 — reconfirmed here)* | No concurrent human PRs to serialize |
| `gate-fast` / `gate-build` aggregator jobs | Exist only to be **required** checks; if nothing is required, nothing needs a stable aggregate name |
| Re-applying branch protection with a 5-check required-status-checks ruleset | This *is* the "gates/requires" the HITL is explicitly declining. A solo maintainer reads the Checks tab and decides; GitHub doesn't need to enforce that decision for them |
| Nightly `schedule` cron for GPU/gitleaks-full-history rehearsal | A new recurring commitment is its own kind of ceremony — infra that runs and must be watched whether or not anything changed |
| "Run clean for a few days" rollout gate (§8.5 of the prior doc) | A soak period by another name |
| Full 5-platform matrix "once per push to main" | Still runs on every trunk push regardless of what changed; a comment-typo commit to `main` still pays for it |

What's **kept unchanged** from the prior investigation: the gitleaks
triage-to-zero precondition (§5 of the final recommendation — still true,
still has to happen before the full-history scan can be trusted either way),
the honest §7 point that no CI mechanism prevents a required check from
rotting again (more true than ever once there *are* no required checks —
see §5 below), and the flags on `docs/compatibility/index.md` and
`dev/steward/branch-protection.md` being stale.

## 2. The organizing idea

CI becomes **informational, not a gate**, and its cost becomes **proportional
to what the diff actually touches**, using the mechanism the repo already
has — `dorny/paths-filter` in the `changes` job (`.github/workflows/ci.yml`
lines 113–129) — extended from one boolean to a handful, instead of the
prior doc's new "taxonomy classifier" concept (which was heavier machinery
for the same idea). Nothing needs a new job type; the existing jobs just get
narrower `if:` conditions.

Today, `changes` computes exactly one thing:

```yaml
docs_only: ${{ steps.filter.outputs.nonmd == 'false' }}
```

and every heavy job (`verify`, `verify-fast`, `rust-workspace-race-report`,
`security`, `default-embedder-tests`, both Windows WAL jobs, `wheel-size-gate`,
`native-artifact-runtime-validation`) shares the single condition
`if: needs.changes.outputs.docs_only != 'true'` — so a change to *one* Windows
diagnostic script runs literally everything else too, and a one-character
comment fix in a `.rs` file runs the full 60-minute `verify` job because it
isn't a `.md` file. That's the direct mechanism behind both examples in the
HITL's correction.

## 3. Concrete shape

### 3.1 Extend the path-filter categories (in place of a new taxonomy job)

```yaml
  changes:
    outputs:
      docs_only: ...            # unchanged
      rust:      ${{ steps.filter.outputs.rust }}
      python:    ${{ steps.filter.outputs.python }}
      typescript: ${{ steps.filter.outputs.typescript }}
      windows_relevant: ${{ steps.filter.outputs.windows_relevant }}
      release_infra: ${{ steps.filter.outputs.release_infra }}
    steps:
      - uses: dorny/paths-filter@...
        with:
          filters: |
            nonmd: [ '!**/*.md' ]
            rust: [ 'src/rust/**', 'Cargo.toml', 'Cargo.lock' ]
            python: [ 'python/**', 'src/rust/crates/fathomdb-py/**' ]
            typescript: [ 'typescript/**', 'src/rust/crates/fathomdb-napi/**', 'package.json' ]
            windows_relevant: [ '**/*windows*', '**/*wal*' ]
            release_infra: [ '.github/workflows/**', 'scripts/release/**' ]
```

Each existing job's `if:` narrows from the single `docs_only != 'true'` to
the category it actually depends on — for example:

- `verify` → `if: needs.changes.outputs.rust == 'true' || needs.changes.outputs.python == 'true' || needs.changes.outputs.typescript == 'true'`
- `windows-wal-checkpoint-diagnosis` / `windows-wal-attribution` → `if: needs.changes.outputs.windows_relevant == 'true'` (once their `os.uname()` bug — doc 1 §1.3 — is actually fixed; broken diagnostics shouldn't run at all in the meantime, blocking or not)
- `wheel-size-gate`, `native-artifact-runtime-validation` → gated on `release_infra` or `rust` as appropriate
- the 9 always-on governance jobs (`board-currency`, `ledger-integrity`, `plan-anchors`, `governed-surface-pin`, `pinned-override-rot`, `c1-contract-conformance`, `transcript-hygiene`, `release-state-views`, `steward-orient`, `docs`) currently carry **no `if:` at all by deliberate design** (see the "Deliberately carries NO `if:` and NO `needs:`" comments in `ci.yml`). That's a separate, pre-existing design decision about the steward/ledger process, not something this doc's diff-scoping mechanism should silently change — but it's worth the HITL flagging explicitly in §6 below whether that governance-on-every-push posture is itself still wanted, since it's the same shape of always-on ceremony this doc is otherwise removing.

A misspelling fixed in a Rust comment: `rust == 'true'` (paths-filter is
path-based, not content-based, so it can't tell a comment fix from a logic
change by itself — see the escape hatch in §3.3 for that gap) still runs the
now-scoped `verify`. A misspelling in a `.md` file: `docs_only` alone,
nothing else. A one-line edit to `windows-wal-attribution.py`: only
`windows_relevant` fires — no Rust build, no Python wheel, no five-platform
anything.

### 3.2 Fast environment preflight — target under 30 seconds

Today's tool bootstrapping is buried: `verify-fast` installs `ripgrep` via
`apt-get` as its sixth step (`ci.yml` line 161–162), *after* checkout,
TMPDIR setup, Python setup, Node setup, and Rust toolchain install — so a
genuine environment problem (a tool `scripts/bootstrap.sh` or an
`agent-verify` suite assumes exists, and doesn't) only surfaces minutes in,
mixed in with real build/test failures, not distinguished from them.

Add one job, first in the graph, that does nothing but check the runner has
what the scripts need — no checkout of the full history, no language
toolchain installs:

```yaml
  preflight:
    runs-on: ubuntu-latest
    timeout-minutes: 1
    steps:
      - uses: actions/checkout@... # shallow, no fetch-depth: 0
      - name: Verify required tools are present
        run: |
          set -euo pipefail
          for tool in rg jq git curl; do
            command -v "$tool" >/dev/null || { echo "::error::missing required tool: $tool"; exit 1; }
          done
```

Everything else (`needs: [changes, preflight]`) waits on this, but since
it's ~5–10 seconds on a GitHub-hosted runner, it isn't a meaningful serial
tax — it just means a missing-tool failure reports in seconds, standing
alone, instead of being buried in a 30–60 minute job's failure log next to
unrelated test output.

### 3.3 Escape hatch for changes path-scoping can't see

Path filters are path-based, not content-based — they can't distinguish "a
Rust comment typo" from "a Rust logic change" in the same file. Rather than
building content-aware diffing (real machinery, disproportionate for this),
give the maintainer (or an agent committing on their behalf) an explicit,
cheap opt-out: a commit-message trailer, checked in one `if:` at the very
top of the workflow, no new job:

```yaml
if: ${{ !contains(github.event.head_commit.message, '[ci-lite]') }}
```

`[ci-lite]` in a commit message skips everything except `preflight`,
`changes`, and the always-fast lint/typecheck step — the maintainer's call,
made per-commit, not a rule the CI infers or enforces. This is the kind of
mechanism that fits "not trying to add gates": it's a way to say *less* CI
is needed this time, never a way CI blocks anything.

### 3.4 No required status checks

Branch protection on `main` is **not re-applied** as a required-status-checks
ruleset. CI still runs, still reports red/green on every commit and the
Checks tab, still stays genuinely informative once §3.1's scoping makes red
mean something specific — but nothing in GitHub configuration prevents a
push or merge based on it. If the HITL wants *some* minimal accident-guard —
`non_fast_forward` (blocks force-push) and `deletion` (blocks deleting
`main`) protection alone, with **no required status checks and no PR-review
requirement** — that's a one-line-each addition and genuinely just guards
against fat-fingering, not a workflow gate. It's optional, not this doc's
recommendation either way; see §6.

### 3.5 Heavy/fragile work: on-demand, not nightly

The prior doc moved full-history gitleaks and GPU/Tegra builds to a nightly
`schedule`. This doc moves them to `workflow_dispatch` (run-it-when-you-want)
plus the existing tag-triggered `release.yml`, and drops the recurring cron
entirely — no new always-on infrastructure to babysit. The gitleaks
triage-to-zero precondition from `ci-cd-final-recommendation-20260821.md`
§5 is unchanged: do that once, then the full-history scan is available
on-demand and at release time, never a recurring commitment and never a
per-push blocker.

## 4. Explicitly out of scope (same as the prior doc)

CUDA/maturin toolchain churn (`ci-cd-final-recommendation-20260821.md`
§1.2/1.7) stays out of scope here too — it's a real release-engineering
defect (GCC/nvcc pinning, build-order bugs), not a CI-triggering problem,
and no amount of scoping or de-gating touches it. The `cargo hack check
--each-feature` GPU-compile proxy from the prior doc's research (doc 4 §3)
is still a reasonable cheap early-warning addition inside the `rust`-scoped
Tier, independent of everything else this doc changes.

## 5. What this doesn't solve, said plainly

Removing required checks removes the specific 2026-08-20 failure mode (a
required check that stays red for days, forcing a choice between living
with permanently-failing gates or deleting them) by removing the gate, not
by fixing the checks. That is a deliberate trade, not an oversight: for a
single maintainer, "CI can be wrong sometimes and I'll notice" is a
reasonable posture that "CI must never be wrong because it blocks me" is
not, especially given the concrete history of gitleaks and the Windows
diagnostic both being wrong for days at a time. The risk this accepts: a
red check nobody is forced to look at can go unnoticed longer than a
required one. Fine-grained scoping (§3.1) partly offsets this — a red
`windows_relevant` check next to a Windows-only commit is a much clearer
signal than one red name among 25 unconditional jobs — but it's still on the
maintainer to look, not on GitHub to force it.

## 6. Open decisions for HITL

1. **The 9 always-on governance jobs** (`board-currency`, `ledger-integrity`,
   `plan-anchors`, `governed-surface-pin`, `pinned-override-rot`,
   `c1-contract-conformance`, `transcript-hygiene`, `release-state-views`,
   `steward-orient`) currently run unconditionally on every push by
   deliberate design, independent of this doc's scoping mechanism. Worth an
   explicit call: keep them always-on (they're comparatively cheap, doc-state
   checks rather than builds), or fold them into the same path-scoping
   (e.g., only run when `dev/plans/**`, `dev/steward/**`, or `dev/design/**`
   changed)?
2. **Whether to keep any branch protection at all** (§3.4) — fully open
   (nothing), or the minimal `non_fast_forward`+`deletion` accident-guard
   with zero required checks.
3. **The `[ci-lite]` trailer's exact scope** — this doc proposes it skips
   everything but preflight/changes/lint; the HITL may want it narrower or
   broader.
4. **Whether `workflow_dispatch`-only (no nightly) is sufficient** for
   catching CUDA/Tegra regressions between releases, or whether an
   occasional (not nightly) manual cadence should be a personal habit rather
   than automated at all.
