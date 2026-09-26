---
title: FathomDB 0.8.27 Slice 70 - TDD chronology
status: COMPLETE
target_release: 0.8.27
---

# Slice 70 TDD chronology

## Pre-move receipt (`a95b5b0f`, docs-only over `6f3b625e`)

| Capture | Result | SHA-256 |
| --- | --- | --- |
| Slice 30 public surface | Equal to the immutable `slice-30/baseline.json`; 13 rows; empty metadata and row diffs | `96663960c0bfdb036d579e6f84c76ed624e0c13707b27111afca803925fe9fd4` |
| Hidden surface | Against `baseline-8e2afb29.json`: empty metadata diff; 0 changed and 0 removed in every row; additive only (the reviewed Slice 50/60 tests and the `write_boundary_atomicity` target) | `cbaf0853bceece74ee9d822233846ea30ba984fcdb3f66184dadfaca74a9788f` |

Captures used Node `v25.9.0`, as the comparator pins. The public capture first
refused to run with 92 GB free (its floor is 100 GB). Removing the
worktree's own regenerable `target/debug/incremental` cache (17 GB) allowed
the capture to run. Post-move hidden captures compare against this capture,
so only Slice 70 changes appear.

## Characterization

1. **slice35 audit inventory (seq 257), `e1a71a88`.**
   - **Before:** `tests/experiments/test_slice35_virtual_mutation_audit.py`
     was red at the baseline. It observed `commit_projection_outcomes`
     `INSERT INTO vector_default` ×2 where the inventory expected
     `INSERT OR IGNORE INTO`.
   - **Cause:** the inventory predates the deliberate 0.8.25 Slice 40 change
     (`2a65a38a`).
   - **Change:** both entries were corrected, and the stale
     `delete_vector_partition_row` helper-caller entry for
     `commit_projection_outcomes` was removed.
   - **After:** 6 passed. No production code changed.
2. **Completion classifier: no test added.**
   - **Draft:** an exhaustive `classify_completion` unit table was drafted
     and ran green on production.
   - **Duplicate found:** the crate already holds
     `projection_generation::tests::completion_classifier_is_closed_over_every_persisted_shape`.
     It enumerates the same domain with the same oracle, plus a foreign
     terminal. The pre-commission inventory had searched only `tests/`.
   - **Outcome:** the draft was discarded, and the existing test is the
     R27-70D owner.
3. **Zero residue after a failed projection commit:
   `tests/slice70_projection_commit_residue.rs`.**
   - **Scope:** two arms, gated `#![cfg(debug_assertions)]`.
   - **Against unmodified production:** 2 passed.
   - **Mutant:** the forced-failure block in `commit_projection_outcomes`
     was moved after `tx.commit()`. Both arms failed:
     - the success arm saw `(terminal, sidecar, vec0, audit) = (1, 1, 1, 0)`;
     - the failed-outcome arm saw `(1, 0, 0, 1)`.
   - **Revert:** the mutant was reverted, and `lib.rs` was byte-compared
     with the saved original.
   - **Hang found and fixed:** the first mutant run hung rather than failed.
     The residue assertion panicked while the worker was paused, and
     `Engine` drop then waited on the paused worker. The test now releases
     the pause before asserting, so a regression fails in about 0.06 s. The
     hung run's revert ran and was verified clean.
   - **Other checks:** crate Clippy with warnings denied, `cargo fmt`, and
     the test-target coverage gate pass.

## Structural move

The ten batches ran in the release worktree from `4df75e07`, where `lib.rs`
held 25,899 lines. Each batch moved items mechanically by span. Moved items
went to the end of the destination module in their original order, and moved
`Engine` methods went into one `impl Engine` block per module. Each new module
is private and starts with `use super::*;`, the Slice 60 pattern. The root
imports only the moved items that the root or a sibling still names, with a
plain `use`, so an import that falls out of use fails clippy as unused.

Visibility was widened only where the compiler required it. Moved items that
a later batch made unnecessary were narrowed again, so the final state is
minimal. A checker compared every top-level chunk and every moved `Engine`
method against `lib.rs` at `4df75e07`, ignoring added `pub(crate)` prefixes.
All 239 chunks checked matched (the count includes the existing
`projection_generation.rs` items, checked against that file), with two
classes of exception:

- **Formatting only (14 chunks).** Where an added `pub(crate)` pushed a line
  past the width limit, `cargo fmt` re-wrapped a signature or field. The
  checker confirmed these match once whitespace and trailing commas are
  ignored.
- **Doc link (1 chunk).** `read_projection_status` carries the link repair
  described below.

### Batches

| Batch | Commit | Module | Moved lines | `lib.rs` lines after |
| --- | --- | --- | --- | --- |
| 1 | `ca9ccdb4` | `vector_storage.rs` | 596 | 25,288 |
| 2 | `e8638910` | `vector_equivalence.rs` | 653 | 24,619 |
| 3 | `7d80f790` | `mean.rs`, `embedding.rs` | 487 (369 + 118) | 24,123 |
| 4 | `587599fe` | `projection_registry.rs` (registry A) | 664 | 23,442 |
| 5 | `9e9a0411` | `projection_registry.rs` (registry B) | 679 | 22,750 |
| 6 | `cdf10a2d` | `projection_registry.rs` (registry C) | 636 | 22,104 |
| 7 | `680d70db` | `projection_runtime.rs` | 865 | 21,233 |
| 8 | `fb5f2f37` | `projection_worker.rs` | 818 | 20,405 |
| 9 | `fafe6398` | `projection_worker.rs`, `projection_commit.rs` | 773 (350 + 423) | 19,622 |
| 10 | `6f2012c8` | `rerank.rs`, `projection_rebuild.rs`, `projection_generation.rs` | 614 (332 + 134 + 148) | 19,001 |

The moved items are exactly the by-name inventory in `design.md`.
`PROJECTION_CANDIDATE_PAGE` moved in batch 9 because it lies inside the listed
`pending_edge_projection_from_where` through `pending_embedding_work` range.
The following constants are used only by moved code but are not named in the
inventory, so they stay at root and the modules reach them through `super::*`:

- `VECTOR_EQUIVALENCE_PROBE_FIXTURE`, `VECTOR_EQUIVALENCE_L2_EPSILON`,
  `VECTOR_EQUIVALENCE_P1_FLIP_FLOOR`, `VECTOR_EQUIVALENCE_VERDICT_CACHE_KEY`,
  and `VECTOR_EQUIVALENCE_FINGERPRINT_RECIPE`; and
- `PROJECTION_RUNTIME_STARTUP_TIMEOUT`.

`ProjectionRuntimeShared` keeps its exact field set and types. Only field
visibility changed.

A follow-up commit, `0d2b336e`, repairs intra-doc links to moved items (see
"Doc links").

### Visibility changes

Every change below is `pub(crate)` on an item that was private at
`4df75e07`.

| Module | Items | Fields | Methods |
| --- | --- | --- | --- |
| `vector_storage.rs` | 19: every moved item except `vector_partition_create_sql`, `create_vector_partition`, `desired_vector_attr_columns`, `migrate_vector_partition_pack1_to_pack2`, `migrate_vector_partition_to_pack1`, and `reshape_vector_partition_nondestructive` | none | none |
| `vector_equivalence.rs` | `VectorEquivalenceOutcome`, `usable_dense_runtime`, `run_vector_equivalence_probe` | `VectorEquivalenceOutcome.{dense_disabled, reason}` | none |
| `mean.rs` | `MeanAccumulator`, `run_pin_and_requantize_pass`, `run_requantize_pass`, `recover_mean_vec_pin`, `identity_requires_mean_centering`, `read_pinned_mean_vec`, `subtract_mean` | none | `MeanAccumulator::{new, add, materialize, count}` |
| `embedding.rs` | `embed_with_watchdog`, `embed_batch_with_watchdog`, `map_runtime_embedder_error` | none | none |
| `projection_registry.rs` | 24, listed after this table | `RowOwnedProjection.{table, cursor_column}`; `StoredProjection.{roles, fts_present, fts_tokenizer, vector_declared, vector_embedder, source}` | `StoredProjection::{writes_fts, writes_vector_state, writes_attributes, wants_eav, wants_property_fts, wants_vector, from_spec, to_spec, has_deferred}` |
| `projection_runtime.rs` | `ProjectionJob`, `ProjectionRuntimeState`, `ProjectionRuntimeShared`, `ProjectionRuntime`, `ProjectionRuntimeStartupRole`, `ProjectionRuntimeStartupReport`, `ProjectionRuntimeStartupMessage`, `ProjectionRuntimeStartupFaultForTest` | 45: every `ProjectionJob`, `ProjectionRuntimeState`, and `ProjectionRuntimeShared` field the root and siblings read, plus `ProjectionRuntime.shared` and `ProjectionRuntimeStartupReport.{role, stage}` | 21 `ProjectionRuntime` methods called from the root (`new`, `new_for_test`, `notify_new_work`, `stop`, `set_frozen`, `wait_for_idle`, `wait_for_workers_idle`, and the `*_for_test` seams), plus the test-only `ProjectionRuntimeStartupFaultForTest::targets` |
| `projection_worker.rs` | `projection_dispatcher_loop`, `projection_worker_loop`, `ProjectionOutcome`, `database_has_pending_projection_work`, `connection_has_pending_projection_work`, `pending_embedding_work` | none | none |
| `projection_commit.rs` | `load_projection_cursor`, `store_projection_cursor`, `record_projection_terminal`, `terminal_state_for_cursor`, `advance_projection_cursor`, `commit_projection_outcomes` | none | none |
| `projection_generation.rs` | `derive_dense_readiness` | none | none |
| `rerank.rs`, `projection_rebuild.rs` | none | none | none |

The 24 `projection_registry.rs` items are:

- **Row-owned projections:** `ProjectionClass`, `RowOwnedProjection`,
  `ROW_OWNED_PROJECTIONS`, `erase_row_projections`,
  `purge_row_projections_for_cursor_in`, `truncate_row_projections_in`, and
  `truncate_all_row_projections`.
- **Registry:** `ProjectionPass`, `StoredProjection`,
  `validate_nested_projection_sources_for_write`,
  `validate_nested_projection_sources_for_body`, `load_projection_registry`,
  and `load_projection_registry_row`.
- **Attribute projection:** `encode_attr_vec0_present`,
  `extract_scalar_attribute`, `vector_attr_insert_fragments`, and
  `project_node_attributes`.
- **Vector enrolment:** `reconcile_inert_vector_enrolments_on_boot`,
  `vector_projection_declared`, `register_vector_kind`,
  `unsupported_vector_kinds`, `boot_graft_declared_vector_backfill`,
  `reenqueue_stranded_vector_rows`, and `rederive_projections_on_boot`.

Batch 4 made nine registry helpers `pub(crate)`. Their root callers moved in
batch 5, so batch 5 narrowed them back to private. Batch 6 did the same for
five helpers.

### Non-verbatim edits beyond imports and visibility

- **Module gate.** `projection_rebuild.rs` holds only `operator`-gated methods.
  With the feature off, its `use super::*;` would be an unused import, so the
  declaration is `#[cfg(feature = "operator")] mod projection_rebuild;`, like
  `data_plane_integrity`. Every method keeps its own `operator` gate.
- **`projection_generation.rs` placement.** This module uses explicit imports,
  so its `use super::{..}` list gained the names the moved items need, plus
  `std::sync::atomic::Ordering`. The moved items sit before its
  `#[cfg(test)] mod tests`, so clippy's items-after-test-module lint stays
  clean. The moved methods sit in a second `impl Engine` block.
- **Public re-exports (batch 10).** These are the only public-surface edits:
  - `pub use rerank::rerank_passages;`
  - `#[doc(hidden)] pub use rerank::{rerank_fused, try_rerank_fused};`

  The two functions keep their own `#[doc(hidden)]`. The `#[doc(hidden)]` on
  the re-export follows the existing `temporal::clock_reads_for_test`
  re-export.
- **Operator-only import.** Once batch 6 moved the default-route callers of
  `extract_scalar_attribute`, its only root-path caller was
  `data_plane_integrity` (`crate::extract_scalar_attribute`). Its root import
  is therefore gated `#[cfg(feature = "operator")]`.
- **Unit-test import.** The root `mod tests` imported
  `KIND_TO_SOURCE_TYPE_CASE_SQL` through `use super::{..}`. Only the tests use
  it, so the import now reads `use super::vector_storage::KIND_TO_SOURCE_TYPE_CASE_SQL;`.
  No assertion changed.

### Doc links

`cargo doc -p fathomdb-engine --no-deps --document-private-items` was run with
`-W rustdoc::broken_intra_doc_links -W rustdoc::private_intra_doc_links` on
the default and `operator,default-reranker,test-hooks` routes. The move first
added 11 unresolved links. Commit `0d2b336e` repairs them:

- **Explicit paths.** The three links in the moved `read_projection_status`
  docs now name their `pub(crate)` targets through
  `crate::projection_commit::`.
- **Plain code spans.** Rustdoc cannot resolve a private item in another
  module, so eight links became code spans:
  - in `write.rs`: `enqueue_declared_vector_backfill` and
    `unenrol_registry_vector_node_kinds`;
  - in `lib.rs`: `probe_verification_fingerprint`, `apply_projection_config`,
    `desired_vector_attr_columns`, `ATTR_VEC0_PRESENT_MARKER`, and
    `project_one_attribute`.

  This follows the Slice 60 `ERASURE_AUDIT_COLLECTIONS` precedent.
- **Code-review correction.** `0d2b336e` actually gave the
  `extract_scalar_attribute` link an explicit
  `projection_registry::extract_scalar_attribute` target. On the operator
  route that added a `redundant explicit link target` warning. The code
  review found it, and it is now a plain code span (see "Code review" below).

After that correction, the sorted warning set on both routes equals the
`4df75e07` set minus one entry. That entry is `dense_readiness` linking to the private
`apply_projection_config`, which is now a code span.

### Scraper and gate retargets

These are path-only edits. No needle, count, coupling assertion, audit entry,
or C1 probe semantics changed. `scripts/c1-conformance-pin.json` is
unchanged.

- **Manifest (`tests/slice35_virtual_mutation_manifest.rs`).** `SOURCE`
  gained every new module and `projection_generation.rs`, not only the modules
  that received a counted needle or a `contains_all` target. That keeps the
  pre-move coverage, since all of this code was previously scanned inside
  `lib.rs` (Slice 60 FIX-1 precedent).
  - None of `vector_equivalence.rs`, `embedding.rs`, `projection_runtime.rs`,
    `projection_worker.rs`, `projection_rebuild.rs`, `rerank.rs`, or
    `projection_generation.rs` holds a counted needle.
- **Audit (`experiments/slice35_virtual_mutation_audit.py`).**
  - **Inventory rows.** Moved `PRODUCTION_INVENTORY` rows left the `lib.rs`
    comprehension and went into one comprehension per destination file:
    `vector_storage.rs` (9 rows), `mean.rs` (1), `projection_registry.rs` (4),
    and `projection_commit.rs` (2).
  - **Helper-caller keys.** These were re-keyed for
    `run_pin_and_requantize_pass` (`mean.rs`), `delete_row_owned_projection`,
    `erase_row_projections`, `purge_row_projections_for_cursor_in`, and
    `truncate_all_row_projections` (`projection_registry.rs`), and
    `rebuild_shadow_state` (`projection_rebuild.rs`).
- **Manifest body identity (corrected by code review).** Every batch
  reproduced the empty `bodies-pre.txt` diff of the helper
  `manifest_bodies.py`.
  - **What the helper checks:** it brace-matches each function's own text.
    It proved that all 15 `contains_all` functions moved byte-identically and
    that every needle sits inside its own function.
  - **What it did not check:** the test's own `function_body` extractor.
    Under that extractor, six bodies had widened past their function. The
    next item was now `pub(crate) fn`, which was not an end marker, or the
    body ran across a `concat!` file boundary.
  - **Impact:** no needle fell in a widened tail, so no check was vacuous.
    The earlier claim that "all 15 bodies were byte-identical" was wrong for
    the test's extractor. The extractor was hardened (see "Code review"
    below).
- **C1 gate (`scripts/check-c1-conformance.sh`).**
  - **Path constants.** Each batch that moved an owner first ran the gate RED
    (table below). It then added a path constant (`REGISTRY`, `RUNTIME`,
    `COMMIT`) and retargeted only the probes naming moved items.
  - **Comment.** The source-count comment was updated to "thirteen files
    (nine Rust modules/tests, one markdown plan, the two crate lib.rs, and one
    Cargo manifest)".
  - **Self-test.** `scripts/tests/test_check_c1_conformance.sh` retargeted the
    fixtures that edit moved text: `DECOY_SIG_ROOT`, `OUTSIDE_VERB_ROOT`,
    `RECEIVERLESS_ROOT`, and the receiver-spelling `MIRROR_ROOT` loop to
    `projection_registry.rs`, and `COMMENT_ONLY_ROOT`'s engine edit to
    `projection_commit.rs`. It also gained three `--list-sources`
    expectations.
  - **Result.** The self-test passed after each C1 commit and at the end.
- **Plan anchors (`dev/plans/plan-0.8.20.md`).**
  - **Batch 1.** The `migrate_vector_partition_pack1_to_pack2` and
    `vector_partition_create_sql` citations now name `vector_storage.rs`. The
    "both in `lib.rs`" sentence became two verified citations.
  - **Batch 3.** The `run_pin_and_requantize_pass` citation now names
    `mean.rs`.
  - **Result.** `lint-plan-anchors` reports 13 active plans and 22 citations.

### C1 RED observed before each retarget

| Batch | Move commit | Retarget commit | Failing clauses |
| --- | --- | --- | --- |
| 4 | `587599fe` | `d81d6a8d` | `C1-AA-CRASH-HEAL-BOOT-REDERIVE`: `fn load_projection_registry` has no definition in `lib.rs` |
| 5 | `9e9a0411` | `c7706652` | `C1-SEAM-ENGINE-BUILD-DROP`, `C1-Q3-SOLE-AUTHORITY`, `C1-Q4-CHEAP-SAME-TRANSACTION`, `C1-AA-NO-BLOCK-ON-EMBEDDING`: `configure_projections` and `apply_projection_config` not found in `lib.rs` |
| 7 | `680d70db` | `8c22d44c` | `C1-AA-NO-BLOCK-ON-EMBEDDING`: `fn notify_new_work` has no definition in `lib.rs` |
| 9 | `fafe6398` | `4981add7` | `C1-AA-ATOMIC-FLIP`: `fn commit_projection_outcomes` has no definition in `lib.rs` |

### Per-batch evidence

Before each commit, the batch check ran:

- the ten feature-route `cargo check -p fathomdb-engine --all-targets` runs;
- the release `--tests` check;
- clippy with `-D warnings` on the default and
  `operator,test-hooks,default-reranker` routes;
- `cargo fmt --check`;
- the focused owner routes, serially;
- the lib tests;
- the slice35 audit test;
- the manifest body-identity check;
- the C1 gate; and
- `lint-plan-anchors`.

Every batch reproduced the `4df75e07` counts exactly:

| Route | Passed |
| --- | --- |
| default owners (29 targets) | 186 |
| `test-hooks,migration-test-hooks` | 31 |
| `operator` | 20 |
| `default-reranker` | 19 |
| `default-embedder` | 1 |
| `--lib` | 78 |
| slice35 audit (pytest) | 6 |

### After batch 10

The following ran at `6f2012c8`, and the doc-link check at `0d2b336e`:

- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.
- `cargo check --workspace --all-targets`: PASS.
- `bash scripts/tests/test_check_c1_conformance.sh`: all tests passed.
- **AC-050c:** `scripts/security/check-removal-changelog.sh --base 4df75e07
  --head HEAD` exited 0 with no output, which means zero removed public
  symbols, including the three reranker functions now behind root
  `pub use`.
- **AC-050a:** `scripts/security/ast_scan.py --language rust` reported the
  Rust surface clean.

## Post-move receipt (`b0289e04`, main thread)

| Capture | Result | SHA-256 |
| --- | --- | --- |
| Slice 30 public surface | Equal to the immutable `slice-30/baseline.json`; 13 rows; empty metadata and row diffs. This includes the three reranker functions, which now sit behind root `pub use`. | `c38153c973264897d2ca30e3b82fc1727753fc941f80f4d35587919069ee0d35` |
| Hidden surface | Against the pre-move `a95b5b0f` capture: empty metadata diff. The only change is additive: the `slice70_projection_commit_residue` target and its 2 tests, in every row. No item row changed or was removed. The redundant `#[doc(hidden)]` on the reranker re-export therefore changed nothing. | `3ffedba2928afd6c4245b38af18cca6908661c2cf51eb945eb6b0e6cd9e3cff9` |

The public capture refused to run again at 90 GB free. The move's builds had
regrown the worktree's `target/debug/incremental` cache (15 GB), so it was
removed before the capture.

## Code review (Opus 5.5, high) — PASS-WITH-FIXES at `b0289e04`

The review found no P1 and no semantic change. It verified the following:

- **Verbatim move:** every moved item is a contiguous substring of the
  baseline after normalizing visibility, whitespace, and link text.
- **Attributes:** every item keeps an identical attribute set.
- **Singleton:** the only `static` (`reranker_singleton`'s function-local
  `OnceLock`) keeps a single instance.
- **Manifest needles:** needle counts are unchanged.
- **C1 gate:** 26/26 clauses, with the negative arms still able to fire.

Its findings were closed as follows:

1. **P2: the chronology's manifest body-identity claim was wrong.** The
   claim was corrected above. `function_body` now also ends at `pub(crate)
   fn` at both indents. A sentinel `fn` follows every `concat!` file, so a
   body can no longer cross a file boundary. Under the hardened extractor,
   13 of the 15 bodies equal the `4df75e07` bodies byte for byte:
   - `commit_projection_outcomes` now ends at its own closing brace
     (tighter);
   - `migrate_vector_partition_pack1_to_pack2` gains the 11 characters of the
     next item's `pub(crate)` prefix, which is inter-item text only.

   Needles and counts are unchanged, and the manifest test passes.
2. **P2: the post-batch-10 captures were missing.** They were run and
   recorded above.
3. **P3: the redundant explicit link target.** It is now a code span.
4. **P3: the residue test could still hang.**
   - `residue()` now returns a `Result`, and `release.wait()` runs before
     unwrapping.
   - The report wait now fails after 60 s instead of hanging.
   - The mutant was re-run. Both arms fail cleanly, with `(1, 1, 1, 0)` and
     `(1, 0, 0, 1)`, in 0.07 s. The production file was restored.
5. **P3: the redundant `#[doc(hidden)]` on the reranker re-export.** It is
   kept, because the hidden capture shows no change.

All fixes touch only one doc comment and two test files. They were verified
by the manifest and residue suites, crate Clippy with warnings denied, and
`cargo fmt`. A full regression rerun was not warranted.
