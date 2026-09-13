---
title: ADR-0.8.26-breaking-v2-actuation-and-fresh-database-boundary
date: 2026-09-13
target_release: 0.8.26
desc: Replace V1 actuation with V2 and accept only fresh 0.8.26 databases
blast_radius: Engine open; actuation request and receipt APIs; Rust, Python, and TypeScript bindings; schema bootstrap; release and compatibility documentation
status: accepted by HITL seq-282
supersedes_in_part: ADR-0.8.25-bounded-atomic-actuation
---

# ADR-0.8.26 — Breaking V2 actuation and fresh database boundary

## Decision

FathomDB 0.8.26 is an explicitly breaking pre-production release for the
actuation surface. `ActuationOperationV2`, `ActuationBatchV2`, and the V2
receipt contract are the only functional actuation grammar. V2 contains the
canonical-node, derived-node, source-dependency, and lifecycle capabilities
introduced in V1 plus provenance-bearing derived-edge actuation.

The public actuation method remains one method per binding and accepts V2. It
does not add a parallel `actuate_v2` family. Static Rust callers using removed
V1 types fail at compile time. Dynamic or native ingress that can observe a
V1-shaped request may inspect only enough of the top-level discriminator to
return a loud, deterministic refusal directing the caller to V2. That refusal
must not parse V1 operations, execute them, translate them, compute their
digest, replay them, or act as a compatibility shim.

This is not a version router. There is one V2 parser and one V2 executor. The
ordinary closed-schema guard rejects every unsupported actuation schema before
nested parsing and reports that schema 2 is required; it does not dispatch on
V1 versus V2 or retain a V1-specific implementation branch.

0.8.26 accepts fresh databases only. It provides no supported database
migration from any earlier FathomDB version. Fresh-database bootstrap may
reuse internal schema-construction machinery, but an existing non-current
database must be refused before mutation rather than upgraded.

## Compatibility boundary

0.8.26 carries no specialized V1-to-V2 compatibility behavior for:

- actuation requests or operation encodings;
- request digests, replay, or operation-ID collision handling;
- V1 receipts or their storage and integrity checks;
- existing provenance, lifecycle, dependency, or source-reference rows; or
- database upgrade or downgrade.

V2 still validates its own provenance, lifecycle, dependency, source-reference,
replay, erasure, and integrity invariants. The decision removes historical V1
compatibility work; it does not weaken the V2 product contract.

Because existing databases are refused, there is no mixed V1/V2 persisted
state or cross-version operation-ID namespace. V2 operation IDs and receipts
are defined only within fresh 0.8.26 databases.

## Required release behavior

- Release notes, the changelog, compatibility guidance, and binding upgrade
  guidance must label 0.8.26 as breaking and fresh-database-only.
- An old-shaped dynamic request receives the minimal V1-retired direction and
  causes no database mutation.
- No public method accepts a functional union of V1 and V2, and no internal
  router selects between versioned actuation implementations.
- A fresh 0.8.26 database opens and exercises the complete V2 surface.
- A representative earlier-version database is refused before mutation. No
  historical-version migration matrix is required.
- V1 request, digest, replay, receipt, integrity, and migration implementation
  paths are removed rather than retained behind flags or adapters.

## Consequences

The release deliberately trades source and database upgrade compatibility for
one actuation grammar, one binding surface, one receipt contract, and no
cross-version replay or migration machinery. Memex must create a fresh 0.8.26
database and target V2 directly. FathomDB does not provide an in-engine data
conversion path.

The breaking boundary applies to actuation and database opening. It does not
by itself authorize breaking graph-evidence, search, read, operator, or other
unrelated APIs. Those remain governed by their own Slice 8 decisions.

## Supersession

This ADR supersedes the V1 preservation, additive-surface, and existing-database
assumptions of
[`ADR-0.8.25-bounded-atomic-actuation.md`](ADR-0.8.25-bounded-atomic-actuation.md)
for 0.8.26. The earlier ADR remains the historical contract of published
0.8.25. Its model-free policy boundary, typed/no-SQL boundary, and atomic
transaction rationale remain applicable to V2.

## Authority

HITL ruling `seq-282` explicitly selects the breaking V2-only actuation and
fresh-database boundary. This ADR records that ruling without adding a
compatibility exception.
