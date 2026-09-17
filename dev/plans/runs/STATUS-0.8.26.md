---
title: FathomDB 0.8.26 release status
status: ACTIVE
---

# FathomDB 0.8.26 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.26.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

The complete 0.8.26 ladder is recorded on `origin/release/0.8.26`; integration
to `origin/main` is pending.

Slices 0–10, 15, 20, 30, 35, 40, 45, 46, and 50 are complete on the 0.8.26
release line. Slice 50 binds the exact candidate, local package witnesses,
116/116 capable-executor gate, five native platforms, distinct Windows WAL
receipt, and both Gitleaks scans in its validated manifest. GPT-6 Astra medium
design/code review and independent completion verification pass.

## Immediate next action

Integrate `origin/release/0.8.26` into `origin/main` through the separately
authorized release workflow. Do not tag or publish from this completion record.

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
