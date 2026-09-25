---
title: FathomDB 0.8.27 Slice 50 - implementation status
status: COMPLETE
implemented_on: 2026-09-25
planning_commit: cc420df3cedeedbebc301693f7b8f5e5ddaf6d8a
implementation_candidate: 1f5b8614813b5a363ec5f81fcb580d48da4a4e8f
closeout_commit: 4752b86158c61d97644eb714bca3796d83108d81
---

# Slice 50 implementation status

## Completed scope

Slice 50 is complete on `release/0.8.27`.

- `record_lifecycle.rs` owns lifecycle state, target resolution, and record
  transition orchestration.
- `provenance.rs` owns provenance contracts, validation, and typed errors.
- `dependency.rs` owns dependency registration, generation, validation, and
  reciprocal lookups while retaining its bounded peer seam with closure state.
- `erasure.rs` owns hard-erasure facades, physical completion, WAL/telemetry
  redaction, and erasure-only cleanup.

All four modules are private; `Engine` remains at crate root and existing public
items retain their root paths. Write/ingest, projection, read/evidence,
runtime/operator, schemas, bindings, and package exports remain assigned to
later slices.

## Plan reconciliation and requirements

Planning reconciled the draft against the Slice 20 erasure/purge corrections
and carryover, the Slice 30 real-surface comparator, the Slice 40 foundation
extraction, the post-Slice-40 hidden/test inventory gates, and the live assigned
functions and consumers. Five independent design-review cycles corrected
operation-specific ordering, accepted the nonterminal closure carryover,
documented the dependency/closure peer seam, added all existing consumers, and
then returned PASS.

The resulting requirements and acceptance criteria were neither widened into
later domains nor reduced to layout checks. They require behavioral parity,
exact transaction/error ordering, one bounded correction to soft-closure
erasure, preserved public and hidden structural surfaces, reviewed additive
test inventory only, complete fast-tier target ownership, and focused plus
workspace-wide verification.

## TDD and review result

RED `c9d73e4e` proved that `erase_source` and `purge` retained nonterminal
`superseded`/`soft_deleted` closure identity. GREEN `2891228e` removed those
rows inside the existing immediate transactions after receipt validation while
preserving physical proof rows and rollback. Structural extraction followed in
`c675ddf0`; `e5236c5e` exposed only the crate-private erasure completion seam
needed by existing root tests.

Code review required a non-vacuous `proving` witness and a complete rollback
snapshot; `80d37a25` closed both without production changes. Broad verification
then exposed the relocation-sensitive Windows WAL guard. RED `d89f552c` and
GREEN `b105a3d8` made its two physical owners independently injectable and
fail-closed without changing product Rust. Final code review returned PASS.

The carried item `TC-6acb0013-bba8-4fee-ac18-27c64442908a` is closed by ledger
entry `seq-256`.

## Final receipts and cleanup

Independent verification at `1f5b8614813b5a363ec5f81fcb580d48da4a4e8f`
passed the 7-test erasure matrix, 260-target ownership gate, equal public and
hidden structural surfaces, 127/127 registered suites, strict security 0/0/0,
candidate-bound Python receipt, workspace Clippy with warnings denied, and
workspace check. The hidden inventory contained only 10 approved additive
tests, with no removals or changes; no baseline was regenerated.

Temporary environments, receipts, native/generated artifacts, captures, and
caches were removed. This slice used the existing durable release branch and
worktree; it created no branch or worktree to merge or remove. The release
worktree remains for uncommissioned Slice 60. Tags, registries, and publication
remain unauthorized.

## Post-close adversarial review (2026-09-25)

A post-close adversarial review of `8e180a68..aa86a44d` found no P0/P1 issues.
It mechanically confirmed the verbatim move and that the new state-machine
tests fail on the pre-fix code. Findings and dispositions:

- A-1/A-2 (records): the locked-matrix claim now records FIX-1's
  `ROLLBACK_TABLES` strengthening, and the design names only
  `excise_collection_record` as operator-gated (`e5221255`).
- B-1/B-2 (tests): an unrelated nonterminal closure rooted at the surviving
  revision plus a byte-exact before/after oracle now fail the over-deleting
  `root_value=?1 OR 1` mutant at each erasure DELETE; case names label every
  assertion and rollback covers all phase and cause pairs (`ee225b04`).
- C-1 (narrow `ProspectiveCanonicalSource` visibility): refuted. The type is
  returned by `load_persisted_canonical_source`, which `evidence.rs` calls, so
  `pub(crate)` is the minimum; the compiler rejects private.

Each layer closed in one fix cycle with an independent verification pass.
