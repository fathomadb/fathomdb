---
title: Vector Subsystem Design
date: 2026-04-30
target_release: 0.6.0
desc: vec0 storage, BLOB encoding boundary, and vector recovery semantics
blast_radius: sqlite-vec integration; REQ-011, REQ-025c, REQ-040, REQ-044, REQ-051
status: locked
---

# Vector Design

This file owns the current vec0 storage boundary, encoding invariants, profile
metadata relationship, query snapshot, erasure, and rebuild-from-canonical
semantics. Embedder identity and runtime selection remain in `embedder.md`;
projection generation/readiness remains in `projections.md`; ranking and hybrid
composition remain in `retrieval.md`.

## Storage shape

`_fathomdb_embedder_profiles` stores the configured profile name, immutable
embedder name/revision, dimension, and optional little-endian f32 workspace
mean. `_fathomdb_vector_rows` is the ordinary-SQL sidecar mapping one vec0 row
ID to its kind and unique canonical write cursor.

The engine creates the dimension-aware `vector_default` vec0 table. Its current
shape is:

- `embedding float[dimension]` for the uncentered f32 vector;
- `embedding_bin bit[dimension]` for the mean-centered sign bits;
- `source_type TEXT partition key`;
- indexed metadata `kind`, `created_at`, and `status`; and
- one `attr_<lowercase UTF-8 hex> TEXT` metadata column for each declared
  filterable projection attribute.

The schema crate owns the ordinary profile/sidecar tables. The engine owns the
runtime-dimension vec0 DDL because SQLite virtual-table construction depends on
the admitted profile dimension. `ADR-0.8.11-filter-grammar-unification.md`
owns which public filters may lower to the indexed metadata columns.

## Encoding and publication

Persisted f32 vectors and the optional workspace mean use little-endian f32
bytes. The binary candidate representation is the sign quantization of
`embedding - mean`, not raw-sign quantization. A projection worker publishes
the sidecar and vec0 row under the shared commit gate and the serving-generation
checks defined in `projections.md`; a terminal vector projection is not ready
unless its sidecar and vec0 rows agree with the canonical write cursor.

## Query snapshot

The reader validates the projection registry and compiles any filterable
attribute columns on the same SQLite snapshot used for the vec0 query. The
vector stage collects a bounded bit-distance candidate set from
`embedding_bin`, then reranks those candidates with the stored f32 embedding.
Post-vector recency or confidence policy is retrieval-owned and is not a vec0
predicate.

## Erasure and recovery

Canonical purge and source excision delete both `_fathomdb_vector_rows` and the
corresponding `vector_default` row before reporting erasure complete. The
dimension/profile metadata is not reconstructed from a vector shadow row.

`rebuild_vec0` treats vector state as derived: it truncates vec0 plus sidecar
state and republishes eligible canonical bodies through the governed projection
path. It does not rewrite canonical nodes/edges or invent missing provenance.
Open-time integrity handling may prune a vec0 row that has no sidecar after a
known historical schema transition; generalized silent repair is not allowed.

## Verification witnesses

- Schema/profile/sidecar: `src/rust/crates/fathomdb-schema/src/lib.rs`.
- vec0 DDL, encoding, query, erasure, and rebuild:
  `src/rust/crates/fathomdb-engine/src/lib.rs`.
- Shape and encoding: `vector_quant_pack1.rs` and `pr2b_mean_recompute.rs`.
- Snapshot/filter lowering: `slice15e_prekn_filterable.rs` and
  `slice45_nested_source_projections.rs`.
- Erasure/rebuild: `excise_source.rs`, `opp12_lifecycle_verbs.rs`, and
  `erasure_projection_registry.rs`.
