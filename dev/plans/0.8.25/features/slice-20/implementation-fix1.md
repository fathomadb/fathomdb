---
title: 0.8.25 Slice 20 implementation FIX-1 response
status: RED
review_cycle: 1
reviewed_commit: b952004d
---

# Slice 20 implementation FIX-1 response

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
