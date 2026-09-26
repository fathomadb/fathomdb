---
title: FathomDB 0.8.27 Slice 70 - TDD chronology
status: IN_PROGRESS
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
