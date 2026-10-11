---
title: FathomDB 0.8.27 prework Slice 7 - build and delivery evidence
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 7 - build and delivery failure evidence

## Plan and delta reconciliation

Outcome: separate recurring pre-build failures from post-build delivery
failures, retain already-closed guardrails, and propose only focused work that
can change a 0.8.27 decision. No environment, workflow, CI, secret-scan,
packaging, registry, credential, runner, or release state is changed here.

The draft is narrowed to recurring roots. Successful 0.8.26 package inventory,
manual full qualification routing, CUDA forward receipts, exact registry
polling, post-publish smokes, model-cache binding, and platform jobs require
retained regressions, not reinvention.

## Requirements and acceptance

| ID | Requirement | Acceptance signal |
| --- | --- | --- |
| PW27-7A | Heavy refactor/build routes start only with owned scratch space, tools, and sufficient disk. | Owning slice records tool versions, route-specific free-space budget, non-incremental/bounded build policy where needed, and cleanup ownership. |
| PW27-7B | Security exceptions remain exact and adversarially tested. | One canonical benign-digest registration rejects path, key/value-shape, and credential-shaped mutations. |
| PW27-7C | Qualification distinguishes source success from delivery readiness. | Pre-tag environment permission is checked read-only; package/platform evidence binds exact artifacts and unavailable routes remain unavailable. |

## Evidence and allocations

Pre-build/current:

- A missing 0.8.27 state makes release preflight report no live release. That is
  expected until prework activation; preflight itself needs no fix.
- Public-doc truth fails on the stale 0.8.25 README claim; Slice 10 owns it.
- 0.8.25 reached 100% filesystem use, the durable target reached roughly
  85.7 GiB, and Windows compilation exhausted disk. Hung/stale Cargo processes
  compounded it. The generic 10 GiB preflight minimum is not a heavy-matrix
  budget.
- The later refactor pilot needed `CARGO_INCREMENTAL=0`, reduced test debug
  info, per-row owned cleanup, stateful-test serialization, and runner-local
  `cargo-public-api`. Slice 30 must revalidate these as pilot-scoped inputs;
  global serialization is rejected.
- Editable native Python environments remain forbidden in worktrees. Ptrace
  denial remains an explicit capability failure, never a skipped/pass gate.

Post-build/delivery:

- The debug-only native-hook leak, manual no-diff routing gap, CUDA cache and
  libcudart omissions, and forward-receipt drift are closed by 0.8.26. Preserve
  their guards.
- Release attempt 1 failed before runner assignment because exact tag policy
  was absent on the CUDA environment. Slice 150/closeout should add a read-only
  exact-tag permission readiness check; it must not widen policy itself.
- Exact registry polling and post-publish smokes are correct. Retry only exact
  version visibility, never runtime/import failures.
- Benign performance-tokenizer digest registration recurred at `7106e95c`.
  Slice 10 should consolidate it into one reviewed exact-path plus exact-shape
  authority with positive and credential-shaped negative fixtures. Current-tree
  Gitleaks remains binding; full-history drift remains advisory.
- Current GitHub authentication is invalid. No privileged action is planned or
  claimed by prework.

Reject required-check aggregation, merge queues, nightly/soak expansion,
global test serialization, broad provisioning rewrites, broad `*.json` secret
exceptions, or full release regressions for these focused findings.

## Implementation, review, verification, and status

Implementation is the durable failure taxonomy and allocation only. No
behavioral code exists to drive RED/GREEN; code review is not applicable.
Evidence was independently audited against the 0.8.26 prework, candidate,
release, and large-file pilot records. Independent package design review and
closeout verification passed.

Status is `COMPLETE`. No branch, worktree, cache, credential, or artifact was
created or removed. Next: reserved Slice 8, then Slice 9.
