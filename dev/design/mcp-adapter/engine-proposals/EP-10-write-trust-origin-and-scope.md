---
title: EP-10 — Engine-stamped origin class and scope (write family prerequisite)
date: 2026-10-17
status: PROPOSED
desc: Draft engine proposal, deliberately brief. First-class origin-class and scope fields, stamped by the engine from the opener's policy rather than from record bodies, enforced on every read path, plus an exact-duplicate rule. Prerequisite for any MCP write tool.
---

# EP-10 — Engine-stamped origin class and scope

Status: Draft. Priority P2 (future work). Not an accepted decision.

## Problem

- Records carry `source_id` (`fathomdb-schema/src/lib.rs:262-269`) and nothing
  that says how far to trust them. MR-010 needs an origin class that no record
  body can claim (MR-010.4).
- `ADR-0.8.25-eligibility-and-frozen-reads.md` declines "owner, scope" search
  terms. It expects applications to map owner and scope onto declared exact
  projections (ADR lines 15-17 and 33-35). A projection is derived from the
  body. A body-derived scope has the same flaw as a body-derived origin: a
  written record can place itself in another scope.
- `read.get`, `read.get_many` and `graph.neighbors` take only a `ReadView`
  (`fathomdb-sdk/src/read.rs:20-37`, `graph.rs:19-27`). `read.list` takes its
  own `Filter` or predicates (`read.rs:62-80`), which the caller chooses. So
  there is no read-path point where an opener-bound scope could be enforced
  today. A scope applied only to search would leak through point reads.
- There is no engine rule for exact duplicates (MR-062).

Adapter status: no workaround. The adapter is read-only (`write: false` is
enforced), refuses to start with a scope configured (MR-014.4, interface §2),
and reports every origin as unavailable.

## Proposed surface (outline only)

- **Origin class.** A nullable `origin_class` column on `canonical_nodes` and
  `canonical_edges`, as an additive ALTER (fresh databases only, under
  ADR-0.8.26). It is set from an opener-level policy
  (`OpenOptions.write_policy`), never from a `PreparedWrite` field or the body.
  Reads return it beside `source_id`. Search and list can filter on it.
- **Scope.** A `scope` column stamped the same way, and
  `ReadView.scope: Option<ScopeSet>` honoured by every view-taking read, including
  point reads, neighbours, list, aggregates (EP-05) and the change feed (EP-16).
  A record outside scope reads as not found.
- **Exact duplicates.** An engine-owned normalization plus a stored body hash
  (see EP-15), so a write of identical content of the same kind returns the
  existing logical id.

## Invariants

- No API path can set either field from caller content.
- A scoped view cannot see a row outside its scope by any read verb. That needs
  a test per verb.

## Interaction with ADRs

This needs a successor to `ADR-0.8.25-eligibility-and-frozen-reads.md`
(owner and scope terms), an amendment to the OPP-12 identity ADRs, and a
governed-surface sign-off. It is the largest item in this set.

## Cost and performance

Unmeasured. The scope predicate goes before every cap, as the frozen-reads ADR
requires for eligibility. So it needs indexes on the vector and FTS paths, which
`ADR-0.8.25-eligibility-and-frozen-reads.md` already notes are the hard part.

## Test plan sketch

Per verb: a record written under scope A is invisible under scope B by direct
reference, by neighbour and in counts. A body containing
`"origin": "owner"` reports the policy class. Two identical writes yield one
record.

## Adapter change once it lands

Write tools (MR-060 to MR-062) become possible. `origin_class` and `source_id`
go on every result. The scope startup refusal is replaced by a capability check
against the linked SDK.

## Open questions for the owner

1. Is origin class one closed set across all applications
   (`owner, agent, untrusted, system, unclassified`), or per database?
2. Should scope be a single tag or a set, and can a record belong to several?
