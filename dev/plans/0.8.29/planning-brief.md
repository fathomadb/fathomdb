---
title: 0.8.29 slice drafting brief
status: PROPOSED
date: 2026-11-09
target_release: 0.8.29
---

# 0.8.29 slice drafting brief

## Assignment boundary

The owner confirmed **0.8.29**. This is release/slice planning, not execution.
Write draft local needs, requirements, acceptance, architecture/design changes,
work breakdown and prerequisites. Prework 0–8 produces proposals only; 8 is a
placeholder. Slice 9 scores findings and holds an interactive HITL review, then
reviews a decision-bound Slice 10 plan with up to four FIX-n cycles and a final
HITL decision. Slice 10 implements only recorded preparation decisions.
Feature slices start at 20 and use multiples of ten.

Use [reconciliation-plan.md](reconciliation-plan.md), the existing release plan,
accepted ADRs and actual source. Preserve already implemented MCP/queue/Block 1
and B2-2/3 plus partial B2-6 signals. Do not infer missing behavior from missing
branch ancestry. No publication, dependency installation/upgrade, environment
change, canonical contract change, cleanup, code fix, feature removal or release
state activation is authorized by drafting. Existing branch/worktree are facts;
their desired creation/switch procedure belongs in Slice 0's proposal and only
approved execution belongs in 10.

Only the parent writes this checkout. Drafting subagents may read the checkout
and write their own output file under /tmp/fathomdb-0829-slice-drafts/. They must
not mutate repository files, Git state, ledgers or environments. Distinct
scratch output files isolate drafts without concurrent checkout writers.

## Common source baseline

- Worktree: /home/coreyt/.codex/worktrees/fathomdb-0-8-29/dev-0.8.29.
- Branch llm/0.8.29-chat tracks origin/dev/0.8.29.
- .29 tip 6edc1155f; completed .28 f43435241 is included. Format 37.
- .28 published; scripts/release-current.py currently has empty success output.
- No .29 release state/board yet. Plan their creation in 10 after Slice 9
  decisions, respecting the owner's new proposal-only prework instruction.
- Cargo axis W 0.8.28; independent embedder-api axis E 0.8.0. Plan coordinated
  0.8.29 bump; never bump independent E just for lockstep.
- Rust minimum 1.95; Node >=25 <26 and npm 11.12.1 in source manifests;
  Python package >=3.10, developer scripts require >=3.11 or guarded tomli.
  Ruff 0.15.17 and Pyright 1.1.410 are reviewed pins.
- Checkout-owned venv/native artifacts required for product verification.
  No borrowed venv or editable binding repointing across worktrees.
- windchill3 R580 and primary CUDA12.6 fixed; never run K620.
  Existing RTX3090s, Windows VM and Jetson runner venues are documented in
  .28 Slice95 status. Locate exact contracts, do not promise host access.
- CUDA13 GCP library qualification exists, second product artifact does not.
  Q5 Tegra CUDA13 unmeasured. Fresh CUDA usage review remains incomplete.
- Full gates for final code phases; scoped validators for planning/metadata.
  No original failed/assembled campaign is a fresh uninterrupted green pass.
- Physical erasure effort complete except concrete defects. D28-09 is bounded
  interruption qualification, not a reopened availability/SLA program.
- Overflow chunking remains parked; monitor standing tripwires. No new ANN,
  caller-provider API, approximate ranking or weak-result threshold by inference.

## Proposed ladder

| Slice | Assignment |
| ---: | --- |
| 0 | Environment and project infrastructure proposal: branch/worktree/switch, venv, npm, Rust/Python/TS versions, platform matrix and compute/runner inventory |
| 1 | Library/Dependabot sweep plan and observed candidate/pin register; no upgrades |
| 2 | Repo-wide cruft inventory/proposals: product/program/dev/developer/public docs plus source/tests; keep/deprecate-in-place/archive/delete |
| 3 | Draft user-needs/requirements/acceptance CRUD and architecture drafts, allocated to slices; do not edit canonical product contracts |
| 4 | Architecture review and high-level code alignment; propose code/architecture changes |
| 5 | Verification adequacy: need->requirement->AC->test traceability, critical paths, meaningful gaps |
| 6 | Stale-doc evidence review across user/developer/contributor audiences, including available transcripts |
| 7 | CI/CD delay review: builds, preflight, correct agent-verify, packaging, gitleaks, CI/runners/registries; pragmatic bounded fixes |
| 8 | Reserved placeholder, no invented work |
| 9 | Collect/score 0–8 proposals, interactive HITL, decision log, derive 10, subagent review and up to four FIX-n cycles, final HITL and plan update |
| 10 | Implement only approved preparation: isolated release workspace, owned tools/envs, state/views, selected dependency/CI/docs preparation |
| 20 | Residual test hygiene, queue race coverage, parallel failures, warnings and chunking constants |
| 30 | Current 100k scale baseline/AC083, write-interleaved/cache suitability, MCP/SDK/open/embedding and 300k/drain measurements |
| 40 | Eligibility cache plus selected batch-invariant metadata work, on format37 visibility; no automatic counters/schema fallback |
| 50 | Fusion identity ADR and duplicate census, exact deferred-content text arm, Unicode-safe record-start snippets |
| 60 | MCP compact text default, structured mode, view flags and filtered direct text search; budgets, paging, all tools/errors |
| 70 | Remaining degradation reasons, facade/SDK/native parity and UTF8 contracts |
| 80 | Unified residual B2-1–8 requirements/design, classify already built signals; truthful coverage/scored spans |
| 90 | Implement only approved B2 residuals after 80 approval AND Block1 follow-up exit |
| 100 | Interrupted erasure transaction/checkpoint/reopen qualification on current accepted semantics |
| 110 | CUDA context-reset safe lifetime/teardown, based on .28 instrumentation, permitted isolated GPU faults |
| 120 | CUDA usage/policy review and second CUDA13 artifact design/delivery; explicit architecture/library/PTX/distribution contracts |
| 130 | CUDA platform/SDK pool-release and soak qualification; non-Tegra init hypothesis, Q5 venue evidence; no host reflash assumption |
| 140 | Owner seq-367: required reset/artifact fork repairs only and necessary repair-release versioning; Candle rebase, marker cleanup and multiarchitecture kernels move to .30 |
| 150 | Conditional candidate-selection/coalescing/SDK-hints-default experiments, checkpoint authorization and separate product promotion gates |
| 160 | Combined qualification/docs/changelog/version/publication readiness; actual publication separately authorized |

Slices 120–150 are explicit PROPOSED placement of carryovers, not old approvals.
Old 120 close becomes proposed 160 close. Preserve original allocation history
with a mapping rather than silently changing accepted decisions.

## Dependency discipline

- 0–8 are proposal-producing. They can research in parallel, with 3/4/5 iterating
  on drafts; 9 requires their outputs. Initial plans do not claim these reviews
  were already completed.
- 10 requires recorded 9 decisions and reviewed/accepted preparation scope.
- 20 follows needed preparation; stable harness -> 30's trusted measurements.
- 40 requires 30 evidence and reviewed invalidation design.
- 50 requires 30 baseline and accepted fusion identity before optimized route.
  40->50 is preferred landing order, not a semantic hard dependency.
- 60 depends on current already-integrated engine flags/filter support and
  accepted/reconciled surface design, not on 40/50 implementation. Final parity
  reruns against integrated fusion.
- 70 design can start early; engine landings serialize 40,50,70,100,110.
- Block1 follow-up exit after integrated 20–70 includes required full/feature/
  performance checks, both-format conformance and approved smoke/remeasure.
  This is an unnumbered milestone, not Slice160; avoid a 90<->160 cycle.
- 80 design can start early; approval requires settled field/identity/
  degradation contracts. 90 requires 80 approval AND Block1 follow-up exit.
- 110 safe-reset design may need qualified fork changes. 120 artifact and 130
  final qualification use final chosen pins from any included 140 work. This
  creates ordering across numerical slice labels, not a circular dependency:
  140's fork unit qualification precedes pin adoption, then 110/120 integration,
  then 130 affected final qualification. State precise conditional edges.
- 150 experiments need explicit checkpoint rulings, current native eligibility
  baseline, resource/cost limits and preregistration. They do not automatically
  gate unrelated product work. No unapproved default or ranking changes.
- 160 needs every selected product slice qualified; each proposal is either
  completed or explicitly deferred, not silently omitted. Publication separate.

## Required contents of each draft

Use frontmatter title, status: PROPOSED, target_release: 0.8.29, slice.
Produce a useful concise plan (roughly 80–130 lines; 8 can be much shorter).
Include:

1. Current maturity/evidence, goal, boundaries, sources actually inspected.
2. Slice prep: draft needs/requirements/AC (release-local IDs), architecture
   reading, design draft, challenging aspects, planned primary-source research,
   fit review and proposed revision. No new canonical AC IDs.
3. Explicit hard/conditional dependencies, preferred sequencing and resource/
   shared-file conflicts; list newly discovered prerequisites for next reviewer.
4. Concrete tasks and deliverables local to the slice (draft requirements/design/
   test mapping alongside plan), with functional falsifiable completion signals.
5. Validation and TDD appropriate to later implementation, independent review,
   stop/decision points, safe integration into current .29. For prework assess
   proposal completeness, not product test execution.
6. Open decisions/risk/maturity; no fabricated review results, host availability,
   latest versions, adoption figures, test passes or HITL decisions.
Link existing source documents using paths relative to the final destination
dev/plans/0.8.29/prework/slice-N/plan.md (0–9) or
dev/plans/0.8.29/features/slice-N/plan.md (10 and >=20).
Do not link files you have not created or verified; name future drafts as
deliverables in code spans.
