---
title: Inspector 0.2.0 Slice 0 checkout decision
date: 2026-11-19
product: FathomDB Inspector
target_release: 0.2.0
status: ACTIVE
desc: Owner ruling for future release checkout creation and entry in Slice 0
---

# Slice 0 checkout decision

Inspector-local ruling **S0-BOOTSTRAP-01**, owner decision 2026-11-19:

> Create and enter the release branch/worktree in Slice 0; keep other environment/infra changes as proposals.

The selection UI's “Recommended” suffix was a label, not part of the ruling.
This Inspector-local decision resolves the checkout timing in the draft ladder.
It does not commission Slice 0 execution in this planning turn.

When Slice 0 is commissioned, its first action creates **and enters** branch
`inspector/dev/0.2.0` at
`/home/coreyt/projects/fathomdb-worktrees/inspector-0.2.0`. Use the approved final
HEAD of `llm/0.8.29-inspector-additional`, including all Inspector port, adopted
scope and integrated planning commits, preserving selected 0.8.29 ancestry to
`6edc1155f039f87cd0a37660cb27ae4373bbce71`. Do not start from raw main or bare
`dev/0.8.29`. Switch the executing session into the release worktree using the
available harness checkout-selection mechanism or session working-directory
control. Creating it while continuing in the planning checkout is insufficient.
Confirm actual branch, HEAD and session working directory after entry; subsequent
commands use the release worktree explicitly.
Check ownership, occupancy and applicable preflight before creation; preserve
and report unrelated or divergent existing work rather than reset or reuse it.

Slice 0–9 reports then use that release checkout sequentially with one writer.
Only this branch/worktree creation and entry is the Slice 0 action exception.
Other environment/infrastructure changes, installs, configuration, upgrades and
source remedies remain proposals. Slice 9 still requires actual item-level
human disposition and final ratification; Slice 10 implements only its approved
set in the existing release checkout and owns no initial checkout bootstrap.
The [Slice 0 plan](plan.md), [master](../plan.md) and
[workflow](../slice-workflow.md) apply these bounds. No branch/worktree,
environment or product changes were performed by recording this decision.
