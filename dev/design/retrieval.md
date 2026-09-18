---
title: Retrieval Subsystem Design
date: 2026-09-17
target_release: 0.8.26
desc: Current lexical, vector, fusion, reranking, graph, frozen-view, explanation, and evidence pipeline
blast_radius: fathomdb-query; fathomdb-engine retrieval/frozen/evidence/graph paths; SDK search and graph interfaces
status: ACTIVE
---

# Retrieval design

This file owns how current retrieval mechanisms compose. Public request,
response, and error spellings remain in `dev/interfaces/`; exact result-limit
rules remain in [`retrieval-result-limits.md`](retrieval-result-limits.md); the
shared vector representation remains owned by [`vector.md`](vector.md).

## Pipeline and authority order

Retrieval is a typed, bounded data-plane operation:

1. validate query, result bound, filter, view or frozen context, explanation,
   evidence, graph, and reranker options before candidate execution;
2. compile text through the safe `fathomdb-query` grammar rather than passing
   caller text directly to FTS5;
3. apply declared projection roles, lifecycle, valid-time, source, attribute,
   and frozen eligibility before each branch's candidate cutoff;
4. collect lexical and vector candidates, hydrate canonical state, and fuse
   contributing arms deterministically;
5. apply only requested optional graph, reweight, or cross-encoder mechanisms;
   and
6. truncate to the public limit, then finalize opt-in explanation/evidence from
   the same authorized result snapshot.

Eligibility-before-truncation is load-bearing. A hidden, expired, deleted,
superseded, ineligible, or wrong-generation row cannot consume a bounded slot
and then disappear after ranking. Current and frozen paths share that rule.

## Candidate arms

The live hit-provenance enum has four values:

- `Vector` — eligible node candidates from the shared dense table;
- `Text` — canonical node-body or declared projected-text FTS candidates;
- `TextEdge` — edge-body text or vector-projected edge-fact candidates; and
- `GraphArm` — nodes contributed by the optional bounded search graph arm.

## Soft-fallback signal

The soft-fallback record is narrower than hit provenance. It reports only a
nonessential `Vector` or `Text` branch that could not contribute; total request
failure remains a typed error. `TextEdge` and `GraphArm` identify returned hit
origins and are not new soft-fallback categories.

### Lexical collection

Node and edge text uses maintained FTS tables. Declared nested-source projected
text participates only through the registered projection grammar. Direct
`search_text_only*` requests preserve the stable prefix contract: the unfiltered
node-only fast path reads the hidden-rank stream through the complete score
group crossing its fixed candidate boundary, restores stable
`(bm25 score, write_cursor)` order, and then applies the public limit. Filters,
edge-bearing workspaces, unsupported shapes, and statement/row failures use the
full stable-sort path. The optimization does not change public ordering.

### Vector collection and fusion

Vector retrieval uses the one schema-owned `vector_default` virtual table plus
its authority sidecars. Phase 1 uses the binary shortlist; phase 2 exact-f32
distance reranks that shortlist. Kind and declared filter attributes constrain
the candidate statement before its limit.

Contributing lexical, vector, and optional graph arms are combined by weighted
reciprocal-rank fusion. There is no public `fusion_mode` switch. Vector-first
tiebreaking and stable cursor ordering make equal-score output deterministic.
Recency and importance/confidence reweights are separate, default-off
mechanisms; they do not redefine the base fusion contract.

Dense-equivalence failure disables vector-dependent work without disabling the
text-only path. Device selection for the embedder and cross-encoder is
independent; a reranker device result never attests the embedding or SQLite
candidate path.

## Cross-encoder reranking

Search may request bounded cross-encoder reranking of its fused pool. The model
contributes nullable `ce_score` values and the configured blend only inside the
reranked pool; depth zero, empty input, or the feature-off/model-unavailable
identity path preserves input ordering and scores. Validation and forced-device
errors occur before or at the owned reranker boundary and do not silently fall
back from forced CUDA.

Standalone `rerank` is a live governed package operation in Python and
TypeScript. It accepts caller passages and reuses the same bounded reranker
mechanism, but it does not query SQLite and does not imply that ordinary search
always reranks. The executable operation map and binding surface oracle are
owned by Slice 55.

## Current and frozen reads

`ReadView` owns current visibility and valid-time policy. A frozen read context
authenticates the database, effective view, validity instant, canonical
high-water boundary, dependency generation, projection generation, and request
eligibility envelope. Reproduction succeeds only while those facts remain
available and coherent; tamper, foreign database, or state drift returns the
typed frozen-read outcome rather than silently reading current state.

Frozen pagination and operational-state reads use their own bound cursor
contracts. A frozen context is not a permanently held SQLite transaction or a
persisted lease. Each operation opens a reader transaction and revalidates the
authenticated boundary.

## Graph retrieval

Two bounded graph mechanisms are current:

- the optional search graph arm seeds from eligible retrieval results and adds
  `GraphArm` candidates before final fusion; and
- the governed `graph_expand` V1 operation accepts query or explicit seeds,
  direction, edge and target kinds, indexed predicates, current or frozen
  context, and explicit depth/work/result bounds.

Graph seed and target eligibility is applied before seed truncation and
expansion. Results are deterministic one-page targets with compact origin and
degradation state. General continuation, full paths, associative diffusion,
and semantic truth inference remain deferred.

## Explanation and evidence

Explanation is an opt-in sidecar finalized exactly once with a nonempty
engine-minted correlation identity. It describes returned results using the
live fusion/reranking, fallback, projection, graph, dependency, and lifecycle
facts; it does not run a parallel ranking implementation. Explanation-off
ranking and result identity remain unchanged.

Compact ranked evidence is also opt-in and resolves under its authenticated
frozen eligibility envelope to exact source revision, locator, hash, bytes or
span, lifecycle, contribution, and dependency facts. Authorization and
nondisclosure precede detailed corruption diagnostics.

Bounded graph expansion has a distinct positional V1 evidence sidecar.
Frozen-only opaque references bind the selected target and winning terminal
edge revisions to the originating normalized request and disclosure envelope.
Resolution returns intrinsic artifact/source evidence; it never fabricates
rank, contribution, or a complete path. Exact behavior is owned by
[`ADR-0.8.26-exact-graph-artifact-evidence.md`](../adr/ADR-0.8.26-exact-graph-artifact-evidence.md).

## Ownership boundaries

- Bindings own language spelling and conversion precedence.
- Projection owners define generation/readiness and declared roles.
- The engine owner defines reader/writer topology and cursor semantics.
- The evidence and graph ADRs define opaque reference security and failure
  precedence.
- Semantic planning, synthesis, answer verification, and model/provider spend
  remain caller-owned.

Historical 0.6.0 stage and two-branch descriptions remain available in Git but
are not current authority.
