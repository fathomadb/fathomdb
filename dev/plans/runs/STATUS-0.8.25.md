---
title: FathomDB 0.8.25 status
status: ACTIVE
target_release: 0.8.25
---

# STATUS — FathomDB 0.8.25

The single writer is `dev/plans/release-state-0.8.25.json`. Edit release facts
there, then regenerate this board's fenced view. The release plan is
`dev/plans/plan-0.8.25.md`.

## Current state

Prework is active on the durable `release/0.8.25` worktree. Slices 0–7 are
sequential; feature work begins at Slice 10. Direct agents execute this release
without Steward or Orchestrator roles.

## Slice ladder

| Slices | Scope | State |
| --- | --- | --- |
| 0–5 | Environment, dependencies, cruft, contracts, architecture, verification | Slice 0 complete on release branch; Slice 1 next |
| 6 | Proposal scoring and interactive HITL decisions | Not started |
| 7 | Approved repository-preparation implementation only | Not started |
| 10–75 | Dependency-linear product and measurement features | Not started |

## Decisions and blockers

- CUDA and ptrace are authorized. Unconfined probes work; sandboxed probes do
  not establish host absence.
- The current `.venv/bin` shim resolves to the primary checkout and cannot
  certify release-branch Python/native behavior.
- Generic release-state `landed` rendering claims `origin/main`; release-branch
  completion therefore uses explicit ladder status until Slice 7 decides a
  generalized branch-completion contract.
- No publication, external-system mutation, or feature implementation is
  authorized by prework.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.25:status-next-action -->**Commission Slice 1 (DEPENDENCIES)** — Dependabot, library, and pinning sweep. **Remaining ladder:** 1 → 2 → 3 → 4 → 5 → 6 → 7 → 10 → 15 → 20 → 25 → 30 → 35 → 40 → 45 → 50 → 55 → 60 → 65 → 70 → 75.<!-- END GENERATED release-state:0.8.25:status-next-action -->

## Verification

Every transition must pass the release-state renderer, developer Markdown
lint, and `git diff --check`. Each Slice 1–5 record is proposal-only.
