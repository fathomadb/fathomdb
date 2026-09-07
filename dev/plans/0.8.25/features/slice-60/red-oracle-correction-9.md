# Slice 60 RED-oracle correction 9 — cross-owner dependency fixture

The FIX-4 dependency oracle originally attempted to create its derived node
through the public `Engine::write` path with a different owner than its
canonical source. That path correctly rejects the input with
`source_mismatch` at `/provenance/sourceRevisionId`; the rejection is an
existing public provenance contract, not a product defect to weaken.

The corrected fixture preserves the intended closure observation by writing
only valid public root/source/edge inputs, then calling the test-hooks-only
real-SQL setup route. That route creates the active cross-owner derived node,
revision, and source link without dependency registration. The test can then
observe unregistered and registered states and the nonterminal closure barrier
without changing `Engine::write` validation.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix4_dependency.rs` | `ac0b67e419e4a6dfe8b6f2236d3d94c44191129cde27f1c3db06222a37c7a7e5` | `d4ae7bdba84c02f2c6e97e38423a75353d9ef9e3264ae76a2014cc8986ea5805` |
