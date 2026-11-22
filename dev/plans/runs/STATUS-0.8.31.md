# FathomDB 0.8.31 release status

**ACTIVE — release implementation on `release/0.8.31`.**
The [state](../release-state-0.8.31.json) owns structured status and next
actions. The [master plan](../0.8.31/plan.md) and [scope](../0.8.31/scope.md)
own settled HITL decisions, requirements, and execution contracts.

## Current state

Slice 0 is active on `release/0.8.31`:
- Merged completed `origin/inspector/dev/0.2.1` (`d6501cc5a`) cleanly at
  `086d5af0a`.
- Landed RED→GREEN worktree Python/TS bootstrap fix (`20269cee4` RED →
  `db271323f` GREEN), verified `/usr/bin/python3.12` selection on Ubuntu 24.04,
  and bootstrapped owned `.venv`, `node_modules`, and `src/ts/node_modules`.
- Authored ADRs `#73`–`#76` (`ADR-0.8.31-*`) and the W12 checkpoint-fence
  review (`dev/plans/0.8.31/features/slice-0/w12-fence-review.md`).
- Capturing `v0.8.30` baseline latency measurements and closed golden
  schema-37 fixture DB prior to opening Slice 10.

## Next action

Finish Slice 0 baseline capture + independent review, close Slice 0, and launch
Slice 10 (`SCHEMA_VERSION` 38+, inline typed node/edge properties + declared
typed indexes, app-declared vector kinds, op-collection kind readback, W12
erasure ergonomics fixes, and `sqlite_master` completeness audit).
