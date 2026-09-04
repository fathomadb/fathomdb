---
title: 0.8.25 Slice 20 design review — cycle 1
status: FAIL_CORRECTIONS_APPLIED
reviewed_design_version: 3
reviewed_on: 2026-09-03
---

# Slice 20 design review — cycle 1

## Verdict

FAIL. Implementation was not authorized from design v3. The findings below
were corrected in design v4 and require a second independent review.

## Findings and disposition

| ID | Priority | Finding | Design v4 disposition |
| --- | --- | --- | --- |
| D20-01 | P1 | Mutable retirement could detach a still-searchable derived artifact from Slice 30 closure. | Removed retirement, relation state, retired reads, and related errors. A dependency is immutable until hard erasure. |
| D20-02 | P1 | Persisting `source_revision_id` duplicated Slice 15's authoritative source link and could drift. | The new table stores only dependency ID, derived revision, and registration boundary; all source access joins and validates `_fathomdb_source_links`. |
| D20-03 | P1 | Public signatures and contextual ID/parser errors were not executable. | Added exact Rust/Python/TypeScript cardinality and constructor contracts; endpoint validation maps to dependency-local reasons and paths. |
| D20-04 | P1 | A standalone transaction/cursor owner could not compose with Slice 25's atomic batch. | Required a side-effect-free prospective validator and transaction-scoped apply helper with an enclosing boundary. |
| D20-05 | P2 | Admission of inactive or superseded source revisions was unspecified. | Slice 20 validates structure only; Slice 30 adds lifecycle/barrier admission before release completion. |
| D20-06 | P2 | Dependency pagination was incorrectly implied to belong to Slice 45. | Allocated dependency continuation explicitly to 0.8.27. |
| D20-07 | P2 | Dependency-ID behavior after erasure was unspecified. | Hard erasure deletes the raw ID and permits reuse; only a non-content global high-water key remains. |
| D20-08 | P2 | Purge/erasure prose did not match actual logical-node purge behavior or both orphan axes. | Specified cleanup for all node revisions and touching edges, outside-set refusal, source-bucket removal, and raw owner/link corruption. |
| D20-09 | P2 | Claimed additive multi-source relaxation contradicted unique derived ownership. | Multi-source now explicitly requires a successor schema and contract. |
| D20-10 | P2 | Interface-document updates were absent from delivery. | Added all four interface documents to acceptance and closure work. |
