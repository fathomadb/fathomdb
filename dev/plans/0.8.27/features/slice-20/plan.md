---
title: FathomDB 0.8.27 Slice 20 - correction-safe source erasure plan
status: COMPLETE
target_release: 0.8.27
observed_on: 2026-09-21
---

# Slice 20 - correction-safe source erasure

## Reconciliation and disposition

This plan approves the bounded F27-01 correction after reconciling the draft
against the current `release/0.8.27` branch.

Changes since the 2026-09-02 draft and the prework plan were written:

1. FathomDB 0.8.25 and 0.8.26 landed, moving the executable baseline to schema
   34 with source-dependency closure, actuation receipts, exact erasure proof,
   graph evidence, and fresh-database contracts already shipped.
2. Memex now has an exact failing correction -> erase characterization. Its
   narrow 0.6.0 exemption removed the cutover block but did not remove the
   privacy defect or the 0.8.27 publication block.
3. Prework Slices 0-9 fixed two incorrect draft assumptions: requested-bucket
   `ExciseReport` counts do not absorb closed dependents, and the accepted raw
   non-PII proof identities are exact audit/closure fields rather than zero raw
   occurrences.
4. Slice 10 changed repository truth, dependency tooling, secret-scan
   authority, and release instrumentation only. It made no product, schema,
   public-API, or erasure-path change.
5. Code-grounded review localized a transaction-order defect not known when
   the draft was written: source erasure deletes a completed soft-closure row
   before redacting an actuation receipt that still references that row; the
   receipt validator then fails closed and rolls the transaction back.
6. RED fixture construction confirmed that the shipped one-source provenance
   invariant requires a direct dependent to share its canonical source's
   bucket. Cross-bucket support concerns the old and replacement source
   revisions, not an out-of-bucket dependent; expanding the cursor set is
   rejected as overbuild.

Assigned functions and surfaces are limited to:

- Rust `Engine::erase_source` / shared `excise_source_inner` transaction;
- existing dependency-closure discovery and physical proof helpers;
- unchanged Python `Engine.erase_source` and TypeScript `Engine.eraseSource`
  adapters/error mapping;
- the Memex exact characterization oracle; and
- clarified erasure behavior in maintained interface/operations documents.

Allocated draft items are P27-01, N27-01, R27-01 through R27-03, and AC27-01
through AC27-06. P27-02 through P27-05 stay postponed, P27-06 and structural
work stay with Slices 30-150, and no Slice 10 or publication work is reopened.

Disposition: approve the behavior correction, narrow it to transaction order
inside the existing requested-bucket physical plan, and reject new verbs,
report fields, schema changes, cross-bucket dependent expansion,
recursive/multi-source closure, snapshot/cursor work, private consumer access,
and structural moves.

## Needs, requirements, and acceptance

Need N27-01 remains unchanged: a supported correction/supersession must not
make lawful source deletion impossible.

| ID | Requirement | Slice-complete acceptance |
| --- | --- | --- |
| R27-01 | `erase_source(bucket)` handles superseded revisions and already-complete direct dependency closure in the shipped one-source model. | AC27-01 same-bucket replacement, AC27-02 original bucket erased first, and AC27-03 replacement bucket erased first all pass through the public verb. |
| R27-02 | Erasure remains atomic before commit, truthful after commit, idempotent, and complete at rest. | AC27-04 proves a real pre-commit blocker leaves the exact protected state unchanged and a post-commit telemetry/WAL failure maps to `ErasureIncomplete`, persists retry work, adds no retry audit, and completes on retry. AC27-05 independently reopens and proves exact erased absence, exact accepted proof rows, and exact unrelated survivors. |
| R27-03 | No new public verb, report field, schema migration, or consumer workaround is introduced. | AC27-06 flips the unchanged Memex scenario to exact success; Rust/Python/TypeScript keep requested-bucket report counts and exact typed incomplete mapping. |

The release-local AC27 identifiers remain outside locked `dev/acceptance.md`.
The requirements are complete as written; no additional feature requirement is
needed for the localized defect.

## TDD implementation sequence

RED:

1. Add one table-driven Rust integration suite for same-bucket,
   cross-bucket-original-first, cross-bucket-replacement-first, and
   already-complete closure cases, with unique content sentinels, exact
   requested-bucket counts, exact survivors, and independent reopen/raw-state
   inspection.
2. Add a real pre-commit refusal case and a post-commit incomplete/retry case.
3. Add one Python and one TypeScript corrected-success smoke plus one typed
   incomplete-result smoke per binding.
4. Run the focused tests against the unmodified implementation and retain the
   genuine failure evidence in `tdd-chronology.md` before production changes.

GREEN:

1. Freeze requested-bucket node/edge/projection counts before any delete so
   report counts cannot drift.
2. Validate the existing direct physical-dependent plan; the one-source model
   guarantees those dependents are already in the requested cursor inventory.
3. Validate/redact affected actuation receipts while both completed correction
   closures still exist; delete every obsolete completed soft closure for an
   erased requested source revision afterward in the same transaction, whether
   or not its current dependent plan is nonempty.
4. Erase requested-bucket identities, projections, canonical rows, telemetry
   ids, and proof scope exactly once; preserve the current durable retry
   contract.
5. Update maintained interface/operations prose only to clarify the corrected
   behavior.

REFACTOR:

- keep the change inside the existing erasure transaction; do not perform
  Slice 50's structural move early.

## Review, verification, and closeout

- Independent read-only design review must pass before RED implementation.
- Independent read-only code review must pass after GREEN.
- A separate read-only verification subagent runs focused Rust/Python/TS and
  blast-radius suites, then the repository-required gate if the focused result
  is clean.
- Record exact RED/GREEN commits/commands/exits, review dispositions, and final
  status. Update release state through its JSON writer and generated views.
- The existing release worktree is retained for the release; no temporary
  branch or worktree is created, merged, or removed by this slice.
