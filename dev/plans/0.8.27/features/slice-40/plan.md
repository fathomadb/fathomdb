---
title: FathomDB 0.8.27 Slice 40 - engine foundation and test seams
status: APPROVED_FOR_IMPLEMENTATION
target_release: 0.8.27
baseline_entry_sha: 5f5c1798a3cffc1416467fd707587954ea75d9c6
---

# Slice 40 plan

## Draft reconciliation

The master-plan draft is approved after narrowing it to four mechanical
foundation extractions. The following changes since the prework plan was
written are incorporated:

1. Slice 20 changed only correction-safe erasure ordering in `Engine::purge`
   and `Engine::excise_source_inner`, and added the locked
   `correction_safe_erasure.rs` matrix. That implementation remains in
   `lib.rs` for Slice 50 and is not refactored here.
2. Slice 30 added the reviewed 13-row real-surface comparator, immutable
   baseline, and exact default/operator/test-hooks feature rows. Slice 40
   creates a clean committed candidate capture after each batch and compares
   it to that immutable baseline; it never recaptures the baseline.
3. Whole-work review hardened Python and NAPI artifact provenance. Those
   binding/build paths are outside this engine-only slice.
4. Current code shows two different hook classes: feature-gated private seams
   and intentionally always-compiled engine test seams that remain excluded
   from the governed facade. The draft sentence “default builds exclude test
   hooks” is therefore adjusted to preserve every current gate exactly and to
   prove that feature-only hooks do not leak into default rows. Slice 40 does
   not silently change hook availability.
5. Existing semantic modules remain the destinations for later domain slices.
   Slice 40 creates only private `errors`, `identity`, `temporal`, and
   `test_hooks` foundation modules; it does not create a parallel subsystem or
   move erasure, dependency, provenance, search, projection, or runtime logic.

Assigned symbols are the shared `EngineError`/`EngineOpenError` contracts and
supporting corruption types; `IdSpaceKind`, `IdSpace`, source/revision/
dependency identity newtypes, `CanonicalHash`, and their shared validation or
derivation helpers; `ReadView`, `BoundaryCrossing`, the frozen-view clock/SQL
and timestamp-normalization primitives; and the existing root hook/rendezvous
and projection-pause support. `Engine` remains at crate root.

## Needs, requirements, and acceptance

This slice consumes release need N27-02 and requirements R27-04/R27-05. It
adds no global acceptance IDs and leaves `dev/acceptance.md` unchanged.

| ID | Slice requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-40A | Foundation code is owned by private semantic modules without changing the crate topology or root API. | AC27-40A: `Engine` remains defined at crate root; the four modules are private; the reviewed surface comparator reports equal for default, operator, test-hooks, and operator+test-hooks rows. |
| R27-40B | Error, identity, and temporal behavior remains byte/variant/order equivalent. | AC27-40B: focused error conversion/source tests, IdSpace/SourceId normalization and invalid-input tests, and half-open temporal/clock tests pass unchanged before and after each owning move. |
| R27-40C | Hook extraction preserves existing feature gates and one-shot/order semantics. | AC27-40C: default and feature checks compile; existing hook suites pass on their declared features; comparator rows show no added/removed/gate-changed symbol. |
| R27-40D | The slice does not absorb later domains. | AC27-40D: the Slice 20 erasure implementation and locked correction matrix are unchanged; no schema, dependency, binding, package, interface, or governed-surface file changes. |

The 300-1,200-line guidance applies to source moved per batch, not final file
size or a release gate. No line-count target is acceptance evidence.

## Implementation and TDD sequence

Each batch follows characterization -> sensitivity proof -> mechanical move ->
GREEN -> refactor. Existing behavioral assertions remain unchanged.

1. Record a clean pre-move comparator and run focused error, identity,
   temporal, and hook suites. Add a layout-independent foundation contract
   test only where the current suites do not directly pin a required behavior;
   prove any new characterization non-vacuous with a temporary mutant that is
   reverted before production edits.
2. Extract `EngineError`, `EngineOpenError`, their supporting corruption types,
   conversions, `Display`, `Error`, and stable-code mapping to private
   `errors.rs`; keep root re-exports and all cfg arms exact. Leave
   `EmbedderChoice` and runtime configuration ownership in `lib.rs`. Run
   error/lifecycle observability tests, commit the clean batch, capture that
   exact HEAD, and compare it to the immutable Slice 30 baseline.
3. Extract shared ID-space, source, revision, dependency, and canonical-hash
   identity types plus caller validation and stable/logical ID derivation to
   private `identity.rs`; keep root paths and internal ownership exact. Domain
   request DTOs and operations remain for Slice 50. Run identity and source-
   validation tests, commit the clean batch, capture that exact HEAD, and
   compare it to the immutable baseline.
4. Extract `ReadView`, `BoundaryCrossing`, `FrozenView`, the clock meter,
   shared node/edge temporal SQL primitives, strict ISO-8601/epoch conversion,
   renderability bounds, and extractor timestamp normalization to private
   `temporal.rs`; retain half-open interval and one-clock-read invariants. Run
   temporal/search validity tests, commit the clean batch, capture that exact
   HEAD, and compare it to the immutable baseline.
5. Extract existing rendezvous/hook and projection-transaction pause support
   to private `test_hooks.rs`, re-exporting only the same root symbols under
   the same cfgs.
   Run the enumerated default/feature checks below, commit the clean batch,
   capture that exact HEAD, and compare it to the immutable baseline.
6. After resolving `git rev-parse HEAD`, candidate capture passes that full
   40-character lowercase SHA to `surface_comparator.py capture --source-sha`
   and writes to a disposable `/tmp/fathomdb-0.8.27-slice40-<batch>.json` only
   after `git status --porcelain` is empty. Comparison uses the tracked
   `slice-30/baseline.json`; both metadata and row diffs must be empty. Capture
   outputs are evidence receipts, not tracked baselines.
7. Run formatting, focused blast-radius tests, and `agent-verify`. Run
   full-workspace clippy/check because the move spans the central engine crate.

Mechanical relocation may produce a compile-RED while imports and
parent/child visibility are rewired; that failure is recorded, not hidden by
changing tests. Any newly discovered semantic defect stops the move and begins
a separate genuine RED/GREEN correction rather than being folded into the
refactor.

Affected hook verification is deliberately bounded:

- default: `slice50_evidence`, `slice35_frozen_read_races`,
  `slice35_after_validation_races`, `slice45_pagination`,
  `slice15e_prekn_filterable`, and
  `slice55_explanation_hook_surface`;
- `--features test-hooks`: `slice20_graph_evidence`, `slice55_explanation`,
  `slice60_fix2_hooks`, `slice40_projection_generation_races`, and
  `slice40_projection_completion`;
- `cargo check -p fathomdb-engine --features slice72-test-hooks` plus
  `cargo test -p fathomdb-engine --lib --features slice72-test-hooks
  contract_fixture_records_actual_forward_overlap`; and
- `cargo check -p fathomdb-engine --features operator,test-hooks`.

The comparator independently checks default, operator, test-hooks, and
operator+test-hooks public rows. Unrelated required-feature targets are not
pulled into each batch merely because they share a Cargo manifest.

## Review, verification, and closeout

- Independent design review must pass before production edits.
- Independent code review examines the complete implementation diff and the
  comparator receipt.
- An independent verifier reruns focused contracts and proportionate gates
  from the final candidate.
- `status.md` records exact commits, comparator hashes/commands, test counts,
  review verdicts, cleanup, and Slice 50 as next only after all findings close.
- This work uses the existing durable `release/0.8.27` worktree and branch; it
  creates no temporary branch/worktree to merge or remove.
