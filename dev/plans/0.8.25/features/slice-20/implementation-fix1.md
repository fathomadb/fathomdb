---
title: 0.8.25 Slice 20 implementation FIX-1 response
status: FIX_IMPLEMENTED_AWAITING_REVIEW
review_cycle: 1
reviewed_commit: b952004d
---

# Slice 20 implementation FIX-1 response

Preserved FIX-1 RED commit:
`a89ca66ce5f42c5b0999c2908257d059d6144f13`.

## Review findings being corrected

- The standalone registration path did not expose the design-v7
  side-effect-free prospective validator and transaction-scoped apply seam
  needed by Slice 25.
- Both lookups performed an unbounded whole-registry integrity scan before the
  bounded query.
- Full-chain validation compared stored source and version identities for
  equality without independently enforcing their public grammars.

## TDD chronology record

The original RED commit
`efbd3f260634e71ea145ef9cef36d4b850a3a831` contained the schema tests, Rust
Engine tests, and the canonical cross-SDK fixture. Python, TypeScript, and
lifecycle executable tests were added later in GREEN commit
`1dcdf1fa54d2c652544889f3f6a0fc582ab83ca3`. That ordering did not satisfy the
plan's requirement that all executable acceptance routes be RED before their
production changes.

Published history is not rewritten and no earlier chronology is claimed. This
FIX-1 adds a new preserved RED commit, on top of the reviewed implementation,
covering the prospective transaction seam, bounded relevant-row reads,
stored-identity grammar corruption, lifecycle rollback, and executable Python
and TypeScript parity before the corresponding production corrections.

An original-base supplemental commit would document that the missing SDK
surfaces failed before GREEN, but creating it now would still be retrospective
evidence rather than pre-implementation chronology. The honest auditable
remedy is therefore this forward FIX-1 RED/GREEN pair plus this record.

## Authority interpretation

Design v7 makes dependency-generation corruption an open-time check. Persisted
dependency-chain corruption remains a runtime `Storage` failure on exact
replay or a relevant read. Lookups validate every row they actually query or
return, but an unrelated corrupt registration is not permission to perform a
hidden unbounded whole-registry fetch. Open therefore does not acquire a new
whole-registry dependency-chain audit.

For source lookup, relevance is established by the authoritative source-link
join. A dependency row whose link is absent or now names another source cannot
be attributed to the requested source without scanning the dependency
registry; the derived-key lookup and exact replay still find that row directly
and fail `Storage`. This replaces the original test's incompatible expectation
that all three routes discover every detached row.

## Implemented correction

- Standalone registration now consumes the same side-effect-free validation
  plan and transaction-scoped apply helper exposed to Slice 25. The caller owns
  the transaction and generation reservation/update; two prospective inserts
  can therefore share one generation without a nested transaction.
- `DependencyProspectiveState` derives valid endpoint chains from ordered
  `PreparedWrite` values, including a canonical source earlier in the same
  batch or an already-persisted canonical source, and tracks earlier planned
  dependency IDs and derived revisions for conflict checks.
- Source lookup fetches exactly 101 ordered relevant rows, validates every one,
  then returns either the complete result or the bound error. Derived lookup
  validates only its zero-or-one keyed row. Neither performs a whole-registry
  materialization.
- Every stored source ID and source-version ID participating in a relevant
  chain is independently checked with its public constructor grammar before
  equality and reciprocal-link checks.
- The dependency-generation singleton is validated only once migration step 28
  is present. Current-schema corruption still fails open, while the engine's
  deliberate pre-step-28 migration-test seam remains usable.
- Older open-state corruption fixtures now preserve the independent dependency
  generation while targeting their intended cache key, and compatibility
  fixtures use a real migrated current-schema database.

Focused Rust, exact locally built Python-wheel and TypeScript native-artifact
tests, the combined local package smoke, and the complete engine test sequence
through all remaining test binaries are green.
