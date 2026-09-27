---
title: FathomDB 0.8.27 Slice 80 - TDD chronology
status: IN_PROGRESS
started_on: 2026-09-27
baseline_sha: bb077cfa
---

# Slice 80 TDD chronology

## Baseline and method

Production source at commission is byte-identical to `bb077cfa`; the commits
after it are Slice 80 planning and review records only. Slice 80 is a structural
move and fixes no known behavior defect.

Every new characterization follows this recorded sequence:

1. add the test without changing production and show it GREEN against the
   shipped behavior;
2. apply only the temporary mutant named in `design.md` and show the test RED
   for the intended assertion;
3. restore production exactly, show the focused test GREEN again, and confirm
   no mutant diff remains.

If a new test is RED against unmodified production, that is a behavioral defect,
not characterization. Stop the move, preserve the RED oracle, implement the
smallest GREEN correction, and obtain review before resuming. Structural moves
must first reach compile RED on the moved boundary and then GREEN through only
the approved imports, visibility, re-exports, and path retargets.

## Existing owner map

The exact named owner suites are in `design.md` under "Characterization" and
are mapped as follows:

| Contract | Existing owners and added proof |
| --- | --- |
| R27-80B snapshot and transaction lifetime | Existing snapshot-race, linearization, frozen-read, and reader-pool owners; add the bounded reader-refusal release characterization. |
| R27-80C eligibility before caps | Existing search pretruncation, graph frontier, page, eligibility SQL, and query-plan owners; no duplicate test. |
| R27-80D ordering and fusion | Existing RRF, three-arm, reweight, tie-order, prefix-stability, and graph byte-determinism owners; no duplicate test. |
| R27-80E codec stability | Existing graph/evidence/frozen/pagination/trace fixtures and property tests; no generated oracle. |
| R27-80F view and filter contract | Existing hybrid, text-only, filtered, explained, existence-refusal, filter-grammar, and unification owners; add graph-arm and projected-text validity characterizations. |

## Characterization RED/GREEN evidence

### Reader refusal releases every worker transaction

- Test: `tests/slice80_reader_transaction_release.rs`.
- Baseline: eight typed in-transaction refusals, with per-dispatch modulo
  progression proving one request reached each of the eight workers, followed
  by bounded erasure completion.
- RED mutant: omit rollback/release on the selected refusal path. The bounded
  erasure assertion must fail or time out for the intended WAL-retention reason.
- GREEN restoration: exact source restoration returned the focused test to
  1/1 passing.

### Graph-arm validity path

- Test: `tests/slice80_search_view_paths.rs`.
- Baseline: the graph neighbor is hidden by the default view, visible with
  `include_out_of_window`, visible at an in-window `valid_as_of`, and existence
  relaxation is refused with `InvalidArgument`.
- RED mutant: remove both neighbor-validity sites identified in `design.md`
  while keeping their parameters referenced. The default-view absence
  assertion must fail.
- GREEN restoration: exact source restoration returned the focused test to
  1/1 passing.

### Projected-text validity path

- Test: `tests/slice80_search_view_paths.rs`.
- Baseline: the projected-text hit has the same default exclusion,
  validity-relaxation inclusion, in-window inclusion, and typed
  existence-refusal behavior.
- RED mutant: remove the single projected-text validity site identified in
  `design.md`. The default-view absence assertion must fail.
- GREEN restoration: exact source restoration returned the focused test to
  1/1 passing.

### Observed chronology

The tests were added against unmodified production. The initial focused run
passed 3/3:

```text
cargo test -p fathomdb-engine \
  --test slice80_reader_transaction_release \
  --test slice80_search_view_paths -- --test-threads=1
```

Each mutant changed only `src/rust/crates/fathomdb-engine/src/lib.rs` and was
restored immediately after its focused run:

| Test | Temporary mutant | RED evidence |
| --- | --- | --- |
| `projected_text_applies_the_search_validity_view` | Replaced `frozen.node_sql("n", 3)` with existence plus dependency eligibility, omitting only validity. | Failed `default view must hide the expired projected hit`; 0 passed, 1 failed. |
| `graph_arm_applies_the_search_validity_view_to_reached_neighbors` | Removed both neighbor-validity sites: `target_node` kept existence/dependency eligibility and `body_validity` became empty. | Failed `default view must hide the expired neighbor`; 0 passed, 1 failed. |
| `in_transaction_refusal_releases_every_reader_snapshot` | On the undeclared projected-field refusal, deliberately forgot the live reader transaction. | Eight typed refusals reached eight distinct workers, then `erase_source` failed with bounded `ErasureIncomplete { stage: "wal_checkpoint" }`; 0 passed, 1 failed. |

After exact restoration, `git diff -- src/rust/crates/fathomdb-engine/src/lib.rs`
was empty and the combined focused run passed 3/3 again. No production behavior
change was required.

## Structural batches

All batches were mechanical moves from `bb077cfa`. The compile boundary after
each move was closed only with imports, the visibility seams below, root
re-exports, and path-only gate retargets.

| Batch | Commit | Destination and focused evidence |
| --- | --- | --- |
| 1 | `02a848f6` | `fusion.rs`; default and hook checks, focused fusion owners, C1, WAL guard, plan anchors, and AC-050c passed. |
| 2 | `71598dae` | filter carriers in `filter.rs`; the same per-batch gates passed. |
| 3 | `695079db` | filter execution in `filter.rs`; active plan anchors were retargeted and passed. |
| 4 | `58972419` | `search_types.rs`; C1 first went RED because its `SearchHit` probe still named `lib.rs`, then GREEN after the probe and self-test fixture arms 12p/12w were retargeted. |
| 5 | `885b0222` | search helpers in `search.rs`; focused search routes and all source gates passed. |
| 6 | `2adf2577` | search graph arm in `search.rs`; graph-frontier and search routes passed. |
| 7 | `3e4c55eb` | `read_search_in_tx` in `search.rs`; focused search, view, and eligibility routes passed. |
| 8 | `52e24968` | frozen/evidence search facade in `search_api.rs`; focused frozen/evidence routes passed. |
| 9 | `d187af8e` | remaining search facade plus `telemetry.rs`; the active plan anchor was retargeted. Midpoint public and hidden captures compared exactly equal to the pre-move captures. |
| 10 | `811ca9e2` | `read.rs`; focused read, canonical-page, and operational-state routes passed. |
| 11 | `bd3eb94b` | `reader_pool.rs`; the Windows WAL guard was retargeted through `READER_POOL_SOURCE` and its recursive fixture passed 313/313. |
| 12 | `2a36a242` | graph directory split and `types.rs`; 37 Slice 60 integration tests and the two unchanged qualified unit-test names passed. C1 self-test and WAL guard passed. |
| 13 | `a125f7b1` | request codec in `codec.rs`; 16 wire tests, C1 self-test, and WAL guard passed. |
| 14 | `51791245` | result codec in `codec.rs`; 16 wire tests, crate Clippy with warnings denied, C1 self-test, and WAL guard passed. |
| 15 | `f333926e` | `traversal.rs`; 41 graph/traversal/view owners passed. The first hidden capture exposed a missing path-only `tc5-benchmark` sibling import; that route was RED, the gated import made it GREEN, and the batch was amended before the final capture. |

The repeated path-sensitive gates stayed GREEN after their owning retargets:
C1 conformance proved all 26 checkable clauses, its recursive self-test passed,
the Windows WAL attribution fixture passed 313/313, the Slice 35 virtual
mutation manifest passed, `slice60_fix1_wire` passed 4/4, plan anchors verified
22 citations, and AC-050c reported no unrecorded public removal against
`bb077cfa`.

### Visibility seam record

- The reader-pool methods and `begin_attributed_reader_tx` became
  `pub(crate)` exactly as predeclared in `design.md`; its data carriers and
  private worker helpers did not widen.
- Root consumers required crate-visible filter SQL/post-filter helpers, the
  importance-map helper, read in-transaction helpers, search capture and
  execution helpers, search-inner methods, the telemetry sink, and the five
  traversal entry points. Their private fields did not widen beyond the
  predeclared search capture carriers.
- The graph split required only sibling visibility:
  `encode_graph_evidence_request`, `GraphExpansionErrorV1::new`,
  `direction_str`, and `parse_canonical_u64` became `pub(super)`. The existing
  test-hook controls and `SCHEMA_VERSION` retained or narrowed to their
  directory-module ownership. Public graph/search types remain root re-exports.
- No reader carrier field, public contract, SQL, feature gate, wire carrier, or
  runtime-state shape changed.

### Final implementation receipts

- Ten explicit no-default/default feature routes passed: engine default,
  `test-hooks`, `slice72-test-hooks`, `operator,test-hooks`,
  `migration-test-hooks`, `tc5-benchmark`, `default-reranker`, and
  `default-embedder`, plus facade default and facade `operator`.
- Public capture at `f333926eec5454a04ab29e561e0c25f88e22aced` contains
  13 rows. It compares exactly equal to both the characterization capture and
  the Slice 30 baseline: no metadata or row differences.
- Hidden capture at the same SHA contains 33 rows. It compares exactly equal
  to the characterization capture. Against the tracked hidden baseline it is
  additive only: no metadata differences and no changed or removed entries.
  The two graph evidence unit tests retain their original qualified names.
- The clean implementation candidate before review is
  `f333926eec5454a04ab29e561e0c25f88e22aced`.

## Review and final verification

Implementation is ready for review. Independent read-only code review by a
`gpt-5.6-sol` subagent at high reasoning and final verification by a separate
`gpt-5.6-terra` subagent remain pending. Their durable records will be
`code-review.md` and `review-verification.md`; neither record is authored by
the implementer.
