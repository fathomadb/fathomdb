---
title: Inspector compatibility — 0.8.28 versus 0.8.29
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Source comparison of storage and query feature differences relevant to Inspector implementation
---

# Inspector compatibility — 0.8.28 versus 0.8.29

> Current-policy note (2026-10-29): Engine startup is now permitted when
> required. The source differences remain evidence; the immutable/no-startup
> feasibility constraint below is historical. See the current
> [requirements](../requirements.md#engine-startup-allowance-2026-10-29).

## Conclusion

Supporting databases produced by both examined source lines should add little
storage-reader complexity. Their schema crate and important stored-data/read
mechanisms are identical. The meaningful differences are mostly transient
query evidence and API types. They justify a small normalized capability/result
boundary and version-specific qualification fixtures, not two independent
Inspector implementations.

This is a static source assessment, not runtime compatibility qualification,
a requirement amendment, or a claim that either version is published. Inspector
implementation has not started. The current requirements still name supported
schema rather than explicitly guaranteeing both product releases.

## Compared snapshots

| Source line | Pinned revision | Meaning |
| --- | --- | --- |
| `release/0.8.28` | `19704c6a760f32404d6cafad52fbce070a078b46` | The 0.8.28 release-work branch, used as the meaningful baseline. |
| `dev/0.8.29` | `8aec60ecea84ddc044e25c5247da787a61c438b6` | Development snapshot incorporating that 0.8.28 baseline and agent-access work. |

The local `dev/0.8.28` at `c02cff1f9` consolidates the release branch and
main-line documentation/ledger work. Its entire `src/`, `Cargo.toml` and
`Cargo.lock` trees match the release snapshot used here, so the source-feature
conclusion also applies to that consolidated branch. The release snapshot
is an ancestor of the examined 0.8.29 snapshot. The 0.8.29 tip advanced from
`e336cfc09` to `8aec60ece` during the review; that advance changes only the
finish plan and ledgers, not the reviewed source. Both still declare workspace
package version 0.8.27 at these commits, and no local v0.8.28/v0.8.29 release
tag was found during this review. Therefore conclusions are pinned to these
source revisions, not inferred from package version strings or registry bytes.

## Unchanged storage and core mechanisms

Both schema constants are 34 and the **entire schema crate is identical**.
The following engine files/directories are also identical between snapshots:

- `open.rs`: admission/schema behavior; no newly available immutable browsing API.
- `identity.rs`, `temporal.rs`: identity and class-specific visibility semantics.
- `read.rs`, `read_api.rs`: canonical/operational reads and their existing limitations.
- `vector_storage.rs`: stored vector representation and binary/f32 storage machinery.
- `projection_registry.rs`, `projection_generation.rs`: declaration/generation mechanisms.
- `dependency_trace.rs`, `graph_api.rs`: provenance tracing and graph API behavior.
- `fusion.rs`: existing result-fusion/identity behavior.
- The entire `fathomdb-embedder` and `fathomdb-embedder-api` directories: no new loader/storage compatibility split.

No version-specific on-disk adapter is indicated by this diff. This does not
prove all possible data states equivalent, or that a common schema number
alone is sufficient; it is stronger evidence that the actual schema and the
listed readers/storage mechanisms have no source delta.

## Feature differences and Inspector impact

| Difference | What changed in 0.8.29 | Inspector implication | Incremental complexity |
| --- | --- | --- | --- |
| Match-centred snippets/highlights | Adds `SnippetOptions`, `SnippetV1`, pure `snippet_for`, and optional search snippet entry points/sidecar. Snippets include body windows, highlights, anchor and cut state; spans are UTF-8 byte offsets with Unicode boundary handling. | Useful Slice 30 reuse opportunity. Preserve full record bodies separately. Capability is request/path-specific: projected-text and BM25F operations have no snippet option. Unsupported evidence on an older query backend is unknown, not fabricated. | Low to moderate presentation/evidence work; potentially less custom snippet logic. |
| Explicit query degradation | `SearchResult` and `SearchExpandResult` gain `degradations`; query embedding queue saturation/expiry can report `Vector / EmbedderOverloaded` while other arms return results. | Slices 40/60 can display an observed skip without confusing a text fallback with successful vector retrieval. A 0.8.28 backend lacks this evidence. Empty or absent metadata is not proof of complete retrieval health; not every embed failure is reported by this carrier. | Low result-normalization work. |
| Public Rust construction | Search/Text/Frozen options and SearchExpandResult become non-exhaustive; snippet/degradation DTOs and query `CompiledQuery.terms` are added. | If consuming these libraries, construct options from `default()` and assign fields; do not instantiate internal result/query shapes or exhaustively match extensible types. A single pinned dependency build avoids compiling two SDK variants by default. | Low compile-time integration work; no database adaptation. |
| Lexical rowid capture | Body/edge/property FTS query streams select rowid for search-local origin/highlight maps; merge fixes fill Block 1 fields on projected fast paths. | Do not copy private positional SQL decoders. Explicitly read named storage facts or use an available unchanged public operation. Check projected/edge routes as well as body FTS when qualifying results. These are query-output columns, not added persisted columns. | Low to moderate route-specific testing. |
| Idle-first reader dispatch | Engine readers switch to rendezvous/idle-first dispatch rather than queued round-robin work. Worker count remains eight. | Can affect latency if an unchanged Engine read API is used, but supplies no general in-flight cancellation API. Inspector-owned connection interruption remains separate. | Little structural work; relevant performance/cancellation tests. |
| Hints | Adds pure `search_hints` and typed agent-oriented recommendations; actual SDK search option changes in the examined diff are snippets, not automatically enabled hint rendering. | Inspector can ignore agent recommendations and display factual state/evidence. No teaching/narrative requirement is introduced. | None unless deliberately reused. |
| MCP/agent-access transport | Adds the MCP application and related queue/response work, with transport-specific budgets, continuations and text/hint shaping. | Local Inspector does not depend on MCP as its transport. Borrowing code is an assessment, not an integration prerequisite or authorization for changes to those projects. | None for the specified local transport; additional adapter work only if separately chosen. |

### Important snippet constraints

Engine-backed snippets are computed on the search transaction after ranking
and truncation; the existing full hit body is retained. A representative vector
branch can still have a real text match, so branch identity is not used as
proof of match-span availability. Combined and per-term point queries have
separate bounded pools; snippet text can depend on the search window. The pure
selector accepts a body and spans, but does not provide the FTS span acquisition
or an immutable query session by itself.

### Database compatibility versus dependency compatibility

An Inspector compiled against one qualified library revision can potentially
read databases produced by both source versions because the stored format is
unchanged. It does not need to dynamically load the engine version that wrote
each file. Conversely, a database produced by 0.8.29 does not itself contain a
search response's snippets, hints or degradation list; those are observations
of the query implementation currently serving the Inspector.

Describe capabilities by the selected Inspector/query backend and actual
operation, not by guessing a producer version from schema 34. If different
backends are supported, normalize optional evidence into available/unavailable
capabilities rather than sprinkle version checks through every UI view.

## Existing hard problems are not version differences

Both snapshots retain the same admission and narrow read/graph mechanisms.
0.8.29 does not establish a public immutable Inspector session, complete graph
edge payloads, arbitrary exact-revision reads, general interruptible Engine
reads, or a new production no-download loader contract. Ordinary Engine open
still has startup side effects incompatible with source-only immutable
inspection. These remain the Slice 20 feasibility inventory and later
Inspector-local adapter obligations; this comparison does not authorize
Engine/MCP changes or waive mandatory acceptance.

## Planned 0.8.29 work is not an implemented difference

At `8aec60ece`, `dev/plans/agent-access-block1-finish-plan.md` records Phase 0
consolidation complete and W4 performance investigation started. Other
workstreams and Block 2 are not complete. Compatibility must be rechecked
against final qualified artifacts, not assumed from this development tip.

| Planned follow-up | Potential Inspector impact if implemented |
| --- | --- |
| W2 broadens degradation to absent embedder/arm timeout and fixes dense-disabled draining. | Normalize extensible reasons and factual availability; do not hard-code one degradation reason or treat an empty list as full health. |
| W3 completes facade exports and snippet offset/parity coverage. | Recheck actual helper availability and preserve UTF-8 byte-offset semantics. |
| W4 performance/headroom work; current plan records an AC-013 regression. | Independently measure the selected Inspector backend and avoid inheriting unqualified latency assumptions. No new storage decoder follows from a timing change. |
| W6 compact MCP text default and agent feedback work. | No direct effect on the specified local Inspector transport. Do not import an agent-facing presentation dependency unnecessarily. |
| W7 identical-body fusion policy/ADR and candidate-selection review checkpoints. | Could eventually affect identity/dedup/rank behavior. No such changed fusion implementation is present in this comparison; qualify final result semantics before claiming reproduction. |
| Long-record guard / CUDA reset survival, contingent on later decisions/design. | May add embedding-extent or runtime facts worth exposing if actually available; not an implemented schema change or a reason to expand Inspector scope now. |

## Recommended implementation and qualification approach

1. Declare support against explicit 0.8.28/0.8.29 source/artifact baselines
   rather than only the word current. Keep the current non-mutating schema
   admission and one common Inspector stored-data model where valid.
2. Pin one available, qualified query/library implementation. Assess 0.8.29's
   pure snippet helper as a reuse candidate, without requiring a second engine
   runtime or assuming development code is already part of this branch.
3. Normalize optional snippets, degradation and evidence in the Inspector
   result boundary. Never turn unsupported evidence into a false healthy,
   uncut, exact or no-match assertion.
4. Prepare real databases through each pinned producer implementation and
   qualify record/operational detail, lifecycle, provenance, lexical routes,
   vectors, graph identities and source/cache preservation against them.
5. Independently test backend-specific snippets, sidecars, query routes and
   load behavior. Preserve identity/content/eligibility invariants across
   producer fixtures; do not require identical presentation bytes or timings
   across differing builds and Unicode dependencies.

These are recommendations for the Slice 20/30/40/50/60 reconciliation, not
changes to current release scope or a completed two-version test gate.

## Reproducible source evidence

Use the pinned SHAs above with `git diff` or `git show`; the files below refer
to those Git objects, not presumed files in the Inspector working tree:

- `src/rust/crates/fathomdb-schema/`: no diff, including `SCHEMA_VERSION`.
- `src/rust/crates/fathomdb-engine/src/search_types.rs`: new transient snippet/degradation fields.
- `src/rust/crates/fathomdb-engine/src/embedding.rs`: overload reporting and other-failure behavior.
- `src/rust/crates/fathomdb-engine/src/snippet.rs`, `snippet_query.rs`: pure selector and FTS span acquisition.
- `src/rust/crates/fathomdb-engine/src/search.rs`, `search_api.rs`, `filter.rs`: snippet plumbing, rowid metadata and projected/body/edge routes.
- `src/rust/crates/fathomdb-engine/src/reader_pool.rs`: idle-first dispatch.
- `src/rust/crates/fathomdb-engine/src/graph_expand/traversal.rs`: additive degradation carrier/non-exhaustive result, not a new graph schema.
- `src/rust/crates/fathomdb-sdk/src/options.rs`, `standalone.rs`: option construction and standalone helpers.
- `src/rust/crates/fathomdb-query/src/lib.rs`: compiled OR operands retained for highlighting.
- `dev/interfaces/rust.md`, `rust-sdk.md`: intended caller-visible contracts; read alongside the actual source rather than treating ADR future stages as implemented.
