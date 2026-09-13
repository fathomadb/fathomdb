---
title: FathomDB 0.8.26 release status
status: ACTIVE
---

# FathomDB 0.8.26 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.26.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.26:status-current-state -->**Next is Slice 9 (PREWORK-IMPLEMENTATION), IN_PROGRESS.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.26:status-current-state -->

Slices 0–8 are complete on `release/0.8.26`. Slice 9 implements only the narrow
preparation bundle authorized at `seq-289`; no product feature or publication
is authorized here.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.26:status-next-action -->**Continue Slice 9 (PREWORK-IMPLEMENTATION)** — approved repository preparation. **Remaining ladder:** 9 → 10 → 15 → 20 → 30 → 35 → 40 → 45 → 46 → 50.<!-- END GENERATED release-state:0.8.26:status-next-action -->

## Open decisions

There is <!-- BEGIN GENERATED release-state:0.8.26:status-live-open-count -->ONE<!-- END GENERATED release-state:0.8.26:status-live-open-count --> live open decision.
D26-01 remains deferred until Slice 15 produces its bounded performance and
erasure-linearization evidence; it blocks Slice 20 only.

## Release boundaries

- 0.8.26 changes affected V1 contracts in place, is breaking, accepts fresh
  databases only, and adds no parallel functional V1/V2 public API pair.
- Scope stays narrow around the selected Memex needs.
- Tagging, publication, registry mutation, and main integration require
  separate authorization.

## Verification

Every slice records focused evidence, independent review, the proportionate
repository gate, and its exact closeout commit. Generated facts must pass
`scripts/check-release-state-views.sh`; worktree/dependency authority must pass
`scripts/preflight.sh`.
