# Slice 135 node FTS row-error repair — 2026-10-07

**Status:** confirmed defect repaired at
`ae6127ee3dbbf44e247acc77e545f34a22137ef5`. This is a logic/exception
result, not the full Phase 1 checkpoint. The package version string is still
0.8.26; the Git SHA and `Cargo.lock` SHA-256
`9e9d7b5e82184a0bddfbe96de28fa0615ef4443fc84639b271c1cff29600ccfe`
identify this candidate source.

The permanent [real-database regression test](test-source.rs) writes two nodes
and a body-bearing edge, verifies a node text hit, closes the engine, changes
the matching node FTS `kind` to invalid UTF-8 BLOB `x'ff'` through SQLite,
verifies FTS still matches and an edge FTS row exists, then reopens. Before the
fix, the search returned `Ok` with an empty result list and the test failed as
shown in the [RED log](slice135-node-fts-red.log). The source change replaces
the modern node FTS `rows.flatten().collect()` with fallible collection so the
row conversion error reaches `EngineError::Storage`.

The [GREEN log](slice135-node-fts-green.log) is from the exact committed SHA.
The new test, adjacent edge FTS regression, search validity, text limit prefix
stability and temporal edge validity suites passed: 13 tests across five test
targets. The [GNU Time report](slice135-node-fts-green-resource.txt) records
the focused run's resource use; it is not a latency comparison. The new test
source SHA-256 is
`b70d82e21fb4370f88019b777a2279fd69a8b27588622b1e4a8c6120ab2fac19`.
The repaired `search.rs` SHA-256 is
`9bf135a69951e24084c8ee371bf9a481a1b3ac92a13faebbc36b4e41ea921c05`.

The other `rows.flatten()` sites in `search.rs` remain audit leads. No other
site is classified by this result. Earlier paired candidate timing and
installed-artifact receipts bind to older source bytes and need rebuilding and
rerunning before they can qualify the final candidate.
