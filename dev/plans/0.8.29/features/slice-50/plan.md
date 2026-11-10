---
title: Slice 50 — Fusion identity, exact deferred content and record-start snippets
status: PROPOSED
target_release: 0.8.29
slice: 50
---

# Slice 50 — Fusion identity, exact deferred content and record-start snippets

Database boundary, owner `seq-368`: .29 requires no database migration. Use the
existing format37 substrate and stored span/visibility data. See the
[release baseline](../../../plan-0.8.29.md#baseline-and-workspace); a design
requiring a storage-format change returns for explicit scope review.

## Current checkpoint scope

Gated scope approved in Slice9 Batch3: identity census/ADR precedes exact text/snippet implementation;30 baseline remains required. No particular identity successor has yet been selected.

The [decision log](../../prework/slice-9/decision-log.md) owns current rulings;
original draft observations below retain their historical evidence limits.

## Goal, maturity and inspected evidence

- Return distinct eligible records with faithful provenance while reducing unnecessary
  FTS body hydration and record-start snippet segmentation; identity changes need approval first.
- Authority: [brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md),
  [release ladder](../../../plan-0.8.29.md), and [canonical identity ADR](../../../../adr/ADR-0.8.0-canonical-identity-substrate.md).
- Inspected [fusion](../../../../../src/rust/crates/fathomdb-engine/src/fusion.rs),
  [search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs),
  [snippet selector](../../../../../src/rust/crates/fathomdb-engine/src/snippet.rs), and
  [snippet queries](../../../../../src/rust/crates/fathomdb-engine/src/snippet_query.rs).
- Fusion currently credits the first body occurrence per arm; equal-body records merge.
  Explanation maps and graph seed exclusion also use bodies: changing only the fusion key is insufficient.
- Current hybrid deferred-identity route still materializes every matched body. Its
  narrow admission excludes filters, relaxed views, graph, explanation and reweighting.
- [Format 35 ADR](../../../../adr/ADR-0.8.28-store-format-35.md) and inspected
  [projector](../../../../../src/rust/crates/fathomdb-engine/src/index_projector.rs) already establish node
  FTS rowid = write_cursor. Current baseline is format 37; do not reimplement this invariant.
- Inspected .28 [historical-view design](../../../0.8.28/features/slice-80/design.md) and
  [filtered direct-text plan](../../../0.8.28/features/slice-90/plan.md): filters/flags already exist,
  including fixed direct-text candidate inputs and ordered limit prefixes.
- Read via `git show exp/0.8.29-ac013-scale-search:<path>`: experiment README and
  `src/rust/crates/fathomdb-engine/src/scale_search_experiment.rs`; old body-based prototype is evidence,
  not approved production code or current performance proof. No product tests ran for this draft.

## Slice preparation: needs, requirements and acceptance draft

- Need S50-N1: users can distinguish equal text from distinct records, revisions and sources.
- Need S50-N2: exact ranking must remain affordable at scale without losing deep-arm agreement.
- Need S50-N3: long unmatched bodies get deterministic, Unicode-safe record-start snippets cheaply.
- S50-R1 / S50-A1: approved identity distinguishes the ADR-selected node/edge/revision/ownerless cases;
  fixtures assert count, order, IDs, cursor, branch, source and explanation provenance, not body sets alone.
- S50-R2 / S50-A2: optimized and complete reference return identical ordered hits, score bits,
  fallback/error metadata and snippets under the accepted identity, including overlap below text rank 200.
- S50-R3 / S50-A3: eligibility precedes caps; flags, filters, historical/frozen reads, graph seeds,
  recency, importance and CE rerank retain reviewed semantics through either exact route or reference fallback.
- S50-R4 / S50-A4: record-start output equals the full-selector reference for generated Unicode and
  every valid option, including text, anchor, cut, windows, spans, highlights and byte budgets.
- S50-R5 / S50-A5: bound hydration/segmentation work is witnessed separately from timing; repeat
  Slice 30 workloads on the qualified candidate and report p50/p99, memory and route counts without
  attributing earlier CS64/native eligibility improvements to this slice.
- These IDs are release-local drafts, not new canonical acceptance IDs. Produce `requirements.md`,
  `design.md`, `test-map.md`, identity ADR proposal and census evidence before implementation.

## Architecture and proposed design revision

- Draft identity options and consequences before selection: per-version cursor with node/edge namespace,
  logical entity identity, and current body identity. Recommend record-version identity for review;
  `SearchHit.id` alone can be shared by revisions or equal-content legacy rows and is not sufficient.
- Specify ownerless FTS identity using table/cursor/row identity, malformed or missing ownership behavior,
  per-arm repeated-record credit, deterministic ties, representative branch and provenance ownership.
  Keep canonical logical_id uniqueness unchanged; fusion identity is a separate retrieval decision.
- Census equal-body groups in an authorized read-only Memex snapshot and representative evaluation
  fixtures: records, logical IDs, revisions, kinds, sources, arm overlap and observed merged provenance.
  Bind corpus/snapshot/query identities; publish aggregates only and record unavailable access honestly.
- Submit census plus rank/provenance examples for HITL identity ruling. Amend conflicting body-dedup
  language in retrieval/historical design and affected interface docs only during authorized execution.
- Establish the complete ranking reference under that decision first. Share identity, RRF contribution,
  rank and tie comparison code between it and the optimized route; preserve weights and post-fusion stages.
- Deferred content computes every eligible match's exact `(bm25, cursor)` rank from lightweight rows,
  accounts for every vector/text/edge overlap at its full rank, then hydrates only candidates provably
  able to enter final output. Prove any stopping bound including ties; never use an approximate prefix cap.
- Preserve distinct edge ranking/corpus rules and rowid mapping; node rowid identity does not imply
  edge FTS rowid identity. Verify all current write/rebuild/reprojection paths preserve node identity.
- Keep one vector candidate/contribution per record at the fusion boundary; specify duplicate vector-ID
  handling without enabling record-grouped candidate-selection experiments or generating overflow vectors.
- Carry cursor-keyed identity through explanation rank/score maps, graph seed exclusion/hydration,
  evidence provenance and snippets. Historical graph traversal still resolves logical entities as designed.
- Retain conservative reference routing for unsupported controls and invariant failures; define which
  errors must propagate. Never silently omit candidates, bypass source-dependency fallback or weaken view authority.
- Prefix-only optimization applies after span normalization proves record-start selection. Prove bounded
  Unicode segmentation against the full selector before skipping whole-body tables; arbitrarily long
  grapheme/word context may require fallback. Preserve ellipses, whitespace/CRLF and UTF-8 byte offsets.

## Challenges, research, fit review and prerequisites

- Hard: approved Slice 10 preparation, trusted Slice 30 baseline and an accepted identity ADR/census
  before changed ranking/provenance or optimized production route. Census access is a new explicit prerequisite.
- Preferred: 40 lands before 50 for attribution/shared engine files; it is not a semantic dependency.
  Coordinate 60/70 final native parity on integrated fusion; serialize engine landings 40/50/70/100/110.
- Planned primary-source research: SQLite FTS5 bm25/rank/content lookup and transaction snapshot docs;
  pinned unicode-segmentation implementation plus Unicode UAX #29 rules for bounded-prefix context.
  Record consulted versions/URLs and counterexamples in `design.md`; do not infer latest-library facts.
- Fit review: compare prototype proof to format37, separate vector IDs, native pre-cap eligibility,
  current direct-text filters and historical/frozen authority. Reject old env knobs and obsolete vector fallback.
- Challenging seams: deep-rank ties, duplicated bodies across node/edge arms, ownerless rows, same logical
  ID across revisions, explanation joins, graph entity semantics and long combining/ZWJ/regional-indicator runs.
- Proposed revision: identity/reference first, then bounded route optimization, then prefix specialization;
  split implementation units if any cannot independently demonstrate RED and exact GREEN.

## Tasks, review and completion signals

1. Complete local needs/requirements/test mapping; inspect existing tests in full before gap claims.
   Use [deep-rank control](../../../../../src/rust/crates/fathomdb-engine/tests/slice71_search_statement_trace.rs)
   and [Unicode properties](../../../../../src/rust/crates/fathomdb-engine/tests/snippet_selector.rs) as starting evidence.
2. Produce census, identity ADR and adversarial design review; obtain recorded HITL ruling before code.
   Independent reviewer challenges scoring proof, identity/provenance and Unicode context, with unresolved issues explicit.
3. Implement approved identity with per-behavior failing tests first, separate RED/GREEN commits and
   recorded failure evidence; audit RED shas. Preserve frozen/historical fixtures instead of regenerating goldens.
4. Implement deferred content and prefix specialization separately; real-database fixtures plus generated
   ranking/Unicode properties cover empty arms, deep overlap, ties, duplicate vectors, node/edge equal bodies,
   ownerless rows, filter/view combinations, rollback/concurrent snapshots and forced-reference route witnesses.
5. Verify installed Python/N-API and Rust SDK parity for IDs, scores, sources, explanations and UTF-8
   snippets; include filtered provider-free direct text, historical flags and current native artifact identities.
6. Run focused lint/typecheck/tests in latency order, then one full `agent-verify` for final code phase
   in a checkout-owned environment; required feature/performance checks and affected MCP conformance follow.
   Repeat Slice 30 baseline protocols with/without optimization; report isolated and composed 40+50 effects.
7. Independent code/verification review checks exact output and actual route/work witnesses. Close only
   with traceable AC evidence, acknowledged performance limits, docs/interface updates and release-state integration.

## Stop and decision points

- Stop ranking/provenance changes until identity ruling; inaccessible census or unresolved ownerless/version
  semantics stays a documented dependency, not assumed approval. No Memex writes are part of this slice.
- Any score/order/provenance divergence, Unicode counterexample or unproven stopping bound blocks optimization;
  retain reference implementation and revise design. No ANN, overflow chunking, threshold/default experiments,
  schema/counter fallback, canonical contract edits or publication is authorized by this planning draft.
