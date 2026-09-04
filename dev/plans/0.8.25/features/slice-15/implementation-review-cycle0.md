---
title: 0.8.25 Slice 15 implementation review — cycle 0
status: COMPLETE
review_cycle: 0
reviewed_on: 2026-09-03
reviewed_commit: 591f7c0d4534f15f31c00e53bc0ff64cd284b041
verdict: FAIL
---

# Slice 15 independent implementation review — cycle 0

## Verdict

**FAIL.** One P1, three P2, and one P3 finding remain.

## Findings

| ID | Severity | Finding | Required correction |
| --- | --- | --- | --- |
| I15-01 | P1 | Versioned nodes participate in separately committed late vector-kind enrollment before database-dependent provenance validation. A failing new-kind write can therefore leave `_fathomdb_vector_kinds` or re-enqueue state behind even though its canonical transaction rolls back. | Add an active-vector/runtime RED fixture that proves canonical rows, provenance registries, vector-kind registry, projection terminals, and queued work are unchanged. Validate provenance before separately committed enrollment or otherwise make the unit truly atomic. |
| I15-02 | P2 | Python and N-API parse shared IDs before dispatching on `role`, so an unknown role with missing or malformed IDs can report an ID error instead of the required `role_invalid` at `/provenance/role`. | Validate role immediately after schema version and before shared or role-dependent fields. Pin unknown-role precedence with omitted and malformed fields. |
| I15-03 | P2 | N-API routes embedded-NUL provenance identifiers through generic FFI validation, producing `FDB_WRITE_VALIDATION`; Python correctly returns the provenance family with a field-specific reason and pointer. | Map provenance-field FFI failures to `FDB_PROVENANCE` with the correct ID reason and canonical pointer, and test Python/TypeScript parity. |
| I15-04 | P2 | The implementation acceptance matrix is materially smaller than design v5, and the checked-in shared fixture is incomplete and unused. | Cover collisions and source-version scope, supersession/rebuild, legacy upgrade, null/empty hashing, erasure/refusal/orphans, persisted corruption, reason/path parity, migration idempotency, and packaged routes. Complete and consume the shared snake/camel fixture from both SDK suites, including roles, locator variants, omission, and errors. |
| I15-05 | P3 | Dynamic unknown-field tokens are interpolated into JSON pointers without RFC 6901 escaping. | Escape `~` as `~0` and `/` as `~1`; test both characters in unknown keys. |

## Review verification

The reviewer verified exact clean commit `591f7c0d`; focused engine, schema,
and facade tests passed; the full non-ignored engine suite passed; workspace
Cargo check with all targets, warnings-denied Clippy, and migration lint passed.
The reviewer could not independently run Python because the review worktree had
no `.venv`, or TypeScript because N-API dependencies were absent. The
implementer's `103/103` fast-suite result remains the binding evidence for
cycle 0.
