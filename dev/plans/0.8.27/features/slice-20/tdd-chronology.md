---
title: FathomDB 0.8.27 Slice 20 - TDD chronology
status: RED
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
