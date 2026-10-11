# Slice 135 edge FTS error repair — 2026-10-07

**Status:** confirmed defect repaired on branch commit
`9186eb3d09b430bfd31b6dded634cca7daec8df3`. This is a follow-up to the
[first logic result](../../logic-first-result-2026-10-06.md), not the full Phase 1
checkpoint or a new latency comparison. The repair changes engine bytes, so
the earlier paired 0.8.26/candidate timing receipts cannot qualify this final
candidate without rebuilding and rerunning the affected cells.

## Contract and test-first result

The permanent real-database
[regression test](../../../../../../../src/rust/crates/fathomdb-engine/tests/slice135_edge_fts_error.rs)
writes a body-bearing edge, verifies the text hit, closes the engine, changes
the edge FTS `kind` to an invalid UTF-8 BLOB through an independent SQLite
connection, verifies that the row still matches FTS, and reopens the engine.
Before the product change, search returned `Ok([])` and the test failed. The
[source change](../../../../../../../src/rust/crates/fathomdb-engine/src/search.rs)
collects row results as `rusqlite::Result<Vec<_>>` and propagates a decoding
error through the existing `EngineError::Storage` route. The test then passed
with default features. It also compiled under the release test configuration
in the full typecheck gate.

The focused neighboring engine suites passed with `operator,test-hooks`:
`slice15b_search_validity` (8 tests), `slice20_fts_rank_stream` (3),
`slice23_text_limit_prefix_stability` (2), and
`tc33_fix2_edge_validity_on_search` (1). These check search validity,
rank-stream fallback, result-prefix stability and temporal edge exclusion.
The other six `rows.flatten()` sites in `search.rs` remain separate audit
leads; this test does not classify them.

## Full verifier

`./scripts/agent-verify.sh` ran from the clean committed checkout. Lint,
typecheck, strict security and the Rust workspace test suite passed. The test
gate registered 184 suites, ran 182, passed 180, failed two and skipped two.
The full gate therefore **failed**:

- `test-steward-orient`: the real checkout briefing was 4,353 bytes against
  its 4,096-byte runtime cap; its suite expects up to 5,120 bytes. The exact
  [compressed log](steward-orient.log.gz) is retained. This failure was also present in the
  first-results run.
- Python: 1,582 tests passed, 30 skipped and three failed. One declaration
  comparator expected 1,131 and found 1,132. Two `verify_embed_db` subprocess
  tests could not import `eval` from the checkout's current Python install.
  The exact [compressed log](python.log.gz) is retained. The same three failures were present
  in the first-results run.

The test gate skipped TypeScript because this checkout lacks its installed
dependencies. A Rust pass is not an installed Rust SDK qualification, and the
full gate's two failures remain open. The [SHA-256 manifest](SHA256SUMS) covers
the retained failure logs. An earlier full-gate attempt on a dirty checkout
exhausted disk while building Rust tests; that attempt did not
qualify the gate. Rebuildable Slice 135 Cargo targets were removed, the repair
was committed, and the clean run above completed with stable disk space.
