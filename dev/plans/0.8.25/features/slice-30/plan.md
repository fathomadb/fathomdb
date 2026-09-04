---
title: 0.8.25 Slice 30 — lifecycle and erasure closure
status: DRAFT_RECONCILED_AWAITING_REVIEW
depends_on: 25
design: design.md
design_status: DRAFT_RECONCILED_FORMAL_REVIEW_REQUIRED
---

# Slice 30 plan

## Outcome and carried obligations

Implement the core subset of R25/AC25-30; Memex needs 5/6; and A25-05 under the
approved [scope adjustment](../../scope-adjustment-2026-09-02.md). Propagate
lifecycle and erasure through the Slice 20 canonical-source-to-derived
dependency form, fence incomplete work, support idempotent restart/resume, and
prove no active or searchable orphan. A25-04 multi-source liveness moves to
0.8.26 with that dependency form.

The retained lifecycle vocabulary is the shipped one: supersession, `active`
or `pending` to `deleted`, `deleted` to `active`, purge, and source erasure.
Slice 30 does not add an `invalidated` state. It applies closure to registered
node and edge dependents, replaces Slice 25's temporary
`dependency_closure_required` refusal with atomic closure admission, and keeps
semantic decisions outside the Engine.

## Requirements and acceptance criteria

- **S30-R1 — atomic admission and fencing.** A source-losing write,
  lifecycle transition, purge, erasure, or actuation batch atomically records
  a closure operation and source-revision barriers with its root mutation.
  Registration, reactivation, and derived writes against a barriered or
  structurally ineligible source fail typed.
- **S30-R2 — direct-dependent closure.** Every Slice 20 registered node or edge
  dependent is made non-searchable for supersession/deletion and physically
  erased for purge/source erasure. Closure is direct only; recursive and
  multi-source semantics remain allocated to 0.8.26.
- **S30-R3 — bounded recovery.** Direct consequences commit atomically with the
  root mutation. Any remaining proof/at-rest phase survives restart and is
  resumed automatically in bounded internal batches; no public journal
  administration is added.
- **S30-R4 — truthful completion.** Completion requires a durable zero proof
  over active canonical dependents, every registered row-owned projection,
  dependency rows, pending projection work, and the existing telemetry/WAL
  erasure boundary. An empty work page is not proof.
- **S30-R5 — additive parity.** The keyed closure status/proof read and typed
  dependency refusal behavior are versioned and equivalent in Rust, Python,
  TypeScript, and the wire fixtures; existing successful no-dependent
  lifecycle calls remain source-compatible.

Acceptance requires unchanged RED fixtures proving: immediate invisibility
after admission; node and edge propagation; registration/reactivation races;
bounded multi-operation recovery; crash/reopen at each durable phase; exact replay;
corrupt/missing indexes fail closed; and raw database/WAL checks find no erased
dependent bytes. Public status must never claim complete while any injected
active, searchable, projection, dependency, or pending-work orphan remains.

## Verification routes

Selected: fast, heavy, all, all-feature/operator, Windows CPU/native
Rust/Python/Node, and packaged lifecycle smokes. GPU/CUDA, live-model, and
pre-publication registry-installed are N/A.

## Draft-to-ready and delivery

1. Reconcile the design with the landed Slice 20/25 schema, transaction seams,
   source-link authority, actual lifecycle enum, and existing erasure boundary.
2. Obtain an independent design review. Resolve at most four FIX-n cycles;
   unresolved P1/P2 findings keep the slice out of READY.
3. Write and commit failing Rust, Python, TypeScript, migration/property, and
   installed-package tests before product implementation.
4. Implement RED to GREEN without editing the binding acceptance tests to make
   failures pass. Preserve compatibility and keep one writer/one provenance
   authority.
5. Obtain independent implementation review, with at most four FIX-n cycles,
   then a separate verification pass. Allocate any accepted residual to the
   owning future slice with requirements/design review, RED/GREEN, code review,
   and verification obligations.
6. Run the selected routes, write `status.md`, advance release state only from
   exact Git evidence, and checkpoint reusable lessons before Slice 35.

Stop on searchable orphans, unverifiable erasure, non-idempotent recovery,
semantic-policy creep, or a public contract that depends on an unlanded later
slice.
