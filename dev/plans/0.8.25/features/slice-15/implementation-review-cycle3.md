---
title: 0.8.25 Slice 15 implementation review — cycle 3
status: COMPLETE
review_cycle: 3
reviewed_on: 2026-09-03
reviewed_commit: da7c3596a0947bec7fdd1f02b4c5aec3713a039a
verdict: FAIL
---

# Slice 15 independent implementation review — cycle 3

## Verdict

**FAIL.** One P2 finding remains; no P1 or P3 finding remains.

## Finding

| ID | Severity | Finding | Required correction |
| --- | --- | --- | --- |
| I15-11 | P2 | Multiple unknown fields produce binding-dependent pointers. Python walks insertion-ordered dictionaries, while N-API's `serde_json` map currently yields lexical order; inserting `zFuture` before `aFuture` therefore reports `/provenance/zFuture` in Python and `/provenance/aFuture` in TypeScript. | Define unknown-field precedence as the lexicographically smallest canonical camel-case pointer token. Apply it consistently to provenance, role-specific, locator, and hash closed objects. Add shared reverse-ordered multi-unknown cases, including `~` and `/`, through direct and wrapped inputs, and prove typed reason/path parity plus atomic no-write. |

## Confirmed corrections

Cycle 3 confirmed that FIX-3 preserves native schema, role, closed-object,
locator-discriminator, and hash-discriminator precedence while retaining
field-specific NUL and lone-surrogate provenance errors.
