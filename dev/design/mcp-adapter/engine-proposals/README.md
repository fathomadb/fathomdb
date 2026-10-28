---
title: MCP adapter engine proposals — index
date: 2026-10-17
status: PROPOSED
desc: Index of draft engine and SDK proposals that came out of building fathomdb-mcp on fathomdb-sdk as is. Drafts for owner review, not accepted decisions.
---

# MCP adapter engine proposals — index

These are drafts. The `fathomdb-mcp` crate was built in adapter-only mode on
`fathomdb-sdk` 0.8.27 (owner decision, 2026-10-17). Each proposal records a
place where the adapter had to report `unavailable`, infer a signal, or work
around missing engine behaviour. None is accepted. Each needs its own ADR, or an
amendment to an existing ADR, before implementation. `architecture.md` §7 lists
the original additions this set expands.

Every claim about current behaviour cites code at file:line. Paths are relative
to `src/rust/crates/` unless they are given in full. A bare file name refers to
the crate last named in the same section. Where a fact could not be verified,
the proposal says so.

Priority:

- **P0**: blocks the MCP state and drift contract (MR-050, MR-051).
- **P1**: removes an adapter inference or workaround.
- **P2**: later. A new capability that is reported `unavailable` today.

Size: **S** is one engine function plus SDK plumbing. **M** is a new governed
read or a result-type change across the three SDKs. **L** is a schema or
write-path change, or a change across several read paths.

## Proposals

| Id | Title | Priority | MRs unblocked | Surface touched | Size |
| --- | --- | --- | --- | --- | --- |
| [EP-01](EP-01-store-state-read.md) | Content-free store-state read on a reader connection | P0 | MR-050, MR-051.1/.3, MR-041.2/.4, MR-006.3 | New `read::visibility_state` and `VisibilityStateV1`; new `ReaderRequest`; parity map | M |
| [EP-02](EP-02-telemetry-query-id-uniqueness.md) | Telemetry query ids unique per sink file | P1 | MR-024.7, MR-006.5 | `enable_telemetry` behaviour; sink `session` record | S |
| [EP-03](EP-03-hybrid-search-degradation.md) | Typed degradation on hybrid search | P1 | MR-032.1/.2/.3, MR-030.3 | `SearchResult.degradations`; embed-dispatch error plumbing | M |
| [EP-04](EP-04-graph-expand-completeness.md) | graph.expand truncation signal and kind-aware work accounting | P1 | MR-053.1/.2 | `GraphExpandResultV1` (in place); incident-edge SQL; two edge indexes; graph codec | M |
| [EP-05](EP-05-kind-enumeration-and-counts.md) | Kind enumeration and per-kind counts | P2 | MR-022, MR-054.1/.3, MR-012.2 | New `read::kinds`; generation-keyed cache | M |
| [EP-06](EP-06-currency-fields-on-reads.md) | Lifecycle and validity fields on NodeRecord and SearchHit | P2 | MR-013, MR-061.2/.3 | `NodeRecord`, `SearchHit`, `GraphTargetV1` (in place) | M |
| [EP-07](EP-07-open-without-create.md) | Open an existing database without creating it | P1 | MR-005.5 | `OpenOptions.create_if_missing`; `EngineOpenError::DatabaseNotFound` | S |
| [EP-08](EP-08-text-search-observability.md) | Telemetry, explain and lifecycle events on text-only search | P1 | MR-024.1, MR-031 (explain in text mode) | `TextSearchOptions.explain`; `search_text_only` finalization | S |
| [EP-09](EP-09-interruptible-reads.md) | Deadlines and interruption for reader-pool reads | P1 | MR-033.1/.2 | `CallControl` on read options; reader progress handler; dispatch policy; `Interrupted` error | L |
| [EP-10](EP-10-write-trust-origin-and-scope.md) | Engine-stamped origin class and scope | P2 | MR-010, MR-011, MR-014, MR-060–062 | Schema columns; `ReadView.scope`; write policy on open | L |
| [EP-11](EP-11-typed-argument-refusals.md) | Typed reasons for argument refusals | P2 | MR-054 | `InvalidArgument` and `InvalidFilter` payloads; empty-query error | M |
| [EP-12](EP-12-result-set-extent.md) | Bounded match count and ranked results beyond the cap | P2 | MR-030.2, MR-041.2 | `SearchOptions.count_bound`; `SearchResult.lexical_matches` | M |
| [EP-13](EP-13-raw-arm-scores.md) | Raw per-arm scores in the explanation | P2 | MR-031 | `PerHitExplain` (in place) | S |
| [EP-14](EP-14-exact-phrase-predicate.md) | Exact literal constraint on search | P2 | MR-020 | `require_literals` on search options; FTS match compilation | M |
| [EP-15](EP-15-fetch-by-content-id.md) | Point reads by content id (h:) | P2 | MR-042.2 | `canonical_nodes.body_sha256`; new `read::get_many_by_id` | M |
| [EP-16](EP-16-governed-change-feed.md) | Governed change feed keyed by write cursor | P2 | MR-055 | New `read::changes`; change log table or stamped column | L |
| [EP-17](EP-17-match-centred-snippets.md) | Match-centred snippets on search results | P1 | MR-040 (snippet shows the match) | `SearchOptions.snippet`, `TextSearchOptions.snippet`; `SearchResult.snippets` sidecar; `snippet_for` selector | M |
| [EP-18](EP-18-reader-dispatch-and-pool-size.md) | Work-conserving reader dispatch and a configurable reader pool. Part A accepted and built; part B waits for the live sweep | P1 | MR-033.1/.2 | `ReaderWorkerPool` dispatch and shutdown (A); `EngineConfig.reader_pool_size` (B) | M |

## Suggested order

1. EP-01, the first-release prerequisite.
2. The S-sized P1 items, EP-07, EP-08 and EP-02, which each delete adapter code.
3. EP-03 and EP-04, which fix signals the adapter reports wrongly today.
4. EP-09, starting with least-loaded dispatch as a cheap first step.
5. The P2 items, in whatever order the owner's MCP roadmap needs. EP-10 gates
   any write tool.

## Findings that correct earlier documents

These are recorded in the proposals and not edited elsewhere, as this change
makes no other doc edits:

- No trigger on `operational_mutations` advances the visibility generation, so
  "op-store tables" in `requirements.md` is only partly true (EP-01).
- With `dense_disabled`, hybrid search fails with `VectorEquivalenceMismatch`
  instead of degrading, and the adapter maps that error to `internal` (EP-03).
- The adapter's `embedder_unavailable` entry misses query-embedding timeouts and
  saturation (EP-03).
- Round-robin reader dispatch lets new calls queue behind an abandoned one,
  which the admission cap does not prevent (EP-09). This held before
  [EP-18](EP-18-reader-dispatch-and-pool-size.md) part A; now a request queues
  behind a busy worker only when all eight readers are busy.
- Erasure redacts only the telemetry sink attached at the time, so per-epoch
  sink files from earlier opens keep erased stable ids (EP-02).
- `p:` ids are synthetic rerank passages, not stored records (EP-15, MR-042.2).
- `graph.expand`'s `complete=true` and its work accounting follow the Slice 60
  design. They are deliberate, not bugs (EP-04).
- The doc comment on `read_get_by_id_in_tx` says superseded versions are never
  returned. The code honours `include_superseded` (EP-06).
