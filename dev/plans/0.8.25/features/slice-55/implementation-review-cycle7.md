---
title: 0.8.25 Slice 55 independent implementation review — cycle 7
status: FAIL
reviewed_commit: e590fd4a93e30ef89350a4c65499d57d0c6d05c5
product_commit: 06aa3b89b7b555716066cf798b99abcbf2e3abd7
---

# Slice 55 independent implementation review — cycle 7

Verdict: **FAIL — implementation review cycle 7**

Reviewed clean candidate: `e590fd4a93e30ef89350a4c65499d57d0c6d05c5`
Product commit: `06aa3b89b7b555716066cf798b99abcbf2e3abd7`

`git status` was clean and `git diff --check` passed. No files or Git state were
changed.

## Blocking findings

1. **P1 — physical and receipt scans silently omit negative SQLite rowids.**
   Candidate queries begin at `rowid > 0` in
   `data_plane_integrity.rs:37`, affecting FTS, attribute/property, dense, and
   receipt enumeration. An exact candidate reproduction inserted a
   `search_index` orphan with `rowid=-1`; `doctor data-plane-integrity`
   incorrectly returned `status=clean`, `checkedCount=0`, and no findings.
   This violates the requirement to scan every stored physical member and
   breaks complete accounting/order.

2. **P1 — normalized dependency-chain decoding still leaks generic storage
   failures.** Several owner and canonical-chain reads still use typed
   `row.get` before corruption classification at
   `data_plane_integrity.rs:682`, `:799`, and `:872`. Replacing a derived
   owner's `artifact_role` with a BLOB produced generic `StorageError` rather
   than `dependency_derived_role_invalid`.

3. **P1 — dependency-generation classification and emitted IDs contradict the
   READY matrix.** A registered generation of zero is treated as
   `dependency_row_invalid`/error at `data_plane_integrity.rs:665`, but READY
   requires `dependency_generation_mismatch`/critical. Also,
   `data_plane_integrity.rs:922` unconditionally adds derived revision to later
   findings, including generation and source-side findings whose exact
   minimum-ID contract excludes it.

4. **P2 — S55-AC4's real-database dependency matrix remains incomplete.**
   `slice55_dependency_chain_fault_matrix` tests only invalid schema. No test
   references five required codes: `dependency_derived_role_invalid`,
   `dependency_source_link_missing`, `dependency_source_role_invalid`,
   `dependency_source_version_mismatch`, or
   `dependency_source_self_link_mismatch`. The plan-required
   `slice55_normalized_chain_round_trip` property test is absent, and
   dependency/projection/explanation fixture JSON files are not consumed.

## Resolved from cycle 6

- Dormant expired-edge FTS/dense retention.
- Canonical-owner deletion versus retained revision.
- ValueRef guarding for dependency rows, source links, generation authority,
  physical FTS/attribute/dense members.
- Null receipt-key enumeration.
- Existing aggregate-bound and nonnegative-row ordering cases.
- Trace authorization, frozen-context handling, and explanation regressions.

## Tests run

- Integrity: 58 passed.
- Dependency trace: 20 passed, 1 ignored.
- Explanation: 17 passed.
- Wire: 8 passed.
- `git diff --check`: passed.

No material new P3 was found. Independent performance and broad repository
verification remain pending.
