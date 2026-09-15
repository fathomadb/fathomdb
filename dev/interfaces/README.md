# Interface Contracts

## Purpose

Internal interface contracts for Rust, Python, TypeScript, CLI, and wire-level
surfaces.

## Keep here

- per-surface contract docs
- internal naming and shape commitments
- binding contract notes that inform implementation

## Do not keep here

- generated API reference
- public tutorials
- implementation details better owned by `dev/design/`

## Canonicality

Canonical for internal interface intent until replaced by shipped public docs.

The Rust, Python, TypeScript, CLI, and wire records are maintained for the
0.8.26 source candidate. Section-local status remains controlling: implemented,
accepted, proposed, and unsigned sections can coexist, and this index does not
promote proposed material. The active architecture is
[`fathomdb-data-plane-architecture-v2.md`](../design/fathomdb-data-plane-architecture-v2.md).

## Lifecycle

Revise when binding or API contracts change.
