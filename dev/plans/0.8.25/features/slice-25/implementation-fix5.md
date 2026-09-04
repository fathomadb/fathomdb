---
title: 0.8.25 Slice 25 implementation FIX-5 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 5
reviewed_commit: 1d340a6a
---

# Slice 25 implementation FIX-5 response

Preserved precedence RED commit: `955cd6a8`. The release owner explicitly
authorized a sixth review/FIX cycle if needed.

## Corrections

- Replace eager TypeScript rejection with a non-committable surrogate
  sanitization pass. The sanitized request still traverses the native parser,
  preserving schema, unknown-field, required-field, ID, and operation
  precedence while preventing N-API from replacing malformed UTF-16 with a
  committable character.
- Preserve field taxonomy by selecting invalid sentinels for identity fields
  and NUL for non-identity strings. N-API carries the exact nested `sourceId`
  pointer when the source-identity validator is reached; the ordinary write
  path remains unchanged.
- Add direct controls for schema precedence, unknown-field precedence,
  top-level operation-ID taxonomy, and nested source-ID path fidelity.
- Configure real filterable, FTS, and vector projection state in the injected
  rollback fixture. Compare exact property/registry/readiness/vector state
  before and after each fault, then run the same request successfully and prove
  every actuation-applicable projection/vector table becomes nonempty after
  the readiness barrier.
