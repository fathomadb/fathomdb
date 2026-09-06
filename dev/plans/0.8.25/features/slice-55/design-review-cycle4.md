---
title: 0.8.25 Slice 55 independent design review — cycle 4
status: PASS
review_cycle: 4
reviewed_commit: 4a5ec9b4dd7854986ab19f5f5d9f1510fc737148
reviewed_on: 2026-09-05
verdict: PASS
---

# Slice 55 independent design review — cycle 4

## Verdict

**PASS.** The independent reviewer evaluated design v8/FIX-3 at exact commit
`4a5ec9b4dd7854986ab19f5f5d9f1510fc737148`. C3-55-01 through C3-55-04 are
resolved. There is no unresolved P1 or P2 finding, so the design may be
promoted to READY and RED implementation may begin. The P3 evidence note below
is mandatory for verification but does not change or block the approved
contract.

## Cycle 3 resolution verification

### C3-55-01 — PASS — Authorization-before-LIMIT and bounded-work claim

Trace `maxWorkUnits` now bounds only authenticated eligible-output
classification, not hidden physical rows examined. Hidden or unverifiable rows
cannot affect response bytes, errors, counts, or bound outcomes. Both lookup
directions use the existing source/derived indexes and prohibit a temporary
sort, materialization, or full candidate vector. The preregistered
50,000-hidden-dependent fixture separately constrains internal performance.
The result remains one page, bounded by `maxRelations`, with depth 1 and no
continuation.

### C3-55-02 — PASS — Index-supported physical-member order

Physical FTS scans use fixed member-class ordinal followed by indexed FTS5
rowid; `canonical_attributes` also uses rowid. Expected body owners use the
existing canonical node/edge cursor indexes. Attribute/property expectations
walk the registry name primary key and canonical-node cursor index. The design
forbids unsupported `ORDER BY write_cursor, attr_name`, requires exact
`EXPLAIN QUERY PLAN` assertions against temp B-trees/materialization, and adds
no schema migration.

### C3-55-03 — PASS — Minimal bounded mutation-readiness receipt subset

`mutation_readiness` explicitly bypasses full `load_receipt`. It selects only
operation ID, schema version, operation count, outcome, resulting write
boundary, pending projection cursor JSON, and projection generation ID. Each
selected variable field has an SQL type/byte-length guard grounded in its
existing grammar/cardinality before Rust allocation or deserialization; the
pending JSON limit remains 2,945 bytes. `mutation_receipt_corrupt` is expressly
limited to this readiness subset and makes no claim about unrelated reason,
affected-revision, closure, refusal, digest, dependency, or source-reference
content.

### C3-55-04 — PASS — Discriminator-free native compatibility

Python and TypeScript wrappers always treat absent additive native fields as
legacy defaults/omissions and do not invent a runtime capability discriminator.
Separate direct candidate-native conformance fixtures require the additions to
be present and valid on every new Engine response. Wrapper compatibility and
candidate-native conformance are therefore tested as distinct claims.

## Regression check

Earlier design closures remain intact:

- the governed and operator APIs have no public-name collision;
- normalized dependency authority still has no reverse row or reverse index;
- trace authorization remains nondisclosing;
- expected synchronous members cover node/edge body FTS, property FTS, and
  canonical EAV;
- Python/TypeScript explanation construction remains additive;
- telemetry reset bytes and mutex-linearized single-source correlation remain
  coherent; and
- exact-candidate wheel verification remains truthful, isolated, local-only,
  and outside release packaging, registry staging, tags, or publication.

No regression reopens a Cycle 1, Cycle 2, or Cycle 3 P1/P2 finding.

## P3 verification note

The implementation and `verification-review.md` must name the exact mechanism
used to measure SQLite VM steps and peak RSS for the preregistered trace
performance fixture. Reporting only numeric results or a generic profiler name
is insufficient: the evidence must identify the SQLite counter/hook and RSS
source, measurement window, reset/baseline treatment, platform, and exact
command. This documentation requirement must be satisfied before Slice 55
closes, but it does not block design READY.
