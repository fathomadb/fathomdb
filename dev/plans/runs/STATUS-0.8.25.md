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

<!-- BEGIN GENERATED release-state:0.8.25:status-current-state -->**Next is Slice 7 (PREWORK-IMPLEMENTATION), NOT_STARTED.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.25:status-current-state -->

Prework is active on the durable `release/0.8.25` worktree. Slices 0–7 are
sequential; feature work begins at Slice 10. The owner-approved 2026-09-02
scope adjustment removes Slices 65/70 and narrows the retained implementation
ladder. Direct agents execute this release without Steward or Orchestrator
roles.

## Slice ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 0 | Environment and infrastructure | Complete on release branch (`321ca576`) |
| 1 | Dependencies and pins | Complete on release branch (`51043e20`) |
| 2 | Repository cruft | Complete on release branch (`51043e20`) |
| 3 | Draft contracts | Complete on release branch (`51043e20`) |
| 4 | Architecture/code alignment | Complete on release branch (`51043e20`) |
| 5 | Verification adequacy | Complete on release branch (`51043e20`) |
| 6 | Proposal scoring and interactive HITL | Complete on release branch (`3a35c1e6`; approved `seq-274`) |
| 7 | Approved repository preparation | Ready |
| 10 | Measurement classification | Not started |
| 15 | Identity and source provenance | Not started |
| 20 | Core dependency registration | Not started |
| 25 | Core atomic semantic actuation | Not started |
| 30 | Lifecycle and erasure closure | Not started |
| 35 | Eligibility and optional frozen reads | Not started |
| 40 | Core projection generation/readiness | Not started |
| 45 | Minimal pagination and operational state | Not started |
| 50 | Compact source-complete evidence | Not started |
| 55 | Basic tracing and integrity | Not started |
| 60 | Minimal constrained graph parity | Not started |
| 75 | Trimmed trustworthy release verification | Not started |

## Decisions and blockers

- `seq-272` and `seq-273` rule every proposal. `seq-274` approves the reviewed
  Slice 7 plan and closes Slice 6. P25-17 keeps all runs/data; P25-20 remains
  narrow.
- The owner-approved
  [scope adjustment](../0.8.25/scope-adjustment-2026-09-02.md) is the current
  feature boundary. Bubble work is allocated to 0.8.26–0.8.28 or Parked;
  experimental work is assigned to 0.8.29/0.8.31/0.8.33 reviews or Parked.

- CUDA, NVIDIA tools including `nvidia-smi`, and ptrace are standing-authorized,
  including unconfined execution when needed. Sandboxed probe failures do not
  establish host absence.
- The current `.venv/bin` shim resolves to the primary checkout and cannot
  certify release-branch Python/native behavior.
- Generic release-state `landed` rendering claims `origin/main`; release-branch
  completion therefore uses explicit ladder status until Slice 7 decides a
  generalized branch-completion contract.
- No publication, external-system mutation, or feature implementation is
  authorized by prework.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.25:status-next-action -->**Commission Slice 7 (PREWORK-IMPLEMENTATION)** — approved repository-preparation implementation. **Remaining ladder:** 7 → 10 → 15 → 20 → 25 → 30 → 35 → 40 → 45 → 50 → 55 → 60 → 75.<!-- END GENERATED release-state:0.8.25:status-next-action -->

## Verification

Every transition must pass the release-state renderer, developer Markdown
lint, and `git diff --check`. Each Slice 1–5 record is proposal-only.

- Release-state view and commission-manifest regression suites pass after the
  live plan/board views and Slice 6 curated inputs were registered.
- The unconfined strict run passed ptrace, egress, security, Python, and the
  other unaffected suites. It remains red on P25-INFRA-03: legacy board and
  briefing tools require `origin/main` landings and cannot represent completed
  release-branch-only prework.
- The unchanged serial Rust workspace run remains red on P25-INFRA-04: 24 test
  targets use retained binaries compiled under the removed `/tmp` worktree.
  A focused 34-test CLI rerun passed after recompilation, distinguishing stale
  target provenance from product behavior. Slice 7 must verify a fresh target.
