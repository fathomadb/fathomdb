---
title: 0.8.25 Slice 25 implementation FIX-4 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 4
reviewed_commit: a1ce8f54
---

# Slice 25 implementation FIX-4 response

Preserved RED commit: `d2f965e5`.

## Corrections

- Add a path-aware TypeScript actuation guard before N-API conversion. It
  rejects every unpaired surrogate, allows embedded NUL only for nested
  `record.sourceId`, and reports the canonical JSON pointer through
  `ActuationError(nested_request_invalid)`.
- Bind keyed receipt replay to the exact normalized request: verify the stored
  operation count and require every pending projection cursor to identify a
  revision created by a put in that request, not merely an affected prior
  revision.
- Enforce the exact eight-source-reference-per-operation formula on both
  persistence and keyed replay while retaining the absolute 1,024 bound.
- Expand injected rollback snapshots to the omitted property-search,
  projection registry/state, and vector registry/row tables.
- Add at-rest database/WAL canaries for lifecycle-target purge and refused
  multi-source redaction, both verified after restart, while retaining the
  ordinary source-erasure and reverse-index-plan checks.
