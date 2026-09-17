---
title: FathomDB 0.8.26 release status
status: ACTIVE
---

# FathomDB 0.8.26 release status

This board is a generated-view consumer of
`dev/plans/release-state-0.8.26.json`. Update machine-owned facts in that state
file and regenerate; keep evidence and qualification prose here.

## Current state

<!-- BEGIN GENERATED release-state:0.8.26:status-current-state -->**Next is Slice 60 (DESIGN-OWNERS), DRAFT.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.26:status-current-state -->

Slices 0–10, 15, 20, 30, 35, 40, 45, 46, 50, and 55 are complete on the
0.8.26 release line. Slice 50 bound candidate `68514f70`, local package
witnesses, 116/116 capable-executor gate, five native platforms, distinct
Windows WAL receipt, and both Gitleaks scans in its validated manifest. Slice
55 preserved the 69 signed surface tokens while enforcing exact parity over 44
canonical Python/TypeScript operations and restoring the approved TypeScript
`rerank` peer. Its 117/117 gate and independent design, code, and verification
reviews passed. A post-completion GPT-6 Astra medium review then closed two P2
rerank boundary defects through committed RED/GREEN remediation and a fresh
117/117 gate. The ladder remains open for Slices 60 and 65; Slice 50 remains
historical evidence but is no longer the final integration candidate.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.26:status-next-action -->**Commission Slice 60 (DESIGN-OWNERS)** — retrieval, recovery, and engine current-owner reconciliation. **Remaining ladder:** 60 → 65.<!-- END GENERATED release-state:0.8.26:status-next-action -->

Do not integrate, tag, or publish until Slice 65 records a replacement exact
candidate and restores the completion claim.

## Open decisions

There is <!-- BEGIN GENERATED release-state:0.8.26:status-live-open-count -->ZERO<!-- END GENERATED release-state:0.8.26:status-live-open-count --> live open decision.
D26-01 is ruled at `seq-290`: Slice 20 implements the opt-in first-generation
V1 graph-evidence sidecar under frozen authority.

## Release boundaries

- 0.8.26 changes affected V1 contracts in place, is breaking, accepts fresh
  databases only, and adds no parallel functional V1/V2 public API pair.
- Scope stays narrow around the selected Memex needs.
- The reopened ladder changes conformance tooling and maintained design
  authority; runtime product changes require separate explicit scope.
- Tagging, publication, registry mutation, and main integration require
  separate authorization.

## Verification

Every slice records focused evidence, independent review, the proportionate
repository gate, and its exact closeout commit. Generated facts must pass
`scripts/check-release-state-views.sh`; worktree/dependency authority must pass
`scripts/preflight.sh`.
