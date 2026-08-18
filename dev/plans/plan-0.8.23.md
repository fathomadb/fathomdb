---
title: FathomDB 0.8.23 — planning-first release ladder
status: ACTIVE
target_release: 0.8.23
---

# FathomDB 0.8.23 — planning-first release ladder

0.8.23 is planned before feature implementation. Publication remains held; no
tag, registry upload, or release dispatch is implied by this plan.

## Goal and scope

The goal is a hygienic, evidence-ready 0.8.23 release plan. Planning Slices
0–6 are review-only; a feature begins only after its explicit commission.

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
- Windows WAL checkpoint attribution across Engine-owned readers, the Python
  SDK, serial lifecycle controls, and an external-reader control.

Existing commits and witnesses are candidate evidence, not a slice closure.
They must be inspected through this ladder rather than inferred as complete.

## Slice ladder — preparation report at Slice 6

| Slice | Mandate | Durable output |
| ---: | --- | --- |
| 0 | Identify required environment setup or changes, including the local Windows development environment. | Environment/precondition inventory. |
| 1 | Identify Dependabot needs; perform and respond to a library-sweep review. | Dependency disposition proposal. |
| 2 | Review repo cruft across program, `dev/`, developer, public, code, and test documentation. Enumerate keep, deprecate-in-place, archive, or delete. | Cruft proposal; no action. |
| 3 | Draft CRUD changes to user needs, requirements, and acceptance criteria; allocate every draft to a feature slice. | Draft requirements allocation; no action. |
| 4 | Review architecture and perform a high-level code-to-architecture alignment review. | Architecture/code proposal; no action. |
| 5 | Review verification adequacy: requirements→ACs, ACs→tests, product goals, and critical paths. | Test/verification proposal; no action. |
| 6 | Consolidate, value-rate, prioritize, and sequence the hygiene and preparation work identified by Slices 0–5. | Hygiene/preparation report and workplan; no feature implementation. |

Slices 0–6 use [the planning foundation](../design/0.8.23-planning-foundation.md).
Slice 6 records the concrete hygiene, environment, contract, design, and
verification preparation work identified by the prior reviews. Feature work is
commissioned only by an explicit subsequent instruction; the report itself
does not select feature scope. No code or feature work starts merely because a
preparation packet exists.

## Reserved-gap policy

Unplanned reliability findings receive a numbered feature slice only after the
Slice 6 report and an explicit commission. They do not bypass the planning
ladder or widen an in-flight feature slice.

## Feature/function candidates and commissions

| Slice | Candidate | Depends on | Design inputs |
| ---: | --- | --- | --- |
| 10 | CUDA environment/artifact contract and protected runner gate | 6 | `0.8.23-gpu-artifacts.md` |
| 20 | CUDA package, release rehearsal, driverless CPU, and GPU smokes | 6 | `0.8.23-gpu-artifacts.md` |
| 30 | Memex embedding-readiness and graph-integration contract | 6 | `0.8.23-embedding-configuration-feedback.md`, `0.8.23-memex-integration.md` |
| 40 | Fixture-scoped scale characterization | 6 | v1 historical: `0.8.23-scale-characterization-protocol.md`; v2 authority: `0.8.23-scale-characterization-v2.md`, `0.8.23-scale-artifact-v2.schema.json` |
| 50 | Gitleaks staged pre-commit and always-on CI guards | 6 | `0.8.23-gitleaks-guards.md` |
| 60 | Windows WAL checkpoint reader-conflict diagnosis | 6 | `0.8.23-windows-wal-checkpoint-reader-conflict.md`, `0.8.23-windows-local-environment.md` |
| 65 | Windows WAL checkpoint root-cause attribution | 6, 60 | `0.8.23-wal-attribution-investigation.md` |
| 70 | Supported dual CPU/GPU runtime policy, diagnostics, artifacts, and exact pre-fusion TC-5 controls | 6, 20 | `0.8.23-slice-70-dual-runtime-device-policy.md`, `0.8.23-slice-70-tc5-vector-stage-hypothesis.md` |
| 71 | Draft cross-encoder reranker CPU/GPU runtime policy, diagnostics, and artifact parity | 70, 20 | `runs/0.8.23-slice-71-draft-plan.md` |

Each feature slice, if approved, must review its assigned candidate features,
drafted needs/requirements/ACs, and design inputs; approve, reject, or adjust
the drafts; write its design review; implement through RED→GREEN TDD; review,
verify, and write its Slice status record. Slice 60 additionally requires a
first-party Windows x64 evidence path before it can be commissioned. A local
Windows development environment is available for controlled reproduction and
development; it is not, by itself, an automated CI or release-evidence path.
Slice 65 is expressly commissioned as a post-Slice-60 investigation. Its
design and RED-to-GREEN instrumentation completed on `origin/release/0.8.23`
at `6b57557c`. Its three hosted enriched binding repetitions
(`95369168206`, `95373316503`, and `95375745327`) completed the discriminator;
independent evidence review closed the result **UNATTRIBUTED / NO REMEDY**.
This authorizes no retry, binding, reader-pool, public API, or production
behavior change. Optional diagnostic work is separate scope and does not block
the next commissioned release slice.

HITL commissioned Slices 10, 20, and 70 on 2026-08-17. Slice 10's local
contract work is complete; its external provenance, runner-policy, and hardware
witness condition is `PENDING_EXTERNAL`. Per HITL seq:249, that condition
blocks release completion and publication, not local Slice 20 or Slice 70 work.
Slice 20's local control plane is complete and its external rehearsal evidence is
pending; Slice 70 is next. Their work remains
subject to their reviewed designs, acceptance criteria, and non-publication
boundary. Slice 20's candidate dry-run CUDA rehearsal is never a canonical tag
producer: canonical Linux x64 publication remains hard-blocked before all
publishers, with no candidate artifact or credential hand-off, until a separate
owned canonical-route design, provenance, and evidence package pass review.

Per HITL seq:252, which supersedes seq:251, Slice 70 still requires its private
TC-5 benchmark work. That work remains isolated from the supported product
device-policy surface and must satisfy its fail-closed vector-stage contract;
the dual-runtime product work does not substitute for it.

HITL subsequently requested a **draft** Slice 71 plan for the cross-encoder
reranker. It follows Slice 70 rather than widening its embedding scope. Slice
71 is not implementation authority until its assigned providers, bindings,
configuration/diagnostic behavior, and package path have been audited and its
design passes independent review.

## Cross-cutting DoD

- Every planning slice writes its required durable record and takes no product
  action.
- Slice 6 presents a concise hygiene and preparation report; the next bounded
  task is explicitly commissioned.
- Every approved feature slice follows review, design, RED→GREEN TDD, code
  review, verification, and a Slice status record.
- Publication remains a separate explicit HITL decision.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.23:plan-landed-roll-up -->
**COMPLETED on `origin/release/0.8.23`; `origin/main` integration is PENDING, in full:** Slices 0 (`916023fe`) · 1 (`2167a0cd`) · 2 (`b363af85`) · 3 (`91e162c2`) · 4 (`a7df1590`) · 5 (`00f865f3`) · 6 (`e98f727d`) · 50 (`ae7cef0e`) · 30 (`776d2c20`) · 60 (`423baf6a`) · 65 (`6b57557c`). SCHEMA is 26; remaining ladder = 70 → 71 → 72 → 40.<!-- END GENERATED release-state:0.8.23:plan-landed-roll-up -->

<!-- BEGIN GENERATED release-state:0.8.23:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 70** (`DUAL-RUNTIME-TC5`) — supported dual CPU/GPU runtime policy, diagnostics, artifacts, and exact pre-fusion TC-5 controls

**Remaining ladder:** 70 → 71 → 72 → 40.<!-- END GENERATED release-state:0.8.23:plan-immediate-next -->
