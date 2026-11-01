---
title: Approved narrow provider contract — implementation handoff
status: implementation authorized by owner seq-359; independent Astra-medium review gates landing
target_release: 0.8.28
observed_on: 2026-11-01
---

# Implementation handoff

Implement the approved [contract](built-in-provider-design.md),
[requirements/AC](built-in-provider-requirements.md) and
[plan](built-in-provider-plan.md) from `2d63cee3c`, under owner `seq-359`.

- Worktree: `/home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-50-provider-contract`.
- Branch: `impl/0.8.28-built-in-provider-contract`.
- Source baseline: `2d63cee3c1071dd14f3e1deb44adfddc21672f19`.
- Landing target: `dev/0.8.28`, after separate Astra-medium implementation review.
- Root owns release JSON/board/ledger and approval records in its own checkout.
- Implementer owns source, tests, interfaces, affected accepted ADR supersession,
  and necessary source-owned design annotations in its isolated checkout.
- Never edit the root checkout, release-state/board/ledgers, or this brief.
- One behavior-specific RED commit with real failure evidence before its GREEN.
  Tests are fixed during implementation; old contract supersession/API adaptations
  are explicit, never weakened silently. Preserve historical characterization data.
- Keep TC-68 recipe/inputs/marker/gate/cadence/error semantics unchanged, including
  90 ordinary first-population / 45 fresh / zero valid-cache calls and restart reuse.
- No precision fork, new cache, key, process cadence, runtime descriptor/monitor,
  owned assets, mean-policy/recovery reorder, diagnostic CLI or rebuild redesign.
- D50-1/D50-2, scan Unit S and later slices are outside this implementation.
- Never skip hooks/force push/push tags/modify CUDA or credentials. No pushes by
  implementer; commits carry the recovered Claude-Session trailer.
- Focused checks while iterating. Final full strict gate only after a final clean
  code tree and checkout-owned Python/tooling preflight; do not borrow another
  checkout's venv or run editable installs from a linked worktree. Root coordinates
  checkout-local package/tool setup and final verification before landing.
- Stop after implementation/focused verification and report commit range, RED
  commands/evidence, tests, intentional adaptations, residual risks and any blocker.
  Do not spawn agents; root owns independent review and landing.
