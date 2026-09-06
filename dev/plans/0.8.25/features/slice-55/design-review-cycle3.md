---
title: 0.8.25 Slice 55 independent design review — cycle 3
status: FIX_3_APPLIED_RE_REVIEW_REQUIRED
review_cycle: 3
reviewed_commit: 77bb06369094154b6a4651367c1321f3066dee2e
reviewed_on: 2026-09-05
verdict: FAIL
---

# Slice 55 independent design review — cycle 3

## Verdict

**FAIL.** The FIX-2 design at
`77bb06369094154b6a4651367c1321f3066dee2e` had three P1
implementability/boundedness findings and one P2 native-compatibility finding.
The reviewer made no changes. Implementation remains unauthorized. FIX-3 is
the final permitted design correction, but it does not alter this preserved
verdict or self-promote the design to READY; an independent cycle-4 review is
required.

## Preserved findings

### C3-55-01 — P1 — Authorization-before-LIMIT preserves response privacy but not bounded work

Putting endpoint eligibility before `LIMIT` prevents inaccessible rows from
changing response counts or errors, but SQLite may still examine arbitrarily
many hidden candidates to find the eligible page. FIX-2 therefore could not
claim `maxWorkUnits` bounded physical trace work. The implementable contract
must define trace units as authorized-output classification only, remove strict
physical rows-visited and no-unbounded-scan claims, require the existing
indexed source/derived lookup plans without temporary sort/materialization,
and preregister a measured ceiling on a many-hidden-dependent fixture. Hidden
rows must remain unable to affect response bytes, errors, or counts. The
one-page `maxRelations` contract and absence of continuation remain fixed.

### C3-55-02 — P1 — Required physical-member order is not index-supported

FIX-2 ordered physical members by unindexed `write_cursor`/attribute keys,
which would require temporary sorting or a schema change. Physical scans must
use fixed member-class ordinal and each FTS table's indexed rowid. Expected
owner scans must use their actual existing canonical indexes; no
`ORDER BY write_cursor, attr_name` or equivalent unsupported order is allowed.
Exact `EXPLAIN QUERY PLAN` assertions must reject a temp B-tree or full-result
materialization. Slice 55 may add no schema migration.

### C3-55-03 — P1 — The 2,945-byte guard bounds only one variable receipt field

The pending-array guard does not make full `load_receipt` bounded because that
loader also allocates unrelated variable digest/refusal/reason/affected/
closure/source-reference fields. `mutation_readiness` must select and name only
the minimal fields needed for readiness. Every selected variable field needs
an SQL type/length guard derived from its existing grammar/cardinality before
Rust allocation or deserialization; the pending-array maximum remains 2,945
bytes. The finding must mean only corruption in that readiness subset and must
not claim validation of unrelated reason, affected-revision, closure, refusal,
digest, dependency, or source-reference content.

### C3-55-04 — P2 — Native object advertising support has no discriminator

The accepted native objects have no capability/version discriminator by which
the wrapper can distinguish an older response from a candidate response that
incorrectly omits an additive field. Wrapper mapping must always treat absent
new fields as legacy defaults. Separate direct native conformance/fixture tests
must require those fields on candidate-native responses. The design must not
invent a runtime support discriminator.

## FIX-3 resolution map

| Finding | FIX-3 resolution | State pending re-review |
|---|---|---|
| C3-55-01 | Trace work units now bound eligible output classification only and explicitly do not bound hidden physical rows. Both directions name their existing source/derived indexes and reject temp sort/materialization. A preregistered 50,000-hidden-dependent release fixture pins identical response semantics plus 10,000,000 VM steps, 5.0 seconds, and 64 MiB RSS, while preserving one page/no continuation. | PROPOSED_RESOLVED |
| C3-55-02 | Physical FTS scans now use fixed class ordinal plus indexed rowid; `canonical_attributes` also uses rowid. Expected bodies use the canonical node/edge cursor indexes, and expected attributes loop the registry name primary key then the node cursor index. Named plan tests reject temp B-tree/materialization and no migration is allowed. | PROPOSED_RESOLVED |
| C3-55-03 | `mutation_readiness` explicitly bypasses `load_receipt` and selects only operation ID, schema/count/outcome/boundary, pending JSON, and projection generation. All selected variable fields have exact SQL type/byte bounds before fetch; pending remains 2,945 bytes. The finding is narrowed to this subset, with unrelated receipt-corruption silence pinned. | PROPOSED_RESOLVED |
| C3-55-04 | Python/TypeScript mapping now always defaults/omits absent additions without a discriminator. Direct candidate-native fixtures independently require presence and validation on new responses. Named wrapper and installed-native tests separate the two claims. | PROPOSED_RESOLVED |

## Re-review gate

Cycle 4 must inspect FIX-3 against current schema indexes, receipt loader and
field grammars, native object shapes, and the accepted response privacy rules.
A P1/P2 finding keeps the design non-READY. Only an independent PASS may change
readiness metadata and authorize RED implementation work.
