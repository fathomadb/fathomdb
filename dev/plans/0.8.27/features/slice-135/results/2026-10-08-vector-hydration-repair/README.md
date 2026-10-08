---
title: Slice 135 vector hydration row-error repair
status: CONFIRMED_DEFECT_REPAIRED_FOCUSED_GATE
target_release: 0.8.27
---

# Vector hydration must propagate malformed-row errors — 2026-10-08

The [real-database regression](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_vector_hydration_error.rs)
writes and projects one vector-bearing canonical node, checks that search
returns it, closes the engine, and changes only that row's `kind` to a SQLite
blob. The fixture checks `typeof(kind) = 'blob'` before reopening. Binary
vector search still selects the row, but its canonical hydration cannot decode
the blob as `String`. The public search must return `EngineError::Storage`
instead of a successful incomplete result.

The staged test was run against pre-fix source
`e8606b8d38ce73a8a6a8579b4cc937961d04c9ff`, whose `search.rs` SHA-256
was `170c07b70df7e8a2c9a4075915f82fcdb07dc761a5533c86e08bed57d306bef0`.
The [RED command](red-command.json) exited 101: [stderr](red.stderr) shows
`Ok(SearchResult { ... results: [] ... })`. The fix changed both canonical
node and edge hydration lookups to treat only `QueryReturnedNoRows` as an
absent candidate and to propagate other SQLite errors. The repaired
`search.rs` SHA-256 is
`eaa3ebb355b579b81a135ef37b8b605cbd1582595e0876db04e62c42ec4f475e`.
The [GREEN command](green-command.json) passed the same test source, SHA-256
`9e626dd33b90fed6b7633194879f2cd4ea0a9f81ffd0a52beec246b5a035d58e`.
The adjacent vector row-error and filtered-KNN suites passed seven tests;
`cargo fmt --all -- --check` and focused engine Clippy passed. These focused
checks do not replace the final full workspace gate or prove every malformed
node/edge condition.

The repair changes candidate engine bytes after the E01–E12 and installed
S01/S02 timing refreshes. Their exact-source receipts remain diagnostics;
affected final-candidate cells must be rerun with rebuilt artifacts.
