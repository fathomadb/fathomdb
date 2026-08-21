---
title: CI/CD redesign hypothesis
date: 2026-08-21
status: PROPOSED
desc: >
  A specific, opinionated proposal for how FathomDB's CI/CD should be shaped,
  reasoned from `ci-challenges-review-20260821.md` (what actually went wrong)
  and `delivery-requirements-map-20260821.md` (what the surface genuinely
  requires). Stage 2 of a 4-stage investigation: stage 3 web-researches this
  hypothesis against external best practice, stage 4 checks the resulting
  design against the catalogued challenges. Analysis and proposal only; no CI
  config, script, or GitHub setting is changed by this doc.
blast_radius: >
  read-only: .github/workflows/{ci,release,aarch64-release-preflight,
  jetson-tegra-cuda-evidence}.yml; dev/design/{release,bindings}.md;
  dev/design/ci-challenges-review-20260821.md;
  dev/design/delivery-requirements-map-20260821.md
---

# CI/CD redesign hypothesis

**Status: PROPOSED.** Nothing here is implemented. No workflow, script, or
GitHub setting changes as a result of this document. This is a hypothesis for
stage 3 (external best-practice research) to pressure-test and stage 4 to
evaluate against the catalogued failure modes — it is expected to be wrong in
places, but it is written to be concrete enough to be *provably* wrong rather
than vague enough to survive any critique.

## 0. Premise

`ci-challenges-review-20260821.md` found that FathomDB's CI became slow and
untrustworthy for two distinct reasons that got tangled together: (a) a
handful of specific gates were broken or permanently red for structural
reasons unrelated to code correctness (the gitleaks full-history scan, the
`windows-wal-attribution` `os.uname()` bug) and never got triaged, and (b)
independent of any single gate being broken, the *entire* ~25-job, ~20-minute
matrix fired on every non-docs push at 75-103 runs/day, with runs stacking
back-to-back on `main`. Both problems were real, but the response conflated
them: the HITL's "ceremony, no true value" judgment was aimed at the
accumulated friction from (a) and (b), and the branch-protection ruleset that
got deleted on 2026-08-20 also removed things that were never part of the
complaint — `non_fast_forward`/deletion protection and the PR-review
requirement. `delivery-requirements-map-20260821.md` then established that a
large fraction of the *cost* (though not the specific failures) is inherent:
2 SDK bindings × 5 published platform/arch targets × GPU-vendor variance ×
3 divergent glibc floors is a genuine surface, not padding, and a
"hypothetically perfect" CI config would still fan out to roughly a dozen
platform-specific legs per push. The problem this hypothesis solves is not
"how do we make CI green" — it's how to design a CI/CD system where **what
runs on a given push is proportional to what that push could plausibly
break**, where **the required-check set stays small and trustworthy enough
that green actually means something and red is always actionable**, and
where **re-applying branch protection doesn't just reinstate the same
back-to-back 20-minute tax that made the HITL delete it**.

## 1. The hypothesis

### 1.1 Four triggers, not one

Today `ci.yml` has effectively one trigger shape (`push` to `main` +
`pull_request`) and one binary split (`changes.outputs.docs_only`, already a
real and well-reasoned mechanism — see its comment block at
`.github/workflows/ci.yml:113-134` — but only two-valued: docs vs.
everything). The hypothesis replaces the single push-triggered monolith with
four triggers, each answering a different question, at a different cost, at
a different cadence:

1. **Every push to an open PR** (draft or ready) — "did I just break
   something obviously, cheaply checkable." Fast tier only (§1.2). Runs in
   under ~5 minutes regardless of what changed. This is the tier that must
   never be allowed to regress back to a 20-minute full matrix, because it's
   the one paid dozens of times per PR during iteration.
2. **PR moved to "ready for review" (or a push after that point), and any
   push to `main` outside a merge queue** — adds the scoped build tier
   (§1.3): the subset of the platform/build matrix that the *changed-file
   taxonomy* says is actually relevant. A pure-docs or pure-`dev/design/`
   change still runs only Tier 1. A change under `src/rust/crates/**`,
   `Cargo.{toml,lock}`, `src/python/**`, `src/ts/**`, or
   `.github/workflows/**` runs Tier 1 + the scoped Tier 2 legs.
3. **Merge to `main` via a GitHub merge queue, not a raw push** — the ONE
   place the full cross-platform matrix (all 5 published targets, CPU-only —
   see §1.4 for why CUDA is excluded here) runs, once per merge attempt,
   serialized. This directly answers §1.6 of the challenges review: the
   full-matrix cost is paid once per thing that actually lands on `main`,
   not once per intermediate push plus again on `main` itself. It also kills
   the "back-to-back full runs with no idle gap" pattern, because the queue
   serializes merges instead of letting N pushes each independently fire a
   full run.
4. **Tag push (release) and scheduled/dispatch rehearsal** — unchanged in
   kind from today's `release.yml` (tag-triggered) but the CUDA/Tegra
   self-hosted legs move off "every push touching release-relevant paths"
   (as `aarch64-release-preflight.yml` currently does) and onto a nightly
   schedule against `main` plus on-demand `workflow_dispatch` — see §1.4.

### 1.2 Tier 1 — fast feedback (every push, PR or not, draft or not)

Composition, all required, all with a hard timeout and none silently
skippable via a `needs:` chain (the existing `shell-lint` job's comment at
`ci.yml:44-62` already states this reasoning correctly — a `needs: changes`
dependency makes a gate's *absence* look identical to success; that
reasoning generalizes to every job in this tier):

- `shell-lint` (as-is: no toolchain bootstrap, ~1 min).
- `gitleaks` **diff-scope only** (the existing "Scan current tracked tree"
  step) — genuinely fast, genuinely gates new secrets, stays a hard
  requirement. The full-history "Scan reachable Git history" step is
  **removed from this tier entirely** (see §1.4 — it moves to scheduled).
- `verify-fast` (lint/typecheck/cheap suites — as today).
- `changes` — expanded from one boolean (`docs_only`) to a small,
  mutually-non-exclusive taxonomy (§1.5), still biased the same safe
  direction the current comment specifies: a misclassification may only
  cause *more* to run, never less.
- The ~13 existing cheap always-on governance jobs (`board-currency`,
  `ledger-integrity`, `plan-anchors`, `governed-surface-pin`,
  `pinned-override-rot`, `c1-contract-conformance`, `transcript-hygiene`,
  `release-state-views`, `commission-manifest`, `design-status`,
  `steward-orient`, `markdownlint`, `docs`) — unchanged, because they're
  already cheap and already always-on for the right reason.

Nothing in Tier 1 touches a self-hosted runner or a platform-specific
toolchain bootstrap. It is deliberately GitHub-hosted-`ubuntu-latest`-only
where possible, so its cost floor is capacity-independent.

### 1.3 Tier 2 — scoped build/validate (ready-for-review PR pushes, main)

`verify` (heavy Rust/Python/TS suites), `wheel-size-gate`, and
`native-artifact-runtime-validation` stop being unconditionally
`docs_only != true` and instead each declare which taxonomy label(s) they
respond to (§1.5). Concretely, on a PR that only touches
`src/rust/crates/fathomdb-schema/**`, the macOS/Windows napi rows and the
Python-only wheel legs still need to run (a schema change can affect every
binding), but a PR that only touches `src/ts/**` binding glue does not need
to run the Jetson-adjacent Tegra evidence workflow or trigger a CUDA
rehearsal. `windows-wal-checkpoint-diagnosis` / `windows-wal-attribution`
leave `ci.yml` entirely — see §1.4, they are not build/validate jobs and
should never have been modeled as required CI in the first place.

`rust-workspace-race-report` stays `continue-on-error: true` and diagnostic
— it already correctly models "real signal, not a gate," per its own
comment; that pattern (report-only, visible, never blocking) is the template
for anything found to be inherently flaky rather than inherently ceremonial.

### 1.4 Handling inherently slow/fragile jobs — demote from "blocks every

push" to "scheduled + visible," never to "silently absent"

Three categories, three different treatments, chosen so none of them can
recreate §1.1/§1.3's failure mode (a check red for days, doing nothing,
because nobody owns triaging it back to informative):

- **Full-history secret scanning.** Move to a nightly scheduled workflow
  against `main` plus `workflow_dispatch`, not a per-push job. Before that
  cutover, the open finding set (35→57 leaks per the challenges review) gets
  triaged to zero once, either fixed or allowlisted with a documented reason
  per entry — the scheduled job is only trustworthy as "new secret
  introduced" signal once the historical noise floor is zero. Findings post
  to a visible surface (issue/board, not silently disappearing) so a new
  finding is never just an unactioned red job again.
- **GPU builds (CUDA on `windchill3`, Tegra on the Jetson Orin box).**
  Neither self-hosted GPU box is in the PR or merge-queue path at all. A
  cheap, GitHub-hosted, GPU-absent proxy — `cargo check --features
  embed-cuda,rerank-cuda` (compiles the CUDA-feature code paths without
  needing a GPU) — becomes a required Tier 2 check, so a PR that breaks CUDA
  compilation is still caught before merge. The actual GPU
  build/run/rehearsal (today's `cuda-contract-preflight` /
  `cuda-package-rehearsal` / `jetson-tegra-cuda-evidence.yml`) runs nightly
  against `main` and on tag push, using a single global `concurrency` group
  per self-hosted label so runs queue rather than pile up (the Jetson
  workflow already documents exactly this reasoning at
  `jetson-tegra-cuda-evidence.yml:17-21` — generalize it to `windchill3`
  too). A queue-depth/staleness check (last-successful-run age) feeds a
  visible status, not a blocking gate — a GPU rehearsal that hasn't run in
  N days should be loud, not invisible.
- **Cross-compiled/containerized wheels (`manylinux_2_28` napi ARM64
  build).** Stays in Tier 2 (GitHub-hosted, no self-hosted contention) but
  scoped to the taxonomy labels that can plausibly affect it
  (`rust_core`, `ts_binding`, `release_infra`) rather than every push.

`aarch64-release-preflight.yml`'s current trigger ("every push that touches
release-relevant paths") gets folded into this scheme rather than living as
a fourth parallel mechanism: it becomes exactly the `rust_core` /
`ts_binding` / `release_infra` slice of Tier 2, so there is one taxonomy
driving path-scoping, not two independently-maintained path-filter lists
that can drift apart.

### 1.5 A small changed-file taxonomy, not per-job path lists

Replace the single `docs_only` boolean with a handful of named booleans
computed once, in one place (`changes` job), consumed by every downstream
job's `if:`:

`rust_core`, `python_binding`, `ts_binding`, `cuda_gpu`
(`fathomdb-embedder`/Candle/`ort`-relevant paths), `release_infra`
(`.github/workflows/**`, `scripts/release/**`, `Cargo.toml` version fields),
`docs_only` (derived: true iff none of the above and no other code path
fired). Every job declares which label(s) gate it; a job with no label is
Tier 1 (always-on). This is intentionally a *short, hand-maintained* list —
not a fully general per-crate dependency graph — because the challenges
review's own evidence (`windows-wal-attribution`, gitleaks) shows that
clever-but-unverified CI logic is exactly what rots into silent ceremony.
Every label keeps the current safe-bias property (ambiguous or
uncomputable diff → treat as touching everything) and gets a fixture test in
the style of `scripts/tests/test_shell_lint_ci_job.sh` /  the existing
"tier-totality fixture" that already enforces `verify-fast`/`verify`
completeness, asserting the taxonomy is exhaustive and that no job is
reachable by zero labels unless it's deliberately Tier 1.

### 1.6 Required-check set robust to job/matrix name churn

GitHub's required-status-check matching is by literal job name string; a
matrix job's generated name (e.g. `wheel-size-gate (linux-arm64-gnu)`)
changes if a matrix row is added, renamed, or reordered, which silently
breaks the branch-protection match rather than failing loudly. The
hypothesis is to **never put a matrix-generated job name directly into
branch protection**. Instead, introduce a small, fixed number of aggregator
"gate" jobs with stable names, each `needs:` on the full real job set for
its tier and checking every dependency's `result` explicitly:

- `gate-fast` — needs everything in Tier 1.
- `gate-build` — needs everything in Tier 2 that Tier 2's taxonomy activated
  for this push (an aggregator can still `needs:` a job that was skipped by
  `if:`; GitHub reports `skipped` as non-blocking for a `needs:` check by
  design, but the aggregator step itself must treat "skipped because
  correctly out of scope" and "skipped because upstream failed" as
  different — `if: always()` + explicit `contains(needs.*.result, 'failure')`
  / `'cancelled'`, not the default implicit success-propagation).
- `gate-merge-queue` — needs the full unscoped matrix, exists only in the
  merge-queue trigger.

`main`'s required-check list (once branch protection is re-applied — a
separate HITL decision this doc doesn't presume) is then exactly
`{gate-fast, gate-build, gate-merge-queue, non_fast_forward, deletion
protection, PR-required}` — a handful of names that never change shape even
as the underlying matrices grow or shrink, plus the two pieces of protection
(`non_fast_forward`/`deletion`) that the 2026-08-20 deletion removed as
collateral damage despite never being named in the "ceremony" complaint.

### 1.7 Self-hosted GPU capacity scheduling

Two single-box runners (`windchill3` RTX 3090, Jetson Orin) cannot be
treated like elastic GitHub-hosted capacity. Concretely:

- Neither runner is reachable from any PR-triggered or merge-queue-triggered
  job — only from `release.yml` (tag push), the nightly rehearsal schedule,
  and explicit `workflow_dispatch`. This removes GPU capacity contention
  from the everyday PR loop entirely, which is the single biggest lever on
  "self-hosted box is a bottleneck for ordinary development."
- One `concurrency` group per runner label (already the pattern for Jetson;
  extend to `windchill3`), `cancel-in-progress: false`, so queued jobs wait
  rather than getting silently cancelled and rescheduled — a GPU rehearsal
  that got cancelled mid-run is worse than one that ran late.
- A max-age-since-last-green surfaced as its own cheap, GitHub-hosted status
  job (reads the most recent successful run of the nightly workflow via `gh
  run list` or an artifact/badge), so "the GPU rehearsal hasn't run in 4
  days because the box was busy with a release" is visible without being a
  blocking gate on unrelated PRs.
- Release-time GPU jobs (today's `build-cuda-linux-x64-gnu`, the rehearsal
  jobs) keep first claim on `windchill3` — a nightly rehearsal that's still
  running when a release tag lands should yield, not queue ahead of the
  real release.

## 2. Tradeoffs and risks in this proposal

- **Aggregator gates are new indirection with their own failure mode.** If
  `gate-fast`'s `needs:` list or its `always()` + result-check logic is
  wrong, it can pass while a real dependency failed — the exact "silently
  absent gate" problem this design exists to prevent, just moved one layer
  up. This needs the same kind of self-testing fixture the repo already
  uses elsewhere (`test_shell_lint_ci_job.sh`), not just a comment asserting
  correctness.
- **A multi-way taxonomy is more surface than one boolean.** `docs_only`
  is simple enough to reason about at a glance; six labels with
  cross-cutting `if:` conditions on a dozen jobs is a bigger maintenance
  object, and the taxonomy itself can drift from the real dependency graph
  (a `fathomdb-schema` change *does* affect every binding, as noted in
  §1.3 — the taxonomy has to be conservative, i.e. broad, on cross-cutting
  crates, or it under-scopes silently).
- **Merge queues are a real infra/workflow change, not just YAML.** They
  change how contributors merge (queue admission, serialized merges,
  possible re-run-on-conflict), add latency to landing a change, and need
  GitHub repo settings this doc doesn't presume — it's a bigger ask than the
  rest of the proposal and may be more machinery than a small/solo-HITL
  team wants. This is the piece most likely to get cut or replaced by
  something lighter in stage 3/4.
- **Deferring the full CUDA GPU build off the PR/merge path trades early
  detection for throughput.** A real CUDA regression can now land on `main`
  and only surface at the next nightly rehearsal (or worse, at release
  time) rather than blocking the PR that introduced it. The `cargo check
  --features embed-cuda` proxy catches compile-time breaks but not
  runtime/numerical ones (e.g. the ULP-level GEMM reduction-order class of
  issue noted in program memory) — this is a real coverage gap being traded
  for capacity, not a free lunch.
- **Full-history gitleaks moving off per-push blocking is a real reduction
  in security backstop strength**, contingent on the one-time triage-to-zero
  actually happening; if that triage is deferred indefinitely (as the
  original findings were), "nightly + visible" quietly becomes "nightly +
  ignored," recreating §1.1 in a new shape.
- **Re-applying branch protection at all risks a repeat of the 2026-08-20
  deletion** if the new required set isn't visibly smaller and more
  trustworthy than the old 16-check ruleset — this proposal only works if
  it's legibly minimal; scope creep back toward "everything required" would
  reproduce exactly the "ceremony, no true value" dynamic.

## 3. Open questions for stage 3 / stage 4

1. Is a GitHub merge queue actually warranted here, or does this repo's real
   push pattern (how much comes through PRs vs. direct/agent-driven pushes
   to `main` or release branches) make it the wrong mechanism entirely? This
   needs checking against how `main` is actually written to, not assumed.
2. What does current GitHub Actions best practice say about required-check
   robustness to matrix churn — is a hand-rolled aggregator job the
   right pattern, or do GitHub's newer ruleset/required-workflow features
   solve this natively? (Flagged explicitly for stage 3's web research.)
3. What's the right rehearsal cadence for the CUDA/Tegra self-hosted legs —
   nightly is a guess; it could be too sparse (regressions sit undetected
   for up to 24h) or too frequent relative to actual `main` change velocity
   on GPU-relevant paths.
4. Where exactly should the six-label taxonomy's boundaries sit so it earns
   its complexity rather than becoming a second, subtler version of the
   `windows-wal-attribution` problem (plausible-looking logic nobody
   re-verifies)? Needs a concrete boundary list, not just named categories.
5. Is draft-PR suppression (running only Tier 1 until "ready for review")
   worth its own complexity, or does taxonomy-based scoping alone already
   keep Tier 2 cheap enough on typical diffs that the draft/ready
   distinction isn't pulling its weight?
6. What is the actual mechanical plan to triage the existing gitleaks
   finding set to zero, and who owns it — this proposal's security-tier
   demotion is contingent on that happening, not a substitute for it.
7. Does `cargo check --features embed-cuda,rerank-cuda` on a GitHub-hosted
   (non-GPU) runner actually compile cleanly given the Candle fork's
   `cudarc` dynamic-loading path, or does the CUDA feature set require
   nvcc/toolkit presence even to type-check — this needs verifying against
   the real crate before being relied on as a PR-time proxy.
