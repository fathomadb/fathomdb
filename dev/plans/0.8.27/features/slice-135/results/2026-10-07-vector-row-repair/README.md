# Slice 135 vector row-error repair — 2026-10-07

**Status:** confirmed defect with a test-first repair committed as
`3f29d649d0213e595c0dab251a449d92fd625792` on the Slice 135 branch.
The RED source is exact candidate
`cdf253cd223a82e954591db532397a3d78a2027a`; the GREEN product source
has `search.rs` SHA-256
`9670e142e7c3097b431500108be99e2296e9e63fccc3fc362644685c557ca142`.
This commit is not yet the frozen final checkpoint candidate. Earlier
candidate timing receipts describe their exact source bytes and require an
affected-cell refresh with rebuilt installed artifacts.

## Contract and fault fixture

The permanent [real-database regression test](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_vector_row_error.rs)
writes and projects one vector-bearing node, verifies it is returned, closes
the engine, and damages only the vec0 full-precision rerank chunk through an
independent SQLite connection. It checks that the physical chunk is damaged
and reopens the engine. Binary KNN still selects the candidate. A successful
empty result would conceal a storage failure and falsely imply that no
matching record exists; the expected public result is `EngineError::Storage`.

The same test source, SHA-256
`6ce45d28aafcd922870a0e398b6223967ad25b48b93ee2b31cba621e52708a93`,
was run against the clean RED checkout and the repaired worktree. The
[RED command](red-command.json) exited 101. Its [stderr](red-cdf253.stderr)
records `Ok(SearchResult { ... results: [] ... })` at the assertion; the
[stdout](red-cdf253.stdout) records the failed test. The [GREEN command](green-command.json)
exited 0, and its [stderr](green-current.stderr) records a fresh
`fathomdb-engine` compilation. The [stdout](green-current.stdout) records
one passing regression test. The focused adjacent `pr_g10_filtered_knn`
suite also passed six tests in the worktree. `cargo fmt --all -- --check` and
`git diff --check` passed after formatting the new test.

The production change in [search.rs](../../../../../../../src/rust/crates/fathomdb-engine/src/search.rs)
propagates each phase-1 vector cursor row error instead of using
`rows.flatten()`, which discarded `Err` values before result hydration.
It does not change the normal successful vector result path.

## Attempt and limits

An initial fixture tried `UPDATE vector_default SET embedding=NULL`; vec0
treated that update as unchanged, so it could not reach the row decoder. The
retained RED run uses the full-precision shadow chunk instead. An initial
GREEN attempt reused a stale Cargo test binary from a shared target directory
and was invalid. Touching the product source mtime forced recompilation for
the retained GREEN run; the source bytes did not change. Neither invalid
attempt is counted as evidence of product behavior.

This is one confirmed vector fault path, not proof that every malformed vec0
row or other `rows.flatten()` site is handled. A current-source full gate and
the affected paired latency refresh remain open. The [SHA-256 manifest](SHA256SUMS)
covers the retained commands and raw logs.
