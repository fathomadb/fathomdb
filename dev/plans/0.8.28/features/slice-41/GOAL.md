---
title: FathomDB 0.8.28 Slice 41 — goal
status: DRAFT (planning docs for review; no tests or code yet)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 41 goal

Ship the COMP-03 tiered evidence rendering as a FathomDB SDK capability, so
an agent that uses FathomDB gets it without writing it.

On LongMemEval-S, 94% of answer turns are user turns, but assistant text
fills about 87% of a 30,000-character evidence budget. Rendering the top 3
retrieved rounds in full and the rest as "user turn plus the first 300
assistant characters" raised "every answer turn fully in the prompt" from
.820 to .921 (FTS, 500 questions, type-weighted) at the same budget. Today
that logic exists only in the offline prototype and the COMP-03 harness.

Slice 41 adds one package-level function, `pack_context`, to the Python,
TypeScript and Rust SDKs. It is implemented once in Rust, so all three
return byte-identical text and provenance. It also adds an optional
role-tagged body convention that keeps user and assistant text separable in
a stored node without changing FTS tokens.

The slice is complete when every AC41 in [requirements.md](requirements.md)
passes, the full gate is green on the final tree, the codex review
approves, and the measurements in [test-plan.md](test-plan.md) § 4 are
recorded. State is in [status.md](status.md).

Paste the block below to continue the slice in a fresh session.

```text
Continue FathomDB 0.8.28 Slice 41 (SDK context-pack helper) in worktree
/home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-41 on branch
slice/0.8.28-41-sdk-evidence-helper. Read AGENTS.md, the memory index, and
dev/plans/0.8.28/features/slice-41/{requirements,design,test-plan,status}.md.
Resume from status.md. Tests are read-only during the fix; do not edit
release-state-0.8.28.json, STATUS-0.8.28.md or generated blocks.
```
