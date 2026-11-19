---
title: Graph seeding and containment consumer review
status: PROPOSED
date: 2026-11-19
---

# Graph seeding and containment consumer review

Memex correctly found stale graph-arm documentation and a missing edge-vocabulary
selector on combined search. Its broader lack-of-containment concern is not
supported as stated: node eligibility already constrains seeds, traversal and
emitted graph candidates. A separate existing API, `graph_expand`, already
provides explicit relevance-selected seeds, edge-kind filtering and seed/depth
origins. Use that route before adding another integrated retrieval mode.

## Sources and verification

Read the complete [Memex consumer draft](https://github.com/coreyt/memex/blob/0785a803/dev/fathomdb/0.8.29-graph-arm-seeding-and-containment.md)
at commit `0785a803`. It explicitly says
**NOT FILED** and is an owner-discussion proposal. It addresses graph retrieval,
not the separate [extractor identity/provenance proposal](ingest-with-extractor-memory-graph-proposal.md).

Source baseline: FathomDB `release/0.8.30` at
`5f4edd9831ddbb5ee12ebe594a3f6cd17fcc46b8`. Compared against `v0.8.29`
(`b64d32f80`). `git diff --name-only v0.8.29 HEAD --` returned no changes for the
inspected search, search API, graph expansion, filter, frozen-context, temporal,
evidence and explanation implementations, the cited Python/TypeScript docs,
Rust interface, and the relevant graph frontier/expansion tests. The findings
therefore apply to the tag and this current release baseline. This review read
source/contracts/tests; it did not execute a runtime reproduction or test suite.

## Disposition of the six asks

| Ask | Verified status | Minimal response and acceptance |
| --- | --- | --- |
| 1. Correct graph-arm docs | Confirmed documentation defect in Python and TypeScript references, Rust API docstring and internal search comment | Replace fused-hit claim with actual edge/node FTS seeding and precise bounds. Validate docs and source comments; assert existing seeding behavior rather than changing rankings to match stale prose. |
| 2. Optional fused-hit seeding | Not an option on combined `search(use_graph_arm=true)`. Existing `graph_expand` accepts caller-ordered logical-node seeds | First document/prove frozen retrieval with graph disabled, followed by explicit expansion. A new integrated seed mode is optional new API/ranking scope; define its ordering before implementing. Test vector-only logical-node relevance, bodyless traversal and no circular fusion. |
| 3. Per-hit seed/depth origins | Partly present. Search explanation has graph rank/structural origin; search evidence has edge-seed or terminal-edge/hop origin. `graph_expand` already identifies winning seed, predecessor, depth, edge kind and direction | Expose the existing route clearly. Only add missing ordinary-search root-seed provenance if that route is still needed, with an explicit versioned explanation/evidence design and same-state tests. |
| 4. Eligibility before graph caps | Already implemented for node kind/attributes, lifecycle, validity and registered dependency eligibility in combined search, including frozen search. It is not merely final-hit filtering | Run/extend the existing targeted pre-cap and bridge tests. Fix only a demonstrated gap. Clarify node eligibility versus independent edge policy; do not apply a node-kind selector to relation kinds. |
| 5. Edge-side kind/namespace selector | Combined graph arm has no independent edge-kind selector. Existing `graph_expand.edge_kinds` already filters traversal in both current and frozen modes | Prefer existing bounded graph expansion. If integrated search needs it, add a separately named optional edge-kind allowlist at edge seeding and traversal, before caps, with API/context binding and tests. Edge-attribute filtering would be additional scope. |
| 6. First-class plane/namespace | No such universal partition is supplied by the inspected API. IDs, kinds and existing declared filterable node attributes are mechanisms, not tenant authorization | Defer a universal partition design until the consumer identifies what existing eligibility cannot express. It affects every retrieval/read/write/projection/evidence/lifecycle boundary and needs separate owner scope. |

## What combined search actually does

All source references below are under `src/rust/crates/fathomdb-engine/` unless
prefixed otherwise.

`src/search.rs:1479`–`:1533` seeds from query-matched edge-body FTS and logical-node
FTS, with a limit of ten matched edges and ten matched node rows respectively.
Both endpoints of each selected edge are candidates. It does not choose the top
ten fused hits; the two-arm fused input is used to exclude the same already
covered canonical record, not as the frontier source (`:1536`, `:3081`–`:3105`).
An edge without a body cannot itself be an edge-FTS seed, but can be traversed
from **either** an eligible entity-FTS seed or an endpoint seeded by a different
matching body-bearing edge. Thus the consumer's "only type-B" qualification is
too narrow. Dense/CE relevance does not select this frontier.

Node eligibility is applied by `append_node_eligibility_sql_view`:

- Edge seed selection requires at least one eligible endpoint before its ten-edge
  cap (`src/search.rs:1592`–`:1640`); resolution then checks each endpoint itself
  (`:1737`–`:1760`). The OR is deliberate current behavior: a matching edge with
  one excluded endpoint may still seed its eligible endpoint. This is not a
  promise that both ends of every seeding edge are in one plane.
- Logical-node FTS applies eligibility before its ten-row limit
  (`:1696`–`:1711`).
- Each traversal query requires an eligible target before `LIMIT 64`
  (`:1824`–`:1864`), and selected body hydration checks eligibility again
  (`:1869`–`:1893`). Excluded bridges are not enqueued for further traversal.
- Candidates are checked before the overall graph cap of fifty. Current traversal
  is both directions, depth at most three, with visited logical-ID cycle control;
  final search result limits and CE processing occur later. These are bounded
  heuristics, not exhaustive graph recall or a configurable direction surface.

`src/filter.rs:600`–`:684` supplies node kind and declared attribute predicates;
attributes are node-scoped. `SearchFilter.kind` is one scalar kind, not a set.
An existing declared/populated plane attribute can constrain multiple node kinds;
a return-only output filter cannot substitute for traversal eligibility. Edge
FTS hits in the text/vector arms have their own relation-kind semantics
(`src/filter.rs:1008`–`:1025`); the graph's node filter must not be blindly copied
onto edge kinds. A shared edge table is not evidence of a missing node filter.
No filter means the caller has requested no plane constraint.

The graph arm applies node existence/validity/dependency predicates and shipped
edge supersession/recency plus temporal-fallback rejection. Node relaxation does
not mean arbitrary edge liveness relaxation. In particular the shipped edge rule
is invalid-time recency, not a newly promised `t_valid <= as_of` rule. See
`src/temporal.rs:143`, `:200` and
`tests/slice60_graph_expand.rs:489`.

An existing direct regression already exercises frozen combined search:
`tests/slice35_graph_frontier_pretruncation.rs:37` places 64 excluded targets
before an eligible 65th edge and requires the latter to surface. This materially
answers Memex's marked-unverified frozen-filter question. Further source tests
cover FTS seeding (`tests/pr_c1_graph_seeding.rs`) and graph evidence
(`tests/slice50_evidence.rs:684`, `:2481`); these were inspected, not rerun here.

## Existing supported retrieve-then-expand workflow

1. Mint one frozen context with the intended node eligibility and a strict view.
   Minting resolves `valid_as_of` once; authentication binds that instant and
   context (`src/frozen_read.rs:214`–`:218`, `:316`–`:334`).
2. Call `search_frozen` with `use_graph_arm=false` and the desired hybrid/optional
   CE settings. This retrieves relevance-selected hits without graph feedback.
   Its eligibility is taken exclusively from the frozen context
   (`src/search_api.rs:46`–`:108`).
3. In returned order, select distinct **logical node** IDs, at most 25, according
   to the caller's requested seed policy. Do not reinterpret `h:` anonymous
   records, `p:` passages or logical edge hits as entity-node seeds. An empty
   selection means no graph expansion; it is not a valid empty explicit request.
4. Call `graph_expand` with those explicit seeds, the **same frozen token**, an
   explicit edge-kind allowlist, chosen direction, depth, work and result bounds,
   and optional explanation/evidence. Node context eligibility controls all
   admitted seeds and intermediates. `target_kinds` is **return-only**: a node
   excluded only by that list may still bridge to another target. Use context
   eligibility for containment (`src/graph_expand/execution.rs:900`–`:911`).
5. Keep the relevance-ranked list and structural expansion result distinct, or
   define an application merge policy explicitly. Graph expansion does not
   synthesize comparable search scores or re-fuse/rerank the original list.

This obtains dense- or CE-selected expansion today when the relevant records are
logical nodes, and traverses bodyless edges without adding fake edge bodies.
The API resolves every explicit seed or refuses the whole operation. It permits
1–25 unique logical seeds, depth0–3, results1–50 and work1–10,000. It rejects
`include_superseded`/`include_inactive` contexts. Exact evidence additionally
requires frozen context, complete provenance and `include_out_of_window=false`.
See `src/graph_api.rs:484`–`:532`,
`src/graph_expand/execution.rs:374`–`:433`,
`dev/interfaces/rust.md:2096`–`:2133`.

Frozen contexts reproduce or refuse; they do not retain a historical SQLite
transaction across the two calls. A visibility-affecting write/projection change
can make the second call fail with state drift; retry the **entire pair** with a
fresh token, not just expansion against new state. Current-only two-call
composition would not establish matched state/time.

`graph_expand` is exhaustive within its work bound and returns no partial result
on exhaustion. Its work counts incident rows **before** filtering; excluded
relations or nodes can exhaust work. This is not the combined arm's eligible
`LIMIT 64` behavior or a guarantee of perfect early-filter efficiency. Existing
proofs include `tests/slice60_graph_expand.rs:184` (direction/kinds), `:309`
(work), `:416` (eligibility), `:779` (frozen races).

Existing expansion origins already carry `seed_logical_id`, `seed_ordinal`,
predecessor, target, hop count, terminal relation and direction
(`src/graph_expand/types.rs:110`–`:169`). Search explanation only supplies
`graph_rank` plus structural inclusion (`src/search_types.rs:332`–`:365`);
search evidence supplies a seeding edge or terminal-edge revision and hop count,
not the root seed/full path (`src/evidence.rs:128`–`:142`). Avoid claiming either
"no provenance exists" or "ordinary search already has full seed provenance."

## Ready documentation correction draft

Replace the Python `use_graph_arm` paragraph at
`docs/reference/python-api.md:169` with:

> `use_graph_arm` (`bool`, default `False`) enables the graph's FTS-seeded third
> retrieval arm. Seeds come from up to ten query-matching eligible edge-body FTS
> rows (their eligible endpoints) and ten query-matching eligible logical-node
> FTS rows. They are not selected from the hybrid or cross-encoder ranking.
> Traversal follows eligible temporal edges in both directions to depth three,
> admits at most fifty graph candidates, and examines at most sixty-four eligible
> incident edges per frontier node. Node search eligibility applies to seeds,
> intermediate nodes and returned candidates before those candidate limits;
> node filters do not provide an independent relation-kind allowlist. Graph
> candidates join the search's RRF pipeline. Bodyless edges can be traversed but
> cannot themselves seed edge-body FTS. For explicit relevance-selected seeds,
> relation filters and compact seed/depth origins, use bounded `graph_expand`.
> The default `False` leaves the two-arm path selected. Non-boolean values raise
> `TypeError`.

Apply equivalent language to TypeScript `useGraphArm`
(`docs/reference/typescript-api.md:176`), the Rust API docstring
(`src/search_api.rs:997`) and the stale internal comments
(`src/search.rs:3082`–`:3090`). Clarify edge invalidation at `t_invalid <= as_of`,
supersession/fallback handling and the distinct `graph_expand` contract. This is
a proposed correction only; the investigation has not edited product docs.

## Proposed bounded next steps and tests

- Correct stale docs, then prove the existing route with a small real-store
  consumer case. Reuse existing frontier, frozen, graph-expansion and evidence
  tests before adding duplicates. Add only discriminating gaps: foreign-plane
  FTS seeds preceding eligible seeds; a filtered bridge with an otherwise eligible
  far target; foreign-node pressure before frontier/result caps; node attribute
  containment over several kinds; edge with only one eligible endpoint.
- Exercise vector-only logical seeds connected by a bodyless allowed edge;
  verify no independent graph-FTS match is required for explicit expansion.
  Confirm original relevance scores/order are untouched. Include h:/p:/edge-ID
  rejection or explicit caller exclusion, not invented identity conversion.
- For the composed route, reuse one authenticated token, vary writes and
  projection publication between calls, and assert matched-state success or
  typed refusal. Cover fixed validity instants, inactive/superseded endpoints,
  dependency closure, expired edges and unsupported relaxation/evidence modes.
- Cover allowed/disallowed relation kinds between two eligible memory nodes,
  all directions, depth0/3, cycles, parallel edges, exact cap/work boundaries,
  and deterministic winning seed/depth/terminal-edge origins. Assert filtered
  rows consume graph-expansion work without appearing in successful output.
- Only if a single ranked combined API is still required, draft an optional
  integrated selector. Pin whether seeds use pre-CE hybrid ranking or a separate
  CE pass; never define them from the final graph-inclusive result. Specify seed
  eligibility, logical-node selection, tie order, stored-record deduplication,
  exclusion of already-covered records, recency/importance/CE ordering, and
  one-snapshot score/explanation semantics. Preserve default numerical/ranking
  behavior and accepted record identity; do not promise equivalence to today's
  graph arm or silently add a second CE invocation.

The docs defect and a demonstrated regression in an existing eligibility
contract are plausible bounded correctness work, subject to the Slice30 owner's
scope reconciliation. Adding integrated edge selectors/fused seeds, changing
seed-edge OR to both-endpoints containment, extending explanation/evidence
schemas or introducing universal namespaces needs explicit additional scope and
design review. Investigation does not assign these changes to Slice30 or alter
Memex's exact0.8.29 pin. Memex's note itself says none of these asks is a0.6.2
dependency. Choose whether existing explicit expansion meets its needs before
commissioning a new search mode.
