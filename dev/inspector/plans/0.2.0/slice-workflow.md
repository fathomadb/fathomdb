---
title: Inspector 0.2.0 slice workflow
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: PROPOSED
desc: Direct slice agents, draft-only prework, explicit human gates and focused feature execution
---

# Inspector 0.2.0 slice workflow

This is the shared method for the [draft ladder](plan.md), subordinate to the
owner's adopted [scope](scope.md) and per-slice boundaries. No slice is executed
by writing its draft. Each future slice has its own directly commissioned agent;
no Steward/Orchestrator or MCP dispatch. Model-specific review/verification calls
are direct. The user supplied the execution prompt included in each slice plan;
its generic implementation sentence never overrides a specific no-action bound.

## Work types and human authority

- 0–7: inspect/review, write reports and local draft proposals only. Apply no
  environment/library/cruft/requirements/architecture/source/test changes.
  The [owner ruling](slice-0/checkout-decision.md) requires future Slice 0 first
  to create and enter the release checkout; all other environment/infra changes
  remain proposals. No release branch/worktree is created by this planning turn.
- 8: placeholder. State the reserved role; do not invent work or runtime passes.
- 9: consolidate 0–8 items, score understanding/risk/effort and recommend include
  or postpone; present interactively and collect actual item-level decisions.
  Build/update 10 from the selected set, have appropriate direct independent
  review, allow up to four FIX-n cycles, present changed plan/residuals and
  capture final human ratification. Never treat silence, time or a selected UI
  default as consent. Stop after four unresolved cycles; surface the issues.
- 10: implement only the ratified approved prework set, with per-item acceptance,
  source boundary, evidence and failure handling. Missing prerequisites remain
  blockers. An approved empty set can close honestly as no change.
- 20–70: execute complete vertical feature/qualification scope, with requirements,
  design, implementation, tests and docs. 70 integrates proof, not unfinished
  basics. No candidate outside scope is promoted by implementation convenience.

## 1. Establish checkout and reconcile the draft

The [owner decision](slice-0/checkout-decision.md) requires future Slice 0 first
to create and enter `inspector/dev/0.2.0` at
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0`, from the approved final
planning HEAD preserving the Inspector port/scope/plans and 0.8.29 ancestry.
All 0–9 reports then use that release checkout sequentially; 10 reuses it for
only its approved items. Other environment/infra changes remain proposals.
No checkout creation is performed by drafting, and primary/main edits are excluded.

Confirm actual branch/HEAD, checkout root, cleanliness, baseline and predecessor
state from Git and durable records. Work in the release checkout created and
entered by Slice 0, with an explicit working directory on every command.
Reuse that checkout sequentially
by default; no per-slice branch/worktree ceremony. Preserve primary/unrelated
checkouts. Concurrent repo writers require isolated worktrees or serialization;
read-only review agents can share the checkout.

Enumerate changes since the slice draft in its plan/design, related source/tests/
interfaces and dependencies, assigned feature/functions and every allocated draft
item. Record evidence and keep/adjust/reject/already-done disposition with a short
reason. Evaluate completeness and overbuilding, then revise before proceeding.
Do not reread and merely assert current. Missing evidence is unknown, not no work.

## 2. Local needs, requirements, acceptance and design

Read canonical [needs](../../needs.md), [requirements](../../requirements.md),
[acceptance](../../acceptance.md), [architecture](../../design/high-level-design.md),
[the single matrix](../0.1/verification-plan.md), prior slice results and applicable
ADRs/interfaces. Keep stable Inspector-local IDs; do not edit the global acceptance
register. Draft CRUD items state reason/source, affected IDs, observable criterion,
implementation owner and prerequisite. For prework these remain local drafts and
proposal allocations until the human disposition, not active contract edits.

Each feature completes its own error, identity, policy, accessibility, resource and
UI behavior; later final qualification is additional. Update current canonical
contracts/matrix when an approved implemented feature changes them, never maintain
competing per-slice copies. Separate callable public reuse from private/test-only
helpers and unsupported or proposed capabilities. Required positive gaps cannot
pass as unavailable-mode handling or a fabricated fixture.

Read architecture and draft design, identify genuine challenges and primary-source
research approaches, test fit, revise, and hand prerequisites explicitly to the
next reviewer. Do not run network research merely because it is mentioned as a
future plan step. Keep Inspector modules small; no copied ranker, synthetic origin
or speculative plugin framework. Scope/dependency changes require the appropriate
human ruling, not a silent plan adjustment.

Before implementing actual design additions/changes, directly commission a fresh
`gpt-6.1-sol`, reasoning `high`, design-review agent. Resolve material findings and
recheck changed areas. Prework design proposals are reviewed as draft artifacts;
review does not authorize applying them. Record concise dispositions, not transcripts.

## 3. Implementation or owned prework deliverable

For feature/code behavior: write an independent-intent failing test, run and verify
the intended RED, commit its failure evidence, then implement the smallest GREEN
in a separate commit. Refactor while preserving the oracle. Do not weaken tests,
regenerate golden ranks/snapshots, mock a database or lump unrelated findings into
one GREEN. Mechanical version changes need no fake failing test. Prework prose
has no artificial RED/GREEN and claims no runtime pass.

Integration tests use real databases and independent intended identities, adjacency,
lexical facts and numeric distances. Exercise the actual DOM/tree/lexical/vector/
graph interactions with keyboard and pointer; payload/label assertions alone are
insufficient. Actual supported head-window vectors and exact revisions are qualified on the
selected source; the S9-010 within-source multiple-window positive is deferred,
with no fabricated rows or support claim. D-01 applies to every detail/label/snippet/link/copy/export and
cached/late observation. Physical deletion is neither delayed nor reconstructed.

Implementation in prework means completing the assigned report, local drafts and
handoff, not applying suggested changes. Slice 10 derives its implementation list
from ratified 9; do not pre-fill it with inferred approvals. Any source/script
changes there retain genuine TDD and independent review obligations.

## 4. Independent code review and Terra verification

For implemented code, directly commission a fresh `gpt-6.1-sol` high reviewer of
accepted contracts, actual diff, RED/GREEN tests and dependency/resource boundary.
Resolve material findings; use focused recheck where adequate. Then directly
commission Terra (`gpt-6.1-terra`) to independently verify the assigned result.
Reviewers/verifiers are read-only source actors and avoid concurrent mutable build
state. They report pass/fail/skips, actual runtime checks and unverified claims.

For prework documents, use appropriate source/traceability/link/scope checks and
independent draft review/verification; do not turn document approval into runtime
certification. Required design Sol-high and Terra verification are planned directly;
no worker framework, Steward/Orchestrator or MCP model wrapper is introduced.

## 5. Pragmatic checks and final gates

Use relevant fast lint/type checks and focused tests in latency order; fix a specific
finding with its reproducer and demonstrated blast-radius checks. Broaden when
changes cross contracts or leave uncertainty. Do not repeat unchanged full suites
for every helper or receipt. Complete applicable source/release gates on the final
phase; scoped or skipped checks are never full-workspace green.

Source/test/executable changes require the repository source gate under its rules.
Use a checkout-owned tool environment; do not borrow another checkout's .venv or
fall through to system Python after preflight failure. The approved setup must
respect worktree native-install constraints and packaging contracts. Missing
models, participants, tools or required tests are unverified, not success.

Markdown uses `./scripts/agent-lint-md.sh`; public docs add docs lint and strict
MkDocs. Only guarded `npm run format:md -- explicit-paths` may fix Markdown.
No prettier or unguarded markdownlint fix. Run normal hooks; no bypass/force-push.
A failed gate caused by unrelated bookkeeping is reported and scoped pragmatically,
not silently waived or expanded into an unrelated cleanup campaign.

## 6. Status, integration and cleanup

Create the slice status record only when execution starts. Record actual baseline,
current state/blockers, changed scope/design, review dispositions, relevant TDD or
report evidence, executed checks/skips and remaining work. COMPLETE requires
assigned outcomes and checks, with no unresolved mandatory work. A draft is not
an execution record. Reassess after two failures of the same mode rather than loop.

If an extra slice-owned branch/worktree was needed, integrate the verified commits
back to `inspector/dev/0.2.0`, confirm Git ancestry and integrated behavior, then
clean only the fully integrated owned temporary checkout/branch and preserve
needed untracked work. Use normal safe cleanup authorization; no forced deletion.
Preserve the release worktree and other users' branches. No main merge, release
tag, registry publication or automatic remote action is implied by slice completion.
