---
title: Actuation Subsystem Design
date: 2026-09-15
target_release: 0.8.26
desc: Atomic caller-decided semantic batches, prospective validation, compact receipts, replay, and lifecycle effects
blast_radius: fathomdb-engine actuation; Rust/Python/TypeScript bindings; schema-34 receipt tables
status: ACTIVE
---

# Actuation Design

This file owns the current implementation design beneath
`ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md` and the public
actuation contracts in `dev/interfaces/`. It does not own caller truth policy:
the caller selects the semantic operations; FathomDB validates and commits
their declared durable effects.

## One changed-in-place V1 grammar

`ActuationBatchV1` contains 1–128 ordered operations and one caller-owned
`operation_id`. `ActuationOperationV1` has exactly five variants:

- `PutCanonicalNode`;
- `PutDerivedNode`;
- `PutDerivedEdge`;
- `RegisterSourceDependency`; and
- `TransitionLifecycle`.

There is no V2 type, parser, method, router, redirect, or historical V1 reader.
The current request, digest, receipt, replay, and integrity rules apply only to
fresh schema-34 databases.

## Admission and prospective validation

Construction rejects malformed IDs, unsupported schema versions, and operation
counts outside 1–128 before writer admission. Engine admission then validates
the complete request before applying domain effects.

A derived edge's endpoints must be active in the complete prospective batch
state, independent of operation position. The prospective state includes
existing current rows plus all applicable puts and lifecycle transitions in
the request. This rule prevents order-sensitive acceptance without permitting
dangling governed edges.

Domain refusals produce a bounded terminal `refused` receipt with a closed
reason and JSON-pointer field path. Infrastructure, closing, storage, digest
collision, and erased-operation tombstone failures remain typed errors.

## Atomic writer transaction

One writer transaction owns validation, canonical node/edge writes, dependency
registration, lifecycle transitions, projection scheduling state, source
references, and terminal receipt persistence. A fault before commit leaves no
canonical, dependency, projection, or receipt subset. Commit failure is an
infrastructure error, not a terminal domain receipt.

Derived edges reuse the canonical edge writer, revision identity,
supersession, erasure, and projection mechanisms. Actuation does not implement
a second edge store or a private lifecycle path.

## Digest, receipt, and replay

The request digest is deterministic and operation-order-sensitive. An exact
`operation_id` plus digest replay returns the stored terminal receipt without
repeating domain work. The same ID with a different digest fails as a conflict.

`ActuationReceiptV1` is compact and bounded. It records the request digest,
terminal outcome/reasons, affected immutable revision IDs, resulting write and
dependency-generation boundaries, pending projection cursors, projection
generation identity, and any dependency-closure operation IDs. Receipt load
recomputes request-derived consequences and fails closed if affected revisions,
source references, projection correlation, bounds, or ordering are incoherent.

Source erasure or purge redacts receipt content and source-reference rows but
retains an opaque operation-ID tombstone. The ID therefore remains permanently
reserved without retaining erased evidence.

## Projection and lifecycle effects

Committed body-bearing nodes and edges use the ordinary projection scheduler.
Pending cursors and generation identity in the receipt support receipt-keyed
readiness without making the receipt a work queue. Supersession and deletion
reuse canonical lifecycle rules; dependency closure remains engine-owned and
is reported through bounded closure IDs when asynchronous closure is required.

## Verification witnesses

- Core implementation: `src/rust/crates/fathomdb-engine/src/actuation.rs`.
- Shared wire fixture: `dev/fixtures/slice35-actuation-conformance-v1.json`.
- Rust transaction/replay/property coverage:
  `slice25_actuation.rs`, `slice25_actuation_verification.rs`, and
  `slice35_actuation_spike.rs`.
- Binding parity: `src/python/tests/test_slice25_actuation.py` and
  `src/ts/tests/slice35-actuation.test.ts`.
- Fresh-database boundary: `slice40_fresh_database_cutover.rs`.

## Explicit non-goals

This design does not add multi-source liveness grammar, caller-independent
semantic judgments, historical receipt compatibility, database migration,
generalized repair, graph continuation/full paths, or publication behavior.
