---
title: FathomDB 0.8.27 Slice 80 - TDD chronology
status: COMPLETE
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

### Post-closeout graph-result codec properties

Follow-up commit `9700991fe648eb8d81efb1cde041a2b1093a3fc0` closes the
result-codec property gap without changing production. The tests live in
`tests/slice60_wire.rs`; their invariants are human-defined and no generated
golden oracle is written. Commit `31e78529` then replaced the eight-argument
test-fixture helper with an input carrier to satisfy Clippy; it changes no test
inputs, assertions, production code, or mutation result.

1. `result_codec_round_trips_coherent_generated_carriers` generates linked
   seed, target, and explanation identities; a nonempty target body; arbitrary
   `u64` write/work values encoded as canonical decimal strings; finite or
   null query scores; and optional valid positional evidence. It decodes,
   encodes, decodes again, and asserts typed equality plus the two canonical
   integer-string fields.
2. `result_codec_rejects_generated_nonzero_first_evidence_position` generates
   a coherent result with evidence, changes only
   `evidence.entries[0].targetIndex` to a generated nonzero `u32`, and asserts
   `GraphCorrupt` at `/evidence/entries/0`.

The exact RED/GREEN chronology was:

| Property | Temporary production mutant | RED evidence |
| --- | --- | --- |
| coherent typed round-trip | In `graph_expand/codec.rs`, changed only `TargetWire.body` from `&target.body` to `""`. | `cargo test -p fathomdb-engine --test slice60_wire result_codec_round_trips_coherent_generated_carriers -- --nocapture` exited 101. Proptest shrank to `body = "0"`; the decoded encoded result had `body: ""` instead of `body: "0"`; 0 passed, 1 failed. |
| positional evidence corruption | In `validate_response_coherence`, removed only `entry.target_index as usize != index` while retaining the schema-version condition. | `cargo test -p fathomdb-engine --test slice60_wire result_codec_rejects_generated_nonzero_first_evidence_position -- --nocapture` exited 101. Proptest shrank to `target_index = 1`; decoding returned `Ok` instead of the required error; 0 passed, 1 failed. |

Each mutant was restored immediately. Proptest's temporary regression file was
deleted rather than committed. `git diff --
src/rust/crates/fathomdb-engine/src/graph_expand/codec.rs` was empty after both
restorations; the restored production file SHA-256 is
`2748b3505d814f6329bcd52ebaf833f223c35f0c77b357dc5deb7783e2166d52`.
The focused GREEN command
`cargo test -p fathomdb-engine --test slice60_wire result_codec_ -- --nocapture`
passed 2/2.

## Structural batches

All batches were mechanical moves from `bb077cfa`. The compile boundary after
each move was closed only with imports, the visibility seams below, root
re-exports, and path-only gate retargets.

| Batch | Commit | Destination and focused evidence |
| --- | --- | --- |
| 1 | `02a848f6` | `fusion.rs`; default and hook checks, focused fusion owners, C1, WAL guard, plan anchors, and AC-050c passed. *Deviation (recorded post hoc, 2026-09-27):* this batch deleted 264 lines from `lib.rs`, below the AC27-80A floor of 300 moved lines. The behavior-preservation evidence is unaffected. |
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

- The reader-pool methods became `pub(crate)` exactly as predeclared in
  `design.md`; its data carriers and private worker helpers did not widen.
  Review moved the shared `begin_attributed_reader_tx` primitive back to the
  crate root, where it remains private and creates no reverse dependency on
  `reader_pool`.
- Root consumers required crate-visible filter SQL/post-filter helpers, the
  importance-map helper, read in-transaction helpers, search capture and
  execution helpers, search-inner methods, the telemetry sink, and the five
  traversal entry points.
- *Correction (post-hoc design review, 2026-09-27).* This record originally
  said private fields "did not widen beyond the predeclared search capture
  carriers". That was false: the design predeclares no field widening, and
  moving `TelemetrySink` to `telemetry.rs` and `EvidenceCapture` to
  `search.rs` widened four fields (`TelemetrySink.path` and the three
  `EvidenceCapture` fields) to `pub(crate)`, violating PW27-4A. Fix-1
  (`b8af4d86`) returned both struct definitions, unchanged, to the crate
  root with private fields. Fix-1 also made six over-visible items private
  again: `capture_telemetry`, `capture_telemetry_with_sink`,
  `mint_explanation_correlation_id`,
  `search_inner_with_frozen_binding_and_expansion`, `direction_str`, and
  `parse_canonical_u64`.
- The graph split required only sibling visibility:
  `encode_graph_evidence_request` and `GraphExpansionErrorV1::new` (called
  from `codec.rs`) became `pub(super)`. `direction_str` and
  `parse_canonical_u64` were also made `pub(super)` but are used only in
  `codec.rs`; fix-1 made them private. The existing
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

## Code-review fixes

The independent `gpt-5.6-sol` reviewer at high reasoning returned **FAIL** on
candidate `ac404a81`, with two P2 architectural findings. Commit `8e449963`
closes both:

1. The `begin_attributed_reader_tx` body moved unchanged from `reader_pool.rs`
   back to root-private ownership. The pool dispatches to read, search, and graph
   handlers; those handlers now depend on the root primitive rather than back
   on the pool. This removes the unapproved `read` ↔ `reader_pool` and
   `graph_expand` ↔ `reader_pool` cycles.
2. Same-file search and filter helpers became private. Graph type seams and
   parent-only aliases narrowed to `pub(super)` or private. An attempted
   `pub(super)` visibility on the underlying execution/traversal definitions
   produced the intended compile RED (`E0364`): Rust cannot re-export those
   items to the crate root beyond their definition visibility. Keeping the
   definitions `pub(crate)` while narrowing the directory aliases to
   `pub(super)` is the minimum compiling seam.

No new behavioral test was warranted for these structural corrections. The
compile/lint and blast-radius evidence was:

- focused reader, graph, traversal, transaction-release, and search-view
  suites: 47 passed;
- ten affected engine/facade feature routes: all passed;
- crate Clippy with warnings denied on the hook and TC5 routes: passed;
- C1 conformance: 26/26; recursive C1 self-test: passed;
- Windows WAL attribution recursive fixture: 313/313;
- Slice 35 virtual-mutation manifest: 1/1; `slice60_fix1_wire`: 4/4;
- plan anchors and AC-050c against `bb077cfa`: passed;
- public surface: 13 rows, exactly equal to the pre-move capture, original
  implementation candidate, and tracked Slice 30 baseline.
- hidden surface: 33 rows, exactly equal to the pre-move capture and original
  implementation candidate; against the tracked `8e2afb29` baseline, 261
  additions, 0 changes, and 0 removals.

## Review and final verification

Code review is closed in `code-review.md`. A separate read-only
`gpt-5.6-terra` subagent returned PASS at clean candidate `3e60cc5d`; the
canonical gate passed 127/127, workspace Clippy/check passed, strict security
passed 0/0/0 (the live AC-037 claim at `3e60cc5d` is UNEVIDENCED; a
HITL-granted re-run passed at `66e27983` as historical evidence for that
candidate only; see `review-verification.md`), and the feature-complete gate
passed 349/357 with 8 documented ignores. Exact-final-candidate live AC-037 is
deferred to Slice 150 after Slice 130. `review-verification.md` records the full
receipts and cleanup.

## Post-hoc design review fix-1 (2026-09-27)

A post-hoc adversarial design review of HEAD `a68e90b7` returned
PASS-WITH-FIXES (`design-review.md`). Fix-1 is mechanical and changes no
behavior, SQL, feature gate, or public or hidden path. Under the TDD exception
for mechanical refactors, it adds no new tests. Commit `b8af4d86`:

- returns `TelemetrySink` and `EvidenceCapture` to the crate root with
  private fields, and narrows the six over-visible items listed in the
  visibility seam record above;
- removes the `#[allow(unused_imports)]` on the `graph_expand/mod.rs`
  test-hook re-export, and drops the three re-exported `*ForTest` types that
  nothing imports;
- in `slice80_reader_transaction_release.rs`, gates only the two debug-only
  guards and the worker-count check on `cfg(debug_assertions)`, rather than
  the whole file, as `design.md` specifies. The loop count is the shipped
  pool size of 8, and every assertion and oracle is unchanged. The test now
  also runs under release `--tests`;
- points the `perf_gates.rs` ground-truth comment at
  `filter.rs::build_vector_phase1_sql`;
- makes three changes to the Windows WAL guard:
  - splits the needles, so the completion pause is checked on
    `READER_POOL_SOURCE` and `binding_connection_inventory_for_test` and
    `checkpoint_at_rest_for_test` are checked on `ENGINE_SOURCE`;
  - makes `assert_contains` check every needle;
  - corrects the one latent needle this exposed
    (`reader_native_state_for_test()` did not match the binding's
    `fn reader_native_state_for_test(&self)`).

  The recursive fixture now passes 314/314.

## Post-hoc test review (2026-09-27)

An independent read-only test review at `66e27983` returned PASS with P3
advisories only. It re-ran each recorded mutant (projected-text validity,
both graph-arm validity sites, and the leaked reader transaction in debug and
release); each failed at its intended assertion and was restored. Rustdoc
JSON surfaces at `bb077cfa` and HEAD were identical at three feature sets,
item `cfg` attributes matched, and lib test names were unchanged (80 = 80).

Recorded limits:

- Removing only the second graph-arm validity site (hydration
  `body_validity`) survives, because the edge-query site already filters.
  The mutant therefore removes both sites, as the design states.
- The graph result codec property gap is closed by follow-up commit
  `9700991f`; see the chronology above. Consideration
  `TC-aa4bea08-f281-47eb-8022-d63250d1daac` is closed.
- `slice60_fix1_wire`'s negative scan lists its files explicitly, so a new
  `graph_expand/*.rs` file would escape it; Slices 90 and 140 must extend it.
- Non-Linux compilation of the moved `graph_expand/execution.rs` arm was not
  checked on this host; a green non-Linux CI build is required before
  release closeout.
