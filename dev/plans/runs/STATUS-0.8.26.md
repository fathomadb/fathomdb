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

Slices 0–10, 15, 20, 30, 35, 40, 45, 46, 50, 55, and 60 are complete on the
0.8.26 release line. Slice 50 bound candidate `68514f70`, local package
witnesses, 116/116 capable-executor gate, five native platforms, distinct
Windows WAL receipt, and both Gitleaks scans in its validated manifest. Slice
55 preserved the 69 signed surface tokens while enforcing exact parity over 44
canonical Python/TypeScript operations and restoring the approved TypeScript
`rerank` peer. Its 117/117 gate and independent design, code, and verification
reviews passed. A post-completion GPT-6 Astra medium review then closed two P2
rerank boundary defects through committed RED/GREEN remediation and a fresh
117/117 gate. Slice 60 reconciled the retrieval, recovery, and engine owners and
made explicitly acknowledged malformed-WAL recovery reachable through a
fail-closed operator-only seam. Independent design/code review and verification
passed, including the strict-security 117/117 canonical gate. Slice 65 now
enforces current-owner authority for all 25 maintained designs, preserves the
69-token/44-operation parity contract, and binds replacement candidate
`8ffb3486`. Its strict-security 118/118 local gate, fresh wheel/npm/CLI profiles,
both Gitleaks scans, five native platform jobs, Windows WAL attribution, and
independent design/code/candidate verification passed.

## Immediate next action

| | |
|---|---|
| **Immediate next action** | Execute the owner-authorized non-publishing release closeout: reconcile the release records, integrate `origin/release/0.8.26` into `origin/main`, and pass the required hosted CI workflows. Do not create or push `v0.8.26`, publish packages, mutate registries, or run post-publication claims without a later explicit authorization. |

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
- Main integration and hosted CI are authorized by the repository owner on
  2026-09-18. Tagging, publication, registry mutation, and post-publication
  claims remain unauthorized.

## Verification

Every slice records focused evidence, independent review, the proportionate
repository gate, and its exact closeout commit. Generated facts must pass
`scripts/check-release-state-views.sh`; worktree/dependency authority must pass
`scripts/preflight.sh`.
