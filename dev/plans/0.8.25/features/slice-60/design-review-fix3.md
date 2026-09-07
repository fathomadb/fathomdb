---
title: 0.8.25 Slice 60 design review — FIX-3 response
status: FIX3_AWAITING_REVIEW
review_cycle: 3
candidate: 814049043a2137c7ad47bd62ff058e8bbe82017d
design_version: 5
---

# Slice 60 design review — FIX-3 response

## Disposition

Design v5 resolves exactly the three Cycle-3 findings without widening the
public scope or authorizing implementation.

| Finding | FIX-3 disposition |
| --- | --- |
| P1 edge temporal authority and node relaxation | Restores the shipped edge admission rule: every edge is nonsuperseded; independently, its `t_invalid` is absent or strictly later than the effective instant. `t_valid` is provenance and never gates. Explicit/query seeds, intermediate nodes, and outputs use the effective node `ReadView`; only their validity-window predicate is dropped by `include_out_of_window`. The temporal matrix pins null/equal/earlier/later `t_invalid`, future `t_valid`, both relaxation modes, both contexts, and all directions. |
| P2 malformed native response mapping | Reuses the operation's `GraphExpansionError` family and `FDB_GRAPH_EXPANSION` code. Unsupported response schemas use `unsupported_schema_version`; every other malformed response uses `graph_corrupt`; Python/TypeScript expose exact reason and camel-case path. Shared Rust-wire/Python/TypeScript fixtures assert the exception class, code, reason, and path and exclude generic argument/value errors. |
| P2 achievable permutation oracle | Restricts canonical byte identity to edge-insertion-only permutations with fixed node rows/cursors, explicit seeds, fixed current context, and explanation disabled. Query seeds and shuffled node writes are explicitly not byte-identity oracles. |

## Final review gate

The design and plan are `FIX3_AWAITING_REVIEW`. Independent Cycle 4 is the
final review allowed by the four-cycle cap and must pass before `READY`. No HITL
or repository-authority choice remains unresolved.
