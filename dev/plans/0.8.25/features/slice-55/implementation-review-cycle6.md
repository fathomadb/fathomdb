---
title: 0.8.25 Slice 55 independent implementation review — cycle 6
status: FAIL
reviewed_commit: b754e8fe3703645edb242e9baee5b5792089a58e
---

# Slice 55 independent implementation review — cycle 6

Verdict: **FAIL**

Candidate reviewed: `b754e8fe3703645edb242e9baee5b5792089a58e`.

## Blocking findings

1. **P1 — legitimate dormant expired-edge projection retention is reported as
   corruption.** The expected edge-body scan excludes clock-expired edges in
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:1024`, after
   which a retained `search_index_edges` member is classified as
   `search_projection_outside_membership` by `body_residue_finding` at
   `:856`. The dense path similarly reports retained expired-edge state as
   outside membership or corrupt at `:1170` and `:1410`. The READY Slice 55
   design at lines 553–559 and the accepted Slice 40 contract require dormant
   expired-edge FTS and dense artifacts to remain legitimate retained state.

2. **P1 — a deleted canonical owner with a retained artifact revision is
   misclassified.** `body_residue_finding` at
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:831-883`
   treats artifact-revision existence as owner existence. If the canonical
   node or edge row is absent but its artifact revision remains, the finding
   is `search_projection_outside_membership`; the READY matrix requires
   `search_projection_owner_missing`. Owner retention needs one shared
   classifier that distinguishes required membership, legitimate historical
   or dormant retention, governed-pruning residue, and absent canonical
   owner, and that is applied consistently to expected and physical body,
   dense, and generation directions.

3. **P1 — malformed SQLite dynamic types can escape as generic storage errors
   or remain invisible.** Early typed decoding in dependency rows around
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:564`, source
   links around `:619`, generation and physical rows around `:958`, `:1042`,
   `:1261`, and `:1353`, and singleton loading around `:1767` plus
   `src/rust/crates/fathomdb-engine/src/lib.rs:25377` can return
   `EngineError::Storage` before integrity translates the malformed state to
   the exact READY finding. BLOB, text, and null storage types in dependency
   identities, generation/schema fields, normalized source-link fields,
   dependency/current-generation singletons, physical FTS and dense rows, and
   generation members must be guarded through SQLite type metadata or
   `ValueRef` before any typed decode. Invalid optional identities must be
   omitted without leaking their bytes.

4. **P1 — null mutation-receipt operation IDs are skipped.** The first-pass
   receipt query at
   `src/rust/crates/fathomdb-engine/src/data_plane_integrity.rs:98` uses
   `WHERE operation_id > ?1`. Because the historical receipt table is not
   `STRICT`, a malformed null primary key is possible and is filtered out by
   SQL three-valued logic. Receipt enumeration must include such rows through
   stable rowid/metadata ordering, count them within the aggregate bound, and
   report `mutation_receipt_corrupt` without an operation ID.

5. **P2 — the retained-state and malformed-storage behavior lacks genuine
   real-database fixtures.** Add committed test-only REDs for expired-edge FTS
   and dense retention, canonical-owner deletion with retained revision,
   every malformed dynamic-type family above, and null receipt keys. The
   tests must prove exact finding-code precedence, normative ordering,
   aggregate remaining-plus-one accounting, privacy-safe optional-ID
   omission, and no generic `EngineError::Storage` escape. Existing tests may
   not be weakened or replaced with text-only or vacuous oracles.

## Accepted for FIX-6

Cycle 5's aggregate-bound, SDK compatibility, request-precedence, normalized
trace-chain, optional-ID, and receipt-generation corrections are resolved and
must not regress. The bounded deadlock diagnosis at
`b754e8fe3703645edb242e9baee5b5792089a58e` passed without a current
reproduction; no product change is justified for that historical observation.

FIX-6 is the final planned correction before implementation review cycle 7.
It must resolve every finding above through a genuine committed RED witness,
then return a clean exact candidate for that independent review.
