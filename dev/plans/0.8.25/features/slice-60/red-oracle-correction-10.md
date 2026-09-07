# Slice 60 RED-oracle correction 10 — nonterminal closure control

The FIX-4 cross-owner fixture attempted to create its nonterminal closure by
calling public `erase_source`. That verb correctly drives an immediate physical
closure completion path, so it cannot witness an active `proving` barrier while
the independently owned derived row remains available for the read-side check.

The corrected fixture uses a test-hooks-only persisted-state control that calls
the internal soft-closure admission path with `SoftDeleted` and `Proving` for
`source-r1`. It asserts the durable closure row before observing the surviving
derived row and graph-expansion barrier. Public `erase_source` and
`excise_source` controls remain separately exercised and unchanged.

| Path | Old SHA-256 | New SHA-256 |
| --- | --- | --- |
| `src/rust/crates/fathomdb-engine/tests/slice60_fix4_dependency.rs` | `d4ae7bdba84c02f2c6e97e38423a75353d9ef9e3264ae76a2014cc8986ea5805` | `87f45913bbd05aec5c0667e96c9e9f4c95d982e6979d948579308c5839f563a9` |
