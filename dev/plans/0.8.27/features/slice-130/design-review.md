---
title: FathomDB 0.8.27 Slice 130 - design review
status: REVIEWED
target_release: 0.8.27
---

# Slice 130 design review

The first independent review used `gpt-6.1-sol` high before implementation.
It found no P1 issue and two P2 gaps:

| Finding | Resolution |
| --- | --- |
| The design did not close `graph.expand`'s dynamic import back into `engine.py` or specify domain import direction. | The design now defines facade-to-owner imports, allows graph/evidence to use canonical search conversion, makes domain `Engine` imports type-only, and routes `graph.expand` directly to its owner while retaining engine mapper aliases. |
| Runtime tests could silently load the release checkout's installed Python files instead of worktree sources. | The design now requires candidate-source path assertions, native artifact path evidence, and no worktree editable install. |

The review also asked for the thin `Engine.embed` and standalone native embed
exports to be explicit; they are now in the boundary.

The subsequent independent `gpt-6-sol` high review passed the revised plan
and design with no remaining blocker or overbuild. It noted an implementation
watchpoint: evidence-to-open frozen-context conversion must retain an acyclic
import direction. The pre-move Python candidate-source fixture used the
release checkout's qualified native binary without an editable installation;
loaded Python paths resolved inside this worktree. The focused pre-move suite
passed 82/82 tests.
