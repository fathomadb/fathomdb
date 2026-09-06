---
title: 0.8.25 Slice 55 independent implementation review — cycle 5
status: FAIL
reviewed_commit: cad416d260f237b75ac6acd981adb0ee77ffaabd
---

# Slice 55 independent implementation review — cycle 5

Verdict: **FAIL**

Candidate reviewed: `cad416d260f237b75ac6acd981adb0ee77ffaabd`.

## Blocking findings

1. **P1 — integrity aggregate bounds, shared authority, ordering, and ID
   privacy remain incomplete.** Integrity must aggregate `remaining + 1`
   across every physical and authority input rather than expand each source
   independently or charge only unique members. Excluded and non-scalar
   attribute-owner scans must be bounded. Lifecycle, supersession, and
   governed-pruning decisions in both directions must use the complete shared
   member classifiers. An owner that is present but outside membership must
   produce `search_projection_outside_membership`, distinct from
   `search_projection_owner_missing`. Physical rowids or equivalent candidate
   keys must survive through reconciliation so the normative final order is
   enforced. Every optional emitted ID must be validated and omitted when
   malformed. Affected implementation regions are
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:120-193`,
   `:839-999`, `:1183-1240`, `:1287-1313`, `:1348-1363`, and `:1655-1716`.

2. **P1 — request precedence and receipt-generation classification are
   incorrect.** Duplicate check detection must precede limit validation. The
   integrity receipt path must reuse or refactor Slice 40's receipt-aware
   physical point classifier with the receipt operation ID and expected
   generation. A retired but otherwise correct receipt generation maps to
   `mutation_readiness_unavailable`, not corruption. Affected implementation
   regions are `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:355-389`
   and `:1501-1587`.

3. **P1 — SDK compatibility and frozen-context validation are incomplete.**
   The real wrapper path must accept absent additive explanation fields using
   the documented legacy defaults or omission while strictly validating those
   fields whenever present. Candidate presence remains asserted at the raw
   native seam, not by making the general wrapper mapper reject legacy
   absence. Python and TypeScript must fully validate and translate malformed
   frozen-context nested containers, token, view, and eligibility fields to
   `FrozenReadError`, with exact nested RFC 6901 paths. Affected regions are
   `src/python/fathomdb/engine.py:102-114`, `:861-972`, and `:1688-1743`, plus
   `src/ts/src/index.ts:1048-1070`, `:1237-1299`, and `:1835-1937`.

4. **P2 — tests remain disconnected or encode false behavior, and chronology
   overclaims coverage.** Add genuine public/full-mapper coverage for
   aggregate bounded physical and authority scans, excluded/non-scalar owner
   scans, rowid-based final ordering, owner-present outside-membership
   classification, malformed optional-ID omission, receipt retired-generation
   mapping, full malformed frozen-context shapes, and exact dynamic production
   query plans. The existing duplicate-versus-invalid-limit and retired-
   generation tests encode behavior contrary to the READY design. Correct
   those design-authorized oracles in isolated mechanical test commits,
   explicitly record the corrections in the chronology, then add genuine
   adversarial REDs. Correct every chronology claim that is not supported by
   the actual tested public or full mapper boundary.

## Accepted for FIX-5

The normalized trace-chain authorization correction is resolved and must not
regress. Trace authorization is not reopened by this verdict.

FIX-5 must resolve every P1/P2 through genuine committed RED witnesses before
production changes and return a clean exact candidate for independent cycle 6.
