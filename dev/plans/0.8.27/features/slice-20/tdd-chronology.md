---
title: FathomDB 0.8.27 Slice 20 - TDD chronology
status: GREEN
target_release: 0.8.27
---

# Slice 20 TDD chronology

## Baseline

The four owning pre-existing Rust binaries passed before new tests:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test slice20_dependency_lifecycle \
  --test slice30_dependency_closure \
  --test erasure_completeness \
  --test erasure_drain_ordering

49 focused tests passed; exit 0.
```

## RED

The new Rust integration test compiled against the unmodified production code:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test correction_safe_erasure
```

Result: exit 101; 1 passed, 2 failed.

- `correction_safe_erasure_matrix_preserves_requested_counts_and_exact_survivors`
  failed because `erase_source` returned `Storage` for the supported
  correction scenario.
- `correction_erasure_postcommit_incomplete_retry_is_truthful_and_audit_idempotent`
  failed because the operation returned the pre-commit `Storage` defect rather
  than reaching the injected post-commit `ErasureIncomplete` stage.
- The independent pre-commit proof-failure rollback guard passed.

Fixture development also rejected an overbroad design assumption before any
production edit: a direct dependent with a different source bucket fails the
persisted one-source provenance invariant with `SourceMismatch`. The plan and
design were narrowed so cross-bucket coverage varies original versus
replacement buckets while closed dependents stay in the original bucket.

The failing tests and reviewed plan/design are committed before GREEN. The
production implementation remains unchanged at this point.

## GREEN

The production change preserves completed source-revision closure rows until
actuation-receipt validation and redaction finish, then deletes those closure
rows in the same immediate transaction. No cursor expansion, recursion,
schema change, or public-surface change was added.

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test correction_safe_erasure

3 passed; exit 0.
```

The new binary plus the four owning regression binaries passed together:

```text
cargo test -p fathomdb-engine --features operator,test-hooks \
  --test correction_safe_erasure \
  --test slice20_dependency_lifecycle \
  --test slice30_dependency_closure \
  --test erasure_completeness \
  --test erasure_drain_ordering

52 passed; exit 0.
```

Binding evidence used an isolated Python wheel built from this checkout and a
fresh TypeScript native debug build:

```text
python -m pytest src/python/tests/test_erase_source.py -q
8 passed; exit 0.

npm run build:native:debug && npx tsc -p tsconfig.json && \
  node --test dist/tests/erase-source.test.js
8 passed; exit 0.
```

The isolated Python wheel SHA-256 was
`77066f543faa9b87afddc636ceb92ae4ec149d97c158cf2b370adf5038972efd`.

The unchanged Memex `_closed_world` correction fixture was then run against
that wheel with only the formerly-failing oracle changed from `storage_failed`
to successful erasure. The durable producer-owned oracle also checks empty
visibility, exact requested counts in the audit, raw projection absence,
accepted proof identity, independent reopen, and idempotent retry:

```text
python -m pytest \
  dev/plans/0.8.27/features/slice-20/memex-consumer-oracle.py -q
1 passed; exit 0.
```

## Code-review repair

The first independent code review found no implementation defect but rejected
the regression evidence as insufficiently exact. The Rust matrix was extended
to prove sentinel absence in the database and WAL, every erased cursor absent
from every registered projection owner, exact survivor bytes, exact accepted
audit/source-bucket proof rows, receipt redaction, and an exact before/after
snapshot of all protected database planes plus telemetry on pre-commit
rollback. The temporary downstream smoke was replaced by the durable Memex
oracle named above. The strengthened Rust suite and consumer oracle both pass.

## Adversarial review FIX-1

- RED `409bdb92`: `correction_then_purge_of_corrected_logical_id_succeeds_and_erases_exact_rows`
  failed with `purge after a dependent-bearing correction: Storage`.
- GREEN `aef78ddd`: closure delete in `purge_inner` moved after receipt
  redaction; focused suite 4/4 passed.
