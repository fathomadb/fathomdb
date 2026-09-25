---
title: FathomDB 0.8.27 Slice 50 - engine erasure and dependency domains
status: APPROVED
target_release: 0.8.27
baseline_sha: 8e180a68c7cdd00a1c2c5b9b60a29d85a12a5481
---

# Slice 50 plan

Independent design review passed after four correction rounds. The durable
record is `design-review.md`.

## Outcome

Close the one explicit Slice 20 erasure carryover, then move the shipped
record-lifecycle, provenance-contract, dependency-registration, and erasure
implementation out of the engine root into private semantic modules without
changing any other behavior, schema, public path, feature gate, transaction,
lock, or physical deletion ordering.

The behavioral delta is limited to removing nonterminal nonphysical closure
rows whose source revision is physically erased. The remaining work is a
behavior-preserving structural slice. It does not redesign other corrected
Slice 20 behavior or implement deferred multi-source, lease, cursor, or
persisted-evidence work.

## Reconciliation since the draft

The master Slice 50 paragraph was written in prework at `a3e6cff6`. The
executable baseline is now `8e180a68`; the following changes are material:

1. Slice 20 made correction/supersession erasure safe in both
   `Engine::erase_source` and `Engine::purge`, then review added bare-
   supersession, every-erased-revision closure cleanup, exact retained-proof,
   and closure-reaching binding coverage. The locked
   `correction_safe_erasure.rs` matrix is the product oracle; its existing
   cases and assertions must not be changed in this slice.
   Its final review also allocated `TC-6acb0013-bba8-4fee-ac18-27c64442908a`
   here: a `proving` or `incomplete` soft closure rooted at an erased revision
   can survive physical erasure. That carryover is accepted as the one narrow
   behavioral correction and extends the locked matrix without weakening its
   existing cases.
2. Slice 30 replaced prose surface assumptions with an immutable 13-row real-
   surface baseline at `def7d894`, candidate-bound artifact receipts, and a
   move-aware removal gate. Slice 50 must compare against that baseline; it
   must not recapture or approve a new public surface.
3. Slice 40 moved shared errors, identities, temporal primitives, and test
   hooks into private modules. `Engine` remains at crate root and the current
   engine root is 32,002 lines. Its explicit allocation leaves dependency and
   provenance request DTOs and operations for Slice 50.
4. Post-Slice-40 review added the hidden-surface oracle, effective-cfg and test-
   inventory rows, release-profile compile probes, and feature-complete/fast-
   tier coverage gates. These are now required structural evidence. They are
   reused, not modified, unless a genuine Slice 50 move exposes a gate defect.
5. The candle-CUDA calibration follow-up changed feature-matrix evidence only.
   It does not move into this slice and does not justify accelerator work.
6. Existing domain modules remain authoritative destinations:
   `dependency_closure.rs`, `dependency_trace.rs`, and public observability
   `lifecycle.rs` are preserved. No parallel closure, trace, or observability
   implementation is created.

Evaluation: approve the draft outcome after adding the explicit Slice 20
carryover, narrowing the ambiguous word "lifecycle" to record lifecycle,
preserving the published observability module, and enumerating the exact root-
owned material below. Reject moving operator diagnostics, write execution,
projection cleanup generally, or read/evidence code merely because it mentions
provenance or lifecycle.

## Assigned inventory and disposition

| Current root-owned material | Slice 50 disposition |
| --- | --- |
| `LifecycleState`, `InitialState`, their conversions, lifecycle-target resolution, and `Engine::transition` | Move to private `record_lifecycle.rs`; keep root re-exports and behavior. |
| Provenance contract types and constructors (`ProvenanceCompleteness`, `SourceLocator`, `WriteProvenanceV1`, provenanced node/edge DTOs, typed provenance error) | Move to private `provenance.rs`; keep `PreparedWrite` and write execution at root for Slice 60. |
| Dependency registration/lookup DTOs and errors; generation, prospective-state, persisted-row validation; `register_source_dependency`, `dependencies_for_source`, and `dependency_for_derived` | Move to private `dependency.rs`; reuse, do not absorb, `dependency_closure.rs` and `dependency_trace.rs`. |
| `ExciseReport`, `ExciseRecordReport`, `purge`, `erase_source`/`excise_source`, record excision, at-rest completion/redaction, and directly owned erasure transaction helpers | Move to private `erasure.rs`; preserve the corrected Slice 20 code and order verbatim before any import/visibility simplification. |
| Projection-row deletion helpers shared with later projection work | Leave at root unless a narrow `pub(crate)` seam is required by erasure; do not reorganize the projection subsystem. |
| `PreparedWrite`, validation/execution, actuation, search/read/evidence, projection runtime, open/runtime, `orphan_provenance`, `trace_source_ref`, integrity and other operator diagnostics | Leave for Slices 60-90. |

`dependency.rs` and `dependency_closure.rs` are bounded peers: closure already
uses dependency generation and persisted-chain validation, while registration
uses closure maintenance and fencing. Preserve that collaboration through
named crate-private seams; do not duplicate either authority or move it back
to the root merely to manufacture a one-way graph.

Existing call-only consumers also follow those helpers to their new owner:
`dependency_trace.rs` reads generation and validates chains, `frozen_read.rs`
captures generation, `evidence.rs` validates chains, and `actuation.rs`
consumes prospective registration/validation. Their behavior and ownership do
not move, but their owning focused suites are part of the dependency batch's
blast radius.

## Requirements and acceptance

The release-local R27-04/R27-05 and AC27-07/08 remain authoritative. Slice 50
adds only the following falsifiable structural criteria:

| ID | Requirement | Acceptance |
| --- | --- | --- |
| R27-50A | Root ownership is reduced along semantic boundaries without adding a public path. | AC27-50A: the four private destination modules own exactly the approved inventory; `Engine`, every public root item, the public `lifecycle` observability path, signatures, feature cfgs, and facade exports compare equal. |
| R27-50B | Correction-safe erasure remains byte-for-byte behavioral authority. | AC27-50B: the locked five-case correction matrix is unchanged and passes; same/cross-bucket order, retained proof, exact reports/survivors, restart and physical absence remain exact. |
| R27-50C | Dependency/lifecycle ordering and atomicity are unchanged. | AC27-50C: drain -> freeze -> mutate -> at-rest ordering, closure admission/proof, already-complete retry, dependency generation, rollback, and exact deletion counts pass their owning suites. |
| R27-50D | The move does not create new semantics or widen internal authority. | AC27-50D: schema stays 34; no SQL, wire, interface, binding, package, or global acceptance change; only the minimum crate-internal visibility needed across sibling modules is introduced. |
| R27-50E | Structural evidence covers normal and hidden Rust surfaces and real tests. | AC27-50E: the immutable real-surface comparison has empty diffs; hidden structural rows and the release probe compare equal, while test-inventory rows permit only exact reviewed additive tests with no removals or changes; fast-tier target coverage remains complete; focused default/operator/test-hook targets pass. |
| R27-50F | The slice closes with independent review and reproducible receipts. | AC27-50F: design review, code review, independent verification, focused tests, repository verification, exact candidate SHA, and cleanup are recorded before state advances. |
| R27-50G | Physical erasure cannot retain nonterminal nonphysical closure identity for an erased source revision. | AC27-50G: for both `erase_source` and `purge`, supported same/cross-bucket dependency shapes with `proving` or `incomplete` `superseded`/`soft_deleted` rows finish with no such row for any erased revision; receipt validation still precedes cleanup, physical proof rows survive, and a refusal rolls back. |

No change to `dev/acceptance.md`, public interfaces, or accepted ADRs is
needed: this slice changes ownership only.

## TDD implementation sequence

Use one writer in the durable release worktree. Read-only reviewers may share
it.

1. Add a bounded property/state-machine regression over supported same/cross-
   bucket dependency shapes, both hard-erasure verbs, and `proving`/
   `incomplete` soft-closure residue. Prove genuine RED against the unmodified
   implementation: successful physical erasure leaves the nonphysical row.
   Include exact physical-proof preservation and rollback controls. Commit or
   stage the failing oracle before production code.
2. GREEN the carryover before moving code. After actuation-receipt validation,
   delete `superseded`/`soft_deleted` closure rows for every erased revision
   regardless of their phase; retain `purged`/`source_erased` proof rows and
   every existing error/transaction precedence. Run the full affected closure,
   erasure, actuation-receipt, and property suites and obtain focused review.
3. Record pre-move focused results and a clean immutable-surface comparison.
   Use existing behavior tests as the before oracle. For any new boundary
   characterization, demonstrate a plausible temporary mutant failure before
   retaining the test; do not test filenames or private module layout.
4. Move record-lifecycle types and transition logic. The move itself is RED
   until imports/visibility and root re-exports compile; restore GREEN without
   changing assertions or public paths. Run lifecycle, existence-axis, facade,
   real-surface, and hidden-surface checks.
5. Move provenance contract types and their pure validation/codec helpers.
   Keep write execution and `PreparedWrite` in place. Run provenance identity,
   mandatory-provenance, dependency, facade, and surface checks.
6. Move dependency contracts, registration/read operations, generation, and
   persisted/prospective validation in bounded sub-batches. Run source-
   dependency properties, dependency lifecycle/closure, correction erasure,
   registration inertness, dependency trace, frozen-read context, evidence,
   actuation, and surface checks after each clean sub-batch.
7. Move erasure coordination and at-rest helpers. Copy the corrected Slice 20
   implementation first, observe compile/test RED from the move, then restore
   GREEN through imports and the narrowest internal seams. Do not refactor SQL
   or reorder calls. Run correction, completeness, drain-ordering, projection-
   registry, source excision, record excision, closure, and rollback owners.
8. Refactor only imports and visibility proven necessary by the four moves.
   Keep behavior assertions unchanged. A discovered semantic defect requires
   a separate RED test and review before its correction.
9. Run final surface comparisons, affected feature routes, focused blast-
   radius suites, `./scripts/agent-verify.sh`, full-workspace Clippy with
   warnings denied, and `cargo check --workspace --all-targets`. Obtain
   independent code review, resolve each finding with RED/GREEN where
   behavioral, and re-review.
10. Use an independent verifier on the final clean candidate. Write TDD,
   review, verification, and status records; update release state from the
   exact candidate/closeout SHAs.

## Non-goals

- no deferred leases, generalized cursors, multi-source dependencies, or
  persisted evidence receipts;
- no binding or SDK refactor;
- no schema, migration, SQL, wire, error, or public-interface change;
- no redesign of closure, trace, lifecycle observability, actuation, or
  projection ownership;
- no accelerator, calibration, packaging, publication, or release-
  qualification work; and
- no full-regression repetition while resolving a narrow review finding unless
  its blast radius makes the broader run necessary.

## Closeout record

`status.md` must name the approved/rejected draft items, exact moved inventory,
RED/GREEN chronology, locked-test status, surface receipts, focused and broad
tests, review verdicts, candidate and closeout SHAs, cleanup, and Slice 60 as
the next dependency. Publication remains unauthorized.
