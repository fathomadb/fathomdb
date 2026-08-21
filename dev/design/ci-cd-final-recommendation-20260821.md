---
title: CI/CD redesign — final evaluation and recommendation
date: 2026-08-21
status: PROPOSED
desc: >
  Stage 4 (final) of a 4-stage CI/CD redesign investigation. Evaluates the
  best-practice-corrected design (stage 3's deltas applied to stage 2's
  hypothesis) against every challenge catalogued in stage 1, then gives a
  concrete, decision-ready recommendation for the HITL. Analysis and
  recommendation only; no CI config, script, or GitHub setting is changed by
  this doc.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis,ci-cd-best-practices-research}-20260821.md;
  dev/steward/branch-protection.md; docs/compatibility/index.md;
  .github/workflows/{ci,release,aarch64-release-preflight,
  jetson-tegra-cuda-evidence}.yml
---

# CI/CD redesign — final evaluation and recommendation

**Status: PROPOSED.** Nothing here is implemented. No workflow, script,
GitHub setting, or ledger entry changes as a result of this document. Every
recommendation below is for the HITL to accept, defer, amend, or reject.

## 0. Inputs

This is stage 4 of a 4-stage investigation. It does not re-derive anything —
it evaluates and recommends against three prior committed docs plus the
challenge catalogue:

1. `dev/design/ci-challenges-review-20260821.md` — ground truth of what
   actually went wrong, from sampling 22 real GitHub Actions runs (Aug
   10-21) cross-referenced against Codex session transcripts, plus the
   confirmed branch-protection ruleset deletion timeline.
2. `dev/design/delivery-requirements-map-20260821.md` — the real delivery
   surface (10 crates, 3 bindings, 5 published platform targets + Jetson
   unpublished, 3 glibc floors, 2 self-hosted GPU runners, 14 published
   packages/3 registries, ~25 CI jobs in `ci.yml` alone).
3. `dev/design/ci-cd-design-hypothesis-20260821.md` — stage-2 hypothesis:
   four triggers, a changed-file taxonomy, aggregator gate jobs, GPU/gitleaks
   demotion off the PR path.
4. `dev/design/ci-cd-best-practices-research-20260821.md` — stage-3 research
   that corrected the hypothesis (drop merge queue, use `concurrency:
   group: main-full-matrix, cancel-in-progress: true` instead) and confirmed
   the rest (aggregator pattern is GitHub's own documented fix; taxonomy +
   aggregator are coupled, not independent; `cargo hack check
   --each-feature` over a single invocation; gitleaks baseline/rotate
   split), closing with 8 concrete deltas.

The **best-practice-corrected design** referenced throughout this doc means:
hypothesis (doc 3) with all 8 of doc 4 §7's deltas applied — most visibly, no
merge queue and no `gate-merge-queue`; a `concurrency`-group-serialized
full-matrix run on push to `main` instead.

## 1. Evaluation — does the corrected design resolve each catalogued challenge?

Each subsection below corresponds 1:1 to a numbered challenge in
`ci-challenges-review-20260821.md` §1, plus the branch-protection timeline in
its §2.

### 1.1 Gitleaks full-history scan — persistent blocker (§1.1 of doc 1)

**Verdict: PARTIALLY RESOLVED.**

The mechanism is right and externally validated (doc 4 §5): move the
full-history "Scan reachable Git history" step off every push, onto a nightly
`schedule` + `workflow_dispatch`; keep the cheap diff-scope "Scan current
tracked tree" step in Tier 1, always-on, genuinely gating new secrets. This
directly kills the specific failure mode — a check red identically on every
push, for days, regardless of what changed.

But the resolution is **conditional on a precondition that is not itself a
CI mechanism**: the 57 pre-existing findings must be triaged to zero (rotated
if live, allowlisted with a documented reason if dead) *before* the nightly
job goes live, per doc 4 §5's two-way classification. If that triage doesn't
happen, the redesign has only changed the shape of the problem — instead of
a permanently-red PR-blocking check, it becomes a permanently-red (or
silently muted) nightly job that nobody looks at, which is arguably worse
because it's no longer even visible on every PR. This is exactly doc 4 §7's
own caveat: "nightly + visible" only stays "visible" if someone owns
re-triaging it; unowned, it degrades to "nightly + ignored." **Resolved in
mechanism, not yet in fact** — the triage is a prerequisite step this
recommendation treats as a hard precondition (§3.4 below), not something the
redesign itself performs.

### 1.2 CUDA/maturin toolchain and release-artifact wiring (§1.2 of doc 1)

**Verdict: NOT RESOLVED — and not meant to be.**

Doc 1 itself verdicts this as "real signal, currently expensive," not
ceremony — the redesign's job is to make the debug loop cheaper, not to make
the underlying cross-compilation complexity disappear. The redesign helps
indirectly: the `cargo hack check --each-feature` GPU-feature compile proxy
(doc 4 §3's strengthening of the hypothesis) catches a class of CUDA-path
breakage cheaply, on GitHub-hosted infra, before it ever reaches the
self-hosted `windchill3` box, which shortens *some* debug loops. But it does
not fix GCC/nvcc pinning churn, glibc-floor divergence, or build-order bugs
(the `fathomdb-cli.tar.gz` ordering failure) — those are genuine release
engineering defects that need to be fixed as defects, independent of any CI
retiering. No part of this redesign claims otherwise; this challenge sits
outside its scope by design.

### 1.3 Windows-only script bug in a required diagnostic (§1.3 of doc 1)

**Verdict: PARTIALLY RESOLVED — fixes this instance, does not prevent a
recurrence of the class.**

The redesign explicitly removes `windows-wal-checkpoint-diagnosis` /
`windows-wal-attribution` from `ci.yml`'s required path entirely (hypothesis
§1.3: "they are not build/validate jobs and should never have been modeled
as required CI in the first place"). That closes this specific instance.

What it does *not* do is provide a structural guardrail against the same
mistake recurring in a different form — an in-progress investigation script
getting wired as a required blocking check without anyone checking it can
actually pass on the target platform. The taxonomy/aggregator machinery
(§1.5/§1.6) makes it *harder* to add a new required check by accident (a new
job has to be deliberately added to an aggregator's `needs:` list and
covered by a taxonomy label, which is more friction than today's implicit
"add a job, it's required if not `docs_only`"), but that is incidental
friction, not a designed check. Nothing in the corrected design adds a
review step or a "does this diagnostic actually pass in CI before it's
required" gate. This is a real gap — see recommendation §3.6.

### 1.4 Legitimate in-flight test failures (§1.4 of doc 1)

**Verdict: RESOLVED, unchanged, by design.**

Doc 1 verdicts this "real signal, correctly blocking" — the redesign
explicitly preserves `verify` (heavy Rust/Python/TS suites) as a required
Tier 2 job, gated to run whenever the taxonomy indicates code paths were
touched. Nothing about this class of failure was ever framed as a problem to
solve; the redesign's entire premise (hypothesis §0) is to stop conflating
this kind of real signal with the ceremony around it, not to weaken it.

### 1.5 Cascading cancellations inflate failure/cancel counts (§1.5 of doc 1)

**Verdict: RESOLVED as a measurement artifact; substantially reduced as a
lived cost.**

Doc 1 itself verdicts this "not a distinct root cause" — a multiplier on
whichever gate fails first, not an independent problem. The redesign reduces
its *practical* impact in two ways: (a) Tier 1 is small and fast, so a
failure there cancels only a handful of cheap jobs, not 10-30 heavy
platform-matrix legs; (b) GPU jobs are off the PR path entirely, so a
gitleaks/verify failure on a PR can no longer cascade-cancel a self-hosted
GPU run that was mid-flight. The cascade mechanism itself (GitHub
auto-cancelling a run's remaining jobs on failure) is unchanged and not
something CI config controls — this challenge was never "wrong," just
inflated-looking, and the redesign shrinks what there is to inflate.

### 1.6 Full, heavy CI on every push — volume/cost (§1.6 of doc 1)

**Verdict: RESOLVED.**

This is the challenge the redesign is most directly built to solve, and the
stage-3 correction strengthens rather than weakens the fix. Tier 1 (every
push, PR or not) stays GitHub-hosted, toolchain-bootstrap-free, under ~5
minutes. Tier 2 (scoped build/validate) only runs the platform/build legs
the changed-file taxonomy says are relevant — a docs-only or `dev/design/`-
only push still runs Tier 1 alone. The full ~25-job, ~20-minute matrix moves
to running once per push to `main`, serialized via `concurrency: group:
main-full-matrix, cancel-in-progress: true` (doc 4 §1, replacing the
dropped merge-queue idea) — which directly kills the "back-to-back full runs
with no idle gap" pattern doc 1 §1.6 measured (five consecutive ~20-minute
runs firing with essentially zero gap). This is a strict improvement over
the merge-queue version of the same idea: same non-overlap property, zero
new trigger event, zero queue-admission UX, zero duplicated required-check
surface (doc 4 §1). The only residual cost is that Tier 2 itself, on a PR
that touches `src/rust/crates/**` broadly, can still be substantial — but
that is proportional to what the push could plausibly break, which is the
stated design goal, not an unresolved volume problem.

### 1.7 Branch-protection ruleset deletion (§2 of doc 1) — mechanism vs. trust

**Verdict: PARTIALLY RESOLVED.** This is the challenge most worth being
honest about, because it has two layers and the redesign only fully
addresses one.

*Mechanism layer — RESOLVED.* The redesign gives a required-check set that
is small (a handful of aggregator names, not 16 raw checks), stable against
matrix-name churn (doc 4 §6, confirmed as GitHub's own documented pattern),
and restores `non_fast_forward`/`deletion`/PR-review protection that were
removed as collateral damage despite never being named in the "ceremony, no
true value" complaint (doc 1 §2's timeline explicitly separates what was
complained about from what got deleted).

*Trust layer — NOT FULLY RESOLVED, and this is the more important half.*
The HITL's judgment was leveled at accumulated friction from checks that
were broken (§1.1, §1.3) or slow-by-volume (§1.6) staying red or expensive
for days without anyone fixing them — i.e., **required checks that had
stopped being trustworthy signals**. The redesign fixes the *specific*
instances that caused the 2026-08-20 frustration. It does not install any
mechanism that prevents a *new* required check from rotting the same way in
the future — no owner, no staleness alarm, no "a required check red for N
days without action gets auto-flagged for triage" process. The aggregator
pattern and taxonomy make the required-check *set* legible and small, which
lowers the odds of silent rot (fewer, more deliberately-added checks), but
"lower odds" is not "prevented." Doc 3 §2 already names this risk plainly:
"this proposal only works if it's legibly minimal; scope creep back toward
'everything required' would reproduce exactly the... dynamic." That's a
property the *initial* required-check set can have, but nothing in the
design enforces it stays that way over time. Recommendation §4 below
addresses this with an explicit norm, not a mechanism, because no CI
mechanism can substitute for someone actually triaging red checks promptly.

### Summary table

| # | Challenge | Verdict |
|---|---|---|
| 1.1 | Gitleaks full-history permanent blocker | PARTIALLY RESOLVED (mechanism sound; triage-to-zero is an unmet precondition) |
| 1.2 | CUDA/maturin toolchain churn | NOT RESOLVED (out of scope by design — doc 1 calls it real signal, not ceremony) |
| 1.3 | Windows diagnostic wired as blocking | PARTIALLY RESOLVED (this instance fixed; no guardrail against recurrence of the class) |
| 1.4 | Legitimate in-flight test failures | RESOLVED (unchanged, correctly still blocking) |
| 1.5 | Cascading cancellations | RESOLVED (own doc calls it a multiplier, not root cause; blast radius shrunk) |
| 1.6 | Full CI on every push — volume/cost | RESOLVED (tiering + taxonomy + `concurrency` group) |
| 1.7 | Branch-protection deletion / trust breakdown | PARTIALLY RESOLVED (mechanism restored + trustworthy; underlying trust practice not mechanically enforced) |

## 2. Recommendation — trigger/tier structure (final, post-correction)

Four trigger contexts, no merge queue:

1. **Every push to an open PR (draft or ready), and every direct push to any
   non-`main` branch.** Fires **Tier 1** only: `shell-lint`, `gitleaks`
   diff-scope, `verify-fast`, the `changes` taxonomy job, and the ~13 cheap
   always-on governance jobs. GitHub-hosted `ubuntu-latest` only. Target:
   under 5 minutes, every time, regardless of diff size.
2. **PR pushes plus direct pushes to `main`, scoped by the changed-file
   taxonomy.** Fires **Tier 1 + Tier 2**: `verify`, `wheel-size-gate`,
   `native-artifact-runtime-validation`, the `cargo hack check
   --each-feature` GPU-compile proxy, and the containerized ARM64 napi
   build — each gated to the taxonomy label(s) it responds to
   (`rust_core`, `python_binding`, `ts_binding`, `cuda_gpu`,
   `release_infra`; `docs_only` runs Tier 1 alone). `aarch64-release-
   preflight.yml`'s current "every push touching release-relevant paths"
   trigger folds into this as the `rust_core`/`ts_binding`/`release_infra`
   slice of Tier 2 — one taxonomy, not two independently-maintained
   path-filter lists.
3. **Push to `main` — the full unscoped matrix, once, serialized.** All 5
   published platform targets, CPU-only, wrapped in `concurrency: group:
   main-full-matrix, cancel-in-progress: true`. This is the corrected
   replacement for the dropped merge-queue trigger: same "no back-to-back
   overlapping full runs" property, zero new infra. If pushes land faster
   than the matrix completes, the newest supersedes the in-flight one rather
   than queuing — acceptable because `main` is written to sequentially by a
   single HITL/agent workflow, not contended by concurrent human PRs (doc 4
   §1's compatibility verdict).
4. **Tag push (release) and scheduled/dispatch rehearsal.** `release.yml`
   unchanged in kind (tag-triggered). The CUDA/Tegra self-hosted legs move
   off "every push touching release paths" onto a nightly `schedule`
   against `main` plus `workflow_dispatch`, gated by a per-runner-label
   `concurrency` group with `cancel-in-progress: false` (queue, don't
   cancel a GPU run mid-flight) — release-time GPU jobs keep first claim
   over a nightly rehearsal still running when a tag lands.

## 3. Recommendation — the required-check set to re-apply

Re-apply branch protection on `main` with exactly these named checks —
aggregators only, never a raw matrix-generated job name:

- **`gate-fast`** — `needs:` every Tier 1 job, `if: always()`, fails on any
  `failure`/`cancelled` result (not implicit success-propagation).
- **`gate-build`** — `needs:` every Tier 2 job the taxonomy activated for
  this push, same explicit-result-check pattern. A job the taxonomy
  correctly skipped (`skipped`, out of scope) must resolve differently from
  a job that failed upstream (`failure`/`cancelled`) — this distinction is
  the entire point of the aggregator and must have its own test fixture
  (in the style of the existing `test_shell_lint_ci_job.sh` tier-totality
  fixture), not just a code comment asserting correctness.
- **`CodeQL`** — kept as-is; already the repo's own prior working instance
  of this exact pattern (`dev/steward/branch-protection.md`'s
  `wheel-size-gate` matrix-churn workaround).
- **`non_fast_forward`** and **`deletion`** protection, and the **PR-review
  requirement** — restored exactly as they were before the 2026-08-20
  deletion; these were never part of the "ceremony, no true value"
  complaint (doc 1 §2) and should never have been removed as collateral.

Total required-check surface: **5 names** (`gate-fast`, `gate-build`,
`CodeQL`, plus the two protection rules), down from 16. `gate-merge-queue`
from the stage-2 hypothesis is dropped entirely, per doc 4's correction — no
merge-queue trigger exists in this design, so there is nothing for it to
gate.

Explicitly **not** required-check material: the full-matrix `main`-push run
(§2.3) is not a PR gate at all, so it has no required-check entry — it is a
post-merge signal, visible on `main`'s commit status, not a blocker. The
nightly GPU/gitleaks-full-history rehearsals are likewise never required
checks; their health is surfaced via a staleness/age status job (doc 3
§1.7), visible but non-blocking.

## 4. What moves to nightly/scheduled vs. stays PR-blocking, and why

| Stays PR-blocking (Tier 1 or Tier 2) | Moves to nightly/scheduled |
|---|---|
| gitleaks diff-scope ("current tracked tree") — fast, catches new secrets | gitleaks full-history scan — slow, doesn't correlate with the diff under test |
| `verify` / `verify-fast` — real correctness signal (§1.4) | — (never moves; this is the challenge catalogue's clearest "real signal" case) |
| `wheel-size-gate`, `native-artifact-runtime-validation` — scoped by taxonomy | full unscoped 5-platform matrix on every push (moves to once-per-`main`-push, not nightly, since it's cheap/GitHub-hosted, not fragile) |
| `cargo hack check --each-feature` (GPU-feature compile proxy, GitHub-hosted, no GPU needed) | actual CUDA build/run on `windchill3`; Tegra build/run on Jetson Orin — both self-hosted, single-box, real hardware |
| containerized ARM64 napi build (`manylinux_2_28`) — GitHub-hosted, no self-hosted contention | — (stays in Tier 2, scoped to relevant taxonomy labels) |
| — | `windows-wal-checkpoint-diagnosis` / `windows-wal-attribution` — leave `ci.yml`'s required path entirely; if the Slice-65 investigation is still active, it should run as a non-blocking diagnostic workflow (or not run in CI at all until its `os.uname()` bug is fixed), never as a required check |

The organizing principle, stated once rather than per-row: a check stays
PR-blocking only if it is (a) fast/cheap enough to pay on every push without
becoming the volume problem in §1.6, or (b) taxonomy-scoped so it only runs
when the push could plausibly affect it. Anything that is neither — because
it's inherently slow (full-history secret scan), inherently hardware-bound
(GPU builds), or was never actually a build/validate job in the first place
(the Windows diagnostic) — moves off the PR path but stays **visible**
(nightly run status, staleness age, or a non-blocking diagnostic workflow),
never silently absent.

## 5. Gitleaks remediation precondition — explicit prerequisite, not a CI mechanism

This must happen **before** the full-history scan is cut over to nightly-only,
not concurrently with it and not after:

1. Enumerate the current finding set in full (35→57 per the challenges
   review — get an exact, current count first; it may have grown further).
2. Classify every finding as one of exactly two categories, per doc 4 §5:
   - **Still-live credential** → rotate it first, then allowlist (baseline
     or `.gitleaksignore`) with a note recording that it was rotated and
     when. Never allowlist a live secret before rotating it.
   - **Confirmed dead/historical** (already rotated, a test fixture, a
     revoked key, etc.) → allowlist with a documented reason per entry.
3. Only after every finding is in one of those two states (zero
   unclassified findings) does the full-history scan move to the nightly
   `schedule` + `workflow_dispatch` trigger. Before that point, keep it
   running (even if red) somewhere visible rather than deleting it outright
   — silently removing a security check is a worse failure mode than a
   known-red one with a tracked remediation plan.
4. Add a periodic allowlist-audit cadence (quarterly is a reasonable
   default) so the allowlist itself doesn't become a second, quieter version
   of the same "green/suppressed forever, unverified" problem the
   `windows-wal-attribution` finding already illustrates.
5. Ownership: this needs an explicit named owner (HITL or a delegated
   agent-driven task), not "someone should do this eventually" — the
   redesign's own risk list (doc 3 §2) already flags that an indefinitely
   deferred triage silently turns "nightly + visible" into "nightly +
   ignored," recreating §1.1 in a new shape.

## 6. `docs/compatibility/index.md` and `dev/steward/branch-protection.md`

Both are flagged stale by the prior docs in this investigation and should be
corrected as part of this work, not left for a later pass:

- **`docs/compatibility/index.md`** currently claims macOS/Windows npm
  packages are unsupported, and states the published version as 0.8.21,
  while `release.yml` has published macOS x64/arm64 and Windows x64 npm
  packages and PyPI wheels on every real release since `v0.1.0`, and the
  workspace is now well past 0.8.23 (`delivery-requirements-map-20260821.md`
  §0). This is the single doc closest to a real user-facing support
  contract, so it is the right doc to extend into the platform/GPU/glibc
  matrix that currently only lives in `scripts/release/glibc-floor-
  contract.sh` and the workflow YAML — not to re-derive a new doc
  elsewhere. Recommend: correct the version number and platform-support
  claims as an immediate, standalone doc fix (independent of and not
  blocked by the CI redesign itself), then fold in the required-check set
  and tier structure from this doc once implemented, so it stops being able
  to silently drift from the shipped pipeline again.
- **`dev/steward/branch-protection.md`** still asserts (as of its
  2026-08-04 edit) that protection is "APPLIED AND VERIFIED," which has
  been false since the 2026-08-20 deletion, with no corresponding ledger
  entry (`ci-challenges-review-20260821.md` §2). Recommend: this doc gets
  fully rewritten, not patched, at the point branch protection is
  re-applied per §3 above — it should describe the *new* 5-check ruleset
  (not the old 16-check one), keep its useful mechanical content (the
  `PUT`-replaces-whole-ruleset warning, the restore-from-checked-in-JSON
  procedure), and add the aggregator-job rationale so a future reader
  understands *why* the required-check set is small, not just what it is.
  Until it's rewritten, its header should be corrected to say "STALE — see
  `ci-cd-final-recommendation-20260821.md`" so nobody trusts its current
  "APPLIED AND VERIFIED" claim in the interim.

## 7. Why this won't recreate the Aug 2026 failure mode

The 2026-08-20 deletion happened because required checks had become
untrustworthy signal mixed with ceremony: a permanently-red full-history
gitleaks scan unrelated to the diff under test, a permanently-red Windows
diagnostic with a trivial unfixed bug, and a full ~20-minute matrix on every
push regardless of size — three different things, all reporting as "CI is
red/slow," none of it actionable by looking at the check name alone. The
HITL's "ceremony, no true value" judgment was a reasonable response to that
experience, even though the deletion swept up two things
(`non_fast_forward`/deletion protection, PR-review gating) that were never
part of the complaint.

What makes the new required-check set different, not just smaller:

- **Every required check is either fast-always-on or taxonomy-scoped to
  what it can plausibly catch** — there is no required check left that is
  structurally guaranteed to be irrelevant to the diff under test (the
  gitleaks-full-history and Windows-diagnostic failure modes specifically).
- **The required-check set is 5 names, not 16, and each is an aggregator
  with a stable name that won't silently break on matrix-name churn** — the
  next time a platform row is added or renamed, branch protection doesn't
  need touching, and a required check doesn't go from "always shows up" to
  "silently absent" the way a raw matrix job name would.
- **Nothing fragile or slow is a required check anymore** — GPU builds and
  full-history secret scanning are real, valuable, and kept, but as visible
  nightly signal with a staleness alarm, not as a thing that can sit
  permanently red on the PR path for five days.
- **The volume problem is solved by tiering, not by removing checks** — a
  future full-matrix run happens once per `main` push via a `concurrency`
  group, not on every intermediate push, so re-applying protection doesn't
  reintroduce the "20-minute run stacking back-to-back with no idle gap"
  experience that was part of what prompted the deletion.

What this redesign does **not** provide, and what still has to be a human
practice rather than a mechanism (per the honest §1.7 verdict above): a
required check can still rot if a real regression starts failing it and
nobody fixes it promptly. The design lowers the odds (fewer, more
deliberately-chosen required checks; taxonomy-scoping so a check only fires
when relevant; visible-not-required treatment for anything fragile) but does
not eliminate the need for someone to treat "required check red for more
than a day or two" as an incident, not background noise. That norm is worth
stating explicitly to the HITL as the thing that actually prevents a repeat,
because no YAML change substitutes for it.

## 8. Sequencing / rollout suggestion

Kept proportionate — this is a recommendation, not a task breakdown:

1. **Gitleaks triage-to-zero first, standalone.** This has to happen before
   any full-history-scan retiering is meaningful (§5), and it's independent
   of everything else in this doc — it can start immediately.
2. **`docs/compatibility/index.md` version/platform correction, standalone.**
   Small, low-risk, already known-stale; no reason to wait for the rest.
3. **Build the taxonomy + aggregator jobs (`gate-fast`, `gate-build`)
   against the current job set, without touching branch protection yet.**
   Verify they compute correctly (fixture tests per §3) and that Tier
   1/Tier 2 timing targets hold, on real pushes, for at least a few days —
   observe before gating on it, the same evidence-first posture the four
   prior docs in this investigation used.
4. **Move the fragile jobs (full-history gitleaks post-triage, GPU builds,
   the Windows diagnostic) to their nightly/non-blocking homes.**
5. **Re-apply branch protection with the new 5-check set**, and rewrite
   `dev/steward/branch-protection.md` to match, once steps 1-4 have run
   clean for a few real days — not on day one of the redesign, so the first
   thing re-applied protection does isn't immediately go red on a
   still-settling taxonomy.
6. **Add the `main`-push full-matrix `concurrency` group last** — it's the
   lowest-risk, most mechanical piece (one YAML block), but sequencing it
   after the rest means it's validated against a CI setup that's already
   behaving, not compounding multiple simultaneous changes.

## 9. Open decisions for HITL

1. **Whether to pursue this now at all.** 0.8.23 was closed as of the most
   recent commit on `main` (`84a9ec59`, "docs(release): close 0.8.23
   publication"); `0.8.23-release-todo.md` in the repo root is itself stale
   against that (last touched 2026-08-19, predates the closure commit) and
   should not be read as describing current release state. With no release
   actively in flight, this is a comparatively low-risk window to invest in
   CI infra — but whether the next priority is CI/CD work versus the next
   release's feature scope is a call only the HITL can make; this doc takes
   no position on relative priority.
2. **Ownership and timeline for the gitleaks triage-to-zero step (§5).**
   This blocks the single highest-value item in the whole redesign (killing
   the most-cited persistent-blocker challenge) and is explicitly not a CI
   mechanism — it needs a named owner and a date, not just a plan.
3. **Whether `docs/compatibility/index.md` gets fixed as its own immediate
   PR or bundled into this redesign's rollout.** Recommendation above (§8.2)
   is standalone/immediate; HITL may prefer to bundle it.
4. **The nightly rehearsal cadence for CUDA/Tegra** — doc 3's open question
   3, unresolved by doc 4's research (no GitHub-native cadence primitive
   exists to lean on, per doc 4 §4). Nightly is a reasonable default but is
   a guess, not a measured answer; the HITL may want a different cadence
   given actual GPU-path change velocity.
5. **Whether the taxonomy's initial label boundaries need a dedicated
   pass** (doc 3's open question 4) before implementation — this doc treats
   that as an implementation-time detail, not a design gap, but it is real
   remaining work.
