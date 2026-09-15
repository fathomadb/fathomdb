---
title: FathomDB 0.8.26 Slice 30 — adversarial remediation design review
status: PASS
reviewed_on: 2026-09-14
---

# Slice 30 adversarial remediation design review

## Scope reviewed

This direct design review covers the remediation requirements, acceptance
criteria, per-issue design, blast-radius statements, and RED/GREEN ordering
added after review of implementation tip `067d74b4` and closeout `1e6a75ef`.

## Challenges and resolutions

1. **Could file resolution change database creation?** No. Final-file
   canonicalization is confined to the inspection-only function, whose contract
   already requires an existing database. `Engine::open` retains the existing
   parent-only helper and its create behavior.
2. **Could request precedence regress?** No. Request validation remains before
   all path resolution and filesystem access. Missing, broken, or inaccessible
   targets still collapse to the existing privacy-safe
   `inspection_unavailable` reason.
3. **Could only part of the product namespace move?** The design requires the
   resolved path to become the single authority for the product lock, WAL,
   rollback journal, immutable URI, and SQLite open. The RED cases separately
   challenge target-lock and target-WAL discovery through a symlink alias.
4. **Does the design silently broaden alias support?** No. It resolves the
   reported symbolic-link defect. Hard-link aliases cannot be unified by
   filesystem canonicalization and remain outside the support claim; no
   serving-path or hard-link policy changes here.
5. **Could the smoke correction require registry access in tests?** No. The
   existing structural shell test is the oracle. It can require the exact
   Cargo requirement and retain the binary identity check without contacting
   crates.io.
6. **Could the state correction move the release ladder?** No. Only Slice 30's
   status/evidence/tip is reconciled. Slice 35 remains next; dependencies,
   decisions, schema, and integration state remain unchanged.

## Blast-radius verdict

- Runtime: one operator-gated inspection path and its process regression.
- Release tooling: one crates.io smoke command and its structural assertion.
- State/docs: the Slice 30 row, release-state evidence/tip, generated views,
  and Slice 30 closeout records.
- Explicitly unchanged: schema, migrations, integrity SQL/report V1, serving
  open, Python/TypeScript/native bindings, package publication, and registries.

## Verdict

PASS. The design is sufficiently narrow, falsifiable, and ordered for TDD.
Implementation may begin with the committed RED regressions; test assertions
must not be weakened during GREEN.
