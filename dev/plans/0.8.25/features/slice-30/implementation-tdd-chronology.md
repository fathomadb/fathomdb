---
title: 0.8.25 Slice 30 implementation TDD chronology
status: ACTIVE
---

# Slice 30 TDD chronology

This record distinguishes observed RED tests from coverage added with or after
implementation. It does not retroactively describe post-hoc coverage as RED.

| Stage | Commit | Evidence |
| --- | --- | --- |
| RED-1 | `5e5a9307` | Initial direct dependency-closure failures. |
| RED-2 | `425ef1d0` | Restart and recovery failures. |
| RED-3 | `f28e00d0` | Governed read-barrier failures. |
| RED-4 | `91b6d2aa` | Physical writer-fence failures. |
| GREEN-1 | `21ff56dc` | Initial implementation. This commit also added test coverage; those additions are compatibility evidence, not pre-implementation RED evidence. |
| PIN-1 | `ee85922c` | Approved governed-surface allowlist and exact pin. |
| FIX-1 RED | `58a9b604` | Reviewer-driven failures for source-view eligibility, measured physical proof rollback, post-commit cursor publication, and strict TypeScript response decoding. |
| FIX-1 GREEN | `3a9aa8cf` | Measured proof, shared source eligibility, cursor publication ordering, strict response decoder, and this chronology. |
| FIX-2 RED | `d5920ef9` | More than 192 ineligible vector candidates demonstrated the pre-hydration truncation gap and missing degradation signal. |

Further cycles append rows here before Slice 30 is closed. The status record
will identify the final implementation and verification commits.
