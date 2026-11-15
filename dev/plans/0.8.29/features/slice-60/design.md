---
title: Slice 60 — MCP surface design
status: PLAN_READY
target_release: 0.8.29
observed_on: 2026-11-15
---

# Slice 60 — MCP surface design

## Authority and current source

This design implements [requirements](requirements.md), the approved
[compact direction](../../../../design/agent-access/compact-text-block.md) §7
and Slice9 Batch4. Entry453b640f contains accepted subset5271, integrated50
`9036507bc98ef31c05ae334aa22ec5101323c7e7`, completed70 and accepted80 design.
[Execution](execution.md) retains the prior source/qualification history;
[plan](plan.md) governs the remaining F11 and its fresh independent review.

Format/filter/budget/grammar behavior below is implemented. The F11 continuation
adds search existence-flag exposure with canonical/continuation/composition
proof and is now implemented and scoped passing in this branch.
Fresh coherent review and final source qualification remain.
Integrated50 satisfies the selected feedback prerequisite; preserve its guard
and record identity.70's actual reasons survive both renderers;80/90's accepted
new fields remain later implementation. No native/build/database/model work ran
in this source-grounded documentary reconciliation.

## Existing versus remaining

Paths in this section are relative to `src/rust/crates/`.

| Source witness | Existing authority / Slice60 delta |
| --- | --- |
| `fathomdb-mcp/src/config.rs` Config/strict keys | Strict result_format selection and text default are qualified; no config delta remains. |
| `fathomdb-mcp/src/edge.rs` router/to_result | Format-bound output schemas and exact structured JSON text are qualified. SearchArgs schema exposes both new nonnull boolean input properties. |
| `fathomdb-mcp/src/tools.rs` SearchArgs/search/search_filter | Direct text already forwards all closed filters and snippets. Both search flags now forward through ReadView for both routes; fetch keeps its own existing view semantics. |
| `fathomdb-mcp/src/shape.rs` CanonicalSearch | Filters and final accounting are implemented. Effective bool flags omit false in serialization, survive continuations and reject explicit overrides, including false/null. |
| `fathomdb-mcp/src/output.rs` and `src/error.rs` | Qualified renderers retain real fields/errors. F11 needs no new output field;90 owns new fields and structured placeholder omission. |
| `fathomdb-sdk/src/engine.rs` search_text_only/search; `src/options.rs` | Both accept ReadView. Integrated50 owns record-version identity; parity uses positional ID/body/order/raw-score controls without adapter deduplication. |
| `fathomdb-mcp/src/session.rs` Continuation/ResponseContext | Canonical request, epoch, offset and validity instant are stored; Unique/Ambiguous feedback folding is implemented. No new state authority or occurrence token. |

Render ordered typed results unchanged, including distinct records/revisions
with identical bodies or refs.50 validates all named feedback refs only after
successful acquisition and final epoch validation, before label resolution or
bridge/write. Existing not_recorded outcomes retain precedence. Compose F11
with that guard without reimplementing it. Fetch resolves latest eligible logical
revision under its own view and does not promise the searched occurrence.

## Protocol and pinned APIs

Verified November10,2026 against [MCP 2025-06-18 tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools#structured-content):
outputSchema is optional; declaring it requires conforming structured results.
Structured results should also have serialized JSON TextContent. Unstructured
content without either field is allowed. Tool execution errors use isError;
protocol faults retain JSON-RPC errors. Client display is not specified.

Cargo pins rmcp and rmcp-macros `=3.5.1`; local registry source was inspected,
not an assumed latest API. In rmcp `src/model/tool.rs`, output_schema is an
Option omitted when None; `src/model.rs` similarly omits absent
structured_content. success/error constructors accept text blocks. In
`src/handler/server/router/tool.rs`, ToolRouter.map exposes ToolRoute.attr;
list_all clones those attrs, and get returns the same attr. In macros
`src/tool_handler.rs`, generated list_tools/get_tool use that router. Thus
construct the ordinary typed router once per server connection, then clear
route output_schema for text format before storing it. This covers discovery
and get_tool consistently without a custom protocol wrapper or macro fork.
Keep seven names and input schemas identical across formats; output schemas
are stable within a configured server. Leave rmcp negotiation/metadata alone.

## Configuration and architecture

Add public Config field `result_format: String`, validated to exactly `text`
or `structured`, default `text`. A crate-private parsed enum may carry the
internal branch; no new public enum/allowlist exception is needed. JSON wrong
type/null/unknown/case variant fails naming result_format; embedded constructed
Config receives identical validation in Adapter::new. Result format is immutable
for the server, separate from retrieval mode; no per-call argument, client-name
heuristic or negotiation. Document this public Config addition in mcp.md.

Keep SDK/engine ownership of eligibility/ranking. Tools produce governed typed
values and call shared format-aware finishing/shaping. Separate text renderers
from structured serialization; edge only translates the finished value into
one TextContent block, optional structuredContent and isError. No second engine
read, body hydration or hint query is added by rendering. All existing error
paths pass through the same selected formatter; unknown tool/protocol/handler
panic paths remain protocol errors. Tool errors contain no hints.

## Exact compact grammar and field inventory

Use the accepted compact design's header/hit ordering. `<q>` is a JSON string,
`<j>` compact JSON, bare words/numbers/bools are server-generated. JSON-quote
refs, kinds, source IDs, terminal edge kinds, projection names, stores and all
error string values. Parse JSON strings with a JSON decoder before looking for
separators; never split quoted fields on whitespace or pipe. Repeated rows
retain order and duplicates. Store grammar stays ASCII/64 bytes; quoting tests
must not make previously invalid store names valid.

| Tool / row | Text representation and exact fields |
| --- | --- |
| search header | `search store=<q> epoch=<n> validity_instant=<n> mode=<word> hits=<n> completeness=<word> [kind=<q>] [continuation=<c>] [continuation_exact=false] response_id=<r> tokens=<used>/<max>[ cut]` |
| search hit | `<q ref> arm=<word> score=<x.xxx> [ce=<x.xxx>] [anchor=<word>] [cut] [kind=<q>] [src=<q>] [hl=<a>-<b>,…] \| <snippet>`; omit whole anchor, absent source/CE/highlights; represent requested empty highlights as `hl=[]`, distinct from absence. |
| search explanations | After its hit, `explain: <j>` (prefixed by two spaces) carries the entire PerHitOut, including ref/nullable ranks/CE/blended. `explanation: <j>` carries trace or unavailable status/reason once; no duplicated per_hit array. |
| expand header/target | Accepted expand header with budget and optional bound; target `<q ref> kind=<q> hop=<n> edge=<q>[ cut] \| <body>`; keep complete/bound/degradation. |
| fetch header/record | Accepted fetch header with items/budget; `record <q ref> kind=<q> bytes=<n>[ cut]` then LF, exactly n verbatim UTF-8 body bytes, then one framing LF. Empty body has bytes=0 and its framing LF. |
| fetch error | `error <q ref> <class>/<code> message=<q> remedy=<q> [candidates=<j>] [detail=<q>] [busy=<j>]`; one item for each input ref in order, including duplicate, missing, malformed and unsupported refs. |
| orient | Accepted orient header/projection/embedding/runtime rows. Preserve projection order, each dense readiness, affected_kinds, vector_unsupported_kinds, runtime available/reason and all real envelope fields. Use JSON arrays for string lists to distinguish empty lists unambiguously. |
| list_stores | `stores: <n>` then existing hint lines and `<q name> open=<bool> readiness=<word>[ default]`; false default is absence of flag. No new top-level default_store field. |
| use_store | `default store: <q store>`; preserve existing session action. |
| feedback | `feedback store=<q> epoch=<n> validity_instant=<n> status=<word> [reason=<word>] tokens=<used>/<max>[ cut]`; retain all envelope fields, including any degradation lines. |
| tool error | `error <class>/<code> message=<q> remedy=<q> [candidates=<j>] [detail=<q>] [busy=<j>]`; retain every current ToolError/Busy field, with absent versus empty list distinct. No budget envelope is invented for errors. |

Put one `degraded: <arm> <reason>` per entry, and existing fixed-template hint
lines between header and records. Preserve hint order, refs_omitted, cause,
mode, store, continuation and suggested budget exactly. Strings inside JSON
lists are JSON-quoted. Plain snippet/expand bodies are reading text: replace
each CR and LF with one ASCII space (CRLF becomes two spaces). This preserves
UTF-8 byte length and therefore existing snippet-relative highlights. Other
Unicode bytes, including Unicode separators, stay unchanged; “one line” means
one LF-delimited record, not a promise about every client font/layout. Fetch
bodies do not flatten anything. JSON metadata can contain escaped CR/LF.

Three-decimal score/CE text is the approved display rounding of the unchanged
typed f64. It must not drive ranking, identity, feedback or threshold decisions.
Structured scores and explanation JSON retain exact serde values; test text
against the documented rounding, underlying output against SDK exact values.
Do not compare rounded strings to raw floats or call rounded equality exact
score parity. No NaN/Infinity is invented as a fallback.

The interface field census must enumerate nested output.rs/error.rs serialized
paths, serde renames/flatten/untagged variants and skipped internal cut_anchors.
Keep state/verdict placeholders in structured during60; omit text state/verdict
and orient kinds/counts as approved. They are the only unavailable-placeholder
omissions; later optionality/disclosure belongs to80/90. “Kept” includes explicit
presentation mappings above, not silently discarded trace/error fields.

## Final accounting and bounded shaping

Accounting measures inner payload UTF-8 bytes, as the existing estimator does;
JSON-RPC envelope/content escaping is not a newly introduced transport-byte
budget. For text, T(u) is the final compact text with count u. Iterate from0:
`u = ceil(len_utf8(T(u))/4)` until both count and bytes stabilize, at most four
renders. Require final u≤max_tokens. Hints, counts, framing newlines, flags and
all quoted escapes count. Cap exhaustion is an internal error, never a silently
under-counted success; test digit boundaries 9/10,99/100,999/1000 and wider ones.

For structured, let S(u) be the final compact JSON with budget.used_tokens=u,
and P the same object without its top-level budget. Preserve the existing
used-definition while replacing the old human text: iterate
`u = ceil((len_utf8(P) + len_utf8(S(u)))/4)` to stability within four renders.
The transmitted text is exactly S(u), and the fit predicate is
`ceil(2 * len_utf8(S(u))/4) ≤ max_tokens`, including both final budget copies.
Displayed used excludes only the structured copy's budget, as before; it is
not the full-fit predicate. Interface examples and tests state this explicitly.
list_stores/use_store have no budget field or max_tokens; preserve that surface.
Orient/feedback use the configured default; search/fetch/expand use call override.

Each shape attempt rebuilds handles, hints, counts and cut flags from its kept
page before converging. Search drops trailing whole hits, never recuts snippets;
next_offset=offset+kept and no continuation when kept=0. Preserve current hints
fallback: if hints lose every hit or overflow an empty page, reshape without
hints. Complete gap-free traversal up to100 is guaranteed only if at least one
whole leading hit fits every nonterminal page's budget after this fallback.
An accepted budget≥256 is not by itself such a guarantee: the reviewed one-hit,
120-ASCII-snippet structured witness needs295 tokens before hints. That witness
is evidence of the boundary, not a universal minimum or a new numeric gate.

When candidates remain but no whole leading hit fits even without hints, and
mandatory zero-hit metadata fits, return a zero-progress success: results=[],
budget.cut=true, completeness=cut_by_budget, continuation=null/absent in text,
and hints absent. Recompute response_id for that empty kept prefix, retain all
other mandatory metadata, converge/count the final result and require final fit.
The page does not pretend the ranked input is empty; rerunning with a larger
budget is needed because there is no advancing continuation. Do not recut the
leading snippet or skip it to take a later smaller hit. If mandatory zero-hit
metadata still cannot fit, typed bad_argument with template remedy replaces an
oversize success. Genuinely empty ranked input instead has cut=false and the
existing empty/empty_not_ready value, with hints only if they fit.

Fetch shortens UTF-8-safe body
prefixes without losing outcomes; if empty bodies plus mandatory errors/hints
cannot fit, retain documented bad_argument (no fetch continuation). Expand
shortens longest bodies then drops trailing targets, setting bound=budget.
If mandatory empty metadata cannot fit for a budgeted tool, refuse rather than
silently dropping contract fields. No new error schema/budget is introduced.
Suggested body_cut fetch budget is computed for the target format's complete
uncut fetch representation, not guessed from an expand header or old two-copy
saving factor; executing it must return those refs uncut on unchanged state.

## Filters, views and continuations

Optional search fields include_superseded/include_inactive require a boolean
when present; null is invalid. Their strict deserializer preserves absent None
and explicit Some(false), with a nonnull input schema. Resolve
both to false, canonicalize effective values and pass to the same ReadView as
valid_as_of. Omit false fields in CanonicalSearch serialization to preserve
existing default-request handle hashes; explicit false and absence are identical.
True flags affect hashes. All existing predicates remain canonical with sorted
BTreeMap attributes. Empty attributes is the unfiltered operation; do not
reinterpret null/type rules beyond existing Option deserialization behavior.

Set TextSearchOptions.filter=search_filter(request), alongside unchanged view,
limit and SnippetOptions. Never call hybrid/property FTS to obtain filtered
lexical hits. SDK/engine validate snapshot-local declarations before lexical
no-hit return; engine SQL applies predicates before candidate caps/fusion/limit.
Node attributes are AND and node-scoped, excluding edges. source_type keeps
engine mapping (`doc`→`article`, edges→`edge_fact`); status/time use existing
projected rows only (empty status sentinel; created_at≥bound). Missing projected
metadata fails to match even if body JSON has matching status/time fields.

Continuations may carry only store, continuation, max_tokens, hints, highlights;
any supplied filter/view/mode/query/limit/explain, including false flags or empty
attributes, fails. Stored canonical filter/view/validity instant is reused;
highlights remains presentation-only. Epoch/reopen/cross-store/LRU rules stay.
Unchanged-state pages preserve ordered prefixes; complete gap-free traversal
up to100 requires the per-nonterminal-page whole-hit fit condition above.
Arbitrary accepted smaller budgets retain the separately specified zero-progress
outcome; they cannot establish all-results traversal. Hosted writes may shift ranks,
so retain continuation_exact=false and predicate enforcement without inventing
state-advanced/drift labels. as_of controls validity, not transaction history.

Registry admission is unchanged. Current source takes an immediately free
permit before inspecting cancellation; queued select is biased permit before
deadline before cancel. Running work retains permit until completion after
abandonment. Formatting tests assert visible busy/detail and continued service;
they must not invent a cancel-always-wins pre-permit guarantee.

## Canonical updates, boundaries and handoff

Same implementing PR updates mcp.md config/schema/result/error/view/filter,
complete field census and migration example `{"result_format":"structured"}`;
compact design's exact grammar/accounting and stale orient test name; active
adapter design/requirements MR-071 format-conditional schema wording and direct
text support. Preserve historical conformance report as dated evidence. Revise
static-list wording to names/input schemas constant, output schemas format-bound.
Update instructions within the existing1200-byte cap with concise names for
cut/anchor/continuation/tokens/quoted values. Correct orient's stale dense-disabled
claim to actual failed-open policy, not a fictitious dense_reason field.

Completed70 supplies embedder_absent/embedder_timeout and integrated HintCause/
MCP translation.60 preserves these actual typed values and retained open-epoch
embedder_unavailable; infer no broader EP03 removal.
80/90 own new unknown-argument/offending-ref details, not_in_this_release,
structured optionality, top-level default_store, span/coverage. Those later
fields must extend this census and both renderers when implemented. No new
snapshot authority, storage or calibrated verdict is smuggled into format work.

Apply quality-roadmap boundary/property/differential methods at these wire and
selection seams; defer broad fuzz/MC-DC/allocation campaigns. Both-format current
stdio checks plus owned official conformance feeds Block1 exit; historical
3-applicable-scenario report is not a current pass. Same-page rendering ratio
and native page-count/token/cost/fetch/accuracy remeasure are separate outputs;
fix the record-start prompt before any authorized rerun. No token-savings target,
new paid run, GPU/reset/toolkit/publication permission follows this design.
