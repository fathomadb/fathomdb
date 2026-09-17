---
title: FathomDB 0.8.26 Slice 60 — current design-owner model
status: APPROVED
target_release: 0.8.26
---

# Slice 60 design — current profiles over historical baselines

## Authority and evidence order

Accepted ADRs decide policy. Requirements state need. `dev/interfaces/` owns
public contracts. Active architecture v2.2 owns system shape. Schema and product
code witness implementation; tests witness enforced edge cases. Maintained
topic designs explain how those authorities compose and do not override them.

Every normative section in the rewritten owners maps to a bounded cluster in
`inventory.md` with both code and test evidence. Exact fields/errors stay with
interfaces and `errors.md`; limits stay with `retrieval-result-limits.md`;
projection-generation repair stays jointly owned with `recovery-0.8.25.md`.

## Shared seam model

- retrieval owns candidate generation, ranking/fusion, view eligibility,
  frozen authority, evidence, graph expansion, and explanation;
- recovery owns operator diagnosis, maintenance, export, and loss-authorized
  recovery boundaries plus the exact CLI action inventory; and
- engine owns admission/runtime topology, canonical identity, transactions,
  cursors, projection publication topology, and shared storage seams.

Cross-links point to narrower owners rather than copying their detailed rules.
The final engine pass triggers a mandatory retrieval/recovery cross-read.

## Retrieval profile

The retrieval owner describes one typed current pipeline:

1. validate query, bound, filter, view/frozen context, explanation, and evidence
   requests before candidate work;
2. execute safe compiled lexical node/edge/projected-text and eligible vector
   branches, applying roles, validity, lifecycle, and frozen authority before
   truncation;
3. use shared `vector_default` binary shortlist plus exact-f32 rerank, fuse
   contributing arms by RRF, and retain typed fallback/degradation state;
4. apply only shipped optional recency, importance/confidence, graph-arm, and
   cross-encoder mechanisms; embedding and reranker device policies remain
   independent;
5. finalize opt-in explanation and compact evidence from the same snapshot; and
6. keep bounded graph expansion as its own governed operation with current or
   frozen context, eligibility-before-expansion, deterministic bounds, and
   frozen-only exact artifact evidence.

Standalone `rerank` is live in Python and TypeScript. It reuses cross-encoder
machinery but does not make reranking unconditional in search. Deferred tuning,
general graph continuation, and full-path evidence remain non-current.

## Recovery profile

The old “doctor is bit-preserving/read-only; recover mutates” shorthand is too
broad. Current actions are classified by effect:

- database-free diagnostics: `gpu`, `platform`, `reranker-gpu`;
- engine-backed diagnosis/readout: `check-integrity`, `verify-embedder`,
  `trace`, `dump-schema`, `dump-row-counts`, `dump-profile`, `dump-mutations`,
  `orphan-provenance`;
- immutable quiescent inspection: `data-plane-integrity`;
- artifact/cache work: `safe-export`, `warm-cache`;
- explicit non-lossy derived-vector maintenance: `recompute-mean`; and
- loss-authorized recovery: `truncate-wal`, `rebuild-vec0`,
  `rebuild-projections`, `excise-source`, and paired
  `excise-collection`/`excise-record-key`.

The operator roots stay absent from SDKs, but governed SDK `purge` and
`erase_source` coexist with reserved-namespace/op-store excision behind
recovery. Only `data-plane-integrity` is described as immutable/quiescent. Each
live recovery action emits one JSON object and success uses the accepted-loss
exit class.

The classification is not inferred around the accepted 0.6.0 ADR. A narrow
0.8.26 successor supersedes only its doctor-wide bit-preserving/read-only
clause. It grandfathers the already-shipped `recompute-mean` command as the one
atomic, non-lossy, derived-vector maintenance exception. Its command-owned
transaction derives the mean from retained uncentered
`vector_default.embedding` values, updates the stored mean, and
recreates/requantizes vector rows with governed metadata preserved. Failure
rolls that transaction back. Shared admission and dependency-closure pre-writer
maintenance remain governed by their existing owners and are not new doctor
authority. This does not authorize another mutating doctor command.
Data-loss-authorized CLI operator recovery/rebuild actions remain under
`recover --accept-data-loss`; governed SDK erasure remains separate.

## Engine profile

Schema 34 and the fresh-only public rule govern admission. A missing or
zero-length database is bootstrapped; a nonempty noncurrent database is refused
under the persistent lock before product mutation. Internal migration machinery
is not a public upgrade promise.

Canonical, dependency, lifecycle, operational, receipt, FTS, and vector state
share one SQLite database. Active canonical identity is `logical_id` alone,
not `(logical_id, kind)`; kind changes supersede rather than fork. Callers do
not supply a separate current `row_id`: committed per-row `write_cursor` values
remain positional identity, while logical/content/passage identity is distinct.

Caller mutations use the mutex-serialized primary writer connection and commit
a validated batch atomically. Async vector projection workers use separate
connections and totally order their write transactions through `commit_gate`.
Pooled readers own current/frozen read transactions and do not move onto a
write lane. Current receipts expose batch high-water cursor, per-row cursors,
and dangling endpoint count.

Write cursors order writes; projection/read boundaries state derived visibility;
neither is an immutable record revision. Shared `vector_default` plus authority
sidecars replaces the historical per-kind vector-table description. Exact
generation, readiness, and repair details stay with their narrower owners.

## RED/GREEN proof shape

The RED is semantic and source-grounded: required current facts are absent and
specific disproven claims are present. GREEN requires the same probes to find
current facts and reject old claims. Lifecycle, Markdown, links, source diff,
and full repository verification remain separate oracles. No generated product
oracle or generalized Slice 65 checker is added.

## Compatibility and change class

Only explanatory/contract documentation changes. Runtime bytes, schemas,
operations, errors, bindings, packages, and publication state do not change.
Historical rationale remains in Git or behind explicit historical links rather
than masquerading as current behavior.
