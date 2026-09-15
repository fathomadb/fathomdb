---
title: ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary
date: 2026-09-13
target_release: 0.8.26
desc: Change affected V1 contracts in place and accept only fresh 0.8.26 databases
blast_radius: Engine open; actuation request and receipt APIs; graph evidence result APIs; Rust, Python, and TypeScript bindings; schema bootstrap; release and compatibility documentation
status: accepted by HITL seq-283
supersedes_in_part: ADR-0.8.25-bounded-atomic-actuation
subsequent_resolution: ADR-0.8.26-exact-graph-artifact-evidence.md records D26-01 at seq-290
---

# ADR-0.8.26 — Breaking V1 contracts and fresh database boundary

## Decision

FathomDB 0.8.26 introduces no parallel functional V1/V2 public API pairs. It
changes affected V1 contracts in place, declares the release breaking, supports
fresh databases only, and defers parallel-version compatibility machinery until
post-1.0.

The actuation surface remains `Engine.actuate` with `ActuationOperationV1`,
`ActuationBatchV1`, `ActuationReceiptV1`, and the related V1 outcome and refusal
types. The current V1 grammar gains provenance-bearing derived-edge actuation
alongside its canonical-node, derived-node, source-dependency, and lifecycle
capabilities. If receipt fields or digest semantics must change to represent the
new grammar truthfully, their V1 contracts change in place as part of this
breaking release.

There is no `actuate_v2`, V2 parser, version router, V1 redirect path, or mixed
V1/V2 execution mode. Dynamic bindings validate the one current V1 grammar.
Requests that still satisfy that grammar may execute, but this incidental
structural overlap is not a compatibility guarantee. Package version 0.8.26,
not a second API generation, identifies the breaking contract revision.

The same rule applies to every affected public surface in 0.8.26. Existing V1
types change in place when required. A genuinely new type may begin at V1, but
no existing V1 type gains a parallel V2 successor and no method routes between
two functional generations. In particular, graph-evidence work must either
change the applicable graph V1 result/target contract in place or introduce a
first-generation V1 sidecar; it must not create `GraphExpandResultV2`,
`GraphTargetV2`, or a parallel graph method.

The 0.8.26 planning inventory has two affected families:

| Family | Names retained |
| --- | --- |
| Actuation | `ActuationBatchV1`, `ActuationOperationV1`, `ActuationReceiptV1`, `ActuationOutcomeV1`, `ActuationRefusalReasonV1`, `LifecycleActuationV1`, and `Engine.actuate` |
| Graph evidence | `GraphExpandRequestV1`, `GraphExpandResultV1`, `GraphTargetV1`, and the existing graph method; any net-new evidence sidecar or point-evidence response begins at V1 |

No other planned 0.8.26 API family has a functional V1/V2 pair. Frozen-search
explanation repair, ranked evidence resolution, operator integrity, and
ordinary writes continue on their existing surfaces.

0.8.26 accepts fresh databases only. It provides no supported database
migration from any earlier FathomDB version. Fresh-database bootstrap may reuse
internal schema-construction machinery, but an existing non-current database
must be refused before mutation rather than upgraded.

## Compatibility boundary

0.8.26 carries no specialized historical compatibility behavior for:

- prior actuation request or operation encodings;
- prior request digests, replay state, or operation-ID collision handling;
- prior receipts, storage representations, or integrity checks;
- existing provenance, lifecycle, dependency, or source-reference rows; or
- database upgrade or downgrade.

The current V1 contract still validates its own provenance, lifecycle,
dependency, source-reference, replay, erasure, and integrity invariants. This
decision removes historical compatibility work; it does not weaken the current
product contract.

Because existing databases are refused, there is no mixed historical/current
persisted state or cross-release operation-ID namespace. Operation IDs and
receipts are defined only within fresh 0.8.26 databases.

## Required release behavior

- Release notes, the changelog, compatibility guidance, and binding upgrade
  guidance label 0.8.26 as breaking and fresh-database-only.
- A fresh 0.8.26 database opens and exercises the complete current V1 actuation
  surface.
- A representative earlier-version database is refused before mutation. No
  historical-version migration matrix is required.
- Historical request, digest, replay, receipt, integrity, and migration
  implementation paths are removed rather than retained behind flags or
  adapters.
- Public bindings expose no parallel functional V1/V2 type or method pair, and
  internal code contains no public-generation router.
- Tests cover the current V1 contract, not interactions between V1 and V2.

## Consequences

The release deliberately trades API-shape and database upgrade compatibility for
one actuation grammar, one binding surface, one receipt contract, and no
cross-version replay or migration machinery. Memex must create a fresh 0.8.26
database and target the current V1 contract. FathomDB does not provide an
in-engine data conversion path.

The breaking boundary applies to affected V1 contracts and database opening. It
does not authorize unrelated API churn. Frozen search, evidence resolution,
operator, write, and other unaffected APIs retain their existing names and
contracts. D26-04 (`seq-284`) requires derived-edge endpoints in the complete
prospective batch state, and D26-05 (`seq-285`) selects the compact
changed-in-place V1 receipt with only proven edge fields. The graph-evidence
shape remains open pending Slice 15 and a later D26-01 ruling.

## Supersession

This ADR supersedes the V1 preservation, additive-successor, and
existing-database assumptions of
[`ADR-0.8.25-bounded-atomic-actuation.md`](ADR-0.8.25-bounded-atomic-actuation.md)
for 0.8.26. The earlier ADR remains the historical contract of published
0.8.25. Its model-free policy boundary, typed/no-SQL boundary, and atomic
transaction rationale remain applicable to the changed-in-place V1 contract.

It also supersedes the V2 naming, V1 redirect, and parallel-version implications
of HITL `seq-282`. The breaking-release, fresh-database-only, no-migration, and
no-historical-compatibility portions of `seq-282` remain in force.

## Authority

HITL ruling `seq-283` establishes the release-wide no-parallel-V1/V2 rule and
the changed-in-place V1 contract. Rulings `seq-284` and `seq-285` establish its
endpoint and receipt boundaries. This ADR records those rulings without adding
a compatibility exception.

## Subsequent resolution

The graph-evidence shape was open when this ADR recorded `seq-283`. HITL later
selected D26-01 option A at `seq-290`, recorded without rewriting this decision
in
[`ADR-0.8.26-exact-graph-artifact-evidence.md`](ADR-0.8.26-exact-graph-artifact-evidence.md).
