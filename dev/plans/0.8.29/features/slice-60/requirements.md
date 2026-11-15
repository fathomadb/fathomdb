---
title: FathomDB 0.8.29 Slice 60 — MCP surface requirements and acceptance
status: PLAN_READY
target_release: 0.8.29
observed_on: 2026-11-15
---

# Slice 60 — MCP surface requirements and acceptance

This is the filtered-body-FTS portion of MCP-SURFACE, allocated by the owner
on 2026-11-04. The [release plan](../../../plan-0.8.29.md) also assigns compact
text output and search view flags to Slice 60; those obligations remain.
This record adds scope and acceptance, not an implementation-complete claim.
Identifiers below are release-local, not additions to `dev/acceptance.md`.

## Need and baseline

Agents need exact metadata and declared-attribute constraints on lexical body
search without opting into hybrid retrieval or enabling an embedder.
0.8.28 Slice90 (D28-14) supplies that engine/SDK operation. At the original
preimplementation baseline MCP accepted all five filter fields but rejected them
in text mode. Qualified subset5271 removed that refusal and forwards the closed
filter through the direct text SDK path. This requirement remains a control;
search existence flags and affected filter/view composition remain F11 work.

The completed0.8.28 Slice90 is already integrated at the actual10 successor
baseline. Preserve its `TextSearchOptions.filter` together with Block1's existing
snippet options. No new tool, filter grammar, storage change or engine retrieval algorithm
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
  .28 Slice90 filtered-body-FTS metadata semantics: status uses existing projected metadata (currently
  the empty-string sentinel); created_after is an inclusive projection-time
  bound. Missing metadata fails to match, including in historical views. Do not
  reinterpret JSON body status/time or populate metadata to satisfy a filter.
- **R29-60-FTS4 — Request identity and paging.** Canonical request identity and
  continuations retain every predicate and the effective view. Continuation
  calls cannot override filters. Existing budget, cap, epoch/stale-handle and
  hosted-state-change rules remain authoritative. With ample budget, `text` and
  `structured` output formats preserve the same selected references, order,
  underlying typed scores and predicates. Compact scores retain the accepted
  three-decimal presentation; compare displayed scores to that rounding of the
  same SDK value, not to a fabricated exact decimal value. Constrained budgets may produce different page sizes;
  each format must retain ranked selection and continuation correctness.
  Complete gap-free traversal requires a budget admitting at least one whole
  hit on every nonterminal page; budgets at the accepted minimum need not make
  progress. A zero-progress page follows the explicit BUD outcome below.
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
  predicates/view. Complete gap-free traversal up to the existing ranked cap
  applies when each nonterminal page's budget admits at least one whole hit;
  independently test arbitrary smaller accepted budgets and the BUD zero-progress
  outcome without claiming full traversal. Every returned prefix remains ordered
  and in-filter. Predicate changes affect canonical request identity;
  attempts to override predicates with a continuation are rejected. Re-derived
  pages after state advance still enforce the filter and retain the existing
  hosted best-effort caveat (`continuation_exact=false`); reopen/cross-store handles retain stale-handle rules.
- **AC29-60-FTS6 → FTS4/FTS5.** Run the filtered MCP cases under both output
  formats: ample-budget result parity, then each format's constrained-budget
  and continuation behavior. Replace
  the obsolete text-filter-refusal tests with observed RED then GREEN tests of
  this approved contract; run scoped MCP search, paging, schema and error tests.

## Format and view needs, requirements and acceptance

These extend the retained FTS1–5/FTS1–6 contract above. Authority is the
[approved compact design](../../../../design/agent-access/compact-text-block.md)
§7, Slice9 Batch4 and [release allocation](../../prework/slice-3/allocation.md).
N29-A/R29-REL-A → R29-3P-10/O3-10 and R29-3P-11/O3-11; agent needs
AN-006/007/009/010/011/014/015/018. Canonical NEED-010/012/020/023/026,
REQ-021/023/029/030/034/053 and applicable AC-023a/b/025/031/032a/b/038/074
constrain the work; no canonical IDs or thresholds are changed here.

| Local need → requirement | Falsifiable acceptance |
| --- | --- |
| N29-60-OUT → R29-60-OUT | AC29-60-OUT: strict server-wide `result_format` defaults to `text`; only `text`/`structured` accepted by JSON and embedded config validation. All seven tools keep identical names/input contracts, with no outputSchema/structuredContent in text mode, exactly one text block, and schema-valid structuredContent plus its exact compact JSON serialization as text in structured mode. Tool errors obey the same setting and isError=true. |
| N29-60-REF → R29-60-REF | AC29-60-REF: JSON-quoted refs/store/kind/source/edge/projection values recover spaces, quotes, backslashes, CR/LF, pipes and Unicode exactly where those values are valid. Search/expand have one LF-delimited hit line. Fetch frames verbatim UTF-8 body bytes, including header-like text and trailing newlines; one ordered outcome per input ref including duplicates/errors/cut bodies. No body bytes are normalized in fetch. |
| N29-60-BUD → R29-60-BUD | AC29-60-BUD: text used_tokens=ceil(final text UTF-8 bytes/4), including its count/hints/cut flags. Structured used_tokens preserves the existing exclusion of its structured budget object; final fit counts both final JSON copies including all budget overhead. Converge within four renders or fail internally; never return an oversize budgeted success. Search drops trailing whole hits and never recuts snippets. Complete traversal requires at least one whole hit fitting each nonterminal page; tiny budgets separately verify zero-progress behavior below. Fetch cuts bodies on UTF-8 boundaries without dropping items, or retains the documented typed refusal; expand cuts bodies then trailing targets and labels bound=budget. |
| N29-60-VIEW → R29-60-VIEW | AC29-60-VIEW: search accepts include_superseded/include_inactive independently and together, default false, and forwards them plus the fixed validity instant to ReadView for text and hybrid. Tests distinguish active/current, pending/deleted, superseded, validity lower/upper boundaries, node/edge semantics and same-build SDK controls. Invalid nonboolean values fail typed validation; continuation plus either flag, including false, is refused. No include_out_of_window argument is added. |
| N29-60-SIG → R29-60-SIG | AC29-60-SIG: the maintained field inventory covers every serializable output/error field (including nested fields) as kept, documented presentation, or approved unavailable-placeholder omission. Retain explanations, highlights, continuation_exact=false, readiness, affected kinds, runtime reason, hints and all current typed error details. No new state authority, calibrated verdict, weak_results threshold, default-store field, coverage report or degradation reason is implemented. |

## Accepted tiny-budget outcome

AC29-60-BUD also requires an independently authored zero-progress case in each
format. If ranked candidates remain but no whole leading hit fits after the
existing hints-off retry, and zero-hit mandatory metadata fits, return success
with zero hits, budget.cut=true, completeness=cut_by_budget, no continuation
and no hints. Its final bytes/count must satisfy that format's fit rule.
This is neither an empty retrieval nor complete traversal; callers can rerun
with a larger budget. Do not recut a search snippet to manufacture progress.
If even mandatory zero-hit metadata cannot fit, return the existing typed
bad_argument refusal. A genuinely empty ranked input separately keeps cut=false
and empty/empty_not_ready semantics, with hints only if their final bytes fit.

## Oracle and scope limits

[Design](design.md) owns framing/accounting and the built/new boundary;
[test plan](test-plan.md) joins every criterion to observed source bodies and
future failing assertions. Ample-budget parity uses a fixed validity instant,
immutable state and identical snippet run window. Current identity comes from
the SDK; duplicate-body cases must follow any accepted Slice50 successor on the
integrated build. Formatting never chooses a representative or sorts results.

Source shows no state-read/drift handle in this adapter: a hosted continuation
re-runs its stored request/offset on current state with continuation_exact=false.
Do not require a new state-advanced label or promise no gaps after writes.
Unchanged-state no-gap traversal requires the per-page one-whole-hit budget
condition; arbitrary tiny-budget prefix/zero-progress checks and reopen/cross-
store/expired-handle refusal remain separate obligations. Epoch alone proves neither freshness nor frozen semantics.

Slice80/90 own structured placeholder omission and not_in_this_release, new
argument/ref detail, default_store and span/coverage reporting; Slice70 owns new
reason vocabulary/remediation. Slice60 retains accepted text omission now and
renders already-real signals/errors. These are coordination seams, not new hard
runtime dependencies. At entry453b640f, qualified60 subset5271, integrated50
`9036507bc98ef31c05ae334aa22ec5101323c7e7`, completed70 and accepted80 design
are present. No new requirement/threshold is selected. F11 search existence
flags and affected FTS/view/continuation composition are implemented and scoped
passing in this branch; [status](status.md) distinguishes that evidence from
remaining coherent review/final source qualification. Accepted earlier evidence
retains its original subjects.

## Selected repeated-reference feedback boundary

Owner 2026-11-14 seq-375 selects the smallest atomic .29 correction: an otherwise
recordable call naming an ambiguous used/unused ref returns existing typed
bad_argument with zero label writes. Preserve current membership validation and
not_recorded precedence; repeated occurrences of the same cursor remain unique.
Integrated50 now implements this selected behavior and its existing tests cover
atomicity and precedence. This reconciliation does not independently rerun or
qualify it. R29-60-VIEW
exposure must compose with it, including both formats and mixed unique/ambiguous
named labels; unmentioned ambiguity does not label arbitrary records. See the
[selected decision](../slice-50/feedback-decision.md). .30 reviews richer occurrence
labels for precise per-result feedback without postponing the .29 correction.
