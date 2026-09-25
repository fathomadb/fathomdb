---
title: FathomDB 0.8.27 Slice 50 - TDD chronology
status: COMPLETE
implemented_on: 2026-09-25
---

# Slice 50 TDD chronology

## Baseline and behavioral RED

The implementation worktree was clean on `release/0.8.27` at
`cc420df3cedeedbebc301693f7b8f5e5ddaf6d8a`. Before edits:

```text
cargo test -p fathomdb-engine --test correction_safe_erasure
```

passed 5 tests.

Commit `c9d73e4e` added a bounded non-vacuous state-machine matrix covering:

- `erase_source` and `purge`;
- supported same-bucket and cross-bucket dependency shapes;
- `superseded` and `soft_deleted` causes;
- proof-invalid `proving` and `incomplete` phases;
- exact physical proof preservation, unrelated-row survival, physical cursor
  absence, and an exact precommit rollback control.

No prior assertion in `correction_safe_erasure.rs` changed. Against the
unmodified implementation, the exact new success oracle failed with:

```text
assertion failed: nonphysical closure retained erased revision identity
left: 1
right: 0
```

The separate exact rollback oracle passed before the fix.

## Behavioral GREEN

Commit `2891228e` changed only the two existing hard-erasure transactions.
After actuation receipt validation, each now deletes
`superseded`/`soft_deleted` closure rows for every erased source revision
without a phase predicate. Physical `purged`/`source_erased` rows remain
excluded. SQL order, transaction boundaries, receipt precedence, rollback,
canonical deletion, proof measurement, commit, cursor, and at-rest ordering
were otherwise unchanged.

```text
cargo fmt --all -- --check
cargo test -p fathomdb-engine --test correction_safe_erasure
```

passed 7 tests, including all five pre-existing cases and both new oracles.

## Structural GREEN

Commit `c675ddf0` mechanically extracted the approved ownership domains:

- private `record_lifecycle.rs`: lifecycle state vocabulary, target
  resolution, and `Engine::transition`;
- private `provenance.rs`: provenance contracts, constructors, and typed
  errors;
- private `dependency.rs`: dependency contracts, prospective/validated
  carriers, generation state, persisted-chain validation, registration, and
  reciprocal lookups; and
- private `erasure.rs`: source/logical/record hard-erasure facades,
  coordination, physical completion, telemetry/WAL redaction support, and
  erasure-owned dependency/artifact cleanup.

`Engine` remains rooted in `lib.rs`. The existing public items are explicitly
re-exported from the root. Cross-module access was limited to crate-private
fields, constructors, generation/validation helpers, and the existing bounded
dependency/closure consumer seams. `PreparedWrite`, write execution,
operator diagnostics, shared projection deletion/registry machinery, and
read/evidence/runtime ownership remain in their later slices.

The first default library check after relocation produced expected privacy and
unresolved-import compile RED. Narrow `pub(crate)` seams restored GREEN; no
item or module was made newly public. The complete correction-safe erasure
suite remained 7/7 green after extraction.

The first all-targets check then exposed one root unit-test caller of
`Engine::complete_erasure_at_rest` (E0624). Commit `e5236c5e` made only that
method crate-private. The unchanged all-targets command passed on rerun.

## Focused verification

The following owner groups passed:

| Route | Result |
| --- | --- |
| lifecycle/existence/dependency-lifecycle and lifecycle reliability/observability | 45 passed, 4 explicitly ignored child/workload entries |
| operator provenance, source-dependency, registration-inertness, closure, and dependency-trace owners | 75 passed |
| actuation, actuation invariants/verification, frozen read, and evidence owners | 50 passed |
| operator correction erasure, erasure completeness/drain/registry, and source excision owners | 32 passed |
| facade re-export/governed/no-recovery controls | 5 default and 6 operator passed |

Exact feature checks all passed with `--all-targets` for the engine crate:

```text
cargo check -p fathomdb-engine --all-targets
cargo check -p fathomdb-engine --all-targets --features operator
cargo check -p fathomdb-engine --all-targets --features test-hooks
cargo check -p fathomdb-engine --all-targets --features slice72-test-hooks
cargo check -p fathomdb-engine --all-targets --features migration-test-hooks
cargo check -p fathomdb-engine --all-targets --features tc5-benchmark
```

The default command's first run supplied the E0624 RED above; its post-fix
rerun passed. A first combined provenance test command correctly refused to
run `provenance_mandatory` without its declared `operator` feature; the same
owner set then passed with `--features operator`.

Additional final checks:

```text
cargo fmt --all -- --check
cargo clippy -p fathomdb-engine --all-targets -- -D warnings
python3 scripts/check-test-target-coverage.py
```

passed. Test-target coverage reported 260 targets, with 53 explicitly owned by
the feature-complete route.

## FIX-1 review closure

Commit `80d37a25` closes the two test-adequacy findings without changing
production code. The `proving` arm now places its target closure after 32
schema-valid nonterminal rows, exactly beyond `maintain_before_writer`'s
bounded window. A fixture-only `BEFORE DELETE` trigger rejects any phase other
than `proving` and writes the observed old phase to a witness table. The
postcondition requires exactly one `proving` witness, so the target is proven
non-vacuously to remain `proving` immediately before hard-erasure deletion.

With both production cleanup statements temporarily restricted to
`phase IN ('complete','incomplete')`, the focused command

```text
cargo test -p fathomdb-engine --test correction_safe_erasure \
  nonterminal_soft_closure_state_machine_erases_identity_and_preserves_physical_proof \
  -- --exact
```

failed on the first `proving` case with the exact retained-row oracle:

```text
assertion failed: nonphysical closure retained erased revision identity
left: 1
right: 0
```

The temporary mutant was restored before commit, leaving `erasure.rs`
byte-identical to `8b398a7f`. The rollback snapshot now includes the complete
`_fathomdb_open_state` table, including dependency-generation and closure-
sequence singleton values. This makes a late refusal prove exact rollback of
those counters as well as the previously covered primary planes.

After restoring production:

```text
cargo fmt --all -- --check
cargo test -p fathomdb-engine --test correction_safe_erasure
```

passed all 7 tests with no ignored cases.

## FIX-2 registered-guard closure

The unconfined post-review `agent-verify` exposed a split-owner assumption in
`scripts/tests/test_windows_wal_attribution_ci_job.sh`: its registered suite
reported 306 passed and 2 failed because it still extracted
`complete_erasure_at_rest` from `lib.rs`. The implementation ordering in
`erasure.rs` was already correct; no product Rust changed.

Commit `d89f552c` added RED fixtures for three independent obligations:

- `ENGINE_SOURCE` must still own the WAL-attribution runtime and inline tests;
- a separately injectable `ERASURE_SOURCE` must own
  `complete_erasure_at_rest`; and
- moving the before-observer behind the real checkpoint must fail.

Before the guard understood the new seam, its full recursive run reported 306
passed and 5 failed: the original two extraction failures plus all three new
fixture failures.

Commit `b105a3d8` defaults `ERASURE_SOURCE` to
`fathomdb-engine/src/erasure.rs`, teaches function extraction to accept
crate-visible methods, and routes only the completion-body ownership and
observer/checkpoint assertions through that source. All existing `lib.rs`
markers, inline-test bodies, runtime ownership checks, and mutation tests
remain on `ENGINE_SOURCE`.

Focused GREEN evidence:

```text
WINDOWS_WAL_ATTRIBUTION_FIXTURE=1 \
  bash scripts/tests/test_windows_wal_attribution_ci_job.sh
254 passed, 0 failed

bash scripts/tests/test_windows_wal_attribution_ci_job.sh
313 passed, 0 failed

python3 scripts/tests/test_slice50_hook_inventory.py
PASS test-slice50-hook-inventory

bash -n scripts/tests/test_windows_wal_attribution_ci_job.sh
```

The full count grew by the two explicit owner assertions and three new
load-bearing mutation fixtures. Both source paths are now independently
injectable, and a wrong/missing erasure owner or swapped before-observer /
checkpoint order fails closed.

## Closeout gates

The post-review closeout gates and their exact receipts are recorded in
`review-verification.md`. Independent code review and verification both pass;
the public surface is equal, hidden structural surfaces are equal with only
reviewed additive test inventory, the canonical gate passed 127/127, strict
security is 0/0/0, and workspace Clippy/check pass. No unavailable or skipped
evidence is reported as a pass.
