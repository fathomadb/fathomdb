---
title: FathomDB 0.8.27 Slice 60 - implementation status
status: COMPLETE
implemented_on: 2026-09-25
planning_commit: 3d5b7c45
implementation_candidate: 5627c78b185d8fae9febe9f642a3b729363c75ea
closeout_commit: 573b0054cdc4a1c7e352d1ecf7acb76e1ead562a
---

# Slice 60 implementation status

## Completed scope

Slice 60 is complete on `release/0.8.27`. It moved the root-owned write,
provider-ingest, and consolidation implementation into six private modules
without any behavior change:

- `write.rs`: `WriteReceipt`, `PreparedWrite`, translation, the
  `Engine::write` facade, and late vector enrolment;
- `write_validation.rs`: pre-transaction validation and prior-cursor lookups;
- `write_commit.rs`: the transactional commit and apply path, revision
  identity, the trigger guard, read-visibility advance, and provenance
  retention;
- `provider.rs`: the shared NDJSON provider transport;
- `ingest.rs`: extractor ingest; and
- `consolidation.rs`: the consolidation provider.

`Engine` stays at the crate root. Every public type keeps its root path
through re-exports. `actuation.rs` already owned actuation, so only its
imports changed.

Cursor primitives, projection and registry code, record-lifecycle predicates,
and write test hooks remain for Slices 70 and 90. Schema, SQL, wire, bindings,
and packages are unchanged.

## Plan reconciliation and requirements

Planning reconciled the prework draft against:

- the Slice 40 foundation and Slice 50 domain moves;
- the existing actuation module;
- the live consumer seams; and
- a write-path test inventory. It found no full-state rollback oracle at any
  write boundary and no fault seam inside the transaction.

Requirements R27-60A-G were added. They cover:

- ownership without new public paths;
- full-state immutability at every refusal boundary, with no consumed cursor;
- zero mutation for provider failures before the first write;
- the validation-before-mutation property;
- no new semantics;
- public, hidden, and test-surface evidence; and
- review receipts.

Two existing behaviors were recorded as outside the structural scope:
provider ingest is not atomic across requests, and a consolidation
supersede/merge consumes an in-memory cursor before a later refusal.

Design review needed four cycles: Fable, then three Opus 5.5 cycles. It
corrected the seam map, the source scrapers, the mutants that were vacuous or
would not compile, and the fixture preconditions. It then returned PASS.

## TDD and review result

Characterization commit `2300e11b` added 10 boundary cases and a 32-case
property over a full `sqlite_master`-derived snapshot. All passed against
unmodified production, and every case failed under its recorded temporary
mutant. No production defect was found.

The moves landed in three commits:

- `261e9f54`: validation and commit;
- `a838cf4f`: the write facade; and
- `d84afac7`: provider, ingest, and consolidation.

Two further commits followed:

- `5716bea4` updated two now-moved symbol citations in the active
  `plan-0.8.20.md`, as `lint-plan-anchors` requires.
- `45f91b65` kept the dead `legacy_revision_id` helper private at the root,
  because AC-050a forbids a `pub(..) legacy_*` item. `revision_hash_field`
  became `pub(crate)` instead.

The independent code review (Opus 5.5, high) mechanically confirmed verbatim
moves and minimal visibility, then passed. Its P2 finding widened the virtual-
mutation manifest to all six modules. That fix is `5627c78b`: path-only, and
proven with a mutant.

The pre-existing slice35 Python-audit failure (since `2a65a38a`) is tracked as
`TC-d0e9c5c9-1f4a-4cee-b175-286fd77efc42` (`seq-257`).

## Final receipts and cleanup

Independent verification passed at `5627c78b`, as recorded in
`review-verification.md`:

- public surface equal;
- hidden surface with additive tests only;
- 127/127 registered suites;
- 0 security violations; and
- workspace Clippy and check clean.

AC-037's live network-namespace layer is **unavailable** on this host
(AppArmor restricts unprivileged user namespaces). It is carried to
Slice 150, not counted as a pass.

This slice used the existing durable release worktree. It created no
temporary branch or worktree to merge or remove. All temporary environments,
captures, caches, and receipts were removed. Tags, registries, and publication
remain unauthorized. Slice 70 is next and is uncommissioned.
