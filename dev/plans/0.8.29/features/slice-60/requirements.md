---
title: FathomDB 0.8.29 Slice 60 — filtered body FTS requirements and acceptance
status: PLANNED
target_release: 0.8.29
observed_on: 2026-11-04
---

# Slice 60 — MCP filtered body FTS

This is the filtered-body-FTS portion of MCP-SURFACE, allocated by the owner
on 2026-11-04. The [release plan](../../../plan-0.8.29.md) also assigns compact
text output and search view flags to Slice 60; those obligations remain.
This record adds scope and acceptance, not an implementation-complete claim.
Identifiers below are release-local, not additions to `dev/acceptance.md`.

## Need and baseline

Agents need exact metadata and declared-attribute constraints on lexical body
search without opting into hybrid retrieval or enabling an embedder.
0.8.28 Slice 90 (D28-14) now supplies that engine/SDK operation. The current MCP
`search` already accepts all five filter fields, but rejects them in `text`
mode and forwards them only on the hybrid route. Its schema descriptions,
tool description and interface document still describe that restriction.

Merge the completed 0.8.28 Slice 90 before implementing this exposure. Reconcile
its `TextSearchOptions.filter` with Block 1's existing snippet options, keeping
both. No new tool, filter grammar, storage change or engine retrieval algorithm
belongs to this addition. SDK parity uses the same merged build, including any
accepted Slice 50 fusion-identity successor; it does not freeze the historical
body-deduplication algorithm. MR-020's exact-literal/phrase proposal is separate.

## Requirements

- **R29-60-FTS1 — Surface.** MCP `search` accepts its existing flat `kind`,
  `source_type`, `status`, `created_after` and `attributes` fields in resolved
  `text` mode, whether explicitly requested or selected by operator config.
  Forward the complete closed SearchFilter through the direct body-FTS SDK
  operation, preserving view, limit and snippet options. Absent/empty filters
  retain unfiltered behavior. Search-mode defaults are unchanged.
- **R29-60-FTS2 — Selection.** Apply predicates inside the engine before candidate
  truncation, body deduplication, ranking and result limit. Preserve node/edge
  kind and source-type rules, node-scoped AND attribute equality, snapshot-local
  declared-filterable validation, validity and both existence flags. MCP must
  not filter returned top-K hits or substitute hybrid/property-FTS retrieval.
- **R29-60-FTS3 — Metadata and provider independence.** Text mode performs no
  query embedding, dense retrieval, CE reranking or graph expansion. Preserve
  Slice 90 metadata semantics: status uses existing projected metadata (currently
  the empty-string sentinel); created_after is an inclusive projection-time
  bound. Missing metadata fails to match, including in historical views. Do not
  reinterpret JSON body status/time or populate metadata to satisfy a filter.
- **R29-60-FTS4 — Request identity and paging.** Canonical request identity and
  continuations retain every predicate and the effective view. Continuation
  calls cannot override filters. Existing budget, cap, epoch/stale-handle and
  state-advanced rules remain authoritative. With ample budget, `text` and
  `structured` output formats preserve the same selected references, order,
  scores and predicates. Constrained budgets may produce different page sizes;
  each format must retain ranked selection and continuation correctness.
  Output format and search mode are separate settings.
- **R29-60-FTS5 — Validation and discovery.** Malformed filter types/values and
  undeclared/non-filterable attributes preserve typed MCP error translation
  (`invalid_request/bad_argument` for engine InvalidFilter). Remove the obsolete
  text-mode filter refusal and hybrid-only descriptions from tool discovery,
  schemas, interface and active design docs when implementing the change.

## Acceptance criteria

Use real SQLite stores and MCP tool calls, with direct SDK calls as the parity
control on the same immutable state, fixed validity instant and ample budget.

- **AC29-60-FTS1 → FTS1/FTS5.** Each field independently, then representative
  conjunctions, is accepted and enforced in explicit and config-selected text
  mode. Check absent filters, an empty attributes object and incompatible
  predicates. Tool-list schema/descriptions advertise the actual supported
  modes; hybrid behavior remains unchanged.
- **AC29-60-FTS2 → FTS2.** More than 100 stronger nonmatching node candidates
  precede a selective match; filtered limit 1 still returns that match. Exercise
  node and edge source/type/kind selection, AND attributes excluding edges,
  duplicate bodies and smaller-limit ID/score prefix stability against the SDK.
- **AC29-60-FTS3 → FTS2/FTS3.** Compose filters with as_of boundary instants and
  each existence flag, including both together, for nodes and edges under their
  engine validity rules. Missing historical metadata fails to match. Unknown
  and declared-but-non-filterable attributes return the typed error even when
  there are no lexical matches; malformed wire arguments are rejected.
- **AC29-60-FTS4 → FTS3.** A fresh provider-free store returns matching lexical
  results for kind/source/attribute filters without a spurious vector fallback
  or provider-required error. Real projected metadata supplies positive empty-
  status and inclusive-time controls; fresh/missing metadata supplies negative
  controls. Prove the direct route invokes no inference or alternate retrieval.
- **AC29-60-FTS5 → FTS4.** On unchanged state, filtered continuations preserve
  predicates/view with no duplicate, missing or out-of-filter results up to the
  existing ranked cap. Predicate changes affect canonical request identity;
  attempts to override predicates with a continuation are rejected. Re-derived
  pages after state advance still enforce the filter and retain the existing
  state-advanced caveat; reopen/cross-store handles retain stale-handle rules.
- **AC29-60-FTS6 → FTS4/FTS5.** Run the filtered MCP cases under both output
  formats: ample-budget result parity, then each format's constrained-budget
  and continuation behavior. Replace
  the obsolete text-filter-refusal tests with observed RED then GREEN tests of
  this approved contract; run scoped MCP search, paging, schema and error tests.

Slice 60 requirements/design review must include these criteria alongside its
compact-output and view-flag criteria before implementation. Update the active
MCP contract with implementation; do not claim this planned support ships yet.
