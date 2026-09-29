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
