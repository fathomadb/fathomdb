---
title: FathomDB 0.8.27 lean slice execution contract
status: ACTIVE
target_release: 0.8.27
---

# FathomDB 0.8.27 lean slice execution contract

Every slice applies this workflow proportionately. Analysis-only prework writes
durable evidence and decisions; it does not manufacture product changes.

## Reconcile the draft

At slice start, enumerate changes since drafting, related work already landed,
the assigned features/functions/surfaces, and items allocated by an earlier
slice. Approve, narrow, adjust, or reject the draft from that evidence. Keep the
slice complete for its stated outcome without absorbing adjacent work.

## Complete contracts and design

Add or update the applicable need, requirement, acceptance, interface, and
design records. Material design changes receive independent read-only review
before implementation. An evidence-only slice records why product design is
not applicable.

## Implement with focused TDD and review

Behavioral work preserves a genuine failing test before production changes,
then follows RED -> GREEN -> REFACTOR without weakening the oracle. Mechanical
documentation and status changes use the repository's documented exception.
Material code changes receive independent review of the actual diff.

## Verify proportionately

Run focused tests and validators first, then inspect the demonstrated blast
radius. Use an independent read-only verifier at slice close. Run
`./scripts/agent-verify.sh` after meaningful repository changes as required by
repository policy; reserve broad packaging/platform/release matrices for a
change whose blast radius warrants them or for the assigned qualification
slice. Never convert unavailable evidence into a pass.

## Close and clean up

Record scope decisions, requirements/design effects, exact commits, tests,
review verdicts, unresolved evidence, and the next dependency. The durable
release worktree has one writer. Read-only reviewers may share it. Any
temporary worktree must be integrated, verified from the destination, and
removed when safe; the release worktree remains until release close.
