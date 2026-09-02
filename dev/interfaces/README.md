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

The reviewed 0.8.25 interface deltas are indexed by the
[`design-documentation matrix`](../plans/0.8.25/design-documentation-matrix.md)
and specified in its fourteen slice designs. They are planned successors, not
implemented interface authority: this directory continues to describe shipped
behavior until each owning feature slice lands and updates the applicable Rust,
Python, TypeScript, CLI, and wire record.

## Lifecycle

Revise when binding or API contracts change.
