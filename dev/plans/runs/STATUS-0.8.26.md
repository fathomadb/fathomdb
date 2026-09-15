---
title: FathomDB 0.8.26 release status
status: ACTIVE
---

# FathomDB 0.8.26 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.26.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.26:status-current-state -->**Next is Slice 46 (DESIGN-CONVERGENCE), DRAFT.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.26:status-current-state -->

Slices 0–10, 15, 20, 30, 35, 40, and 45 are complete on the 0.8.26 release
line. Slice 45 establishes one active v2.2 architecture for the as-built source
candidate, retains the 0.6.0 snapshot as superseded history, reconciles the
source-accurate caller/projection write lanes and feature boundaries, and adds
an 18-control authority guard to local and Markdown-only CI. Independent design
and code reviews pass. The unchanged unconfined canonical gate passes 111/111
suites after the documented sandbox-only AC-036 ptrace denial. Slice 46 is
next; Slice 50 retains exact-version package/platform verification.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.26:status-next-action -->**Commission Slice 46 (DESIGN-CONVERGENCE)** — maintained technical-design convergence. **Remaining ladder:** 46 → 50.<!-- END GENERATED release-state:0.8.26:status-next-action -->

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
