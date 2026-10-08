---
title: Slice 135 Phase 1 capability exercise register
status: INTERIM_THREE_SDK_OPERATION_EXERCISE
target_release: 0.8.27
---

# Phase 1 capability exercise register

The executable [canonical operation map](../../../../../src/conformance/governed-operation-parity.json) is the scope authority for governed operations. Its SHA-256 at the initial inventory snapshot is `da673d3de1c7962e6ccd1c0d2d2c552cf3941338b5e4cd67f13b7ad87ab9bb17`. That snapshot was branch `llm/0.8.27-slice-135` at `ea91ec0e8878d49c44706234ff2ea9c454b55cb5`, before the confirmed edge-FTS-error repair. This living register is not the frozen measurement protocol; only rows explicitly marked with a receipt have executed installed-artifact evidence.

The [integrated-candidate installed Python exercise](results/2026-10-07-python-integrated-candidate/README.md)
accounts for all 44 governed operations: **40 selected cases executed, zero
failed, one committed-closure gap, three provider/model cases unavailable**.
Its 47 reopened real-database snapshots and operation partition were
independently audited. The earlier run exposed a typed frozen-error defect,
which was repaired and rerun. The
[installed TypeScript exercise](results/2026-10-07-ts-capability-exercise/README.md)
has **39 executed, zero failed, two explicit positive-path gaps and three
unavailable**. Its gaps are committed dependency closure and successful
dependency trace. The [external Rust Cargo exercise](results/2026-10-07-rust-sdk-capability/README.md)
has **42 selected cases executed, zero unattempted gaps and two unavailable
provider/model cases**. Its `rerank` result exercises the identity path;
cross-encoder model qualification remains open. The Rust consumer resolves
the SDK by source path; it is not a published-crate installation, and 0.8.26
has no Rust SDK peer. "Executed" means one selected asserted call, not every
condition of an operation's contract.
The [repaired-source Rust S02 refresh](results/2026-10-07-rust-sdk-s02-current-refresh/README.md)
repeats the external consumer sequence on the vector-repaired source with an
independent reopened-state check. It does not change the operation counts or
qualify Rust S02 latency.

The [paired Python S01](results/2026-10-07-python-s01-paired/README.md) and
[paired TypeScript S01](results/2026-10-07-ts-s01-paired/README.md) receipts
cover installed text, vector-bearing and hybrid workload shapes. Their earlier
candidate measurements predate the repaired search source. The
[repaired-source Python S01](results/2026-10-07-python-s01-vector-repaired-paired/README.md)
and [Python S02](results/2026-10-07-python-s02-vector-repaired-paired/README.md)
paired subsets now have independent exact-wheel and raw-state audits. The
[repaired-source TypeScript consumer](results/2026-10-07-ts-vector-repaired-candidate/README.md)
repeated the 39-operation exercise against rebuilt npm packages, and the
[TypeScript S01 paired subset](results/2026-10-07-ts-s01-vector-repaired-paired/README.md)
passed independent audit. The repaired-source
[TypeScript S02 paired subset](results/2026-10-07-ts-s02-vector-repaired-paired/README.md)
also passed independent raw and order audits over 100 whole sequences per
version, with host-only paging warnings in every pair. Integrated
[Python S02](results/2026-10-07-python-integrated-candidate/README.md)
and [TypeScript S02](results/2026-10-07-ts-s02-current-refresh/README.md)
functional refreshes
passed with independent state checks. The subsequent
[TypeScript S02 paired result](results/2026-10-07-ts-s02-paired-integrated/README.md)
uses a product-only timer and 100 independently audited whole sequences per
version; it is a diagnostic latency subset, while the earlier Python
functional receipt's single validation-inclusive timer does not qualify
latency or contention. A later [vector row-error repair](results/2026-10-07-vector-row-repair/README.md)
changed candidate engine bytes after the historical receipts. The
repaired-source S01/S02 subsets above use rebuilt installed artifacts. The
[Rust S02 candidate-only timing](results/2026-10-07-rust-s02-candidate-timing/README.md)
passed 100 measured sequences and independent audit; bounded contention
remains. The [installed Python S02 bounded-contention
pair](results/2026-10-07-python-s02-contention-paired/README.md) passed 100
real-database sequences per version with actual shared-handle reader/writer
overlap, semantic assertions and independent reopened-state checks. Its
all-warning paired latency diagnostic does not qualify TypeScript or Rust SDK
contention. The [installed TypeScript contention feasibility
pair](results/2026-10-07-ts-s02-contention-feasibility/README.md) subsequently
passed a real-database shared-handle sequence and independent persisted-state
audit on both versions; its one-off timing remains unqualified. The later
[baseline-only pilot](results/2026-10-08-ts-s02-contention-baseline-pilot/README.md)
passed five audited blocks. The
[TypeScript contention subset](s02-ts-contention-comparison-protocol.json)
froze before candidate timing. Its
[paired campaign](results/2026-10-08-ts-s02-contention-paired/README.md)
passed independent audit over 100 measured real-database sequences per
version with actual shared-handle overlap, semantic assertions and reopened
state checks. Its observed p50 increase is a diagnostic latency lead, not a
full Phase 1 verdict. Rust SDK contention remains. The engine-only results
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
