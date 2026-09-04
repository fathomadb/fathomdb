---
title: 0.8.25 Slice 25 independent implementation review — cycle 4
status: COMPLETE
review_cycle: 4
reviewed_on: 2026-09-04
reviewed_commit: a1ce8f54
verdict: FAIL
---

# Slice 25 independent implementation review — cycle 4

## Verdict

**FAIL.** The prior replay, refusal-grammar, collection, rollback, erasure,
and cross-SDK corrections pass. Three implementation defects and one
acceptance-coverage gap remain.

## Findings

- TypeScript `Engine.actuate` bypasses the pre-N-API UTF-16 validator. An
  unpaired surrogate in nested `record.sourceId` is committed after lossy
  replacement instead of returning `ActuationError(nested_request_invalid)`
  at the canonical field path.
- A persisted pending cursor is accepted when its revision appears anywhere
  in `affected_revision_ids`; that incorrectly includes a superseded revision
  which the request did not create.
- Persisted source references enforce only the 1,024 global limit, not the
  exact `operations_count * 8` bound.
- The rollback snapshot omitted five promised projection/vector tables, and
  the raw database/WAL erasure matrix did not directly cover lifecycle-target
  purge, refused multi-source receipt redaction, and restart.

Focused Rust/schema tests and TypeScript typecheck passed at the reviewed
commit.
