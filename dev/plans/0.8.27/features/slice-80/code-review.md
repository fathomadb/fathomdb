---
title: FathomDB 0.8.27 Slice 80 - code review
status: PASS
target_release: 0.8.27
reviewed_range: bb077cfa..ac404a81
review_fix_candidate: 8e4499637e9d40ac6fcb9579f352b9f643e86709
---

# Slice 80 code review

The reviewer was an independent, read-only `gpt-5.6-sol` subagent at high
reasoning. It reviewed the commissioned plan, design, implementation commits,
and clean candidate `ac404a81`. Its initial verdict was **FAIL**, with two P2
architectural findings. Both findings are closed by `8e449963` without a
behavioral change.

## Findings and resolutions

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P2 | Moving `begin_attributed_reader_tx` into `reader_pool` created unapproved `read` ↔ `reader_pool` and `graph_expand` ↔ `reader_pool` cycles: handlers called the pool-owned helper while the pool dispatched to those handlers. | Moved the helper body unchanged back to the crate root and restored private visibility. `reader_pool` now depends on the read, search, and graph handlers; those handlers depend only on the root primitive. The only retained module cycle is the approved `search` ↔ `graph_expand` seam. The plan and design inventory now say so explicitly. |
| 2 | P2 | Several helpers and graph-module aliases were widened to `pub(crate)` even though their users did not require crate-wide visibility. | Same-file search and filter helpers are private. `SCHEMA_VERSION` and `graph_expansion_degradation_codes` are `pub(super)` and their implementation helper is private. The graph directory aliases used only by the crate root are `pub(super)`; type aliases used only inside the directory are private. The root graph-handler import is private. |

The execution and traversal definitions re-exported to the crate root remain
`pub(crate)`: making those definitions `pub(super)` produced compile RED
(`E0364`) because Rust does not permit a parent module to re-export a child
item beyond the item's own visibility. Their `graph_expand/mod.rs` aliases are
`pub(super)`, so the reachable seam is still limited to the crate root.

## Verification of the fixes

- Focused reader, traversal, graph-expand, transaction-release, and
  search-view suites: 47 passed, 0 failed.
- Ten affected engine/facade feature routes compiled, including
  `test-hooks`, `slice72-test-hooks`, `operator,test-hooks`,
  `migration-test-hooks`, `tc5-benchmark`, `default-reranker`, and
  `default-embedder`.
- Engine Clippy passed with warnings denied on the
  `test-hooks,migration-test-hooks` and `tc5-benchmark` routes.
- C1 conformance passed 26/26 and its recursive self-test passed. The Windows
  WAL attribution fixture passed 313/313. The Slice 35 virtual-mutation
  manifest passed, `slice60_fix1_wire` passed 4/4, plan anchors passed, and
  AC-050c found no unrecorded public removal against `bb077cfa`.
- Public capture at `8e4499637e9d40ac6fcb9579f352b9f643e86709`
  contains 13 rows and is exactly equal to the pre-move capture, the original
  implementation candidate, and the tracked Slice 30 baseline.
- Hidden capture at the same SHA contains 33 rows and is exactly equal to the
  pre-move capture and original implementation candidate. Against the tracked
  `8e2afb29` hidden baseline it has 261 additions, 0 changes, and 0 removals.

Both review findings are **CLOSED**. Independent final verification remains a
separate gate and is not part of this review record.
