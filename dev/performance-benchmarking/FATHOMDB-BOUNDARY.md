# FathomDB's boundary and measured performance

## Thesis

FathomDB is a local-first, provenance-preserving memory data plane. It is not
the complete semantic memory system. Its job is to make externally chosen
semantic policies safe, attributable, executable, reversible, and fast.

The surrounding semantic component decides what to remember, interprets
meaning, plans evidence, chooses models, synthesizes answers, verifies semantic
claims, and controls spend. FathomDB stores canonical evidence, maintains
derived projections, enforces lifecycle and visibility, executes governed
retrieval, and returns structurally explainable results.

## Performance inside the boundary

The program establishes a strong data-plane foundation:

- FathomDB A0 scored 75.19% on the matched LOCOMO comparison versus 67.21%
  for the pinned Mem0 OSS baseline. Temporal, single-hop, and open-domain
  categories favored FathomDB; multi-hop favored Mem0.
- At 50,000 records, the approved streamed FTS path measured 12.95 ms steady
  p50, 30.74 ms p99, 69.47 queries/s, and 424 MiB peak RSS while preserving
  exact ordered retrieval against the full-sort control.
- Source-linked projection, supersession, erasure, and orphan canaries passed.
- FathomDB produced direct answers in the small GLOBAL-01 witness, while the
  broader system treatment exposed a global-coverage gap.

These are not interchangeable claims. Retrieval latency and lifecycle fidelity
measure FathomDB directly. LOCOMO answer quality, multi-hop reasoning, and
global synthesis measure FathomDB plus caller-side retrieval policy, evidence
packing, answer generation, and judging.

## Product responsibilities

| FathomDB data plane | External semantic component |
| --- | --- |
| Canonical identity, records, versions, validity, provenance | Memory admission and semantic interpretation |
| Atomic keep/supersede/invalidate/erase mechanisms | Conflict, merge, and supersession judgments |
| FTS, vector, rerank, graph, filtering, and expansion primitives | Query intent, decomposition, traversal strategy, stopping |
| Projection readiness and lifecycle propagation | Extraction, ontology, entities, relationships, summaries |
| Bounded evidence with source references and structural explanation | Answer generation, semantic verification, repair, abstention |
| Typed actuation of caller decisions | Model/provider/GPU/network/spend/HITL policy |

The practical completeness test is whether the external component can operate
through governed FathomDB APIs without raw SQL, shadow indexes, duplicated
liveness logic, or manual projection cleanup.

## What the experiments changed

The runs do not justify putting an agentic semantic control plane inside the
Engine. They do justify strengthening four data-plane contracts:

1. Classify measurements by system layer, because GLOBAL-01's first witness
   did not use `Engine.search` and its held-out run was still end to end.
2. Add an opt-in source-complete evidence resolver while keeping ordinary
   `SearchHit` compact.
3. Extend combined graph expansion with direction, canonical edge kind, target
   kind, read view, bounds, continuation, and path explanation.
4. Extend the governed predicate grammar and stable pagination so clients do
   not need large over-fetch/filter loops for natively indexable conditions.

The versioned architecture is
[`fathomdb-data-plane-architecture-v1.md`](../design/fathomdb-data-plane-architecture-v1.md),
and the 0.8.25 delivery plan is
[`fathomdb-data-plane-foldback-v1.md`](../plans/fathomdb-data-plane-foldback-v1.md).

## Next steps

Implement the four contracts through requirements, acceptance criteria,
reviewed design, TDD, binding parity, lifecycle tests, and compact verification
runs. Preserve the external semantic boundary. Reopen multi-hop, global, or
semantic-consolidation comparisons only after the external component supplies
a materially different treatment; do not retune the rejected PROGRAM profiles.
