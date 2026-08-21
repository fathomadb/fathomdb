---
title: CI/CD best-practices research vs. the redesign hypothesis
date: 2026-08-21
status: PROPOSED
desc: >
  Stage 3 of a 4-stage CI/CD redesign investigation: external, sourced
  research on 2025-2026 CI/CD best practice for GitHub merge queues,
  path-filtered CI, fragile/slow job handling, self-hosted GPU runner
  scheduling, secret-scanning strategy, and required-check robustness —
  evaluated against `ci-cd-design-hypothesis-20260821.md`'s specific
  proposals, not a generic survey. Stage 4 checks the resulting revised
  design against the catalogued failure modes in
  `ci-challenges-review-20260821.md`. Analysis only; no CI config, script, or
  GitHub setting is changed by this doc.
blast_radius: >
  read-only: dev/design/{ci-challenges-review,delivery-requirements-map,
  ci-cd-design-hypothesis}-20260821.md; dev/steward/branch-protection.md
---

# CI/CD best-practices research vs. the redesign hypothesis

**Status: PROPOSED.** Nothing here is implemented. No workflow, script, or
GitHub setting changes as a result of this document. This is stage 3 of the
4-stage investigation: it pressure-tests
`ci-cd-design-hypothesis-20260821.md` against external, sourced 2025-2026
practice, one hypothesis section at a time, and closes with a short list of
concrete deltas for stage 4 to carry forward.

## 0. Method

Six research areas were dispatched in parallel (one focused web-research pass
per area, each grounded in the specific FathomDB evidence from stages 1-2 —
not a generic "CI best practices" query): GitHub merge queues; path-filtered/
conditional CI; slow/fragile job handling (secret scanning, GPU/CUDA,
cross-compiled native wheels); self-hosted GPU runner scheduling; secret-
scanning strategy (gitleaks specifically); and required-status-check
robustness against matrix-name churn. Each pass prioritized primary sources
(GitHub's own docs, tool maintainers' own docs/READMEs, GitHub Community
discussions, GitHub's changelog/blog) over secondary blog commentary, and was
explicitly pointed at FathomDB's own prior findings — including one piece of
evidence not otherwise summarized in the stage-1/2 docs but load-bearing
here: `dev/steward/branch-protection.md` records that FathomDB has **already
hit the matrix-name-churn problem once, for real**, on `wheel-size-gate`
(bare name `wheel-size-gate` on a docs-only PR vs. the expanded
`wheel-size-gate (ubuntu-latest, x86_64-unknown-linux-gnu, linux-x64, 2_28,
7400000)` on a normal PR), and worked around it by requiring the stable
`CodeQL` aggregate instead of the raw per-row `Analyze (...)` matrix jobs it
summarizes. All URLs cited below are as returned by each research pass; none
were independently re-fetched by the synthesizing pass.

## 1. GitHub merge queues

**Best practice.** A merge queue admits a PR only after it already passes
required checks, then builds a temporary branch combining the target branch
plus every PR ahead of it in queue plus the new PR, runs CI against that
speculative merge, and fast-forwards on green; entries can be batched, and a
failed batch is bisected to find the culprit. Workflows must add `merge_group`
as an *additional* trigger alongside `pull_request`/`push`, or required checks
simply never fire on queue entries (GitHub Docs — [Managing a merge
queue](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/configuring-pull-request-merges/managing-a-merge-queue);
[Tenki Cloud — GitHub Merge Queue in
2026](https://tenki.cloud/blog/github-merge-queue-setup)).

**Compatibility verdict: poor fit, on two independent grounds.**

1. **It does not solve the matrix-name-churn problem FathomDB already hit —
   it doubles it.** Tenki Cloud states this explicitly: matrix-generated
   check names "need to appear as passing under both event triggers"
   (`pull_request` and `merge_group`). The exact `wheel-size-gate` failure
   mode from `dev/steward/branch-protection.md` would now need to hold
   identically across two trigger contexts instead of one. The aggregator-
   job fix (§6 below) is required *regardless* of whether a merge queue is
   adopted — a merge queue doesn't reduce that need, and mildly increases the
   surface it has to cover.
2. **It solves a concurrency problem FathomDB doesn't have.** Multiple
   sources converge that merge queues earn their complexity when *concurrent*
   human PRs compete for merge order against a moving base — [Graphite:
   "for small teams with up to 10 developers, you probably don't need a merge
   queue unless you're creating a high volume of
   PRs"](https://graphite.com/guides/merge-queue-vs-direct-merging-benefits);
   [Tenki Cloud: "probably isn't worth it when your team is small (under 5
   people) and rarely has concurrent
   PRs"](https://tenki.cloud/blog/github-merge-queue-setup), where the
   simpler "require branches up to date before merging" branch-protection
   option suffices. FathomDB is a single-HITL repo with agent-driven
   (Codex/Claude Code) sequential commits, not concurrent human PRs racing
   for merge order — the exact problem class a merge queue exists to solve.

**Verdict on the hypothesis: corrects it.** §1.1(3)/§1.6's `gate-merge-queue`
proposal should be **dropped**, not kept as "the one place the full matrix
runs." The hypothesis's own actual goal — "full matrix runs once per thing
that lands on `main`, not once per push, with no back-to-back overlapping
full runs" — is achievable without a merge queue: a `concurrency: group:
main-full-matrix, cancel-in-progress: true` on the full-matrix workflow
triggered on push to `main` gets the same non-overlap property with zero new
trigger events, zero queue-admission UX for the HITL to learn, and zero
second copy of every required-check name to keep in sync. This also avoids
introducing a second new serialization mechanism at the exact moment the
HITL deleted branch protection for feeling like ceremony with no true value
— stacking a merge queue on top risks reproducing that reaction. Open
question 1 from the hypothesis is answered: **not warranted given current
push patterns; revisit only if genuinely concurrent human-authored PRs
materialize.**

## 2. Path-filtered / conditional CI

**Best practice.** "Skipped required check counts as passed" is GitHub's
actual, intended, documented behavior, not a bug — a job skipped via `if:`/
`needs:` reports `success` for merge-gating purposes. This is a well-known,
named footgun ([Christian Emmer — "Skippable GitHub Status Checks Aren't
Really
Required"](https://emmer.dev/blog/skippable-github-status-checks-aren-t-really-required/);
GitHub's own [Troubleshooting required status
checks](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/troubleshooting-required-status-checks);
[community discussion on `paths-ignore` + branch
protection](https://github.com/orgs/community/discussions/54877), where
GitHub explicitly warns against making a `paths`/`paths-ignore`-triggered
workflow a required check for this exact reason). The standard mitigation,
independently converged on by multiple 2025 sources
([devopsdirective.com, Aug
2025](https://devopsdirective.com/posts/2025/08/github-actions-required-checks-for-conditional-jobs/);
the packaged [`alls-green`
action](https://github.com/marketplace/actions/alls-green)), is a downstream
aggregator job with `if: cancelled() || contains(needs.*.result,
'cancelled') || contains(needs.*.result, 'failure')` — stays green
(skipped-as-success) on the happy path, fires and fails only on real
failure/cancellation. For job/step-level (not workflow-level) path
filtering, [dorny/paths-filter](https://github.com/dorny/paths-filter) is
the standard tool — GitHub's native `paths:`/`paths-ignore:` only gates
whole-workflow triggering, not per-job `if:` conditions, so a `changes`
detection job feeding `needs.changes.outputs.X` into downstream jobs is the
documented two-job pattern, matching what FathomDB's `docs_only` mechanism
already does. On taxonomy shape: no source recommends full dependency-
graph-aware affected-detection (Nx/Turborepo-style) for a repo this size (10
crates); [Ned Batchelder's May 2025
writeup](https://nedbatchelder.com/blog/202505/filtering_github_actions_by_changed_files.html)
on doing exactly this at comparable scale confirms a small hand-maintained,
safe-biased taxonomy is the pragmatic norm, but flags a concrete gotcha: **a
multi-line/YAML-folded `if:` expression can silently fail to parse in GitHub
Actions** — keep every taxonomy condition single-line.

**Compatibility verdict: good fit, with one dependency to make explicit.**
FathomDB's `dev/steward/branch-protection.md` already deliberately relies on
skip-equals-pass ("GitHub treats a skipped required check as satisfied") for
docs-only PRs — sound only if the skip logic is airtight, safe-biased, and
tested, which is exactly the property the hypothesis's fixture-test
requirement (§1.5) targets.

**Verdict on the hypothesis: confirms, and reinforces a coupling the
hypothesis stated but under-emphasized.** §1.5 (taxonomy) and §1.6
(aggregator gates) are not independent proposals — the taxonomy is only
safe to use for required-check scoping *because* §1.6's aggregator pattern
exists to prevent an ordinary `if:`-skip from silently satisfying a required
check for the wrong reason. The doc should state that dependency directly
rather than presenting them as two separate line items. Add the single-line
`if:` expression constraint as a concrete implementation note under §1.5.

## 3. Fragile/slow jobs (secret scanning, GPU/CUDA, cross-compiled wheels)

**Best practice.** The dominant, cross-source pattern: PR path runs the
cheap/fast/GitHub-hosted subset; genuinely fragile/slow/hardware-dependent
legs move to `schedule` (cron) + `workflow_dispatch`, off the PR critical
path — general guidance
([trask/repository-practices](https://github.com/trask/repository-practices/blob/main/docs/general-github-actions-practices.md);
[github/awesome-copilot CI/CD
instructions](https://github.com/github/awesome-copilot/blob/main/instructions/github-actions-ci-cd-best-practices.instructions.md))
explicitly recommends trimming the matrix on PRs, keeping the full grid for
`main`/nightly, and moving exhaustive/slow suites off the PR-blocking path.
napi-rs's own CI model has no GPU/CUDA leg at all in its documented `[node
versions] x [OS]` PR matrix ([napi-rs
docs](https://napi.rs/docs/introduction/getting-started)), consistent with
routing GPU concerns to a separate mechanism entirely. GPU-specific tests in
comparable large projects are isolated to self-hosted/scheduled paths while
CPU-bound tests stay on GitHub-hosted ([PyTorch GPU CI
discussion](https://github.com/pytorch/pytorch/pull/150434); [GitHub
Community discussion on GPU
testing](https://github.com/orgs/community/discussions/187935)).

**A compile-only proxy for hardware-dependent code is a real, externally
validated pattern, not a hypothesis invention.** `cargo hack check
--each-feature` in the wgpu project ([wgpu pull request
7034](https://github.com/gfx-rs/wgpu/pull/7034)) validates every
feature-flag combination compiles without the actual GPU backend present;
the rust-gpu project states outright that GPU kernel code is standard Rust
so "no GPU hardware is needed in CI to test the logic" ([rust-gpu blog,
Jul 2025](https://rust-gpu.github.io/blog/2025/07/25/rust-on-every-gpu/)).
No source treats compile-only-proxy-plus-scheduled-real-run as insufficient;
every real GPU project still runs actual hardware tests, just off the
PR-blocking path. Cross-compiled CUDA-wheel matrices at FathomDB's shape
(multi-arch, multi-glibc-floor) are the documented norm for this problem
class, not unusual ([cibuildwheel
FAQ](https://cibuildwheel.pypa.io/en/stable/faq/); [RAPIDS GitHub Actions
docs](https://docs.rapids.ai/resources/github-actions/), which ship CUDA
wheels across manylinux + multiple CUDA versions using the same
per-arch-per-CUDA-version matrix shape).

**Compatibility verdict: fully compatible.** Forked-Candle per-GPU-vendor
Cargo features, CUDA/nvcc/gcc-13 exact pins, and two self-hosted GPU boxes
are exactly the shape of fragility every source treats as "keep off the PR
path, run scheduled + gated." Nothing in this research suggests FathomDB's
situation needs different treatment.

**Verdict on the hypothesis: confirms, and extends one detail.** The
compile-only-proxy + nightly-real-GPU-run split (§1.4) is externally
validated, not a novel invention — cite wgpu's `cargo hack check
--each-feature` and rust-gpu's stated rationale as direct precedent in the
doc. Extension: prefer `cargo hack check --each-feature` (or at minimum a
restricted `--feature-powerset` over just the GPU-relevant features) over a
single `cargo check --features embed-cuda,rerank-cuda` invocation — wgpu's
pattern exists specifically because checking only the "everything on"
combination misses breakage in other individually-valid combinations (e.g.
`embed-cuda` without `rerank-cuda`). This also directly answers open
question 7: the proxy's basic shape is a proven pattern elsewhere, but its
single-invocation form under-covers relative to what comparable projects do;
whether it compiles cleanly against the Candle fork's `cudarc` path still
needs verifying against the real crate, unchanged from the hypothesis's own
caveat.

## 4. Self-hosted GPU runner scheduling

**Best practice.** GitHub's only native mechanism here is `concurrency:`
groups plus runner labels — there is no native queue-depth/capacity-
awareness feature beyond that. A concurrency group serializes jobs sharing a
`group:` key; `cancel-in-progress: true` kills the older run when a new one
lands in the same group, `false` queues it to wait (as of 2026-05-07, a
single concurrency group can queue up to 100 waiting runs via `queue: max`,
and `queue: max` is mutually exclusive with `cancel-in-progress: true` by
design — [GitHub Docs: Control the concurrency of workflows and
jobs](https://docs.github.com/actions/writing-workflows/choosing-what-your-workflow-does/control-the-concurrency-of-workflows-and-jobs);
[GitHub Changelog,
2026-05-07](https://github.blog/changelog/2026-05-07-github-actions-concurrency-groups-now-allow-larger-queues/)).
Runner groups/labels are purely a routing/eligibility mechanism, not a
scheduler — there is no built-in priority system, so "release jobs get first
claim over a nightly rehearsal" has to be hand-built. GitHub-hosted GPU
runners (NVIDIA T4, Azure-backed) have been GA since mid-2024 ([GitHub
Changelog,
2024-07-08](https://github.blog/changelog/2024-07-08-github-actions-gpu-hosted-runners-are-now-generally-available/))
but require a paid plan and don't match FathomDB's actual need: exact
CUDA 12.6/nvcc/gcc-13 validation against a real consumer RTX 3090 driver
stack, and Jetson Orin is embedded ARM64 hardware with no cloud equivalent
at all — self-hosted is structurally required here, not a cost choice a
hosted GPU runner could undo. RAPIDS — a real GPU-CI-heavy comparable OSS
project — explicitly recommends *not* running GPU CI on every push/PR,
instead gating via an opt-in PR label maintainers apply deliberately, or
running nightly/scheduled regression ([RAPIDS GitHub Actions GPU
CI](https://docs.rapids.ai/deployment/stable/developer/ci/github-actions/)).

**Compatibility verdict: good fit, with one nuance.** RAPIDS's norm is
"PR-triggerable-but-gated" (opt-in label) rather than the hypothesis's
"PR-unreachable entirely." For FathomDB specifically, full exclusion is
more defensible than RAPIDS's label-gate: FathomDB's GPU legs are
release-blocking on a *solo* box (not a fleet), so a mistaken or
malicious label-add can't accidentally consume the only unit of capacity —
the stakes of an accidental trigger are higher here than in a
multi-runner fleet.

**Verdict on the hypothesis: confirms.** Concurrency-group-per-runner with
`cancel-in-progress: false` (§1.7) is exactly GitHub's intended pattern for a
single-box exclusive resource, and matches the reasoning the Jetson workflow
already documents for itself. Staleness-visibility and release-priority-
over-nightly are **not** covered by any native GitHub feature — both must be
hand-built exactly as the hypothesis assumes; nothing in research suggests
either is unnecessary or that GitHub has a built-in alternative. This
answers part of open question 3 negatively-but-usefully: there is no
GitHub-native cadence/priority primitive to lean on, so the nightly-cadence
guess and the release-priority rule both remain hand-rolled decisions for
the HITL, not something external tooling settles.

## 5. Secret-scanning strategy

**Best practice.** Gitleaks' own maintained GitHub Action
([gitleaks/gitleaks-action](https://github.com/gitleaks/gitleaks-action))
documents a layered trigger pattern nearly identical to the hypothesis:
`pull_request` + `push` + `workflow_dispatch` + a daily `schedule: cron` —
PR pushes get scanned, but the full-history completeness guarantee is
treated as a scheduled/dispatchable concern, not re-run in full on every
push (note: `fetch-depth: 0` is required for the action to see full history
at all — GitHub Actions' default shallow checkout otherwise silently blinds
the scanner past the latest commit). Community guidance converges on the
same split — local pre-commit hook + PR diff scan for fast new-secret
prevention, full-history scan as a recurring safety net, not a per-push
blocker ([turbogeek.co.uk](https://www.turbogeek.co.uk/gitleaks-secret-scanning-devsecops/);
[oneuptime.com](https://oneuptime.com/blog/post/2026-01-25-secret-scanning-gitleaks/view);
[laoutaris.org](https://laoutaris.org/blog/gitleaks/)). Gitleaks has
first-class, documented tooling for FathomDB's exact pre-existing-findings
situation: **baselines** (scan once, save a JSON baseline, pass
`--baseline-path` on subsequent scans to suppress only previously-known
findings while still catching new ones) and **`.gitleaksignore`**
(per-finding suppression by fingerprint, since v8.10.0) — documented as
mechanisms for *reducing noise*, not *accepting risk*: sources are explicit
that live-looking secrets in a baseline should be rotated immediately, and
each allowlist exception should carry a documented justification for audit
purposes ([gitleaks/gitleaks
README](https://github.com/gitleaks/gitleaks); [DeepWiki: gitleaks/gitleaks
§4.4 Allowlists &
Baselines](https://deepwiki.com/gitleaks/gitleaks/4.4-allowlists-and-baselines)).

**Compatibility verdict: strong fit — this is the tool's own idiomatic
mechanism, not a bespoke workaround.** FathomDB's 35→57 unresolved findings
map directly onto gitleaks' baseline/`.gitleaksignore` primitives.

**Verdict on the hypothesis: confirms, with one correction to make
explicit.** §1.4's "fix or allowlist with a documented reason per entry"
phrasing only *implies* the rotate-vs-allowlist distinction; sources treat it
as mandatory, not optional. Stage 4 should state the triage-to-zero step as
a two-way classification per finding, not a single fix-or-document choice:
(a) rotated/dead secret → safe to allowlist with a documented reason, or (b)
still-live credential → **must be rotated before allowlisting**, never
allowlisted first and rotated later. Also add a periodic (e.g. quarterly)
allowlist-audit step — an allowlist with no re-review recreates the exact
"green forever, unverified" shape the repo's own `windows-wal-attribution`
finding already warns against in the challenges review. This answers open
question 6's "who owns it" partially: gitleaks' own tooling defines *how*,
but ownership and the rotate/allowlist split still need an explicit HITL
decision, unchanged from the hypothesis's own framing that this is
contingent, not automatic.

## 6. Required-check robustness / aggregator pattern

**Best practice.** GitHub's own official
["Troubleshooting required status
checks"](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/collaborating-on-repositories-with-code-quality-features/troubleshooting-required-status-checks)
doc explicitly documents both failure modes FathomDB independently hit —
matrix jobs producing per-row check names, and path-filtered/skipped
workflows leaving required checks stuck pending — and its own recommended
fix is stated plainly: **"Use `always()` with `needs` for required checks
that depend on other jobs."** Multiple GitHub Community discussions converge
on the identical shape: a stable-named job with `if: always()`, `needs:
[matrix-job]`, checking `needs.<job>.result` explicitly to distinguish
success/skip from failure/cancel, used as the one name entered into branch
protection ([#26822](https://github.com/orgs/community/discussions/26822),
[#9141](https://github.com/orgs/community/discussions/9141),
[#60792](https://github.com/orgs/community/discussions/60792),
[#12395](https://github.com/orgs/community/discussions/12395)). GitHub's
newer "required workflows via repository rulesets"
([blog](https://github.blog/enterprise-software/ci-cd/enforcing-code-reliability-by-requiring-workflows-with-github-repository-rules/);
GA per
[#69595](https://github.com/orgs/community/discussions/69595)) operates at
the *whole-workflow* level, not the intra-workflow job/matrix level, and
does not documentedly resolve matrix-name churn within a single workflow —
it is orthogonal (useful for e.g. requiring that a separate nightly
rehearsal workflow exists/ran), not a replacement for the aggregator
pattern.

**Compatibility verdict: exact match to what FathomDB already independently
discovered.** `dev/steward/branch-protection.md`'s `CodeQL`-as-stable-
aggregate workaround for the `Analyze (...)`/`wheel-size-gate` matrix-name
problem is a hand-found instance of exactly the pattern GitHub's own docs
recommend by name.

**Verdict on the hypothesis: confirms, fully, including the specific
mechanism.** §1.6's `gate-fast`/`gate-build`/`gate-merge-queue` design —
`if: always()` plus explicit `contains(needs.*.result, 'failure')` /
`'cancelled'` rather than relying on implicit `needs:` success propagation —
is not merely reasonable, it is GitHub's own documented recommended
approach, stated close to verbatim. No gap was found in the design itself:
a skipped job resolves to `needs.<job>.result == "skipped"`, distinct from
`"failure"`/`"cancelled"`, which is exactly the distinction the explicit
check relies on to avoid `always()` becoming a rubber stamp. This directly
answers open question 2: **the hand-rolled aggregator is the right pattern,
not a workaround GitHub has since obsoleted** — keep `gate-fast` and
`gate-build` as specified. (`gate-merge-queue` itself is dropped per §1
above, independent of this verdict on the aggregator mechanism.)

## 7. Revised hypothesis deltas

Concrete, actionable changes to carry into stage 4:

- **Drop `gate-merge-queue` and the merge-queue trigger from §1.1(3)/§1.6.**
  Replace with `concurrency: group: main-full-matrix, cancel-in-progress:
  true` on the full-matrix workflow, triggered on push to `main` (not via
  PR/merge-queue admission). Achieves the same "no back-to-back overlapping
  full runs" goal without a new trigger event, queue-admission UX, or
  duplicated required-check surface. Answers open question 1: not warranted
  at current push volume/team shape.
- **Keep `gate-fast` and `gate-build` exactly as specified in §1.6** —
  confirmed as GitHub's own documented recommended pattern
  (`if: always()` + explicit `needs.*.result` check), not a risky
  workaround. Answers open question 2 affirmatively.
- **State explicitly that §1.5 (taxonomy) and §1.6 (aggregator gates) are
  coupled, not independent proposals** — the taxonomy is only safe for
  required-check scoping because the aggregator pattern exists to prevent an
  `if:`-skip from silently satisfying the wrong check.
- **Add a concrete implementation constraint to §1.5**: every taxonomy `if:`
  condition must stay single-line — multi-line/YAML-folded `if:` expressions
  are known to silently fail to parse in GitHub Actions.
- **Strengthen the CUDA compile-proxy in §1.4** from a single `cargo check
  --features embed-cuda,rerank-cuda` invocation to `cargo hack check
  --each-feature` (or a GPU-scoped `--feature-powerset`), matching the wgpu
  precedent — a single "everything on" invocation misses breakage in other
  individually-valid feature combinations.
- **Split the gitleaks triage-to-zero step in §1.4 into an explicit two-way
  classification per finding** (rotate-then-allowlist for live-looking
  secrets vs. allowlist-with-reason for confirmed-dead ones), plus add a
  periodic allowlist-audit cadence (e.g. quarterly) — not just "fix or
  document."
- **No change needed to §1.7** (self-hosted GPU concurrency-group design) —
  confirmed as GitHub's intended pattern for a single-box exclusive
  resource; staleness-visibility and release-priority remain correctly
  identified as hand-built, since GitHub has no native equivalent.
- **No change needed to §1.2/§1.3's Tier-1/Tier-2 split or the diff-scope-
  gitleaks-stays-blocking decision** — both confirmed as matching documented
  practice (napi-rs/RAPIDS/general GH Actions guidance; gitleaks' own action
  trigger pattern) without correction.
