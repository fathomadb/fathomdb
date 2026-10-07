---
title: Slice 135 Phase 1 capability exercise register
status: INTERIM_THREE_SDK_OPERATION_EXERCISE
target_release: 0.8.27
---

# Phase 1 capability exercise register

The executable [canonical operation map](../../../../../src/conformance/governed-operation-parity.json) is the scope authority for governed operations. Its SHA-256 at the initial inventory snapshot is `da673d3de1c7962e6ccd1c0d2d2c552cf3941338b5e4cd67f13b7ad87ab9bb17`. That snapshot was branch `llm/0.8.27-slice-135` at `ea91ec0e8878d49c44706234ff2ea9c454b55cb5`, before the confirmed edge-FTS-error repair. This living register is not the frozen measurement protocol; only rows explicitly marked with a receipt have executed installed-artifact evidence.

The [installed Python repair and exercise](results/2026-10-07-python-frozen-error-fix/README.md)
accounts for all 44 governed operations: **40 selected cases executed, zero
failed, one committed-closure gap, three provider/model cases unavailable**.
The earlier run exposed a typed frozen-error defect, which was repaired and
rerun. The [installed TypeScript exercise](results/2026-10-07-ts-capability-exercise/README.md)
has **39 executed, zero failed, two explicit positive-path gaps and three
unavailable**. Its gaps are committed dependency closure and successful
dependency trace. The [external Rust Cargo exercise](results/2026-10-07-rust-sdk-capability/README.md)
has **42 selected cases executed, zero unattempted gaps and two unavailable
provider/model cases**. Its `rerank` result exercises the identity path;
cross-encoder model qualification remains open. The Rust consumer resolves
the SDK by source path; it is not a published-crate installation, and 0.8.26
has no Rust SDK peer. "Executed" means one selected asserted call, not every
condition of an operation's contract.

The [paired Python S01](results/2026-10-07-python-s01-paired/README.md) and
[paired TypeScript S01](results/2026-10-07-ts-s01-paired/README.md) receipts
cover installed text, vector-bearing and hybrid workload shapes, but predate
the repaired search source and need final-candidate refresh for checkpoint
latency. Current [Python S02](results/2026-10-07-python-s02-current-refresh/README.md)
and [TypeScript S02](results/2026-10-07-ts-s02-current-refresh/README.md)
functional refreshes
passed with independent state checks. Their single validation-inclusive
timers do not qualify S02 latency or contention. The engine-only results
never substitute for an SDK row. `S03` may use qualified benchmark data as a
workload; dedicated gold scoring remains Phase 2.

| Canonical operation | Phase 1 exercise route | Rust SDK | Python wheel | TypeScript package |
| --- | --- | --- | --- | --- |
| `engine.open` | S02 startup/reopen | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `admin.configure` | S02 startup | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.write` | S02 write | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.actuate` | S03 actuation | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.register_source_dependency` | S03 dependency | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.dependencies_for_source` | S03 dependency | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.dependency_for_derived` | S03 dependency | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.read_dependency_closure` | S03 dependency | External Cargo: selected case executed | Gap: committed closure result | Gap: positive committed closure result |
| `engine.transition` | S03 lifecycle | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.purge` | S03 lifecycle | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.erase_source` | S02 erasure | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.search` | S01/S02 hybrid | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.freeze_read_context` | S02 frozen | External Cargo: selected case executed | Installed wheel: repaired and executed | Installed package: selected case executed |
| `engine.search_frozen` | S02 frozen | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.search_expand_frozen` | S03 graph | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.search_text_only` | S01/S02 text | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.search_projected_text` | S03 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.search_with_evidence` | S02 evidence | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.resolve_evidence` | S02 evidence | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.resolve_graph_evidence` | S02 graph evidence | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.trace_dependency` | S03 dependency | External Cargo: selected case executed | Installed wheel: selected case executed | Gap: successful trace result |
| `engine.close` | S02 close | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.get` | S02 read | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.get_many` | S03 read | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.collection` | S03 read | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.mutations` | S03 read | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.list` | S03 read | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.ingest_with_extractor` | S03 provider | Unavailable: provider/model qualification | Unavailable: provider/model qualification | Unavailable: provider/model qualification |
| `engine.consolidate_with_provider` | S03 provider | Unavailable: provider/model qualification | Unavailable: provider/model qualification | Unavailable: provider/model qualification |
| `graph.expand` | S02 graph | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `graph.neighbors` | S02 graph | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `graph.search_expand` | S03 graph | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `rerank` | S01 model only | External Cargo: identity path executed; cross-encoder unqualified | Unavailable: provider/model qualification | Unavailable: provider/model qualification |
| `engine.embed` | S01 model | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.crossed_boundary_since` | S03 read | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `engine.configure_projections` | S02 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.projections` | S02 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.projection_status` | S02 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.embedding_readiness` | S02 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.projection_generation_status` | S03 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.mutation_projection_status` | S03 projection | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.canonical_page` | S03 pagination | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.operational_state` | S03 operational | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |
| `read.operational_state_page` | S03 operational | External Cargo: selected case executed | Installed wheel: selected case executed | Installed package: selected case executed |

## Remaining conditions

- A selected positive case does not establish every filter, temporal,
  validity, provenance, error-precedence, cancellation or concurrency branch.
  Use the Pareto/coverage and robustness matrices to select the next cases.
- Python and TypeScript still need a committed dependency-closure result;
  TypeScript also needs a successful dependency trace. The retained negative
  cases are evidence for refusals, not a positive substitute.
- Qualified extraction and consolidation providers remain unavailable in all
  three exercises. The standalone cross-encoder `rerank` model remains
  unqualified; only the Rust identity path ran.
- Rust lacks a published-crate install and a same-SDK 0.8.26 peer. Pair
  comparable engine behavior at the engine boundary and report Rust SDK
  candidate-only behavior separately.

## Additional contract axes outside the governed operation list

The final inventory must also resolve runtime defaults/configuration and errors, bounded metrics, supported platform and package loading, filter/validity/provenance semantics, vector/hybrid model conditions, concurrent/cancellation behavior, and resource cleanup. These are conditions on operations rather than extra operation IDs. A capability is counted as exercised only when a real execution and an independent assertion are linked to its exact candidate and feature/platform combination.
