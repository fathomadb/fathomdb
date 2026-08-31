# Performance program learning register

This register preserves useful findings that do not belong in a track plan or
product decision. It is evidence and design input, not authorization. Track
closeout must add durable, source-linked learning here when a finding would
otherwise survive only in narration.

## Retrieval and efficiency

- The turn-level A0 profile remains the accepted general retrieval treatment.
  Deeper hybrid/CE retrieval improved evidence recall directionally but did not
  improve the fixed ANSWER-01 result.
- Parent expansion is not a general default. It adds context without a
  demonstrated answer-quality gain.
- Streamed BM25 boundary-tie completion removed the growing full-sort fallback
  cost while preserving exact ordered top-100/top-10 results. Reader mmap
  increased RSS and was unnecessary after the algorithmic fix.
- GPU TC-5 established vector-stage fidelity at 17,272 documents. CPU is not a
  general fidelity oracle; use it only for a specifically justified
  equivalence bridge.

## Difficult query shapes

- Multi-hop evidence recall can improve while correctness, groundedness, and
  attribution regress. Retrieval selection must include evidence usability,
  not recall alone.
- Compact evidence packing reduced context substantially but lost
  groundedness. Deterministic abstention and citation validation made the run
  robust; they did not make the rejected treatment accurate.
- The tested exact-anchor graph bridge was too weak: it changed few questions
  and did not improve answer F1. Graph storage quality was not the limiting
  factor; associative selection and evidence planning were.
- The GLOBAL-01 source-linked lazy treatment improved directness/cost signals
  but lost headline coverage and assertion recall. The first witness bypassed
  `Engine.search`; the held-out run used it in both arms but remained an
  end-to-end system comparison.

## Temporal and semantic memory

- Native validity, supersession representation, erasure, and source linkage
  passed synthetic lifecycle probes.
- The available TimelineQA and LongMemEval distributions do not provide the
  source-derived validity-window manifest needed for an external temporal
  retrieval claim.
- EXTRACT-01 produced only a descriptive +1/78 knowledge-update difference and
  failed value-changing Boston-to-Austin consolidation. FathomDB has the
  lifecycle mechanism; the external semantic component must supply a reliable
  conflict verdict.
- Two remaining CORPUS-01 supersession questions were correctly labelled
  `insufficient_evidence`; evaluation must permit abstention rather than force
  a hard answer unsupported by its sources.

## Product-boundary findings

- FathomDB's strongest differentiators are local efficiency, canonical
  identity, lifecycle integrity, provenance, and deterministic retrieval.
- Query decomposition, semantic conflict resolution, global synthesis, and
  answer entailment belong to the external semantic component.
- FathomDB still needs complete mechanisms for constrained combined graph
  expansion, governed native filtering/pagination, and opt-in source-complete
  evidence resolution.
- A bare `SearchHit` cannot reconstruct an exact historical source revision.
  Keep hits compact and issue an opaque Engine-owned evidence reference for
  opt-in resolution.
- Preserve the current single-source provenance model. Multi-source causal
  provenance is a separate capability and must not be implied by an evidence
  resolver.

## Operational lessons

- Each test needs a fresh database, resolved configuration, artifact digests,
  device/runtime evidence, safe committed receipt, and external content root.
- Paid loops need checkpointing, provider-aware backoff, malformed-cell
  continuation, deterministic spend ceilings, and explicit exhausted-cell
  quality failures.
- `.env*` files are secret-bearing local inputs and must remain ignored and
  uncommitted.
- A foreign `.venv` symlink is incompatible with the Python test-hook ownership
  contract. Worktrees may invoke a shared interpreter explicitly or through an
  ignored local interpreter shim, but must not let an editable native rebuild
  rebind the shared environment to worktree sources.
- Expired release-candidate manifests and stale editable native modules are
  repository/setup debt, not evidence that a performance treatment failed.

## Evidence pointers

- Retrieval and answer selection: [ANSWER-01](tracks/answer-01-shortlist-scoring.md)
  and [MEMORY-01](2026-08-24-memory-01-result.md).
- Fidelity and scale: [SCALE-01](tracks/scale-01-tc5-fidelity.md),
  [SCALE-02](2026-08-22-scale-02-rank-boundary-result.md), and the
  [production note](2026-08-23-scale-02-stream-default-implementation.md).
- Lifecycle and semantic memory: [TRACE-01](tracks/trace-01-projection-lifecycle-integrity.md),
  [TEMPORAL-01](2026-08-23-temporal-01-trace-validity-result.md), and
  [EXTRACT-01](2026-08-23-extract-01-implementation.md).
- Difficult query shapes: [GRAPH-01](2026-08-30-graph-01-result.md),
  [GLOBAL-01](2026-08-29-global-01-lazy-coverage-result.md), and
  [REASON-01 v2](2026-08-30-reason-01-compact-ledger-v2-result.md).
- Human gold boundary:
  [CORPUS-01](2026-08-23-corpus-01-supersession-human-review-result.md).

## Maintenance

Add a learning only when it is supported by a named receipt, result note,
source inspection, or reproduced operational incident. Link the source in the
same change. Move a learning into architecture, requirements, an ADR, or a new
track when it becomes normative; retain the historical statement here with a
link to its promoted destination.
