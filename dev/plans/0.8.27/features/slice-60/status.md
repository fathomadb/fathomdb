---
title: FathomDB 0.8.27 Slice 60 - implementation status
status: COMPLETE
implemented_on: 2026-09-25
corrected_on: 2026-09-26
planning_commit: 3d5b7c45
implementation_candidate: d5a5bd39b3ee8a04bd080df451204564c6849bd1
closeout_commit: b6677f251f2ba1f0f51602fdf8b7bf35fc24fd13
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

## Original closeout receipts and cleanup

Independent verification passed at `5627c78b`, as recorded in
`review-verification.md`:

- public surface equal;
- hidden surface with additive tests only;
- 127/127 registered suites;
- 0 security violations; and
- workspace Clippy and check clean.

On the 2026-09-25 original closeout executor, AC-037's live network-namespace
layer was **unavailable** because AppArmor restricted unprivileged user
namespaces. That historical receipt did not count the layer as a pass.

This slice used the existing durable release worktree. It created no
temporary branch or worktree to merge or remove. All temporary environments,
captures, caches, and receipts were removed. Tags, registries, and publication
remain unauthorized. Slice 70 is next and is uncommissioned.

## Post-closeout adversarial review

A requested post-closeout adversarial review on 2026-09-25 found that the
original closeout did not exercise either real writer `tx.commit()` exit,
despite AC27-60B claiming commit-boundary coverage. It also reconciled the
approved ownership inventory with AC-050a's ban on visible legacy-name seams
and removed a broken rustdoc link introduced by the move.

The requirements and design correction is `1e3d275c`. Test FIX-1 is
`100fa230`; it adds a private `test-hooks`-only, one-shot SQLite commit-refusal
seam and cases for both trigger-suppressed and row-trigger commit paths. The
seam introduces no public or hidden API path when the feature is disabled.

The post-closeout review completed within every limit:

- design: Cycle 1 FAIL, FIX-1; Cycle 2 FAIL, FIX-2; Cycle 3 PASS;
- tests: Cycle 1 FAIL, FIX-1; Cycle 2 PASS; and
- code: Cycle 1 PASS, with no code fix required.

The first post-closeout receipts were 13/13 debug and 12/12 release boundary
cases with `test-hooks`, identical 58-warning baseline and candidate rustdoc
sets, workspace formatting/lint/Clippy/check PASS, and 122/122 fast-verifier
suites passed with no skips or exclusions. The complete cycle-by-cycle audit
is in `adversarial-review.md`.

## Owner follow-up correction

Owner review on 2026-09-26 found that the release state still named the
pre-review candidate, the corrected plan's final surface comparisons had not
been rerun, the security prose combined incompatible receipts, and the
test-only abort seam could carry into a later write.

RED commit `81d723b1` adds two cases. One arms the marker, receives a
validation refusal, and requires the next valid write to succeed. The other
causes a deferred foreign-key commit failure before the abort hook can fire,
then requires the following operation and write to succeed. Both failed
against `100fa230`.

GREEN commit `d5a5bd39` consumes the marker when a non-empty write acquires its
connection and scopes the SQLite commit hook around exactly one transaction,
explicitly removing it after every commit attempt. The complete boundary
suite passed 15/15 in debug and 14/14 in release with `test-hooks`.

Final candidate evidence at `d5a5bd39` is measured, not inferred:

- the 13-row public surface compares equal to the immutable Slice 30 baseline,
  with empty metadata and row diffs;
- all eight hidden structural rows and the 41-item release probe compare equal;
- test inventory has zero removals and zero changes, with only reviewed
  additions; the two new tests appear in exactly the three applicable
  `test-hooks` inventories;
- target coverage is complete at 261 targets, including 53 feature-complete-
  only targets;
- the full canonical verifier passed 127/127 suites with no skips or
  exclusions;
- on the named 2026-09-26 unconfined executor, AC-037's live layer ran and
  passed; security was 0 violations, 0 blockers, and 0 downgrades; and
- full-workspace Clippy with warnings denied, Cargo all-target check, and the
  candidate-bound Python native receipt passed.

These final receipts supersede the candidate-specific and security conclusions
of both earlier closeouts. Tags, registries, and publication remain
unauthorized; Slice 70 remains uncommissioned.
