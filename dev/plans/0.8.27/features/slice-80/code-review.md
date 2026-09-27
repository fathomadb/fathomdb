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
| 1 | P2 | Moving `begin_attributed_reader_tx` into `reader_pool` created unapproved `read` ↔ `reader_pool` and `graph_expand` ↔ `reader_pool` cycles: handlers called the pool-owned helper while the pool dispatched to those handlers. | **Historical resolution at review time; corrected below.** Moved the helper body unchanged back to the crate root and restored private visibility. `reader_pool` now depends on the read, search, and graph handlers; those handlers depend only on the root primitive. The review then claimed the only retained module cycle was `search` ↔ `graph_expand`; the post-hoc correction below records why that claim was false. |
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

> **Post-hoc correction (2026-09-27).** The post-hoc adversarial design
> review found that the finding 1 resolution overstated its effect. Moving
> `begin_attributed_reader_tx` back to the root removed the *handler*-level
> dependency on `reader_pool`. At module level, however, the `impl Engine`
> facades that share modules with the handlers still close three cycles:
> `read` ↔ `reader_pool`, `graph_expand` ↔ `reader_pool`, and
> `graph_expand` ↔ `search_api`. So `search` ↔ `graph_expand` is not the only
> retained module cycle. `design.md` now states the invariant at
> facade-vs-handler granularity. The earlier Slice 90 allocation is superseded:
> Slice 85 must eliminate all four cycles, including
> `search` ↔ `graph_expand`. Only the three inherited earlier-slice cycles
> named in `design.md` are initially eligible for narrow allowlisting.
>
> The finding 2 closure was also incomplete. Six items stayed wider than
> their users required, and four struct fields had widened
> (`TelemetrySink.path` and the three `EvidenceCapture` fields). Fix-1
> (`b8af4d86`) narrows the items and restores the private fields. The table
> above is left as originally recorded.

Both review findings are **CLOSED**. Independent final verification ran as a
separate gate and is recorded in `review-verification.md`; it is not part of
this review verdict.

## Post-hoc adversarial code review (2026-09-27) — PASS-WITH-FIXES

An independent read-only review of `9a31e979..c152d74b` (`src`, `scripts`;
also covering docs-only `bd1347d7`) found no P0-P2 defects. Workspace Clippy
and check, engine Clippy on `test-hooks`, `operator,test-hooks`, and
`tc5-benchmark`, the engine lib tests, the Slice 80, Slice 35, and Slice 60
wire tests, the WAL guard (314/0), and C1 (26/26) passed. Name resolution,
statics, module-path-dependent output, doc/derive attachment, and
`#[doc(hidden)]` counts are unchanged from `bb077cfa`; rustdoc warnings are
identical at base and head.

| # | Severity | Finding | Resolution |
| --- | --- | --- | --- |
| 1 | P3 | `search_inner_with_frozen_binding_and_stats` was `pub(crate)` although private at `bb077cfa` and used only in `search_api.rs`. | Made private. |
| 2 | P3 | `read.rs` cited a `:4170` line anchor that was already wrong at baseline and cannot resolve. | Anchor removed. |

Both fixes are mechanical; workspace Clippy, the three engine feature-set
Clippy runs, `cargo fmt --check`, and the engine lib tests (78 passed)
passed after them, so no further cycle was run.

## Follow-up review boundary

Commit `9700991f` adds the missing graph-result codec properties and changes no
production code; `31e78529` is its lint-only test-helper follow-up. The two
temporary production mutants and focused GREEN are recorded in
`tdd-chronology.md`. This existing code-review verdict predates both commits
and therefore does not review or bind them. Independent rereview is
required before `review_fix_candidate`, Slice 80 `sha`, `reviewed_candidate`,
or `closeout_sha` is updated.
