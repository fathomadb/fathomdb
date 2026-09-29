---
title: FathomDB 0.8.27 Slice 85 - TDD chronology
status: COMPLETE
target_release: 0.8.27
baseline_sha: 4c75bfec2985f4001690673cc5b38dfdce2081bf
implementation_candidate: 7a2f9bf90783f545603516502bac0016d4b93a14
---

# Slice 85 TDD chronology

## Baseline and method

The capable-host commissioning receipt at `4c75bfec` supplied the exact
pre-move canonical, native, public, and hidden baselines. Implementation used
compile RED for mechanical moves and explicit RED fixtures for new behavior or
enforcement. No test was weakened to make a move pass.

## RED/GREEN batches

| RED | GREEN | Contract closed |
| --- | --- | --- |
| `2dfab005` | `b192f67d` | Standalone `syn` extractor: modules, imports, Engine fields/methods, callable references, cfgs, and exact edge identity. |
| Mechanical compile boundaries | `309ce8d3` through `851d64f7` | Structural state, WAL attribution, reader transaction/error, telemetry/search carriers, read and graph facades, and reader protocol moved to their reviewed semantic owners. |
| `ae5b3fd5` | `c02769e0` | Snapshot filtering returns its characterized narrow error and preserves mapping/order. |
| Focused compile and behavior failures | `7646c376`, `29a593d9`, `90671456` | Narrow graph expansion errors, typed private reader-request factories, explicit governed imports, and removal of the four prohibited cycles. |
| Windows source-scraper failure after request ownership moved | `5e38934e` | The 317-case Windows WAL attribution fixture follows `reader_pool` ownership. |
| Lint-shell cache-pipeline failure | `2c57f267` | Missing/fresh/stale boundary-tool cache checks fail closed without pipe sensitivity. |
| Initial code-review mutation failures | `1d917643` | Item-level SCCs, all 16 configurations, inline modules, root/report-only reach, aliases, shadows, cfg/cfg_attr, relevant macros, and exact policy identities. |
| `01be1a8d` | `b4419714` | Root-glob bare references, same-module helper paths, unknown-feature rejection, and alias-sensitive root re-export exactness. |
| `ce2d9cd3` | `1b40a245` | Type-only dependencies, local macros, and lexical block shadow expiration. The graph evidence type now names its semantic owner explicitly. |
| `2437594d` | `2b91e023` | New local macro definitions fail closed in governed, admitted, and report-only modules. |
| `0bc283e2` | `29d01f91` | The sole reviewed local macro exception binds a deterministic body fingerprint and stale policy entries fail. |
| Integrated lint-fixture failure | `7a2f9bf9` | The miniature actionlint runner supplies the new boundary-check stub and again reaches its intended first lint command. |

## Gate sensitivity

The closeout mutation suite passes 17 library tests and 2 binary tests plus
production-source mutants covering cache state, globs, root/report-only paths,
all former cycles, item identity, test/non-Linux configurations, shadows,
inline modules, direct request variants, type paths, local macros, macro body
drift, stale macro policy, unclassified files, and missing classifications.
The production gate reports 71 classified modules, 18 governed modules, and
16 explicit configurations.

## Focused behavior and build evidence

At exact candidate `7a2f9bf9`:

- default all-target and release-test engine checks passed;
- `reader_pool` passed 7/7, graph traversal 18/18, frozen read 5/5, and filter
  grammar 19/19;
- Terra additionally passed `slice60_graph_expand` 19/19, two request/error
  ownership tests, feature checks for `test-hooks` and `tc5-benchmark`, and the
  317-case Windows WAL fixture;
- the official 13-row public capture is exactly equal to the pre-move capture;
- the official 33-row hidden capture has no structural, release-probe,
  changed, or removed entries. Its only delta is the new narrow-filter unit
  test appearing in 13 applicable inventory rows; and
- the candidate-bound Python native receipt passed after one focused FFI
  test, with module SHA-256
  `c27a160d3bc1b43794e88aef809cbf456a0d26bd17ace424d86d565ac904b1e2`.

Per the owner's closeout direction, no final full regression gate is claimed.
The verification remained targeted to the structural blast radius and the
acceptance-specific surface/native checks.

## FIX-1 (design review cycle 1)

Design review cycle 1 of `7a2f9bf9` failed the enforcement half (D-1..D-13).
FIX-1 ran on branch `slice-85-fix` from `b4e90afe`. Each RED was committed
before its fix and run against the pre-fix gate with every mutant reported.
The RED harness is a scratch copy of `scripts/tests/test_module_boundary_gate.sh`
whose `expect_failure`/`expect_success` print instead of exiting. GREEN is the
unmodified script: `bash scripts/tests/test_module_boundary_gate.sh` exit 0,
plus `scripts/check-module-boundaries.sh` and
`cargo test --locked --manifest-path dev/tools/module-boundary-gate/Cargo.toml`.
No existing assertion was weakened. Two fixture expectations moved to the
compact configuration expression that D-6 introduces. `test-configuration`
now asserts `kind=import configurations=test at search.rs:` exactly, and
`local-helper-cycle` writes `all`. The fixture also restores the clean policy
after `local-helper-cycle`, and copies the engine `Cargo.toml`.

| Finding | RED (commit: failing mutants on the pre-fix gate) | GREEN (commit: pass) |
| --- | --- | --- |
| D-1 macro bodies | `e702eb43`: `macro-vec`, `macro-vec-repeat`, `macro-params`, `macro-assert-matches`, `macro-assert-matches-capital`, `macro-matches-guard`, `macro-format`, `macro-write`, `macro-statements` gate passed; `macro-unparsed` missed `unparsed macro body source=search …` (P19/P20; P21 control `macro-control` already failed as required) | `0bd2fb75` + regeneration `d6cb5d63`; four governed `serde_json::json!` bodies parse through the json grammar |
| D-2 capitalised paths | `e702eb43`: `capital-const`, `capital-variant`, `capital-constructor`, `capital-unit` gate passed (P1/P15/P16); `f2922aad`: struct literal and tuple/struct patterns; `69934ddb`: let type annotation | `0bd2fb75`; nine root re-export indirections it exposed in `graph_expand` now name `crate::evidence::…` / `crate::filter::…` |
| D-3 re-export laundering | `17ae57ae`: `reexport-laundering`, `reexport-laundering-chain` missed `forbidden dependency search -> reader_pool` (P12) | `7b325e03` + `4905b286` |
| D-7 freeze scope | `17ae57ae`: `reported-to-reported` positive failed (P13); `out-of-scope-edge-line` missed its diagnostic | `7b325e03`; policy header rewritten in `4905b286` |
| D-13 edge kinds | `17ae57ae`: `reexport-kind`, `type-kind` missed `kind=reexport` / `kind=type` | `7b325e03` |
| D-6 configurations | `6766f914`, corrected in `f24f3ebb`: `operator-configuration`, `release-configuration`, `all-features-configuration`, `manifest-feature-removed`, `manifest-axis-removed`, `empty-configuration` gate passed; `manifest-derived-feature`, `undeclared-axis`, `test-configuration` missed (P3/P4) | `f7774ab7` + `a5289be4`: 16 → 144 configurations |
| D-10 Engine method cfg twins | `98c580af`: `exclusive-engine-method-twins` positive failed (P14); `overlapping-engine-method-twins` missed its configuration-named diagnostic | `5cb878de` + `9523871f` |
| D-4 cycle accounting | `db2a3169` (+ `89912f86` report capture): `governed-reported-cycle` (P7), `allow-cycle-needs-admitted`, `admitted-cycle-unallowed`, `stale-allow-cycle` (P8), `stale-report-cycle`, `admitted-relabelled`/`admitted-without-entry`/`root-not-admitted` (P9), `stale-admission`, `admission-hides-no-execution`, `forbidden-submodule-cycle`, and the `--report` SCC row all RED | `2f57e4f1` |
| D-5 type and receiver cycles | `db2a3169`: `type-only-governed-cycle` (P6), `typed-receiver-cycle` (P5), `arc-receiver-cycle`, `untyped-receiver`, `unreviewed-same-name-receiver`, `stale-external-receiver` RED; `0af8656e`: `arc-constructed-receiver-cycle` RED | `2f57e4f1`, `36dd8bc2` |
| D-8 error impl colocation | `6b349270`: structural check `GraphExpansionError impls must be colocated with graph_expand::types` | `24af4084` (byte-identical 36-line block) |
| D-9 counter fields | `6b349270`: `GraphExpandRetentionCountersForTest fields must stay private` | `e58885fc`; `slice60_fix5_rss` 1/1 under `test-hooks` |
| D-11 board row | n/a (record) | this closeout commit |
| D-12 runtime/RSS | n/a (record) | `8151ea35` (masked-graph refactor); this closeout commit records 1.64–1.67 s and 34.8–35.1 MB |

`8151ea35` is a behaviour-preserving refactor, and its policy regeneration is
byte-identical. Graphs keep per-edge configuration masks. Cycle detection
runs once on the union graph and re-examines only its SCCs per
configuration. Warm runtime fell from about 6 s to 1.7 s.

Engine rewrites demanded by the strengthened gate. Each is
behaviour-identical and names the same item:

- `graph_expand/codec.rs` and `graph_expand/execution.rs`: nine struct
  literals and patterns spelled `crate::GraphEvidenceSidecarV1`,
  `crate::GraphEvidenceSidecarEntryV1`,
  `crate::EvidenceErrorReasonV1::EvidenceUnavailable`, and
  `crate::SnapshotFilterError::{InvalidFilter, Sqlite}` now name their
  semantic owners, `crate::evidence::…` and `crate::filter::…`.
- `read.rs` `read_list_in_tx`: `Predicate::to_sql_clause(pred, …)` and
  `Predicate::bind_value(pred)`.
- `read.rs` `validate_operational_context`:
  `SearchFilter::is_unfiltered(&…)`.
- `search_api.rs` `Engine::search_filtered_with_limit`:
  `Filter::to_search_filter(&filter)`.
- `graph_api.rs`: `GraphExpandRetentionCountersForTest::load_relaxed(&…)`,
  part of D-9.

Cycle findings: the item graph and the governed-module graph have no SCC in
any of the 144 configurations. The three inherited `allow-cycle` pairs were
therefore stale and are removed. `dependency_closure` and `evidence` become
reported. `errors` stays admitted through the named `GraphExpansion` payload
admission. `frozen_read ↔ projection_generation` is a module-level 2-cycle
only, with no item-level cycle, so no directive records it. The whole-crate
module-level graph has one large SCC of 33 or 34 modules, depending on
`operator`. It contains eight governed modules: `filter`, `frozen_read`,
`graph_expand::traversal`, `graph_expand::types`, `read`, `search`,
`search_types`, and `structural_state`. They are reached through the central
`EngineError` payloads and unrelated items in shared reported modules. It is printed by `--report` as `module-scc`
inventory, and the plan's FIX-1 amendment records why it is not enforced.

## FIX-2 (design review cycle 2)

Design review cycle 2 of the FIX-1 head `51f2a9a3` found residual gate gaps
(D-14..D-22). FIX-2 ran on branch `slice-85-fix` from that head. Each RED
test was committed before its fix. It was run against the gate of the
preceding commit (a scratch harness whose `expect_*` print instead of
exiting), and every new mutant was reported RED there: either the gate
passed, or the expected diagnostic was missing. GREEN is the unmodified
`bash scripts/tests/test_module_boundary_gate.sh` (exit 0), plus
`scripts/check-module-boundaries.sh` and the gate crate's `cargo test`.

No mutant was removed and no asserted diagnostic was weakened. Two existing
fixtures changed their inputs only:

- `stale-external-receiver` writes its policy line in the new entry format
  (`… as_str 1 value String`); its asserted diagnostic is unchanged.
- The `reported-cycle-accounted` and `admitted-cycle-allowed` positive
  fixtures add the `module-cycle` inventory line their deliberate 2-cycle
  needs, just as they already add its `edge` lines.

| Finding | RED (commit: failing mutants) | GREEN (fix + policy commits) |
| --- | --- | --- |
| D-14 call generics, qualified-self, trait paths | `69fef3b1`: `turbofish-call` (graph_expand → search), `qself-type-call`, `qself-trait-call`, `qself-trait-reference`, `qself-trait-type`, `free-fn-turbofish`, `const-generic-turbofish`, `impl-trait-header`, `generic-bound`, `where-bound`, `impl-trait-argument`, `dyn-trait-reference`, `boxed-dyn-trait`, `supertrait`, `self-super-chain` gate passed; control `generic-control` passes. `e3c13b7b`: `super-root-reexport`, `super-root-indirection` (a `super::` path to a root re-export resolved to nothing) | `bc5502f8` + `964017e3` (8 new real edges); root-relative resolution in `a7b32fe0` |
| D-15 alias laundering | `fb194410`: `type-alias-laundering`, `generic-type-alias-laundering`, `inline-module-reexport-laundering`, `inline-module-reexport-call` missed `forbidden dependency search -> reader_pool` | `a7b32fe0` + `2ee23f0a` (3750 → 3833 edges, all previously invisible real dependencies); engine re-spelling `3f183b37` |
| D-16 reported-only rules, mistyped receivers | `b5b468a8`: `reported-unparsed-macro`, `reported-untyped-receiver`, `untyped-receiver-cycle`, `mistyped-local-constructor`, `generic-parameter-receiver` gate passed; `mistyped-foreign-constructor`, `typed-receiver-missing-method` missed the unresolved-receiver diagnostic | `b0ae8767` + `2745671e` (5 `unparsed-macro` entries for the `proptest!` bodies; receiver entries 21 → 152; 27 edges to nonexistent methods dropped) |
| D-17 descendant forbids | `e345502b`: `codec-search`, `codec-search-api`, `new-submodule-search` missed `forbidden dependency graph_expand::… -> …` | `5ed4a85b` + `206c2268` (15 forbid lines → 5 family roots, a strict superset) |
| D-18 shipped closures | `bcf8a008`: `operator-ml-split-cycle`, `product-ml-split-cycle` missed `unapproved governed cycle search <-> telemetry` | `fa0d9c81` + `20f261aa` (144 → 232 configurations; 12 edges relabelled) |
| D-19 module-level cycles | `2254b572`: `module-two-cycle`, `module-scc-join` gate passed; `stale-module-cycle`, `stale-module-scc` missed their diagnostics; `7399a6f7` adds the inventory line to two positive fixtures | `f20b5e7a` + `ce706334` (6 `module-cycle`, 34 `module-scc`) |
| D-20 statement and parent-mod cfg | `12a01934`: `statement-unknown-feature`, `statement-cfg`, `let-cfg`, `arm-cfg`, `parent-mod-cfg` missed their diagnostics; unit test `statement_expression_and_inner_cfg_are_evaluated` failed | `f3b9c436` + `7d93f304` (relabelling only, 3806 edges before and after) |
| D-21 receiver identity | `c93a26b3`: `receiver-swap` (the reviewer's same-count swap in `search::read_search_in_tx`) gate passed; `in-crate-listed-external`, `typed-receiver-edge`, `typed-receiver-missing-method` missed their diagnostics | `921e9b61` + `f7a76d29` (233 entries: 128 external, 105 typed; 25 new typed edges) |
| D-22 records | n/a (record) | this closeout commit |

The receiver entries were classified from compiler output. In a scratch copy
of the engine, each counted call's method was renamed at its exact span.
`cargo check -p fathomdb-engine --lib --profile test --features
test-hooks,operator,tc5-benchmark` then reports each receiver's type in
E0599, and three passes cover calls whose errors rustc suppressed. The two
calls under `default-embedder`/`default-reranker` were read from their
declarations.

Engine change demanded by the stronger gate, behaviour-identical: in
`frozen_read.rs` `mint_inner`, `super::validate_filter_attributes_on_snapshot`
and `super::SnapshotFilterError::{InvalidFilter, Sqlite}` reached the
`filter` items through their crate-root re-exports. They now name
`crate::filter::…`, with the same items and match arms. Once `super::` paths
that climb to the root resolve, a governed module using one is root
re-export indirection.

Newly visible inventory: `frozen_read ↔ errors` and `read ↔ temporal` join
the four previously named module-level 2-cycles. None is an item-level or
governed-module cycle. No forbidden dependency or item-level cycle surfaced
in any of the 232 configurations.
