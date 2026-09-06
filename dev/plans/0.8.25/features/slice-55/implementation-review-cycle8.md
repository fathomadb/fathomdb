---
title: 0.8.25 Slice 55 independent implementation review — cycle 8
status: FAIL
reviewed_commit: 31bef1768df6af62080ef108c32d605371d0f6b3
product_commit: f70cc5c31c3294a53d5dfa0f6b39a7e94280feb8
---

# Slice 55 independent implementation review — cycle 8

Verdict: **FAIL — implementation review cycle 8**

Reviewed clean candidate: `31bef1768df6af62080ef108c32d605371d0f6b3`
Product commit: `f70cc5c31c3294a53d5dfa0f6b39a7e94280feb8`

The review found no regression in the cycle-7 corrections for signed rowids,
guarded normalized-chain roles, generation zero, minimum finding IDs, the
ten-code dependency matrix, property/fixture coverage, or SDK/trace behavior.

## Blocking findings

1. **P1 — projection-generation enumerates and charges a non-contractual
   canonical-owner direction.** `projection_generation_findings` in
   `data_plane_integrity.rs:1937-1952` enumerates canonical owners and consumes
   one work unit for each before it scans physical members. READY design lines
   397-399 define exactly two authority units—the current-generation singleton
   and current generation record—followed by one unit for each physical member.
   A canonical owner is neither an authority nor a physical member for this
   check. This can exhaust `max_work_units` when only canonical owners are
   present and can invent a missing-physical owner candidate that this check
   does not authorize. Existing expectations in
   `slice55_data_plane_integrity.rs:642-685` encode the implementation rather
   than the READY contract: the corrupt-member fixture expects four units where
   three are authorized, and the canonical-owners-only bound fixture expects an
   error where success with `checked_count=2` is required.

2. **P1 — normalized owner schema versions are not validated.** The
   corruption-safe `StoredArtifactOwner` load omits `schema_version`, so a
   schema-version-2 derived or canonical owner can pass the dependency-chain
   check as clean. READY requires stored owner rows to be classified through
   their exact versioned role contracts: an invalid derived owner maps to
   `dependency_derived_role_invalid`/error and an invalid canonical source
   owner maps to `dependency_source_role_invalid`/error, with the matrix's
   minimum identifier sets.

3. **P1 — a derived self-reference has the wrong precedence, severity, and
   identifiers.** When `source_revision_id == derived_revision_id`, the current
   dependency classifier reaches `dependency_source_link_mismatch`/critical and
   emits duplicate/source identifiers. READY names self-reference as
   `dependency_derived_role_invalid`/error. It must be classified before the
   general source-link mismatch and return exactly the dependency ID plus one
   derived revision ID.

4. **P1 — canonical unsigned dependency generations above SQLite's signed
     range can become the read boundary.** The dependency-generation singleton
   accepts canonical numeric text above `i64::MAX`, including `u64::MAX`, even
   though the persisted generation domain is `0..=i64::MAX`. Such a singleton
   must produce `dependency_generation_mismatch`/critical and must never be
   exposed as `read_boundary.dependency_generation`.

5. **P2 — real-database regression coverage is missing for the three newly
   exposed corruption classes.** There is no fixture proving that invalid
   derived/source owner schema versions map to their exact READY finding paths,
   that derived self-reference wins over link mismatch with exact severity and
   minimum IDs, or that canonical generation text above `i64::MAX` is rejected.
   There is also no direct real-database proof that canonical owners alone
   cannot consume the projection-generation work bound or create an absent
   physical-member candidate.

## Required correction boundary

- Enumerate and charge only the current-generation singleton, current
  generation record, and bounded physical current-generation members for
  `projection_generation`.
- Inspect `StoredArtifactOwner.schema_version` through the existing
  corruption-safe SQLite value path and apply the READY derived/source role
  finding matrix.
- Give derived self-reference precedence over source-link mismatch and emit
  exactly `dependency_derived_role_invalid`/error with the dependency ID and
  one derived revision ID.
- Constrain dependency-generation singleton text to `0..=i64::MAX` before it
  can become the boundary.
- Add genuine real-database REDs for each correction before production changes.

No broader refactor, schema change, reverse state, SDK surface change, or
trace-contract change is authorized by this review.
