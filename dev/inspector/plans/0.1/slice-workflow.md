---
title: Inspector v0.1 slice execution workflow
date: 2026-10-28
product: FathomDB Inspector
target_release: 0.1
status: PROPOSED
desc: Lean workflow for preparation Slices 15/16 and feature Slices 20 through 60
---

# Slice execution workflow

Legacy v0.1 workflow/source context, retained for reuse when work is
commissioned. [Adopted 0.2.0 scope](../0.2.0/scope.md) carries unfinished
obligations but is not an execution commission or a new slice plan. New release
planning must reconcile this historical workflow; its source-era references
and actors do not by themselves establish current execution approvals.

This workflow binds all seven slice plans when commissioned. Slice 15 execution
is recorded in its status; other slice drafts do not establish execution.

Slices 15 and 16 are preparation slices: execute their assigned document,
traceability, design and handoff work, with independent Sol-high review and
Terra verification of the planning deliverable. Application TDD, fresh code
review and runtime checks below apply when feature code is implemented; do not
invent RED/GREEN or claim application passes for prose. Required source/test
changes are not silently commissioned by a preparation plan.

Slices 20–60 are feature slices. When commissioned, perform the complete
implementation and verification steps; a plan/design alone is not completion.
Every feature consumes the completed Slice 15 requirements/verification matrix
and Slice 16 architecture/capability baseline before refining its local work.

## Boundary and working checkout

- Work only on Inspector: `src/inspector/`, `dev/inspector/`, `docs/inspector/`,
  plus explicitly enumerated minimal build/test/docs wiring for this product.
  Engine, SDK, schema, MCP and agent-access implementations, contracts and
  dependency pins are read-only. Existing APIs/libraries may be consumed
  unchanged; inspecting their source does not authorize changing them.
- Establish Inspector-local adapters only where their semantics can be
  demonstrated using existing contracts and real-data conformance. Do not
  copy private engine modules or invent unobserved origin/rank/readiness facts.
  Engine startup is allowed under the current requirements; qualify its
  lifecycle effects, cache constraints and observation/resource ownership.
- If a required feature needs an unavailable Engine/MCP capability, record
  the concrete gap and seek a scope decision. Do not implement that dependency,
  silently defer a mandatory item, or declare the slice complete.
- Reuse the current task checkout/branch and execute sequentially by default.
  No new worktree or branch is required. One writer per checkout remains
  binding; design/code reviewers and verifiers are read-only.

## 1. Reconcile changes since the draft

Verify Git branch, worktree, cleanliness and predecessor status. Read the
slice plan/design, master plan, completed predecessor outputs, shared
verification matrix where created, relevant needs/requirements/AC and reviews.
Use `git diff`/`git log` from the draft's recorded baseline to enumerate:

1. Changes to the drafted plan, design, requirements or acceptance.
2. Related in-repo implementations, tests, interfaces and dependencies completed
   since drafting; identify work that now exists versus work still missing.
3. Changes to assigned features/functions and their runtime/data assumptions.
4. Every allocated draft item, review finding and earlier-slice follow-up.

Keep a short delta table in the slice plan: item, evidence, impact,
keep/adjust/reject/already-done decision and rationale. An unchanged item can
share one row with related unchanged items. Do not merely reread the draft
and claim it current. A separate `change-plan.md` is optional only for a
substantial change that needs guidance; do not create duplicate registries.

Evaluate the enumeration for completeness, feasibility and overbuilding.
Update/approve/reject/adjust the plan before implementation. Rejecting a draft
approach is allowed; dropping mandatory product scope requires an explicit
scope decision. Resolve actual blocking human choices without reopening
settled decisions or sending routine engineering choices for approval.

## 2. Complete the slice requirements and design

Update needs, requirements, acceptance signals and fixtures for the assigned
feature and its blast radius. Keep stable INS/V01 IDs and map additions to
user outcomes. Make this slice's acceptance complete and executable; do not
leave its own error, identity, Engine-lifecycle or UI behavior to Slice 60.
Later slices own broader cross-feature qualification, not unfinished basics.
For preparation slices, make the central criteria/methods/architecture complete
for their stated outputs and route unresolved required policy/feasibility to
explicit blockers. Each feature maintains its assigned verification rows and
feeds shared contract changes back to the canonical baseline and affected slices.

Expand the draft design to the needed Inspector-local contracts, data flow,
errors, resource ownership, dependency capabilities and actual view behavior.
Identify source-backed reuse and new Inspector work separately. Update the
product design or related docs where a local choice changes their meaning.

Directly spawn an independent design-review subagent with model
`gpt-6.1-sol`, reasoning effort `high`. Give it the current needs/req+AC,
slice design, relevant code/contracts, delta decisions and Inspector-only
boundary. Resolve material findings and have changed areas rechecked before
implementing them. Record a short disposition in `status.md`, not a large
review transcript or ceremonial approval packet.

## 3. Implement the slice using TDD

After requirements, design and implementation steps are planned and the
design review is addressed, implement the slice according to that plan.

For each behavior change, write an independent-intent failing test first,
run it and confirm the intended failure, commit the RED with failure evidence,
then implement the smallest correct GREEN in a separate commit and refactor
while preserving the tests. Avoid one broad test that forces a big-bang
implementation. Do not edit tests merely to make a wrong implementation pass,
regenerate golden oracles, mock the database, or move Engine work into this
slice. Test persistence/read semantics against real temporary fixture databases;
assert UI interactions through the actual application, not only labels.

Deliver a usable vertical feature, documentation and its tests. Track narrowly
scoped progress in `status.md`; do not produce separate receipt files for each
command. Ordinary docs-only changes do not require artificial RED tests.

## 4. Review and verify directly

Directly spawn a fresh code-review subagent with model `gpt-6.1-sol`, reasoning
effort `high`, on the implemented diff and accepted slice contract. Give it
RED/GREEN references, tests, dependency boundaries and known limitations.
Address material findings; recheck changed areas as needed.

Directly spawn a Terra test/verification subagent (`gpt-6.1-terra`) with the
final revision, slice acceptance, test commands, fixtures and blast radius.
Terra independently runs relevant checks, samples the RED/GREEN evidence,
exercises real database and browser flows, and reports pass/fail/skips and
unverified claims. It does not implement fixes or share mutable build state
with a concurrent writer. Failed tests remain evidence for the main writer
to fix; Terra verifies the resulting change.

Use direct subagent tools. Do not use MCP model dispatch, Steward or
Orchestrator agents. Optional Luna/Astra assistance is direct and scoped; it
does not replace the specified Sol reviews or Terra verification.

## 5. Pragmatic verification scope

Run fast lint/type checks and focused tests before broader relevant suites.
For a specific finding, rerun the reproducer, affected tests and checks for
its demonstrated blast radius. Do not rerun full regressions for every local
fix. Broaden when changes cross boundaries, shared Inspector contracts change,
failures leave uncertainty, or a required repository/release gate applies.
Complete required code-phase checks on the final tree without redundant full
reruns; Slice 60 owns the planned integrated v0.1 qualification.

Document-only work uses scoped Markdown/link/MkDocs checks. Source/testing work
uses appropriate repository and Inspector checks, with a checkout-owned tool
environment. A missing tool/runtime/model or skipped required test is reported
as unverified, not converted into a pass. Cancelled work is not an empty result.
Follow the repository's retry budget: rethink after two same-mode failures.

## 6. Slice status and cleanup

Create/update the slice's `status.md` when execution begins. Keep only:
current state/revision, reconciled scope or blockers, requirements/design
changes, Sol review dispositions, RED/GREEN references, Terra verification,
remaining limitations and cleanup/integration result. Suggested states are
DRAFT, ACTIVE, BLOCKED and COMPLETE; do not mark COMPLETE with required work
or verification outstanding. Keep it short and derive completion from Git
and executed evidence, not narration.

If execution used an extra branch/worktree, integrate its commits back into
the task's `dev/0.8.30` branch (or an explicitly designated successor), verify
the integrated revision, then remove/archive only that slice-owned worktree
and delete only its fully integrated temporary branch using normal hooks.
Preserve needed untracked work. Do not remove the pre-existing task worktree
or unrelated branches. If no extra checkout was created, record cleanup as
not applicable. Do not force-push, publish, or merge this task to main merely
to close a slice.
