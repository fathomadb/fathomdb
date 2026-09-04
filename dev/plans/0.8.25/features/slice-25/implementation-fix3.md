---
title: 0.8.25 Slice 25 implementation FIX-3 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 3
reviewed_commit: 6e9b2ba2
---

# Slice 25 implementation FIX-3 response

Preserved replay RED commit: `39070d9e`.

## Corrections

- Replace permissive receipt-path checks with one exact, closed grammar that
  admits every mapper output and rejects near misses. Remove the un-emitted
  lifecycle `logicalId` path.
- Prove exact replay for nested provenance and missing-dependency refusals and
  cover every refusal reason/path rule with admitted and rejected tables.
- Add bounded property tests for persisted receipt round-trip, replay
  equivalence, digest ordering, and committed collection formulas.
- Expand injected-operation rollback checks across canonical, provenance,
  dependency, lifecycle, projection, FTS, receipt, cursor, and generation
  state.
- Add exact maximum/one-over checks for affected revisions and pending
  projection cursors, including request-membership validation for each pending
  cursor.
- Add embedded-NUL source identity cases in Rust, Python, and TypeScript.
- Reconcile REQ-064 and SDK validation with the accepted `SourceId` contract:
  content/control strings still reject NUL, while the closed source-identity
  field preserves every Engine-valid value through ordinary write and
  actuation.
- Add raw database/WAL erasure canaries and prove the bounded receipt-redaction
  lookup uses the reverse index.

The Python package check must be performed from a freshly built isolated wheel;
the worktree's native-module symlink is not an accepted verification source.
