---
title: 0.8.25 Slice 60 — constrained combined graph expansion design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 1
target_release: 0.8.25
depends_on: 55
readiness_gate: 0.8.25 Slice 7 completion
---

# Slice 60 — constrained combined graph expansion design

## Authority and scope

This design owns R25/AC25-60, Memex need 15, the graph-path portion of need
12, and A25-05/A25-06. It is grounded in data-plane architecture v2 and extends
the existing graph store and bounded traversal. It does not add ontology,
relation inference, query decomposition, or a graph-specific semantic router.
Memex maps domain terms such as `slot` to the generic constraints here.

The design is reviewable but cannot become READY until Slice 7 activates
architecture v2. Slice 35 supplies frozen snapshots and eligibility; Slice 45
supplies the opaque continuation codec; Slice 50 supplies evidence references;
Slice 55 supplies explanation reasons and correlation.

## Requirements-to-design comparison

| Obligation | Existing behavior | Required design decision |
| --- | --- | --- |
| Query or explicit seeds | `graph.search_expand` seeds from ranked search hits | Add a closed seed-source union; never silently combine sources. |
| Direction and relation constraints | `neighbors` supports direction; combined expansion is effectively `both` and cannot filter relation | Carry direction and exact canonical `edge_kind` filters into the recursive query. |
| Target and attribute constraints | Not supported by combined expansion | Apply target kind and Slice 35 indexed eligibility before seed and frontier truncation. |
| One frozen view | Search and expansion can observe separate transactions | Require one Slice 35 snapshot token for seed search, traversal, hydration, and evidence. |
| Bounds and continuation | Depth is bounded but combined output has no governed page continuation | Retain depth 0–3 and add 1–50 path pages with Slice 45 opaque cursors. |
| Exact path evidence | Expanded nodes expose only node and hop count | Return seed, ordered edge/revision sequence, target, hop count, and evidence references. |
| Explanation | Current explanation identifies a graph arm, not paths or exclusions | Use Slice 55 reasons for ineligible, not selected, unavailable, and degraded. |

## Predecessor disposition

| Design or evidence | Disposition |
| --- | --- |
| `dev/design/retrieval.md` | **Preserve/reuse.** The fixed search pipeline and compact default results remain authoritative. |
| `ADR-0.8.0-graph-model-and-edge-addressing.md` | **Preserve/reuse.** The neutral binary property graph and canonical edge kind remain authoritative. |
| `ADR-0.8.0-graph-traversal-scope.md` | **Preserve/amend by successor.** Retain depth 0–3, cycle guards, and bounded local traversal; this design adds combined-path constraints and continuation. |
| Existing G5/G6 `neighbors` and `search_expand` contracts | **Compatibility predecessor.** They remain supported and unchanged; the governed v1 request is additive. |
| GRAPH-01 `protected_bridge_v1` | **Historical rejected evidence.** Do not reintroduce exact-anchor expansion or treat graph storage quality as proof of retrieval value. |

No predecessor document is edited or promoted by this design.

## Public and wire contract

All names below are normative contract names; SDKs render them idiomatically.
Every request and response carries `schema_version: 1` on the wire.

`GraphExpandRequestV1` contains:

- exactly one seed source: `query { text, ranked_limit }` or
  `explicit { logical_ids }`;
- `direction`: `incoming`, `outgoing`, or `both`;
- a Slice 35 hard eligibility envelope applying to every seed, node, and edge;
- `traversal`: sorted, duplicate-free optional `edge_kinds`, separately named
  `intermediate_node_kinds`, and indexed `intermediate_predicates`;
- `terminal`: separately named `target_kinds` and indexed `output_predicates`;
- the frozen-snapshot token;
- `max_depth` in `0..=3`, `page_size` in `1..=50`, and an optional Slice 45
  continuation cursor;
- `include_path_evidence` and `include_explanation`, both defaulting to false.

Query `ranked_limit` and explicit seed count are each `1..=100`. Empty strings,
empty explicit seeds, duplicate explicit seeds, an unknown enum, an unsupported
predicate, or a limit outside its range rejects before any read. Depth zero is
valid: it returns eligible seed nodes and no paths.

`GraphExpandPageV1` contains the bound snapshot identity, ordered seed results,
ordered `GraphPathV1` rows, an optional continuation, and optional explanation.
Each path contains `seed_logical_id`, `target_logical_id`, `hop_count`, and the
ordered `GraphPathEdgeV1` sequence. Each edge names its immutable edge revision,
canonical edge kind, logical endpoints, direction as traversed, and—when
requested—its Slice 50 `EvidenceRef`. Node evidence is likewise opt-in. Bare
seed hits retain the existing compact `SearchHit` shape.

Unknown request fields or variants fail with `UnsupportedVersion`. Additive
unknown response fields may be ignored. An unknown response variant affecting
identity, visibility, lifecycle, ranking, or continuation must surface
`UnsupportedVersion`, never a default value.

## Execution and ordering

1. Decode and validate the request, cursor, snapshot, eligibility envelope,
   declared projections, and all bounds before opening traversal.
2. For query seeds, execute `Engine.search` under the same snapshot and hard
   eligibility and truncate only after eligibility. For explicit seeds, resolve
   all IDs under that envelope before retaining their caller order.
3. Compile seed eligibility into seed selection. Compile edge kind,
   `intermediate_node_kinds`, and `intermediate_predicates` into recursive
   traversal eligibility. Compile `target_kinds` and `output_predicates` into
   terminal/output eligibility. A live intermediate node remains traversable
   when it fails only a terminal predicate; it is pruned only by an explicit
   intermediate constraint. No constraint may be client-side emulation.
4. Traverse breadth first with the existing cycle guard and materialize the
   complete bounded path set for depth `max_depth`. Attempting to discover path
   state 10,001 fails the request with `GraphExpansionBoundExceeded`; it emits
   no page and no continuation. A path is output-eligible only if every member
   is visible/live, every traversal constraint passes, and its terminal node
   passes every output predicate.
5. After complete discovery, sort by `(hop_count, seed_ordinal, edge_kind, from_logical_id,
   to_logical_id, edge_revision_id, full_path_revision_tuple)`. Page only after
   this order is established. The continuation resumes strictly after the last
   tuple and is bound to the full request, snapshot, projection generations,
   and ordering version.
6. Hydrate path/evidence/explanation data under the same snapshot. If the bound
   state cannot be reproduced, return a typed snapshot/cursor outcome; never
   return a partial page under a newer view.

The 10,000-state ceiling is a fail-closed materialization bound, not a partial
success mechanism. Every successful cursor therefore pages a complete globally
ordered set. The cursor's request digest binds the ceiling and ordering version;
later pages reproduce the complete set under the frozen snapshot before
keyset-resuming. No authenticated frontier is needed and no discovered prefix
is ever represented as globally complete.

## Persistence and migration

No graph-schema migration is introduced. The design reuses canonical nodes,
canonical edges, immutable revision identity from Slice 15, declared attribute
projections, and existing endpoint indexes. Cursors and evidence references are
opaque capabilities, not durable graph rows. The request/response wire schema
and ordering version are persisted only in receipts and serialized cursors.

## Invariants and typed failures

- Constraints execute before seed/frontier/path truncation.
- One request has one frozen visibility and validity boundary.
- Every emitted path is continuous, direction-correct, cycle-guarded, and
  source-resolvable; its hop count equals its edge count.
- Erased, invalid, superseded, or ineligible path members make the entire path
  invisible. They do not leak through explanations or stale evidence handles.
- Ranked top-K seed selection and ordered path pagination remain distinct.
- The accepted default search path and `use_graph_arm = false` do not change.

Typed outcomes include invalid argument, unsupported constraint/projection,
unsupported version, seed invisible/not found, snapshot unavailable/drifted/
expired, cursor mismatch/expired/drifted, projection unavailable/degraded,
`GraphExpansionBoundExceeded`, and storage failure. Bound exhaustion returns
no paths or continuation and cannot be qualified as a degraded successful page.

## Compatibility, lifecycle, and performance

Existing G5/G6 calls preserve their signatures and 50-result behavior. The new
request is additive and must have Rust, Python, TypeScript, wire, and applicable
Windows CPU/native parity. CUDA is exercised only when query seeding invokes an
accepted dense/rerank arm; traversal itself is CPU/SQLite work.

The performance claim is bounded execution, not graph-quality improvement.
Receipts record seeds considered, path states examined, paths returned,
constraint rejection counts, depth, page count, continuation count, degraded
state, latency distribution, and peak resources. No result promotes graph
expansion as a default retrieval treatment.

## Mapped RED/GREEN and verification

| Acceptance boundary | Required RED/GREEN proof |
| --- | --- |
| Direction/kind/target matrix | Real-database fixtures for all directions, mixed edges, and typed unknown values, including `A(kind X) -> B(kind Y) -> C(kind X)` with terminal `target_kinds={X}` and no intermediate-kind constraint. |
| Constraint before truncation | Adversarial fixture where the only eligible seed/path lies below the unfiltered cap; query-plan inspection proves native predicates. |
| Frozen traversal | Concurrent mutation, supersession, validity-boundary, and projection-generation races return the original page or a typed snapshot failure. |
| Deterministic continuation | Multi-page walk repeats byte-identically across reopen and has no duplicate/omission; request/order/generation mismatch rejects. |
| Exact evidence | Every edge/path resolves to its immutable revision and exact source; injected erasure and stale references disclose no bytes. |
| Bounds and safe order | Depth/seed/page bounds and cycles terminate; a shuffled graph with more than 10,000 same-hop paths fails with no page/cursor, while a below-cap multi-page walk has no duplicate, omission, or reorder across insertion permutations and reopen. |
| Compatibility | Existing `neighbors`/`search_expand` and default `Engine.search` fixtures remain byte-for-byte compatible where serialized. |

Run focused engine/query tests, SDK and wire fixtures, lifecycle/erasure tests,
heavy graph matrices, all/all-feature suites, Windows CPU/native Rust/Python/
Node, registry-installed graph/search smokes, and CUDA seeding tests when dense
or rerank is configured. Operator and live-model routes are not applicable.

An independent review may require at most three FIX-n cycles. Any unresolved
P1/P2 issue, post-truncation constraint, unbounded path, mixed snapshot, or
exact-anchor default treatment blocks READY.
