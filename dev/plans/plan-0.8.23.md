---
title: FathomDB 0.8.23 — planning-first release ladder
status: ACTIVE
target_release: 0.8.23
---

# FathomDB 0.8.23 — planning-first release ladder

0.8.23 is planned before feature implementation. Publication remains held; no
tag, registry upload, or release dispatch is implied by this plan.

## Goal and scope

The goal is a decision-ready 0.8.23 release scope. Planning Slices 0–6 are
review-only; feature implementation begins only after Slice 6 and HITL approval.

## Release requirements and acceptance criteria

Planning work must produce the durable outputs named below, allocate every
approved draft to one feature slice, and preserve an explicit defer/postpone
disposition for every excluded candidate.

## Release candidates under review

- Linux x86_64 CUDA-capable Python and npm artifacts with CPU-default,
  driverless usability, trusted-runner evidence, and installed-artifact smokes.
- Typed cross-SDK embedding-readiness feedback and a documented Memex graph
  integration contract.
- Fixture-scoped scale characterization, not a supported-scale promise.
- Staged pre-commit and always-on CI Gitleaks scanning.
- Windows WAL checkpoint reader-conflict diagnosis, contingent on first-party
  Windows evidence.

Existing commits and witnesses are candidate evidence, not a slice closure.
They must be inspected through this ladder rather than inferred as complete.

## Slice ladder — mandatory stop at Slice 6

| Slice | Mandate | Durable output |
| ---: | --- | --- |
| 0 | Identify required environment setup or changes, including the local Windows development environment. | Environment/precondition inventory. |
| 1 | Identify Dependabot needs; perform and respond to a library-sweep review. | Dependency disposition proposal. |
| 2 | Review repo cruft across program, `dev/`, developer, public, code, and test documentation. Enumerate keep, deprecate-in-place, archive, or delete. | Cruft proposal; no action. |
| 3 | Draft CRUD changes to user needs, requirements, and acceptance criteria; allocate every draft to a feature slice. | Draft requirements allocation; no action. |
| 4 | Review architecture and perform a high-level code-to-architecture alignment review. | Architecture/code proposal; no action. |
| 5 | Review verification adequacy: requirements→ACs, ACs→tests, product goals, and critical paths. | Test/verification proposal; no action. |
| 6 | Collect, score, and present every proposal for HITL decision. | Decision package; **stop**. |

Slices 0–6 use [the planning foundation](../design/0.8.23-planning-foundation.md).
Slice 6 scores each proposal for understanding, risk, effort, and
include-versus-postpone. No code or feature work starts before HITL responds to
that package.

## Reserved-gap policy

Unplanned reliability findings receive a numbered feature slice only after
Slice 6 and HITL disposition. They do not bypass the planning ladder or widen
an in-flight feature slice.

## Feature/function candidates — not commissioned

| Slice | Candidate | Depends on | Design inputs |
| ---: | --- | --- | --- |
| 10 | CUDA environment/artifact contract and protected runner gate | 6 | `0.8.23-gpu-artifacts.md` |
| 20 | CUDA package, release rehearsal, driverless CPU, and GPU smokes | 6, 10 | `0.8.23-gpu-artifacts.md` |
| 30 | Memex embedding-readiness and graph-integration contract | 6 | `0.8.23-embedding-configuration-feedback.md`, `0.8.23-memex-integration.md` |
| 40 | Fixture-scoped scale characterization | 6 | `0.8.23-scale-characterization-protocol.md` |
| 50 | Gitleaks staged pre-commit and always-on CI guards | 6 | `0.8.23-gitleaks-guards.md` |
| 60 | Windows WAL checkpoint reader-conflict diagnosis | 6 | `0.8.23-windows-wal-checkpoint-reader-conflict.md`, `0.8.23-windows-local-environment.md` |

Each feature slice, if approved, must review its assigned candidate features,
drafted needs/requirements/ACs, and design inputs; approve, reject, or adjust
the drafts; write its design review; implement through RED→GREEN TDD; review,
verify, and write its Slice status record. Slice 60 additionally requires a
first-party Windows x64 evidence path before it can be commissioned. A local
Windows development environment is available for controlled reproduction and
development; it is not, by itself, an automated CI or release-evidence path.

## Cross-cutting DoD

- Every planning slice writes its required durable record and takes no product
  action.
- Slice 6 presents a concise, scored include/postpone package and stops.
- Every approved feature slice follows review, design, RED→GREEN TDD, code
  review, verification, and a Slice status record.
- Publication remains a separate explicit HITL decision.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.23:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 5** (`VERIFICATION`) — verification-adequacy review

**Remaining ladder:** 5 → 6 → 10 → 20 → 30 → 40 → 50 → 60.<!-- END GENERATED release-state:0.8.23:plan-immediate-next -->
