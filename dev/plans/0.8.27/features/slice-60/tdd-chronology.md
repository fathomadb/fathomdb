---
title: FathomDB 0.8.27 Slice 60 - TDD chronology
status: IMPLEMENTED
implemented_on: 2026-09-25
---

# Slice 60 TDD chronology

## Baseline

The implementation worktree was clean on `release/0.8.27` at `3d5b7c45`.
Production source there is byte-identical to the planning baseline
`517e0545`.

## Characterization

Commit `2300e11b` added `tests/write_boundary_atomicity.rs` and changed no
production source. Its oracle:

- drains the projection worker (`drain` must return `Ok`) before both
  snapshots;
- asserts that no `_fathomdb_dependency_closures` row is `proving` or
  `incomplete` (the closure precondition);
- dumps every `sqlite_master` row and every row of every ordinary table
  (virtual tables excluded, their shadow tables included) through a separate
  connection, using the typed `ValueRef` cell encoding; and
- after a refused `write` or provider call, compares the full snapshot and then
  requires the next valid write to receive `row_cursors == [pre_cursor + 1]`.

The snapshot assertion runs before the error assertion, so a mutant that lets
the call commit is reported as a state difference rather than as an
unexpected `Ok`.

Each fixture seeds two active nodes, an edge with a body, and a canonical
provenanced node, one item per write. It then declares a searchable vector
projection with a live `FixedEmbedder`. Every refused `Engine::write` batch
carries the unenrolled, vector-committable kind `note`. The `pre_tx_hook` case
is `#[cfg(debug_assertions)]`. The `enrolment_raise` and `execution_raise`
cases, and the TEMP-trigger drop helper, are
`#[cfg(any(debug_assertions, feature = "test-hooks"))]`. The property
(32 cases, `failure_persistence: None`) inserts one of five closed structural
faults into 1-6 generated valid nodes and edges.

All 11 tests passed against unmodified production. No documented error
differed from the observed error. The design listed an unrenderable edge epoch
as a candidate structural fault, but that refusal is `InvalidArgument`, not
`WriteValidation`. It was therefore left out of the property's closed set.

### Non-vacuity mutants

Each mutant was applied to `lib.rs` alone. Only the targeted case was run
(`cargo test -p fathomdb-engine --test write_boundary_atomicity <case> --
--exact`), and the file was then restored. Each run exited 101.
`git diff --stat -- src/rust/crates/fathomdb-engine/src` was empty at the test
commit.

| Case | Mutant | Observed failing assertion |
| --- | --- | --- |
| `structural` | `validate_batch` validates `batch[0]` and repeats its plan | `a refused call must leave every durable state plane byte-identical` |
| `db_dependent` | `validate_payload` skipped | same snapshot assertion |
| `enrolment_raise` | `let _ = register_vector_kind(tx, kind);` in apply | same snapshot assertion |
| `pre_tx_hook` | cursor computation and `next_cursor.store` hoisted above the hook | `a refused write must neither publish nor consume a cursor` (`left: [8]`, `right: [6]`) |
| `late_provenance` | `enrol_and_unstrand` committed before `commit_batch` | same snapshot assertion |
| `execution_raise` | row-trigger path applies and commits each item in its own `BEGIN IMMEDIATE` | `projection worker must drain: Scheduler` |
| `visibility_last` | `let _ = advance_read_visibility(&tx);` | `projection worker must drain: Scheduler` |
| `provider_handshake` | sentinel `self.write` before `session.handshake()` | same snapshot assertion |
| `provider_request_id` | sentinel `self.write` before the extract `.request(..)` | same snapshot assertion |
| `consolidate_verdict` | `tx.execute_batch("COMMIT; BEGIN IMMEDIATE")` after each applied verdict | same snapshot assertion |

For `execution_raise` and `visibility_last`, the post-call drain precondition
fires first. The committed rows are left with no worker notification, and in
`visibility_last` the exhausted generation also blocks the worker's own commit.
A diagnostic rerun of those two mutants with a non-fatal drain (not
committed) reached the snapshot assertion and failed on it in both cases.

A first mutant pass used one four-item seed batch. The `structural` mutant
then broke fixture setup (`fixture write: Storage`), and three mutants failed
at `unwrap_err` before the snapshot comparison. Seeding one item per write and
comparing the snapshot before the error produced the table above. Both edits
were made before the test commit.

## Pre-move receipt

At `2300e11b` the focused owners passed. All suites run serially
(`-- --test-threads=1`), matching the canonical gate:

| Route | Result |
| --- | --- |
| 31 default targets: the boundary suite, write, validation, provider, ingest, consolidation, actuation, erasure, `slice20_*`, `slice30_dependency_closure`, and `slice35_virtual_mutation_manifest` owners | 255 passed |
| `--features operator`: `provenance_mandatory` | 2 passed |
| `--features test-hooks`: `slice20_graph_evidence`, `slice20_fts_rank_stream` | 37 passed |
| `--lib` | 78 passed |

A first parallel run of the same default command stopped on
`pr_g0_identity`, with 6 tests failing on `open: RuntimeConfiguration(TooLate)`.
That target races a raw `rusqlite` open against the process-global SQLite
configuration when its tests run in parallel. It passes 8/8 serially at the
same commit, and the canonical gate
(`scripts/test-rust-workspace.sh --serial`) is serial. No source changed.

The Slice 30 public comparison was equal: 13 rows, empty metadata and row
diffs. The hidden comparison against `baseline-8e2afb29.json` had empty
metadata diffs and equal rustdoc and release-probe rows. Its only differences
were additive tests in 15 inventory rows:

- 10 tests approved earlier, as recorded by Slice 50;
- the 11 new `write_boundary_atomicity` tests; and
- the new `fathomdb-engine::write_boundary_atomicity` test target.

## Structural moves

The moves were made mechanically by line span. A checker confirmed that every
top-level chunk, and every method inside a moved `impl Engine` block, occurs
verbatim in `lib.rs` at `2300e11b` once the added visibility prefixes are
stripped. Each move went compile-RED on missing names and returned to GREEN
through visibility and imports alone.

| Commit | Move |
| --- | --- |
| `261e9f54` | `write_validation.rs` (`WritePlan` through `prior_edge_cursors_by_triple`, with the external-ref checks) and `write_commit.rs` (`CommitBatchError` through `apply_batch_in_transaction`, minus `projection_batch_has_no_custom_triggers`, plus `enforce_provenance_retention`) |
| `a838cf4f` | `write.rs`: `WriteReceipt`, `PreparedWrite`, `storage_write_shape`, `batch_is_admin`, and one `impl Engine` block holding `write`, the late-enrolment trio, and `write_inner` |
| `d84afac7` | `provider.rs`, `ingest.rs`, and `consolidation.rs` |
| `45f91b65` | `legacy_revision_id` returned to its original place in `lib.rs` (see the deviation below) |

Public types keep their root paths through `pub use`. Crate-internal seams use
root `pub(crate) use`, the Slice 50 pattern. The only other non-move edit is
an import path: the `evidence.rs` tests now name
`crate::write_commit::CANONICAL_BODY_HASH_CALLS`.

### Deviation: `legacy_revision_id`

The seam table gives `legacy_revision_id` `pub(crate)` in `write_commit.rs`.
That cannot land. AC-050a (`scripts/security/ast_scan.py`, run by
`agent-verify`) forbids any `pub(..)` item named `legacy_*`, and the first
full gate at `5716bea4` failed on it:

```text
src/rust/crates/fathomdb-engine/src/write_commit.rs:68: [rust-public-symbol] forbidden name: legacy_revision_id
```

The helper is `#[allow(dead_code)]`. Its only consumer is one root unit test,
so moving that test would change the test inventory. Commit `45f91b65`
therefore returns the helper verbatim and private to its original position in
`lib.rs`, and restores the root test import to its original form. The helper
calls `revision_hash_field`, which now lives in `write_commit.rs`, so that
function gets `pub(crate)` and a root `pub(crate) use`. That is the one seam
beyond the design's table. Relative to `2300e11b`, `lib.rs` now differs only by
deleted spans plus the added `mod` and `use` lines.

No assertion changed. Step 6 (import and visibility tidy) found nothing to
change: clippy with `-D warnings` was already clean.

### Scraper amendment

These are reviewed, path-only test-infrastructure edits:

- In `261e9f54`, the `tests/slice35_virtual_mutation_manifest.rs` `SOURCE`
  became `concat!` of the `include_str!` of `lib.rs` and `write_commit.rs`.
  `experiments/slice35_virtual_mutation_audit.py` re-keyed
  `apply_batch_in_transaction` to `write_commit.rs`.
- In `d84afac7`, `SOURCE` added `consolidation.rs`. The
  `prune_edge_projection_shadows` tuple left the `lib.rs` comprehension of
  `PRODUCTION_INVENTORY` and became a `consolidation.rs` `MutationSite`. Its
  helper-caller key was re-keyed to match.

Every needle, count, and coupling assertion is byte-identical.

### Unresolved doc links

`cargo doc -p fathomdb-engine --no-deps --document-private-items` was run with
`-W rustdoc::broken_intra_doc_links` at `2300e11b` and at `d84afac7`. It
reported 58 and 59 unresolved-link warnings. The one new warning is
`lib.rs` `ERASURE_AUDIT_COLLECTIONS` (`/// These rows are **exempt from
[`enforce_provenance_retention`]**`), because the helper is now private to
`write_commit.rs`. The doc text was left verbatim. The design's
`erasure.rs:1430` mention produced no warning.

## Per-batch evidence

The owner routes above were rerun serially after each move, with identical
counts (255, 2, 37, 78) and no failures. The five non-default all-targets
feature checks passed before each commit. For the narrow `45f91b65` fix, only
the affected owners were rerun: `write_boundary_atomicity` and
`slice15_identity_provenance` (11 and 11 passed) and the root
`legacy_revision_id` unit test (1 passed). AC-050a then reported the Rust
surface clean.

| Commit | Public capture SHA-256 | Hidden capture SHA-256 |
| --- | --- | --- |
| `2300e11b` | `1108030ba417e909b88d2fd5d3db8b7c1983b872b31e9b1b395d39d0cd1e173d` | `b19641f24466bc9648efb303afc586e98106137b801cd0638ed220cdfb51e7f5` |
| `261e9f54` | `e1f37b8e71b23080bb44772fbf474e5fbedb788a7b3fd46a0f886204cf2857ea` | `27a12a803bb2d67facc8c4c5bbcd1adc09f15cbd24f35de59562fbded8d1e4ec` |
| `a838cf4f` | `7324bec678aa599bd10c56c270fc43f9b0e574674e87224345f730976754f80c` | `046138b302ead4c8fa940b9385d8952c83a0118d7d6b970f1633d34575561451` |
| `d84afac7` | `748e374ad508548e7cbc5151f820c897b16a22d40d0836b1347cf1cee244b837` | `53e4ad9dea1a5df27078eb455ffdca845c24002010b86be309d72dcde4a14d1e` |
| `45f91b65` | `eff4852f286f460180443c396bab32773c334ec88f69ea702703e7368fa2656c` | `f799860b5c870555c357f2eb430b4a155e46e92472374b37b8976431f22e9ae3` |

Every public comparison was equal against the immutable
`slice-30/baseline.json`
(`06212f662b2a3897447fe294d65d87ae93255b9b511d9fdd57eab3cf8490a723`). Every
hidden comparison against `baseline-8e2afb29.json` held the same 22 additive
entries as the pre-move receipt, with no removals or changes. No baseline was
regenerated. Captures used Node `v25.9.0`, as the comparator pins.

## Plan-citation follow-up

The first `./scripts/agent-verify.sh` at `d84afac7` failed at
`lint-plan-anchors`. It stopped because `dev/plans/plan-0.8.20.md` cites
`fn enforce_provenance_retention` and `fn commit_batch` in `lib.rs`.
Commit `5716bea4` changes only those two cited paths, to `write_commit.rs`.
`./scripts/lint-plan-anchors.sh` then reported 13 active plans and 20
citations verified.

## Gate

At the Rust candidate `45f91b65`:

```text
cargo check -p fathomdb-engine --all-targets                                  # pass
cargo check -p fathomdb-engine --all-targets --features operator             # pass
cargo check -p fathomdb-engine --all-targets --features test-hooks           # pass
cargo check -p fathomdb-engine --all-targets --features slice72-test-hooks   # pass
cargo check -p fathomdb-engine --all-targets --features migration-test-hooks # pass
cargo check -p fathomdb-engine --all-targets --features tc5-benchmark        # pass
cargo fmt --all -- --check                                                    # pass
./scripts/agent-typecheck.sh                                                  # pass
python3 scripts/check-test-target-coverage.py      # pass: 261 targets, 53 feature-complete only
PYTHONPATH=. python -m pytest -q tests/experiments/test_slice35_virtual_mutation_audit.py
                                                   # 5 passed, 1 failed (pre-existing)
./scripts/agent-verify.sh                          # pass: 127/127 suites, 0 skipped or excluded
cargo clippy --workspace --all-targets -- -D warnings                         # pass
cargo check --workspace --all-targets                                         # pass
```

`agent-security` reported 0 violations, 0 blockers, and 1 downgrade. The
downgrade is the environmental AC-037 live network-namespace layer:
rootless user namespaces are unavailable in this sandbox. The offline catch ran
and passed, AC-036 passed, and the run was not repeated unconfined.

Between those runs, two `agent-verify` attempts failed only on the
environment:

1. With no checkout `.venv`, the pinned Ruff and Pyright were missing, `pip`
   was externally managed, and the native module was not built.
2. With a disposable checkout-owned `.venv` (Python 3.12 plus the `[dev]` pins,
   nothing editable, no `maturin develop`), 8 Python tests failed. Their child
   processes could not import `fathomdb` from `src/python`.

A `.pth` entry naming `src/python` fixed the second failure; those 24 tests
then passed. The unchanged full gate passed after that. The conftest
candidate-bound receipt names `45f91b65` and native module SHA-256
`b349d7b146a7e349bdb887bb992dce9f4889fcab79d93ad4845eeb5cc9090b49`.

The slice35 audit failure predates this slice. It fails identically at
`2300e11b`, and has since `2a65a38a` changed
`commit_projection_outcomes`. The inventory expects two
`INSERT OR IGNORE INTO vector_default` sites and a
`delete_vector_partition_row` caller in that function. The source now has two
`INSERT INTO vector_default` sites and no such caller. Every entry this slice
re-keyed matches the scan. The stale expectation is outside Slice 60 scope and
was left unchanged.

## Code-review FIX-1

The independent code review (P2 #1) found that the scraper amendment covered
only `lib.rs`, `write_commit.rs`, and `consolidation.rs`. Before the move,
`lib.rs` also held the code that now lives in `write.rs`,
`write_validation.rs`, `provider.rs`, and `ingest.rs`, so the manifest's
unclassified-mutation counts no longer saw it. The fix adds those four files
to the manifest's `SOURCE` `concat!`. It remains a path-only change: no
needle, count, or assertion changed.

`cargo test -p fathomdb-engine --test slice35_virtual_mutation_manifest`
passed 1/1. As a non-vacuity check, appending a `"INSERT INTO search_index(`
literal to `ingest.rs` made the test fail with
`unclassified mutation: "INSERT INTO search_index(` (left 2, right 1). The
temporary mutant was then reverted.

The review's NIT #3 is recorded as a scoped exemption, not a test change. The
validation-before-mutation property may generate an all-edge batch that
carries no late vector kind. It still asserts `WriteValidation` and an
unchanged full snapshot. Late-enrolment rollback is owned by the
`late_provenance` and `enrolment_raise` table cases.

The stale `commit_projection_outcomes` expectation in the slice35 Python audit
predates this slice (`2a65a38a`). It is tracked as
`TC-d0e9c5c9-1f4a-4cee-b175-286fd77efc42` (ledger `seq-257`).

## Post-closeout adversarial Test FIX-1

The post-closeout design correction added two missing real-commit cases. They
are `#[cfg(feature = "test-hooks")]` because that private feature supplies
rusqlite's commit-hook API:

- `trigger_suppressed_commit_refusal_leaves_full_state_unchanged`; and
- `row_trigger_commit_refusal_leaves_full_state_unchanged`.

Each test was added before production changed. Each RED run exited 101 at the
full-snapshot assertion because the write committed. Production then gained a
private TEMP-marker consumer, armed through the existing `execute_for_test`
surface, and one commit helper shared by both `tx.commit()` exits. The marker
is dropped before `BEGIN`, and the installed SQLite commit hook requests
rollback only on its first invocation. No public or hidden item was added.

The complete suite passed 13/13 in debug with `test-hooks` and 12/12 in release
with `test-hooks`; the debug-only pre-transaction hook accounts for the count
difference. For non-vacuity, each commit-helper call was independently changed
to ignore the consumed marker. Its corresponding test exited 101 at the
full-snapshot assertion, and each mutant was reverted.

The moved-symbol doc link at `lib.rs` `ERASURE_AUDIT_COLLECTIONS` is now a
plain code span. With private items documented and broken intra-doc links
enabled, both exact pre-move commit `2300e11b` and the Test FIX-1 tree emitted
58 unresolved-link warnings, with identical sorted warning-message multisets.
The move no longer adds a warning.

## Owner follow-up RED/GREEN

Owner review on 2026-09-26 identified two carry-over paths in the private
`test-hooks` commit-abort seam.

RED commit `81d723b1` added:

- `armed_commit_abort_does_not_survive_validation_refusal`; and
- `commit_error_before_abort_hook_does_not_poison_next_write`.

Against `100fa230`, the first test failed because the marker was consumed only
after validation, so the following cursor probe received `EngineError::Storage`.
The second used a deferred foreign-key violation to make commit fail before
the abort hook fired; the still-armed hook then aborted the next trigger-
cleanup commit.

GREEN commit `d5a5bd39` moves marker consumption to the start of every
non-empty write after connection acquisition. `commit_batch` installs the
connection hook immediately before its one transaction body and removes it
after every commit attempt, whether that attempt succeeds or fails for another
reason. Both new tests then passed, and the complete boundary suite passed
15/15 debug and 14/14 release with `test-hooks`.

Final structural and repository receipts at clean `d5a5bd39`:

- public capture SHA-256
  `dba5b143b362f84887913ad9d51600c0c9d91614b98ef19531cfc8fc43e289ff`,
  equal to the immutable 13-row baseline with empty metadata and row diffs;
- hidden capture SHA-256
  `132c5176e82e590a457716960345ab29a62afdfb2e90f41079ffa54798ec1895`,
  with all eight structural rows and the 41-item release probe equal, zero
  test removals or changes, and reviewed additions only;
- target coverage 261 targets, including 53 feature-complete-only targets;
- full `agent-verify`: 127/127 suites, zero skipped or excluded, with the
  2026-09-26 unconfined executor running and passing live AC-037 at security
  0 violations / 0 blockers / 0 downgrades; and
- workspace Clippy with warnings denied and Cargo all-target check: PASS.
