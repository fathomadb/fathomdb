---
title: FathomDB 0.8.26 — lean slice execution contract
status: DRAFT
---

# Lean slice execution contract

Every 0.8.26 slice applies this workflow proportionately. A slice may add
specialized gates, but it does not omit a phase silently.

## 1. Reconcile change since drafting

At slice start, review and enumerate:

- changes to the drafted slice plan;
- related in-repository work completed since drafting;
- the assigned features, functions, and public surfaces; and
- draft items allocated to the slice by Slice 8 or an earlier slice.

Evaluate that delta, then approve, reject, narrow, or adjust the plan. Keep the
slice complete for its outcome without expanding into adjacent work.

## 2. Complete contracts and design

Add or update the user needs, requirements, and acceptance criteria required by
the accepted scope. Update the slice design so each criterion has a coherent
implementation and failure model. Have a read-only design-review subagent
review material design additions or changes; resolve findings before
implementation. Record `not applicable` with a reason when an analysis-only
slice changes no product design.

## 3. Implement with focused TDD and review

For behavioral work, commit or otherwise preserve the failing test first, then
make it pass and refactor without changing the intended oracle: RED → GREEN →
REFACTOR. Mechanical documentation/status changes may use the repository's
documented exception. Review implementation in the actual diff before
acceptance; use an independent code reviewer for material code changes.

Analysis-only slices implement only their durable findings, decisions, or plan
artifacts. They do not manufacture a code change merely to satisfy this phase.

## 4. Verify proportionately

Use an independent read-only subagent to challenge the completed slice and its
evidence. Start with the focused tests and validators that reproduce the
finding or cover the blast radius. Run `./scripts/agent-verify.sh` before
completing meaningful repository changes, as required by repository policy.
Reserve broader integration, packaging, platform, or release regression suites
for changes whose blast radius warrants them or for their assigned release
slice. Never weaken a gate to make evidence green.

## 5. Close and clean up

Write a slice status record containing scope decisions, requirement and design
changes, exact commits, tests, review verdicts, unresolved/external evidence,
and the next dependency. A slice does not require a new branch or worktree. If
it creates one, merge the accepted work back through the chosen integration
route, verify from the destination, and remove the temporary worktree/branch
when safe. Preserve the release worktree until the release itself closes.

Keep ceremony proportional: resolve the named work, inspect its blast radius,
and avoid full release regressions for an isolated finding unless repository
policy or the demonstrated impact requires them.
