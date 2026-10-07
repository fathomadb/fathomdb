---
title: Slice 135 Phase 1 capability exercise register
status: INTERIM_PYTHON_40_OF_44_EXECUTED
target_release: 0.8.27
---

# Phase 1 capability exercise register

The executable [canonical operation map](../../../../../src/conformance/governed-operation-parity.json) is the scope authority for governed operations. Its SHA-256 at the initial inventory snapshot is `da673d3de1c7962e6ccd1c0d2d2c552cf3941338b5e4cd67f13b7ad87ab9bb17`. That snapshot was branch `llm/0.8.27-slice-135` at `ea91ec0e8878d49c44706234ff2ea9c454b55cb5`, before the confirmed edge-FTS-error repair. This living register is not the frozen measurement protocol; only rows explicitly marked with a receipt have executed installed-artifact evidence.

The engine-only first-results receipts cover a narrower text/write/erase/reopen workload; those runs do not close an SDK row. The exact-candidate [installed Python capability exercise](results/2026-10-07-python-capability-exercise/README.md) accounted for all 44 governed operations and exposed one typed-error defect. The subsequent [installed-wheel repair](results/2026-10-07-python-frozen-error-fix/README.md) leaves **40 operations executed** with selected real-database assertions, one committed-closure gap and three unavailable provider/model cases; no operation currently fails its selected case. "Executed" does not mean every condition of an operation's contract was tested. The earlier [candidate wheel smoke](results/2026-10-07-python-wheel-qualification/README.md) and [0.8.26 wheel smoke](results/2026-10-07-python-wheel-baseline/README.md) remain narrow historical evidence. The routes below are proposed full Phase 1 cases from the [protocol draft](phase1-protocol-draft.md); each needs exact command, artifact/source identity, assertions and receipt link, or a named gap in the checkpoint. `S03` may use qualified benchmark data for shape, with basic state or result checks. Dedicated gold scoring remains Phase 2.

The [paired installed Python S01 receipt](results/2026-10-07-python-s01-paired/README.md)
adds positive, source-bound text and vector-bearing/hybrid retrieval calls
through the installed wheel after a real-database write, projection drain and
reopen. Its basic ID/branch assertions and query timing close the S01 Python
workload shape, not all error, filter, lifecycle or provider conditions on
those operations. A [TypeScript native-suite and installed-package smoke](results/2026-10-07-ts-native-qualification/README.md)
now proves a narrow installed consumer path against a real database. It does
not close TypeScript S01/S02 or the broader accepted-operation contracts.
The published Rust crate and full S02 qualification remain gaps. A
[source-bound external Rust Cargo consumer](results/2026-10-07-rust-sdk-s02-feasibility/README.md)
now exercises the marked candidate-only Rust calls on a real database,
including provenance-backed evidence, ready vector projection, erasure and
reopen. This is not a published-crate installation or a baseline SDK pair.
An [installed TypeScript S02 functional pair](results/2026-10-07-ts-s02-feasibility/README.md)
now proves one real-database whole-sequence path on each version, including
direct canonical-table checks after erasure and reopen. One pair with a
validation-inclusive timer does not qualify S02 latency, contention or the
full operation contracts.
An [installed Python S02 whole-sequence feasibility receipt](results/2026-10-07-python-s02-feasibility/README.md)
now exercises the specifically marked Python calls below across one fresh
write/project/retrieve/graph-evidence/erase/reopen sequence on each version.
Its one pair and validation-inclusive wall times are not a qualified S02
latency comparison; wider contract conditions remain gaps.
An [installed TypeScript S01 noise pilot](results/2026-10-07-ts-s01-noise-pilot/README.md)
adds an exploratory candidate text/vector-bearing/hybrid result check and
ten audited baseline-only blocks. The candidate's ten-sample probe does not
qualify S01 latency or its wider search contract. The later
[paired installed TypeScript S01 result](results/2026-10-07-ts-s01-paired/README.md)
provides source-bound text, vector-bearing and hybrid calls through both
installed packages, with basic ID and branch checks. It does not cover the
broader filter, validity, lifecycle or error conditions on these operations.

| Canonical operation | Phase 1 exercise route | Rust SDK | Python wheel | TypeScript package |
| --- | --- | --- | --- | --- |
| `engine.open` | S02 startup/reopen | External S02; broader gap | Installed wheel: selected case executed | S02 feasibility; broader contract gap |
| `admin.configure` | S02 startup | Gap | Installed wheel: selected case executed | Functional smoke |
| `engine.write` | S02 write | External S02; broader gap | Installed wheel: selected case executed | S02 feasibility; broader contract gap |
| `engine.actuate` | S03 actuation | Gap | Installed wheel: selected case executed | Gap |
| `engine.register_source_dependency` | S03 dependency | Gap | Installed wheel: selected case executed | Gap |
| `engine.dependencies_for_source` | S03 dependency | Gap | Installed wheel: selected case executed | Gap |
| `engine.dependency_for_derived` | S03 dependency | Gap | Installed wheel: selected case executed | Gap |
| `engine.read_dependency_closure` | S03 dependency | Gap | Gap: committed closure result | Gap |
| `engine.transition` | S03 lifecycle | Gap | Installed wheel: selected case executed | Gap |
| `engine.purge` | S03 lifecycle | Gap | Installed wheel: selected case executed | Gap |
| `engine.erase_source` | S02 erasure | External S02 with canonical counts; broader gap | Installed wheel: selected case executed | S02 feasibility with canonical counts; broader contract gap |
| `engine.search` | S01/S02 hybrid | External S02 vector/hybrid and invalid limit; broader gap | Installed wheel: selected case executed | S01 paired basic and S02 feasibility; broader contract gap |
| `engine.freeze_read_context` | S02 frozen | External S02; broader gap | Installed wheel: repaired and executed | S02 feasibility; broader contract gap |
| `engine.search_frozen` | S02 frozen | Gap | Installed wheel: selected case executed | Gap |
| `engine.search_expand_frozen` | S03 graph | Gap | Installed wheel: selected case executed | Gap |
| `engine.search_text_only` | S01/S02 text | External S02 text and invalid NUL; broader gap | Installed wheel: selected case executed | S01 paired basic and S02 feasibility; broader contract gap |
| `engine.search_projected_text` | S03 projection | Gap | Installed wheel: selected case executed | Gap |
| `engine.search_with_evidence` | S02 evidence | External S02 limit-one; broader gap | Installed wheel: selected case executed | S02 limit-one feasibility; broader contract gap |
| `engine.resolve_evidence` | S02 evidence | External S02; broader gap | Installed wheel: selected case executed | S02 feasibility; broader contract gap |
| `engine.resolve_graph_evidence` | S02 graph evidence | External S02; broader gap | Installed wheel: selected case executed | S02 feasibility; broader contract gap |
| `engine.trace_dependency` | S03 dependency | Gap | Installed wheel: selected case executed | Gap |
| `engine.close` | S02 close | External S02; broader gap | Installed wheel: selected case executed | S02 feasibility; broader contract gap |
| `read.get` | S02 read | External S02 positive and erased/reopen; broader gap | Installed wheel: selected case executed | S02 positive and erased/reopen checks; broader contract gap |
| `read.get_many` | S03 read | Gap | Installed wheel: selected case executed | Gap |
| `read.collection` | S03 read | Gap | Installed wheel: selected case executed | Gap |
| `read.mutations` | S03 read | Gap | Installed wheel: selected case executed | Gap |
| `read.list` | S03 read | Gap | Installed wheel: selected case executed | Gap |
| `engine.ingest_with_extractor` | S03 provider | Gap | Unavailable: provider/model qualification | Gap |
| `engine.consolidate_with_provider` | S03 provider | Gap | Unavailable: provider/model qualification | Gap |
| `graph.expand` | S02 graph | External S02 evidence; broader gap | Installed wheel: selected case executed | S02 evidence feasibility; broader contract gap |
| `graph.neighbors` | S02 graph | External S02 positive and erased/reopen; broader gap | Installed wheel: selected case executed | S02 erased/reopen check; broader contract gap |
| `graph.search_expand` | S03 graph | Gap | Installed wheel: selected case executed | Gap |
| `rerank` | S01 model only | Gap | Unavailable: provider/model qualification | Gap |
| `engine.embed` | S01 model | Gap | Installed wheel: selected case executed | Gap |
| `read.crossed_boundary_since` | S03 read | Gap | Installed wheel: selected case executed | Gap |
| `engine.configure_projections` | S02 projection | External S02; broader gap | Installed wheel: selected case executed | S02 feasibility; broader contract gap |
| `read.projections` | S02 projection | External S02 ready check; broader gap | Installed wheel: selected case executed | S02 ready/reopen checks; broader contract gap |
| `read.projection_status` | S02 projection | Gap | Installed wheel: selected case executed | Gap |
| `read.embedding_readiness` | S02 projection | Gap | Installed wheel: selected case executed | Gap |
| `read.projection_generation_status` | S03 projection | Gap | Installed wheel: selected case executed | Gap |
| `read.mutation_projection_status` | S03 projection | Gap | Installed wheel: selected case executed | Gap |
| `read.canonical_page` | S03 pagination | Gap | Installed wheel: selected case executed | Gap |
| `read.operational_state` | S03 operational | Gap | Installed wheel: selected case executed | Gap |
| `read.operational_state_page` | S03 operational | Gap | Installed wheel: selected case executed | Gap |

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
