---
title: 0.8.25 Slice 55 independent design review — cycle 2
status: FIX_2_APPLIED_RE_REVIEW_REQUIRED
review_cycle: 2
reviewed_commit: 213d5824d80f02c843703a60fb5a1dd16ab57ee1
reviewed_on: 2026-09-05
verdict: FAIL
---

# Slice 55 independent design review — cycle 2

## Verdict

**FAIL.** The FIX-1 design at
`213d5824d80f02c843703a60fb5a1dd16ab57ee1` had four P1
implementation-shaping findings and two P2 compatibility/verification
findings. The reviewer made no changes. Implementation remains unauthorized.
FIX-2 updates the design and plan, but does not alter this preserved verdict or
self-promote the design to READY; an independent cycle-3 review is required.

## Preserved findings

### C2-55-01 — P1 — Trace bounds disclose inaccessible relations

The FIX-1 candidate counted hidden, ineligible, and malformed-unverifiable
relations before completing authorization. A caller could therefore distinguish
them from no relation through work counts or bound errors. Authorization and
endpoint eligibility must be inside the bounded indexed candidate selection
and precede count, limit, bound, and corruption classification. Hidden,
ineligible, or independently unverifiable relations must be observationally
identical to absence. `trace_corrupt` is permitted only after both endpoints
are independently eligible. Dedicated nondisclosure fixtures must pin this
precedence, including hidden rows beyond either caller cap.

### C2-55-02 — P1 — The integrity matrix cannot detect missing synchronous projections

FIX-1 scanned stored projection members, which can detect orphans but not a
required member that has been deleted. The design needs bounded expected-owner
scans for every required synchronous body, property, and attribute projection,
with exact canonical authorities and member classes. It must define missing-row
codes and severities, stable keys, deterministic order, work accounting, and
deletion RED fixtures, while retaining all legitimate historical/lifecycle and
Slice 40 exclusions.

### C2-55-03 — P1 — “Additive” explanation fields are not compatible with current SDK constructors

Appending required fields to the current Python dataclasses and TypeScript
user-constructible interfaces would break existing construction, and Python's
defaulted field order constrains the change. Response-wire presence must be
separated from construction compatibility. New Engine responses must populate
the fields, Python additions need safe trailing defaults, TypeScript additions
must be optional on user objects and normalized at the native boundary, and
older-object/native decoding plus ABI/source-compatibility must be tested. The
successor ADR must state this precise evolution rule.

### C2-55-04 — P1 — The claimed hard work bound does not cover actual receipt parsing or singleton authorities

FIX-1 omitted singleton/authority rows from accounting and could allocate or
deserialize an unbounded receipt JSON value before applying the pending-pair
bound. Every authority row, including each singleton and receipt row, must
cost a unit. The receipt byte length/type/array length must be checked in SQL
before text allocation or deserialization. Receipt-row versus pending-pair
accounting, malformed and oversized JSON, empty arrays, and aggregate
cap-plus-one behavior need exact contracts and RED cases.

### C2-55-05 — P2 — Correlation allocation silently changes the accepted telemetry reset contract

Moving telemetry IDs into a generally shared allocator would change the
existing enable/re-enable reset bytes and create ambiguous concurrent
ownership. The design must choose an exclusive concrete ID source that yields
exactly one ID per explained search while leaving non-explain telemetry IDs,
event bytes, and reset behavior unchanged. Telemetry-off, re-enable, toggle
race, and concurrent explained/non-explained cases must be pinned.

### C2-55-06 — P2 — The focused Python command cannot prove exact-candidate installed behavior

The `PYTHONPATH=src/python` command proves only source-wrapper behavior and may
load a stale editable native extension. Installed evidence needs an isolated
venv, a wheel built from the exact reviewed candidate, the wheel SHA-256,
offline non-editable installation, an asserted installed module/native import
origin, and the exact focused smoke command with no `PYTHONPATH`. Source-wrapper
tests must be labeled separately and cannot satisfy this gate.

## FIX-2 resolution map

| Finding | FIX-2 resolution | State pending re-review |
|---|---|---|
| C2-55-01 | The trace candidate query now proves both endpoints inside its authorization/eligibility joins before stable-key limit, work accounting, bounds, or corruption. Hidden, ineligible, and unverifiable rows produce exactly the no-relation result; `trace_corrupt` requires two independently eligible endpoints. A byte/count/cap nondisclosure matrix is named. | PROPOSED_RESOLVED |
| C2-55-02 | Integrity now scans canonical expected owners and stored members in bounded directions for `search_index`, `search_index_v2`, `search_index_edges`, `canonical_attributes`, and `property_search_index`, with exact class order, stable keys, units, missing codes/severities, retained exclusions, and one deletion RED fixture per class. | PROPOSED_RESOLVED |
| C2-55-03 | New responses require populated correlation/structural values, while Python appends safe defaulted fields and TypeScript keeps user-constructible additions optional with strict new-native normalization. Older Python/TypeScript objects, older native responses, Rust readers, and PyO3/N-API ABI shape are explicit tests; the successor ADR owns the split. | PROPOSED_RESOLVED |
| C2-55-04 | Every singleton, registry/generation/receipt authority row and every pending pair is counted. Receipt metadata is bounded in SQL before text fetch; the 2,945-byte/128-pair ceiling, empty-array row cost, invalid/oversized handling, and aggregate cap-plus-one order are exact and have named RED tests. | PROPOSED_RESOLVED |
| C2-55-05 | Post-reader finalization exclusively chooses the existing sink allocator when telemetry is enabled or an independent explanation-only allocator when it is not. Existing `q0-N` event bytes and enable/re-enable reset remain unchanged, with exact off/re-enable/race/concurrency tests. | PROPOSED_RESOLVED |
| C2-55-06 | The plan labels `PYTHONPATH` tests as source-wrapper-only and gives an isolated exact-candidate wheel command using the fail-closed verifier, offline install, SHA-256, installed Python/native origin assertions, and a Slice 55 real-database smoke with `PYTHONPATH` unset. | PROPOSED_RESOLVED |

## Re-review gate

Cycle 3 must inspect FIX-2 against current implementation and accepted Slice
15/20/25/35/40/50 contracts, including the existing SDK constructors and
telemetry reset bytes. A P1/P2 finding keeps the design non-READY. Only an
independent PASS may change readiness metadata and authorize RED implementation
work.
