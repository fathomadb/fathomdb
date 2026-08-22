---
title: CI challenges review — Aug 10-21 run sampling
date: 2026-08-21
status: PROPOSED
desc: >
  Evidence-based enumeration of the CI/CD challenges that made the Aug 10-21
  window slow and tedious, drawn from two rounds of stratified GitHub Actions
  run sampling (17 + 5 runs) cross-referenced against Codex CLI session
  transcripts, plus the confirmed timeline of the `main` branch-protection
  ruleset deletion. Analysis and recommendations ONLY; no CI config, script,
  workflow, or GitHub setting is changed by this doc.
---

# CI challenges review — Aug 10-21 run sampling

**Status: PROPOSED.** Nothing here is implemented. This document changes no
workflow, script, GitHub setting, or ledger. Every recommendation is a
proposal for the HITL to accept, defer, or reject.

## 0. Scope and method

`gh run list` on `fathomadb/fathomdb` returned 406 runs since 2026-08-10
(success=276, failure=80, cancelled=48; run volume climbed from single digits
around Aug 12-15 to 75-103 runs/day by Aug 17-21). Two stratified samples were
diagnosed against real job logs (`gh run view --json jobs`, `gh run view
--log-failed`), not just conclusions:

- **Round 1 (17 runs, Aug 10-21):** 31431462970, 31443634708, 31447394993,
  31452356955, 31571945944, 31903375373, 31977440603, 31979915110,
  32032409499, 31996534452, 32163861345, 32313607704, 32311445186,
  32424855656, 32426949907, 32452522675, 32441413348.
- **Round 2 (5 runs, spread Aug 11-21):** 32332311868, 31968356436,
  32022209240, 32512655687, 31452374198.

Both rounds were cross-referenced against Codex CLI session transcripts
(`/home/coreyt/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`) for the same time
windows, to see what the driving agent was actually experiencing, not just
what the API reports.

## 1. Enumerated CI challenges

### 1.1 Gitleaks full-history scan — the single most persistent blocker

The `gitleaks` job's "Scan reachable Git history" step (distinct from its
"Scan current tracked tree" step, which passes) scans the **entire git
history**, not the diff. Local pre-commit gitleaks hooks only scan the
current diff, so a clean local commit routinely fails this step in CI with no
local way to reproduce or clear it.

**Hit in both rounds, repeatedly, over 5+ days:** Round 1 —
31977440603, 31979915110, 32032409499, 31996534452, 32163861345,
32311445186, 32313607704 (every `release/0.8.23` and closeout-branch CI run
sampled). Round 2 confirms the *same* failing step recurring on two more
dates: 32332311868 (Aug 20, `ci/release-0.8.23-cuda-control-plane-20260820`)
and 31968356436 (Aug 16, `release/0.8.23`) — this is not a new issue, it is
the same unresolved finding set (35 leaks Aug 16-17, growing to 57 by Aug 19)
recurring on every push to the affected branches, never allowlisted or
triaged to zero.

**Verdict: ceremony/infra friction, not real signal in its current form.**
The findings are almost certainly pre-existing/historical, not new secrets
introduced by the commits under test — a check that fails identically and
permanently regardless of what changes is not gating anything. It needs
either a baseline allowlist (fixing it once) or narrowing to diff-scope (to
match what local hooks already enforce), not indefinite red status.

### 1.2 CUDA/maturin toolchain and release-artifact wiring

Failures traced to GCC/nvcc pinning churn (`31447394993`, `32424855656`
building the CUDA wheel — `💥 maturin failed`) and a build-order bug where a
release job consumed a CLI tarball (`tar: /input/fathomdb-cli.tar.gz: Cannot
open`) before an earlier job produced it (`32452522675`, self-hosted
`windchill3` RTX-3090 runner). Round 2's `32512655687` ("fix(release):
declare CUDA N-API glibc floor", now landed as `d8dd3602` on `main`) failed
`verify-fast`'s cheap-suite stage during active work to pin the glibc floor
for the containerized N-API build — in-flight hardening, not flakiness.

**Verdict: real signal, currently expensive.** This is genuine release-eng
complexity (cross-compiling CUDA wheels across driver/compiler/glibc
combinations on self-hosted GPU runners) surfacing as CI failures because
it's being debugged *in* CI rather than caught earlier. Legitimate to keep
blocking; the cost is the debug loop being expensive per iteration, not that
the gate is wrong.

### 1.3 Windows-only script bug in a required diagnostic

`windows-wal-attribution` fails identically every run with
`AttributeError: module 'os' has no attribute 'uname'` — `os.uname()` is
POSIX-only, used inside a script written for the Slice-65 Windows-WAL
checkpoint-conflict investigation (`dev/design/0.8.23-windows-wal-checkpoint-reader-conflict.md`,
`dev/plans/runs/0.8.23-slice-65-wal-attribution.md`). Round 1:
32311445186, 32313607704. Round 2 confirms recurrence: 32022209240
(Aug 17). Transcript correlation (session
`rollout-2026-08-17T04-33-49-...`) shows this was an actively-worked,
deliberate Windows WAL diagnosis effort at the time — not abandoned code —
but the diagnostic script itself has a portability bug that makes it fail
identically on every Windows run regardless of the underlying investigation's
progress.

**Verdict: ceremony/infra friction (the bug), wrapping real work (the
investigation).** The underlying Slice-65 question is real; the script
crashing on `os.uname()` on every single Windows run is a trivial, fixable
defect that has been failing "required" CI for days without being patched.

### 1.4 Legitimate in-flight test failures

Round 1's 31977440603 (`verify` job) failed 3 real suites —
`EmbedderRequiredError`, an embedder-helper-surface parity assertion, an
edge-FTS assertion — genuine TDD-in-progress signal, not infrastructure.
`test-python` alone ran 14+ minutes as part of that job.

**Verdict: real signal, correctly blocking.**

### 1.5 Cascading cancellations inflate the failure/cancel picture

Once one gate in a run fails (most often `gitleaks` or `verify`), GitHub
auto-cancels the remaining 10-30 downstream/matrix jobs in that same run
(publish, wheel-size-gate matrix, native-artifact-runtime-validation matrix,
etc.). Round 1: 31443634708, 31452356955, 32426949907 and others. Round 2's
31452374198 (Aug 11, cancelled) shows this cleanly — 8 wheel/runtime-matrix
jobs cancelled off the back of one upstream stop, not 8 independent problems.
**Verdict: not a distinct root cause** — it's a multiplier on whichever gate
failed first, but it makes the raw failure/cancel counts (80 failed + 48
cancelled of 406) look worse than the number of *independent* problems
actually is.

### 1.6 Full, heavy CI runs on every push — a volume/cost problem, independent of correctness

**Operational observation (HITL, direct/confirmed — not independently
verified against transcripts beyond the corroborating check below):** every
push kicked off a CI run, and **every commit that touched any non-markdown
file triggered the full, heavy CI process** — there was no lightweight
incremental path for ordinary code changes. `dev/steward/branch-protection.md`
documents a `changes`-path filter that skips `verify`/`security`/
`default-embedder-tests`/`rust-workspace-race-report` **only on docs-only
PRs**; the moment any non-markdown file is touched, the full matrix runs
regardless of how small the actual change is.

**Corroborating check:** five consecutive successful full `CI` runs on
`main` sampled directly from `gh run list` on 2026-08-21 (IDs 32459127146,
32457748776, 32456248894, 32454931956, 32453613811) each took **19-21
minutes wall-clock**, spaced roughly 19-21 minutes apart — i.e. full CI runs
were firing back-to-back on `main` with essentially no idle gap between them.
At the Aug 20-21 push cadence (100+ runs/day), that is consistent with the
full ~20-minute matrix being the *steady-state* cost of every push, not an
occasional tax.

**Verdict: real, distinct contributor to "CI became slow/tedious" —
independent of any single correctness gate.** Even with zero failing checks,
a ~20-minute full run on every non-doc push, at a 75-103 runs/day cadence,
is a throughput problem in its own right. This is a volume/cost issue that
sits underneath and compounds every failure mode above: the same
gitleaks/Windows/CUDA failures would have been far cheaper to iterate on if
ordinary pushes triggered a fast, scoped check instead of the full matrix.

## 2. Branch-protection ruleset — timeline (confirmed from git, GitHub API, and Codex transcripts)

| Date (UTC unless noted) | Event | Evidence |
|---|---|---|
| 2026-08-04 | 16-required-check ruleset (`default_ruleset`, id `20166133`) applied and verified on `main` under `coreyt/fathomdb` | `dev/steward/branch-protection.md`, `dev/steward/branch-protection-ruleset.json` |
| 2026-08-10 ~08:53 local | Repo transferred `coreyt/fathomdb` → `fathomadb/fathomdb` (new org, created 2026-08-10) to host an org-scoped self-hosted GPU runner group for the CUDA release build — unrelated to CI/guardrail fatigue | commit `1b845503` "ci: trigger CodeQL after org transfer"; Codex session ~08:53, quote: *"ACCEPT. The transfer is reflected by the `fathomadb/fathomdb` remote, the two-runner group is accurately described, and tag-release access remains explicitly unclaimed pending Slice 0's tested main-dispatch or exact-tag procedure."* |
| 2026-08-10 ~10:19-10:42 local | Branch protection actively **rebuilt** post-transfer after a stale `CodeQL` required-check entry blocked PR #216 | Codex quote: *"Almost—the correct CodeQL merge-protection rule is now present, but the old required status check is still there... That obsolete entry is why #216 is still blocked."* |
| 2026-08-20 23:27:20 UTC | Ruleset **deliberately deleted**: `gh api --method DELETE repos/fathomadb/fathomdb/rulesets/20166133`, following an explicit HITL instruction earlier in the same session | HITL: *"remove the gh main protections, etc. this is all for ceremony. no true value."* Codex: *"The only active `main` ruleset is `default_ruleset`: it requires PRs, blocks non-fast-forwards/deletions, and blocks writes pending CodeQL, with no owner bypass. I'm removing that ruleset now."* → *"The PR/CodeQL ruleset is deleted, and `main` has no separate legacy branch protection."* |
| 2026-08-21 (today) | Confirmed still unprotected: `gh api repos/fathomadb/fathomdb/rulesets` → `[]`; `gh api repos/fathomadb/fathomdb/branches/main/protection` → `404 Branch not protected` | Direct API check, this session |

The deletion followed a session where Codex had already triggered a wave of
unwanted CI runs and been told to stop touching CI entirely (*"I should have
stopped after the direct-push rejection instead of creating #242... I will
not start, merge, approve, poll, or otherwise touch any more CI."*) — the
"ceremony, no true value" judgment was leveled at that accumulated friction
(closely tracking §1.1 and §1.3 above), but the deletion removed the entire
ruleset, including `non_fast_forward` and `deletion` protection and the
required-PR-review gate, which were not part of that complaint.

**`dev/steward/steward-ledger.jsonl` has no entry for the deletion**, and
`dev/steward/branch-protection.md` still asserts (as of its last edit,
2026-08-04) that protection is "APPLIED AND VERIFIED." **That file is now
stale and should be corrected or superseded** — not done as part of this
review, which is diagnostic only.

## 3. Open decision for HITL

`main` currently has **no branch protection at all**: no required checks, no
PR requirement, no force-push or deletion protection. Two separate questions:

1. **Re-apply protection, and if so, what shape?** A trimmed ruleset that
   drops the two challenges shown above to be ceremony rather than signal —
   the full-history gitleaks scan (§1.1) and the broken Windows diagnostic
   (§1.3) — while keeping the checks catching real signal (§1.4, and CUDA
   work once past its current debug churn) would preserve `non_fast_forward`
   /`deletion` protection and PR-gating without reintroducing the specific
   friction that prompted the "no true value" call.
2. **Address the volume/cost problem (§1.6) independently of which checks
   are required** — e.g. a scoped/fast-path CI tier for ordinary non-doc
   pushes, distinct from the full ~20-minute matrix, so that re-applying
   protection doesn't simply reintroduce the throughput problem alongside it.

Neither is actioned by this document.
