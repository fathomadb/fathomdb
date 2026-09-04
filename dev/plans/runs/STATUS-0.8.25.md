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

<!-- BEGIN GENERATED release-state:0.8.25:status-current-state -->**Next is Slice 15 (IDENTITY), NOT_STARTED.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.25:status-current-state -->

Prework and Slice 10 are complete on the durable `release/0.8.25` worktree.
Feature work continues with Slice 15. The owner-approved 2026-09-02
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
| 7 | Approved repository preparation | Complete on release branch (`fdbae48a`) |
| 10 | Measurement classification | Complete on release branch (`f383ec82`) |
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
- Release-branch Python/native behavior is certified through the isolated
  release-wheel verifier; the primary-checkout `.venv` is not evidence.
- Generic release-state completion now distinguishes release-branch completion
  from `origin/main` reachability. The `landed` set remains empty until an
  independently authorized push and integration.
- No publication, external-system mutation, or feature implementation is
  authorized by prework.

## Immediate next action

<!-- BEGIN GENERATED release-state:0.8.25:status-next-action -->**Commission Slice 15 (IDENTITY)** — revision identity and canonical source provenance. **Remaining ladder:** 15 → 20 → 25 → 30 → 35 → 40 → 45 → 50 → 55 → 60 → 75.<!-- END GENERATED release-state:0.8.25:status-next-action -->

## Verification

Every transition must pass the release-state renderer, developer Markdown
lint, and `git diff --check`. Each Slice 1–5 record is proposal-only.

- Slice 7 fast verification passes 103/103 suites; the appropriately sized
  heavy route passes 2/3 applicable suites with one explicit exclusion.
- A fresh serial Rust workspace run passes and contains no removed-worktree
  target provenance. The isolated release-built Python wheel passes real
  open/write/search/close and property checks.
- Dependency policy, loader behavior, RustSec, protected pins, traceability,
  strict ptrace, documentation, and release-state views pass.
- The CUDA feature suite passes all 79 tests on RTX 3090 GPU 0 with CUDA 12.6,
  including CPU/GPU logit agreement and real GPU load/score.
- Independent implementation review passes after FIX-1/FIX-2 with no remaining
  P1/P2/P3 finding; a separate read-only verifier confirms the evidence.
- Slice 10 passes independent design and implementation review after the
  bounded FIX-3 cycles. Its 59 focused tests, portable and historical audits,
  fast tier, and applicable heavy suites pass; the sole heavy skip is the
  unavailable TypeScript dependency tree.
- A diagnostic release-wide long gate exposed the pre-existing AC-013 vector
  latency failure and was stopped after that failure. It is release debt, not
  a Slice 10 regression.
