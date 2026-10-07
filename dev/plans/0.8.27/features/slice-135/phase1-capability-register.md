---
title: Slice 135 Phase 1 capability exercise register
status: INTERIM_INSTALLED_PYTHON_S01_AND_TS_SMOKE
target_release: 0.8.27
---

# Phase 1 capability exercise register

The executable [canonical operation map](../../../../../src/conformance/governed-operation-parity.json) is the scope authority for governed operations. Its SHA-256 at the initial inventory snapshot is `da673d3de1c7962e6ccd1c0d2d2c552cf3941338b5e4cd67f13b7ad87ab9bb17`. That snapshot was branch `llm/0.8.27-slice-135` at `ea91ec0e8878d49c44706234ff2ea9c454b55cb5`, before the confirmed edge-FTS-error repair. This living register is not the frozen measurement protocol; only rows explicitly marked with a receipt have executed installed-artifact evidence.

The engine-only first-results receipts cover a narrower text/write/erase/reopen workload; those runs do not close an SDK row. A candidate [installed Python wheel functional smoke](results/2026-10-07-python-wheel-qualification/README.md) now exercises the specifically marked operations against a real database, with state assertions and reopen. The exact [0.8.26 installed wheel](results/2026-10-07-python-wheel-baseline/README.md) passed that same smoke, and the two graph matrices agree on semantic fields. **Functional smoke is not an S01/S02 timing result or complete contract qualification.** Every other row remains an explicit Phase 1 execution gap for installed SDK behavior. The routes below are proposed full Phase 1 cases from the [protocol draft](phase1-protocol-draft.md); each must be replaced with exact command, artifact/source identity, assertions and receipt link, or kept as a named gap in the checkpoint. `S03` may use qualified benchmark data for shape, with basic state or result checks. Dedicated gold scoring remains Phase 2.

The [paired installed Python S01 receipt](results/2026-10-07-python-s01-paired/README.md)
adds positive, source-bound text and vector-bearing/hybrid retrieval calls
through the installed wheel after a real-database write, projection drain and
reopen. Its basic ID/branch assertions and query timing close the S01 Python
workload shape, not all error, filter, lifecycle or provider conditions on
those operations. A [TypeScript native-suite and installed-package smoke](results/2026-10-07-ts-native-qualification/README.md)
now proves a narrow installed consumer path against a real database. It does
not close TypeScript S01/S02 or the broader accepted-operation contracts.
The installed Rust rows and S02 remain gaps.

| Canonical operation | Phase 1 exercise route | Rust SDK | Python wheel | TypeScript package |
| --- | --- | --- | --- | --- |
| `engine.open` | S02 startup/reopen | Gap | Functional smoke | Functional smoke |
| `admin.configure` | S02 startup | Gap | Gap | Functional smoke |
| `engine.write` | S02 write | Gap | Functional smoke | Functional smoke |
| `engine.actuate` | S03 actuation | Gap | Functional smoke | Gap |
| `engine.register_source_dependency` | S03 dependency | Gap | Functional smoke | Gap |
| `engine.dependencies_for_source` | S03 dependency | Gap | Gap | Gap |
| `engine.dependency_for_derived` | S03 dependency | Gap | Functional smoke | Gap |
| `engine.read_dependency_closure` | S03 dependency | Gap | Gap | Gap |
| `engine.transition` | S03 lifecycle | Gap | Gap | Gap |
| `engine.purge` | S03 lifecycle | Gap | Gap | Gap |
| `engine.erase_source` | S02 erasure | Gap | Gap | Gap |
| `engine.search` | S01/S02 hybrid | Gap | Functional smoke | Functional smoke |
| `engine.freeze_read_context` | S02 frozen | Gap | Functional smoke | Functional smoke |
| `engine.search_frozen` | S02 frozen | Gap | Functional smoke | Gap |
| `engine.search_expand_frozen` | S03 graph | Gap | Functional smoke | Gap |
| `engine.search_text_only` | S01/S02 text | Gap | S01 paired basic; broader contract gap | Gap |
| `engine.search_projected_text` | S03 projection | Gap | Gap | Gap |
| `engine.search_with_evidence` | S02 evidence | Gap | Functional smoke | Expected-error smoke only |
| `engine.resolve_evidence` | S02 evidence | Gap | Functional smoke | Gap |
| `engine.resolve_graph_evidence` | S02 graph evidence | Gap | Functional smoke | Gap |
| `engine.trace_dependency` | S03 dependency | Gap | Gap | Gap |
| `engine.close` | S02 close | Gap | Functional smoke | Functional smoke |
| `read.get` | S02 read | Gap | Gap | Functional smoke |
| `read.get_many` | S03 read | Gap | Gap | Gap |
| `read.collection` | S03 read | Gap | Gap | Gap |
| `read.mutations` | S03 read | Gap | Gap | Gap |
| `read.list` | S03 read | Gap | Gap | Gap |
| `engine.ingest_with_extractor` | S03 provider | Gap | Gap | Gap |
| `engine.consolidate_with_provider` | S03 provider | Gap | Gap | Gap |
| `graph.expand` | S02 graph | Gap | Functional smoke | Gap |
| `graph.neighbors` | S02 graph | Gap | Gap | Functional smoke |
| `graph.search_expand` | S03 graph | Gap | Gap | Gap |
| `rerank` | S01 model only | Gap | Gap | Gap |
| `engine.embed` | S01 model | Gap | Gap | Gap |
| `read.crossed_boundary_since` | S03 read | Gap | Gap | Gap |
| `engine.configure_projections` | S02 projection | Gap | Gap | Gap |
| `read.projections` | S02 projection | Gap | Gap | Gap |
| `read.projection_status` | S02 projection | Gap | Gap | Gap |
| `read.embedding_readiness` | S02 projection | Gap | Gap | Gap |
| `read.projection_generation_status` | S03 projection | Gap | Gap | Gap |
| `read.mutation_projection_status` | S03 projection | Gap | Gap | Gap |
| `read.canonical_page` | S03 pagination | Gap | Gap | Gap |
| `read.operational_state` | S03 operational | Gap | Gap | Gap |
| `read.operational_state_page` | S03 operational | Gap | Gap | Gap |

## Existing behavior test leads

| Family | Existing checkout-native real-database test leads | Remaining binding question |
| --- | --- | --- |
| Open, write, read and close | Rust SDK `behavior.rs::read_namespace_round_trips_written_nodes`; Python `test_functional_search.py`, `test_functional_retrieve.py`; TypeScript `functional-search.test.ts`, `functional-retrieve.test.ts` | Execute installed artifacts with exact state assertions and reopen. |
| Search, projection and readiness | Rust SDK `behavior.rs::projected_text_search_reads_a_declared_property_projection`; Python `test_slice20_dense_readiness.py`; TypeScript `slice20-dense-readiness.test.ts` | Exercise configured vector/hybrid plus readiness and status through installed artifacts. |
| Graph and evidence | Rust SDK `behavior.rs::graph_namespace_traverses_edges`, `evidence_search_resolves_each_reference_under_the_same_context`; Python `test_functional_graph.py`, `test_slice20_graph_evidence.py`; TypeScript matching suites | Assert graph expansion and evidence resolution at the installed boundary. |
| Lifecycle, dependency and actuation | Rust SDK `behavior.rs::lifecycle_and_erasure_commands`; Python `test_slice30_dependency_closure.py`, `test_slice25_actuation.py`; TypeScript matching suites | Exercise dependency and actuation calls through Rust SDK; add installed state/restart checks. |
| Frozen read and pagination | Rust SDK `behavior.rs::frozen_search_uses_shared_defaults`; Python `test_slice45_pagination.py`; TypeScript `slice45-pagination.test.ts` | Assert successful pages and frozen results at each installed boundary. |
| Provider operations | TypeScript `functional-consolidate.test.ts` contains a real consolidation round trip | Find or add successful provider ingestion and Python/Rust consolidation tests. |

These names identify candidates for the Phase 1 execution map; a file can contain shape-only or mock-native tests. Select named cases by inspecting their assertions, then retain exact command output and artifact identity before changing a row from Gap.

## Existing evidence and material gaps

- Signature/live-set parity checks establish the 44 callable operations but do not execute each one against a real database. The Rust SDK `behavior.rs` suite uses real temporary databases through the source SDK, yet it is not an installed-crate artifact run. Nineteen operations have no positive invocation there; `engine.embed` only checks no-embedder failure, while `read.crossed_boundary_since` and `graph.search_expand` do not assert returned behavior.
- Python and TypeScript have broader checkout-native real-database suites for reads, lifecycle, dependencies, projections, frozen reads and graph operations. The installed wheel and packed npm consumer smokes cover narrower subsets. Existing test names are leads until the exact test case, real-native mode, source/artifact and result assertions are checked and rerun on this candidate.
- Python provider ingestion and consolidation lack a verified successful real-provider SDK case in this inventory. TypeScript has a real consolidation round trip in `functional-consolidate.test.ts`; neither binding has a verified successful provider ingestion case. `rerank` is a standalone operation and has no database boundary; its real-model/identity cases require a separately labeled result.
- 0.8.26 has no Rust SDK. Rust SDK observations are candidate-only; comparable 0.8.26 engine operations may be paired at their own boundary. Python and TypeScript installed-artifact comparisons can be paired when capabilities match.

## Additional contract axes outside the governed operation list

The final inventory must also resolve runtime defaults/configuration and errors, bounded metrics, supported platform and package loading, filter/validity/provenance semantics, vector/hybrid model conditions, concurrent/cancellation behavior, and resource cleanup. These are conditions on operations rather than extra operation IDs. A capability is counted as exercised only when a real execution and an independent assertion are linked to its exact candidate and feature/platform combination.
