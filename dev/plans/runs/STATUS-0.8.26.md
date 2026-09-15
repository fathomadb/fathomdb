---
title: FathomDB 0.8.26 release status
status: ACTIVE
---

# FathomDB 0.8.26 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.26.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.26:status-current-state -->**Next is Slice 40 (DERIVED-EDGE), DRAFT.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.26:status-current-state -->

Slices 0–10, 15, 20, 30, and 35 are complete on the 0.8.26 release line. Slice
35 proved the changed-in-place five-operation V1 actuation grammar, complete
prospective derived-edge endpoints, compact bounded receipts, cross-binding
conformance, and the fresh-database prototype boundary. Slice 40 owns the real
schema/open cutover; Slice 50 retains exact-version post-publication work.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.26:status-next-action -->**Commission Slice 40 (DERIVED-EDGE)** — atomic derived-edge V1 actuation. **Remaining ladder:** 40 → 45 → 46 → 50.<!-- END GENERATED release-state:0.8.26:status-next-action -->

## Open decisions

There is <!-- BEGIN GENERATED release-state:0.8.26:status-live-open-count -->ZERO<!-- END GENERATED release-state:0.8.26:status-live-open-count --> live open decision.
D26-01 is ruled at `seq-290`: Slice 20 implements the opt-in first-generation
V1 graph-evidence sidecar under frozen authority.

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
