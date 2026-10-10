---
title: FathomDB 0.8.27 hidden-surface successor 01f4063f — diff record
status: APPROVED
target_release: 0.8.27
previous_baseline: baseline-8e2afb29.json
immediate_before: 1476d83e06b45feefbadabd995eb265f2c9217ea
capture_source: 01f4063f7ba19e4d9c38377c3d1956f35903ff9c
---

# Successor baseline `01f4063f`

## Identity and scope

The immutable [successor manifest](baseline-01f4063f.json) captures reviewed
Slice 140 product source `01f4063f7ba19e4d9c38377c3d1956f35903ff9c` after the
completed Slice 117 handoff. Later receipt commits change documentation only.
The capture used the unchanged hidden-surface tool, pinned
`nightly-2026-04-24` (`rustc 1.97.0-nightly (36ba2c771 2026-04-23)`),
`x86_64-unknown-linux-gnu`, with the existing required CUDA inventory access.
An initial queued attempt ended at executor SIGTERM after rustdoc rows;
the unchanged retry completed all 37 rows. No baseline was overwritten.

| Evidence | SHA-256 |
| --- | --- |
| [Tracked before capture](../slice-140/hidden-surface-before-1476d83e.json) | `8244ad078a917016297332c9454404029fb02e51cf2415d4e812e4c04f7d6506` |
| Successor capture | `0e041e0cb3951b70d831acaebd3d626fae9555fabb606ef88effb2f58d0362b1` |
| Immediate diff | `8462a3f3c1dd9308bb216f2d5a22eb46409c5cdda50b204592da066a13aca13b` |
| Inherited diff | `c6b882c71be5245c5e4808aea7f520344c8d453624bc8b11c5358842dce16acf` |

## Immediate Slice 140 API delta

The [exact immediate diff](baseline-01f4063f-immediate-diff.json) compares
completed Slice 117 `1476d83e0` with the successor and exits 1. The only
metadata difference is derived `row_identities`: required test features
regroup inventories from 35 to 37 rows. Toolchain, target, and schema match.
The exact JSON enumerates every changed row/path and former/new signed
predicate, including carriers, fields, variants, and impls.

- `engine-default`, `engine-slice72-test-hooks`, `engine-migration-test-hooks`,
  and `engine-tc5-benchmark` each remove the same 101 entries: 74 functions,
  14 impls, four structs, five fields, two variants, one enum, and one module.
  These are the inventoried test fixtures and their nested/derived paths.
  Merely enabling another test feature does not implicitly enable `test-hooks`.
- `engine-test-hooks` and `engine-operator-test-hooks` retain all paths and
  each change 101 effective cfg signatures to `any(feature = "test-hooks",
  test)`. Shapes/visibility are unchanged; `CacheStatusReply` additionally
  signs its five fields with that same inherited cfg instead of
  `debug_assertions`. There is no field/type/payload change.
- Both facade API rows are equal. The default engine release-probe drops its
  40 formerly selected fixtures, leaving the one facade proof. That selection
  change is supplementary, not an absence proof: the independent external
  consumers reject each primary fixture witness at its individual line in
  default dev/release and compile it under explicit `test-hooks` in both.
- The [primary inventory](../slice-140/test-seam-inventory.json) gives current
  owners, former/chosen source gates, and direct integration callers. Its 75
  module/type/function witnesses cover the nested handle, guard methods, and
  derived members transitively. The Removed changelog records explicit
  nested names required by AC-050c; the targeted checker passes 64 detected
  removals. The reader pause interface explicitly records the unit and
  integration routes. The visible `_graph_frontier_stats_for_test` exception
  is preserved, with its callers and separate-decision boundary recorded.

## Test disposition

Both captures enumerate 295 test targets and have zero `test-build` failure
entries. There are 134 target predicate changes: newly declared fixture
integration targets and existing feature targets gain `test-hooks`, including
transitive support-file consumers. Actual behavior assertions/bodies are
unchanged. Source-derived matrix coverage still covers every target.

The aggregate `(crate, kind, path, run/ignored)` test-entry union is **1,924** in
both captures and exactly equal: zero lost tests, zero new tests, zero ignore
status changes. The explicit engine hook inventories are equal (1,293 tests;
1,449 with operator hooks), as is the migration+operator+test-hooks inventory
(1,458). Smaller default/non-hook inventories are the intended fixture-route
separation; additional required-feature rows retain their tests.

| Inventory/API row | Before entries | After entries |
| --- | ---: | ---: |
| `engine-default` | 2624 | 2523 |
| `engine-migration-test-hooks` | 2625 | 2524 |
| `engine-operator-test-hooks` | 2938 | 2938 |
| `engine-slice72-test-hooks` | 2649 | 2548 |
| `engine-tc5-benchmark` | 2692 | 2591 |
| `engine-test-hooks` | 2738 | 2738 |
| `facade-default` | 167 | 167 |
| `facade-operator` | 203 | 203 |
| `release-probe` | 41 | 1 |
| `test-targets` | 295 | 295 |
| `tests-engine-default` | 1153 | 434 |
| `tests-engine-migration-test-hooks` | 1159 | 438 |
| `tests-engine-operator-test-hooks` | 1449 | 1449 |
| `tests-engine-slice72-test-hooks` | 1154 | 435 |
| `tests-engine-tc5-benchmark` | 1157 | 435 |
| `tests-engine-test-hooks` | 1293 | 1293 |
| `tests-facade-default` | 14 | 14 |
| `tests-facade-operator` | 12 | 12 |
| `tests-req-fathomdb-cli--none` | 129 | 129 |
| `tests-req-fathomdb-embedder--default-embedder` | 49 | 49 |
| `tests-req-fathomdb-embedder--default-embedder+onnx-embedder` | 71 | 71 |
| `tests-req-fathomdb-embedder--embed-cuda` | 51 | 51 |
| `tests-req-fathomdb-embedder--loader-test-hooks` | 72 | 72 |
| `tests-req-fathomdb-embedder--none` | 43 | 43 |
| `tests-req-fathomdb-embedder--tc5-benchmark` | 51 | 51 |
| `tests-req-fathomdb-engine--default-embedder` | 1171 | 438 |
| `tests-req-fathomdb-engine--default-embedder+default-reranker+slice72-test-hooks` | 1200 | 0 |
| `tests-req-fathomdb-engine--default-embedder+default-reranker+slice72-test-hooks+test-hooks` | 0 | 1340 |
| `tests-req-fathomdb-engine--default-embedder+operator` | 1245 | 0 |
| `tests-req-fathomdb-engine--default-embedder+operator+test-hooks` | 0 | 1468 |
| `tests-req-fathomdb-engine--default-embedder+test-hooks` | 0 | 1311 |
| `tests-req-fathomdb-engine--default-reranker` | 1171 | 452 |
| `tests-req-fathomdb-engine--migration-test-hooks+operator+test-hooks` | 1458 | 1458 |
| `tests-req-fathomdb-engine--operator` | 1226 | 463 |
| `tests-req-fathomdb-engine--slice72-gpu-tests` | 1208 | 0 |
| `tests-req-fathomdb-engine--slice72-gpu-tests+test-hooks` | 0 | 1348 |
| `tests-req-fathomdb-engine--tc5-benchmark+test-hooks` | 0 | 1297 |
| `tests-req-fathomdb-query--none` | 7 | 7 |
| `tests-req-fathomdb-schema--none` | 117 | 117 |
| `tests-req-fathomdb-sdk--none` | 38 | 38 |

## Changes inherited since the previous baseline

The retained `baseline-8e2afb29.json` precedes several already completed
slices. Its [exact inherited comparison](../slice-140/hidden-surface-inherited-diff.json)
with the tracked immediate-before capture separately records prior changes:
configuration/error additions, provider-control seam removal, the engine-owned
reader guard replacing the global clear route, operator observations,
facade diagnostic helpers, new robustness/runtime tests, and new Rust SDK/CUDA
inventories. These are not Slice 140 additions. See the release master plan
and each completed slice's status for their implementation/review provenance.
The immediate comparison isolates the intended change in this slice.

The comparator also pairs same-kind additions/removals into a heuristic
`changed` entry; that pairing does not establish a rename. The exact
`added`/`removed` path lists and before/after manifests are authoritative.

## Review and remaining gates

The independent `gpt-6-sol` high source review passed at `18ef03da9`, and
accepted the immediate public reader-guard delta. The independent `gpt-6-sol` high hidden-surface review returned **APPROVED**
with no P0/P1/P2. It matched tracked artifacts byte-for-byte to capture
receipts and verified the 101 intended removals/retained explicit gates,
unchanged facade, 295 targets with 134 feature-predicate additions only,
1,924-entry test union, justified row regrouping, inherited comparison, and
public reader-guard delta. This was static artifact/source review; it did not
independently rerun rustdoc or tests. The prepared
release checkout's full `agent-verify`, runnable package examples, and Terra
acceptance verification remain open. Slice 150 consumes the accepted
successor for final qualification and retires the oracle only after that
qualification; this capture is not final release qualification.
