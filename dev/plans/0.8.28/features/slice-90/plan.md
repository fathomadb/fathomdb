---
title: FathomDB 0.8.28 Slice 90 — filtered direct body FTS
status: design approved; implementation in progress
target_release: 0.8.28
---

# Slice 90 plan and design

Baseline: `6726a08132ebf923ba0850184dc406a75d9ab0a4` on dev/0.8.28.

## Reconciliation and scope

1. The October 13 D28-14 draft and October 31 allocation require the existing
   closed SearchFilter on direct node/edge body FTS, before truncation,
   deduplication, ranking and limit. No standalone Slice 90 plan existed.
2. Slice 40 already compiles metadata and declared filterable attributes into
   node and edge FTS SQL. Slice 80 moved node predicates to canonical state,
   admitted independent ReadView existence flags, and retained historical rows.
   Reuse that serving path and its snapshot-local attribute validation.
3. Direct text requests currently hardcode no filter. The fixed node cap is 100;
   edge candidates are uncapped. The optimized node rank stream is only used
   without a filter. Keep these limits and ranking algorithms unchanged.
4. Rust SDK TextSearchOptions, Python wrapper/native and TypeScript wrapper/native
   omit the filter. Existing positional query/view calls must remain valid.
5. Slices 50/60/70 changed provider admission, format, and vector eligibility;
   Slices 81–85 changed erasure and format 36. This slice needs no vector,
   provider, storage, erasure, schema or migration change. Pending explicit
   erasure intent keeps its existing visibility until maintenance.
6. Allocated draft items are D28-14 only. Accept its scope; reject a new filter
   grammar, arbitrary JSON predicates, property-FTS redesign, MCP, ranking
   overhaul and benchmark campaign. Property FTS and hybrid search stay distinct.

## Requirements and local acceptance

- S90-1: all SearchFilter fields are forwarded in Rust core/SDK, Python and
  TypeScript. Metadata equality, inclusive created_after and AND attributes use
  existing semantics. Unknown/non-filterable attributes return InvalidFilter
  from the same reader snapshot, including queries with no lexical matches.
- S90-2: node and edge predicates act in SQL before candidate bounds and fusion.
  A selective node beyond 100 stronger unfiltered matches is returned at limit 1.
  Node-scoped attribute predicates exclude edges; source_type=edge_fact admits
  edges, kind uses edge relation kind, and existing edge status/time semantics
  remain. Filter rejection cannot let a duplicate body mask an eligible hit.
- S90-3: ReadView validity and each existence flag compose with filters. At fixed
  time/corpus/filter, smaller limits are exact ordered ID/score prefixes of larger
  limits, including node/edge duplicate bodies. None/empty filters retain the
  unfiltered contract and existing positional calls.
- S90-4: direct search remains provider-free, FTS-only, with no vector fallback,
  CE rerank, graph expansion or property search. Real fresh provider-free stores
  exercise it; existing no-provider recovery tests remain relevant. Provider-free
  means callable without inference, not new metadata population: status and
  created_after still require existing vector metadata. Missing rows fail to
  match, status uses the empty-string sentinel, and created_after is inclusive
  projection time, not JSON body metadata. Historical views retain this rule.
- S90-5: public docs/types match executable bindings, malformed filter strings
  retain existing FFI validation, invalid view/limit/filter types retain typed
  rejection. Fresh native artifacts exercise both language wrappers, not a stale
  extension. Existing body-prefix, view and filter-owner tests stay green.

## Design

Add `Engine::search_text_only_filtered(query, Option<SearchFilter>, &ReadView,
limit)` as the single full-arity body; existing core methods delegate with None.
This is a parameterized form of the existing operation, not a new retrieval mode.
It validates limit/open/query in the existing order and sends an owned filter
through `SearchReaderWork::text_only`. Normalize an empty SearchFilter to None so
it retains the unfiltered optimized path. Existing search execution validates
attributes on its reader transaction and compiles both SQL predicates before
rank collection. No filtering of returned top-K hits is added. Preserve the existing metadata
source: status/created_after consult vector_default (including its missing-row
fail-to-match behavior); do not substitute JSON body status or timestamps.
Tests pin projected positive controls, inclusive boundaries and provider-free
negative controls. This reads metadata without executing dense retrieval.

Rust SDK adds `filter: Option<SearchFilter>` to TextSearchOptions, default None.
Python adds keyword-only `filter` after positional view, forwarding the existing
flattened native metadata/attribute arguments (new native arguments keyword-only;
existing native positional limit retained). TypeScript exports TextSearchOptions
as SearchOptions plus optional SearchFilter; destructure filter before splitting
ReadView/limit and append filter to the existing native positional arguments.
Use the established native SearchFilterInput converter. Do not extend shared
SearchOptions, which would imply unsupported filters on other operations.

Update the three interface documents in the implementation commit. No ADR is
needed: the closed filter semantics and accepted D28-14 placement are unchanged.

## TDD implementation and verification

Astra design review precedes code. Commit failing real-database tests, observe
RED, then implement GREEN for (1) core selective metadata/attribute/edge/view and
prefix behavior; (2) SDK forwarding; (3) Python and TypeScript binding forwarding
and validation. Read existing owner tests before reusing fixtures. Tests may fail
at the missing public API during RED, then must exercise real results on GREEN.
Use assertions with controlled JSON metadata, declared projections and fixed
validity time. Retain existing tests unchanged except public signature fixtures
that deliberately encode the extended contract.

Run focused engine/SDK tests, touched-crate formatting/lint/check, Python/TS
native builds and affected binding tests. Astra code review and a separate Astra
verification subagent run scoped blast-radius checks and inspect acceptance.
Run one final full gate for this code phase if the checkout preflight permits;
report setup failures and inherited failures without inventing a green result.
Resolve review findings with narrow tests. Use gh only for CI work; no new CI,
GPU or paid benchmark run is needed to establish this lexical-only behavior.

Write concise status with evidence and limits, update release JSON and generated
views, fast-forward dev/0.8.28 and release/0.8.28 after checking both are clean,
and remove this task's temporary worktree/branch. Publication remains separate.

## Design review

Independent Astra review requested explicit status/time metadata semantics. The
clarification above preserves the authoritative G10 contract rather than adding
JSON metadata population. Astra rereview APPROVE: provider-free lexical filters
work while unavailable status/time metadata fail to match. API shapes, snapshot
validation, pre-cap predicates and bounded scope approved.
