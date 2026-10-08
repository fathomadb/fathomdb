---
title: Slice 135 edge explanation row-error repair
status: CONFIRMED_DEFECT_REPAIRED_FOCUSED
target_release: 0.8.27
---

# Edge explanation row-error repair

The opt-in `search_explained` attribute-filter route promises an exact
`dropped_edge_hits` count. On the post-Slice-132 candidate, its count query
ignored statement/query failures and used `rows.flatten()` to discard row
decoding failures. A failed count could therefore return a successful search
with a plausible zero instead of a truthful error.

The [real-database regression test](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_edge_explanation_row_error.rs)
first establishes that one valid edge produces count 1. A `test-hooks`-only
SQL projection then casts that count cursor to BLOB without changing the
persisted corpus, so the same query's `row.get::<i64>` fails. Before the
repair, the assertion for `EngineError::Storage` failed
([RED stdout](red.stdout), [stderr](red.stderr), exit 101). The repaired
[search path](../../../../../../../src/rust/crates/fathomdb-engine/src/search.rs)
propagates statement, query and per-row errors. The focused test passed
([GREEN stdout](green.stdout), [stderr](green.stderr)); the existing valid
edge-count case also passed ([stdout](adjacent.stdout),
[stderr](adjacent.stderr)). The new target compiled with default features
and ran zero tests there because the injection is feature-gated
([stdout](default.stdout), [stderr](default.stderr)). Focused engine Clippy
with `test-hooks,default-embedder`, all targets and warnings denied passed.

Earlier attempted fixtures that changed persisted edge cursor types were
invalid discriminators: normal reopen rejected corruption, and an FTS update
could not be installed through the test seam. Their failures were not counted
as RED evidence. The final hook isolates only the explanation cursor decode;
it does not substitute for a persistent-fault/reopen test.

The final `search.rs` SHA-256 is
`63dec070ff87eaa37225c64aa6238c635ea6fcedec2aebf66b6c0318d7a06383`;
the test SHA-256 is
`1d435a7dcc497d62c9c201526bbf9afc682c57dea11df423542b211c6dad097a`.
The product source bytes changed from the earlier measured candidate
`3f29d649d0213e595c0dab251a449d92fd625792`. Prior latency receipts
remain evidence for their exact source, but cannot qualify the final
post-repair candidate without a source-bound refresh. The full repository
gate remains open.
