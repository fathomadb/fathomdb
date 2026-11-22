---
title: FathomDB Inspector — Fable-medium adversarial review
date: 2026-10-28
target_release: 0.8.30
desc: Independent Anthropic MCP review and author validation notes for Inspector requirements and design
status: ACTIVE
blast_radius: review record only; proposed findings are not accepted requirements or implementation changes
---

# FathomDB Inspector — Fable-medium adversarial review

> Current-policy note (2026-10-29): the owner subsequently permitted Engine
> startup when required. Earlier immutable/no-startup/preservation assumptions
> below are historical. Current [requirements](../requirements.md#engine-startup-allowance-2026-10-29)
> distinguish Inspector read operations from qualified Engine lifecycle effects.

## Scope and provenance

The owner authorized Anthropic models through the AI CLI MCP for this review
only. The MCP `fable` selection resolved to `claude-fable-5-1`; effort was
requested as `medium`. The reviewer examined commit `8d6c13343` on
`dev/0.8.30`, read-only, and returned normally. No second Anthropic review
or remediation run was commissioned.

Reviewed documents:

- [Needs, requirements, and acceptance criteria](../needs-requirements-acceptance.md)
- [High-level design](../design/high-level-design.md)

The report below is the reviewer's output, preserved verbatim. Its file/line
references refer to that reviewed commit. Findings are proposals for
consideration, not owner rulings. Requirements and design were not changed
as part of this review.

## Author validation notes

- **Migration premise in findings 2 and 8 is incorrect for this branch.**
  Public open accepts a nonempty database only at the current schema and
  rejects older versions before product mutation; it does not provide the
  forward migration fallback asserted by the reviewer. See the
  [engine authority](../../design/engine.md#open-path) and
  [open implementation](../../../src/rust/crates/fathomdb-engine/src/open.rs).
  Explicit supported-schema scope and legacy/incomplete metadata coverage
  remain useful questions. A fixture cannot assume a current public migration
  ladder. The claim that legacy metadata dominates real user databases was
  not established by this review.
- **Finding 1 combines different erasure obligations.**
  The pending-redaction collection cited is engine-owned telemetry-redaction
  bookkeeping containing stable identifiers, not itself proof that erased
  canonical payload remains. See
  [erasure implementation](../../../src/rust/crates/fathomdb-engine/src/erasure.rs).
  Governed eligibility, retained historical/inactive data, physical closure,
  telemetry redaction, and operator visibility need separate treatment.
  An Inspector disclosure policy is a product question; ordinary application
  visibility must not silently be assumed to define all operator inspection.
- **Finding 9's recovery recommendation is not adopted.**
  Nonempty WAL handling is a valid scope/UX question, but this report does
  not establish that data-loss-authorized recovery is the necessary remedy.
  Existing [inspection guidance](../../../docs/operations/integrity-inspection.md)
  explicitly says not to delete or checkpoint sidecars just to pass admission.
  No destructive recovery action follows from this review.
- **Finding 10 does not override the owner's UI requirement.**
  Structure-appropriate views and an interactive graph remain required.
  Automated payload checks and human-evaluation criteria can strengthen
  validation without making the graph canvas optional.
- **Reuse proposals are not integration decisions.**
  MCP/agent-access work may provide mechanisms worth reusing. The need to
  assess overlap does not establish that all services must share a new
  session abstraction or that work on another branch already exists here.
- Lexical table declarations alone do not establish active routing for every
  database. Detailed design should identify supported lexical structures and
  actual request routes rather than presenting every historical table as
  an active registered projection.

The most actionable follow-up topics are explicit system/legacy data coverage,
class-specific lifecycle fixtures, vector stage/metric attribution, lexical
route attribution, supported-schema and attachment scope, and a verified
no-download query-runtime path. No finding is marked remediated by this record.

## Reviewer report

**Model:** this review ran on Claude Fable 5.1 (`claude-fable-5-1`), the current Fable release. "Fable-5" is not a distinct resolvable version. Effort was requested as medium. Nothing was edited, staged, or spawned.

**Overall verdict: proceed to detailed design with fixes.** The identity rules, operational-mode split, quiescent-attachment model, and the existing-versus-new inventory are accurate against the code at 8d6c13343. The gaps that matter are about what real databases actually contain (legacy revisions, engine-written rows, owed erasure) and about engine facts the vector and lexical views will expose (two-stage metric, four FTS tables). None blocks the proposal; several would produce a wrong mental model if shipped as written, which is the one failure the product exists to prevent.

## Findings, most consequential first

**1. High. Read-only is not the same as non-disclosing, and the proposal never says which it means.**
`needs-requirements-acceptance.md:244` (INS-X01) and `high-level-design.md:179-183` define safety purely as "no mutation". The engine's governed read paths withhold rows: `ReadView` defaults exclude superseded, inactive (`pending`/`deleted`/`purged`) and out-of-validity rows (`temporal.rs:34-54`), and erasure leaves a durable owed-redaction queue, the operational collection `erasure_pending_redaction`, whose rows "hold only stable ids the engine is about to remove" (`erasure.rs:41-71`). `doctor check-integrity` reports `E_ERASURE_INCOMPLETE` per owed physical row (`dev/interfaces/cli.md:38`). An Inspector reading tables directly would show content the application has promised to erase, and would list the redaction queue as an ordinary collection. Consequence: a GDPR-sensitive leak path and a mis-modelled "what is stored" answer. Remediation: add a cross-cutting condition (INS-X06, nondisclosure) stating which view the Inspector reads under, that engine-internal collections and system rows are labelled as such, and whether a database with owed erasure work is refused or inspected with owed rows withheld. Add an owed-erasure case to F-04/F-05.

**2. High. Legacy and migrated-incomplete revisions are the common real-world state and appear in no fixture or concept.**
`_fathomdb_artifact_revisions.artifact_role` is `canonical_source | derived_semantic | legacy` and `completeness` is `complete | migrated_incomplete` (`fathomdb-schema/src/lib.rs:964-974`); legacy revision ids are synthesised with an `_fdb:m:` prefix (`identity.rs` around line 60). The conceptual table at `needs-requirements-acceptance.md:111` knows only source and derived, and every fixture (lines 258-266) is freshly written. The first audience's databases (Memex and similar) were migrated through many schema steps, so "unknown/unresolved" will be their dominant provenance state, untested. Remediation: add a "legacy / migrated-incomplete revision" row to the concept table and INS-07.2 label set; add a fixture built from a tracked pre-0.8.25 database file migrated by the current engine, exercising INS-01.3, INS-04.2 and INS-07.1.

**3. Medium-High. Engine-written rows live in `canonical_nodes` and the three-class model has no place for them.**
`RowKind::{Leaf, Coverage, Graph}` (`write_types.rs:9-19`) routes engine-internal writes with `SourceId::engine_derived(row_kind)` (`write.rs:511-538`). These are engine-owned, rebuildable rows stored in the canonical table, neither caller source records nor caller-authored derived records. The proposal treats structural role as a classification axis only (`needs-requirements-acceptance.md:108`), so INS-02.2 counts and INS-01.3 labelling would present them as caller canonical content. Remediation: add an "engine-derived structural row" concept; require INS-02.2 to count leaf rows separately from engine-derived rows; F-01 either seeds one or states that none can exist in the fixture and the label requirement still applies.

**4. Medium-High. Nodes and edges have different lifecycle and validity axes; the proposal assumes one.**
Nodes carry `state`, `superseded_at`, `valid_from/valid_until` (`lib.rs:366-368, 584-585, 704-705`). Edges carry `superseded_at`, `t_valid/t_invalid`, `confidence`, `extractor_model_id`, `temporal_fallback` and no `state` column (`lib.rs:846-860`); graph traversal filters on `t_invalid` (`graph_api.rs:602`). INS-01.2's "same terminology across all views" and INS-06.1's "eligible under a specified view" are undefined for edges, and F-04's "ineligible records" never requires an edge case. Remediation: split the lifecycle concept row by class; add an edge made ineligible by `t_invalid` and a node made ineligible by `state` to F-04; reword INS-01.2 to "same terminology where the underlying facts are the same; differing axes are labelled differently".

**5. Medium. "The engine's actual vector metric" is two metrics, and the view must say which.**
Phase 1 ranks 1-bit `embedding_bin` by Hamming distance (bits possibly mean-centred), then reranks a bounded candidate set by `vec_distance_l2` on retained f32 (`vector_storage.rs:45-56, 554-585`; `search.rs:1493-1553`). INS-09.3 (`needs-requirements-acceptance.md:236`) and `high-level-design.md:256-262` speak of one metric. An exact-L2 neighbourhood will disagree with search's vector arm whenever the bit stage excluded a candidate, which is exactly the "why wasn't this returned" confusion the tool targets. Remediation: INS-09.3 names both stages, discloses mean-centring, and states that the search arm's candidate set is bounded; F-07's "independently checked vector distances" names the metric being checked.

**6. Medium. There are four lexical projections with different fields and tokenisers; INS-09.2 describes one.**
`search_index` (body), `search_index_edges`, fielded `search_index_v2` (kind/body/status), and `property_search_index` (`lib.rs:224-228, 338-343, 425-430, 497-503, 930-935`), scored by SQLite `bm25()` or by a Rust-side BM25F over `fts5vocab` tokens (`search.rs:613-770`). No highlight or snippet call exists in the engine. Remediation: the structure view lists each lexical projection as its own declaration; INS-09.2 requires the route a request actually used to be identified. Match-centred snippets were built on `feat/agent-access-block1` (destination `dev/0.8.29`); the proposal should name that work as the reuse target rather than imply a fresh evidence design.

**7. Medium. Two sibling read-only access layers are unreconciled.**
The MCP server crate and the agent-access Block 1 program (admission queue, budgets, snippets, compact text) are read-only local services over the engine with the same budget, cancellation and token-estimate concerns as `high-level-design.md:285-307`. Block 1 dropped in-flight timeouts because the engine cannot interrupt a running read. The Inspector can interrupt only because it proposes owning its own connections, yet lines 123-125 and 240-241 also require reusing engine compilation, eligibility, fusion and ranking, which today execute inside the Engine-owned reader pool (`search_api.rs` dispatches `ReaderRequest` to `reader_pool`). The design rightly calls this factoring the largest risk (line 332). Remediation: add a section placing the Inspector against MCP and agent-access, and scope the "read-only inspection session" as a shared engine seam rather than a third service layer. State plainly that interrupt applies to Inspector-owned connections only.

**8. Medium. Supported schema range is unstated, and the fallback is a mutating open.**
`SCHEMA_VERSION` is 34; `Engine::open` migrates forward and the Inspector cannot. The design fails closed on unknown future schemas (`high-level-design.md:176`) and lists a "schema-version adapter" as new work (line 322) but never says whether older schemas are readable. If "current only", the only way to inspect an older file is to open it with the SDK, which mutates it. Remediation: INS-02.4 states the supported range; the unsupported-schema refusal must not direct users to a mutating open.

**9. Medium-Low. Nonempty WAL refusal routes users to a lossy verb.**
A crashed or non-checkpointing writer leaves a nonempty WAL. The precedent path refuses it (`operator/data_plane.rs:133-147, 396`), and the only in-product remedy is `recover --accept-data-loss --truncate-wal`. F-05 lists the case as a refusal but the UX consequence is unexamined. Remediation: decide whether a consistent WAL-present database may be read through a non-immutable read-only connection, or document the lossy-flag path as the supported answer.

**10. Medium-Low. Form mandates have no oracle.**
`needs-requirements-acceptance.md:173` says UI layout is not the oracle, yet INS-09.1 and INS-09.4 (lines 234, 237) mandate a diagram that "cannot be replaced" by a table. Remediation: define the testable service contract (graph payload with node and edge identities, direction, expansion; structured-value payload for detail) and move the form mandates into the human-evaluation checklist with explicit pass criteria.

**11. Low. Model download is still possible.**
The embedder loader downloads into the user cache when a model is absent (`fathomdb-embedder/src/loader.rs:11, 233-260`); the only root override is a test knob. INS-X01's "no implicit model download" needs a no-download loader mode, missing from the Retrieval row at `high-level-design.md:327`.

**Confirmed accurate** against code: `Engine::open` admits a writer and starts projection work; the immutable integrity path holds the lock via read-only `try_lock` without writing metadata (`data_plane.rs:366-382`); `read_get` under `include_superseded` silently resolves to the highest cursor, so exact-revision reads are genuinely new work; `graph_neighbors` returns nodes only and `graph_expand` targets carry only a terminal edge kind, so canvas edge payloads are new; latest-state writes do not retain history; content ids are body hashes for NULL-logical rows and passage ids are rerank ordinals.

## Suggested requirement, AC and design improvements

- Add INS-X06 nondisclosure, and an "engine-internal structure" label requirement under INS-01.1.
- Add concept rows for legacy/migrated revisions, engine-derived structural rows, and class-specific lifecycle axes.
- Add fixture F-08 (migrated legacy database file) and extend F-04 with an edge ineligibility case and an owed-erasure case.
- INS-09.2 and INS-09.3 enumerate the actual lexical projections and both vector stages.
- Design: add "Relationship to MCP and agent-access" and "Supported schema range" sections; add the no-download loader mode to the new-work table.

## Unresolved product decisions versus implementation details

Product decisions the owner must make:

- Disclosure policy for owed-erasure, purged and inactive rows.
- Supported schema range for attachment.
- Nonempty-WAL handling.
- One shared read-only session seam across Inspector, MCP and agent-access, or three services.
- Which vector metric(s) the neighbourhood view presents.
- Whether the Inspector is an in-repo crate and release artifact, and the ADR covering its binary contract alongside the CLI scope ADR's rejected Option B.
- Quiescent-only versus live attachment (already listed in the proposal).

Implementation details, not decisions: UI framework, packaging, page and chunk sizes, performance targets, keyset continuation, loopback credential scheme.

**On the Astra review**, read last: its two material findings (operational-mode split, external-writer precondition) are correctly resolved in the current text. Findings 1 through 8 above were not raised there.
