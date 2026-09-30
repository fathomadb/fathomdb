---
title: FathomDB 0.8.27 Slice 85 - TDD chronology
status: HISTORICAL
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

## Original candidate gate sensitivity

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

## Historical review fixes

The following summarizes historical RED/GREEN evidence, not qualification of
recovery HEAD. The complete pre-recovery chronology remains in this file at
`df1ffd000`; frozen inventories, general resolution and redundant mutant
variants described there are retired by the recovery plan.

| Batch | Representative RED | GREEN / disposition |
| --- | --- | --- |
| Design FIX-1 | `e702eb43` macro/capitalized paths; `6766f914` operator/configuration; `6b349270` error colocation/counter privacy | `0bd2fb75`, `f7774ab7`, `24af4084`, `e58885fc`. Useful corrections retained; inventory/receiver variants retired. |
| Design FIX-2 | `69fef3b1` qualified/type paths; `e345502b` descendant forbids; `12a01934` statement/parent cfg | `bc5502f8`, `5ed4a85b`, `f3b9c436`. Supported syntax, descendant prohibitions and pre-filter cfg scanning retained. Consumer profiles and receiver/SCC freezes retired. |
| Design FIX-3 | `548de06f` serde/include/receiver/projection variants | `e14c05bf`. Full serde/projection/receiver semantics retired; bounded unsupported-syntax guards remain. |
| Design FIX-4 | `51d5b3fd` renamed include and qualified-self serde | `be4890c3`. Cheap include guard retained; no general string-path resolver. |
| Test FIX-1 | `c213659c` redirected target stale binary; `2445ab0c` search-owned error dependencies; `5cb3d56b` broken error mappings in scratch source | `b38e9164`, `d3f4c093`, `5cb3d56b`/`fa700c8c`. Wrapper, dependency prohibitions and real SQLite payload/typed-route tests retained. |
| Test FIX-2 | `809b0e9f` feature-gated seam scrape; `9a2b6c92` manifest-only stale guard | `71bd7a39`, `7d36ef53`. Both fixes retained. The 36 compiling-sibling additions in `733d047e` are replaced by one compiling witness per retained mechanism. |
| Test FIX-3 | `674b41be` inline/root aliases and extern-crate forms | `bbbd16cb`. Engine-used direct/root paths and cheap extern-crate rejection retained; speculative forms do not expand the recovery grammar. |
| Test FIX-4 | `5c9b3d7a` chained/glob/module-alias variants | `62a9dfbe`, `17f6f4c6`; inventory regeneration `7b09c533`. General namespace resolver and regenerated inventories retired. |

Historical TDD limitations remain: `90671456`, `1d917643`, and `2c57f267`
landed implementation and tests together rather than separate RED commits.
The original record also identifies a non-bisect-clean intermediate at
`3f183b37`; D-15 RED was measured at `e3c13b7b`. Recovery does not rewrite
these events as compliant TDD.

Test FIX-2 recorded serial feature execution: `test-hooks` 1,170 passes across
177 binaries; `operator` 1,110; `operator,test-hooks` 161;
`migration-test-hooks,operator,test-hooks` 2; `migration-test-hooks` 82;
`tc5_vector_stage` 3. Its benchmark envelope still failed at 144 bytes versus
128. Those results predate recovery; they are not final-candidate evidence.

## Recovery evidence

Recovery work and its exact-candidate qualification are recorded once in
`recovery-receipt.md`. Historical receipts and immutable accepted baselines
remain unchanged; no release binding is advanced by this work.
