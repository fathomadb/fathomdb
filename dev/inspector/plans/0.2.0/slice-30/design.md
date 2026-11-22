---
title: Inspector 0.2.0 Slice 30 — native lexical design
status: ACTIVE
date: 2026-11-19
---

# Slice 30 — native lexical design

## Baseline and existing versus new

Execution baseline `31fb465357300177fe493837ea63d0f39c4b2523`, unchanged Engine
`6edc1155f` / format37. [Scope](../scope.md), [plan](plan.md), canonical
[requirements](../../../requirements.md), [acceptance](../../../acceptance.md)
and [HLD](../../../design/high-level-design.md) govern. Slice 20 supplies exact
opaque handles, independently scoped reads, D01, child ownership, bounded
observations and Back/focus. New Inspector-owned lexical module and worker
operations supply indexed text, native queries and evidence UI. No Engine/SDK/
MCP/schema/API/pin/model change or copied compiler/ranker/tokenizer.

## Selected native routes

| Route | Actual controls and meaning |
| --- | --- |
| Body/edge | `search_text_only_filtered_with_snippets`: limit1–100, public `SearchFilter`, `ReadView`, highlights-enabled `SnippetOptions` (512 UTF8 bytes, one window,20 combined plus20 identity point queries,40 total maximum). Node/edge body dedup then RRF, descending higher better; representative branch identifies returned cursor class, not all contributors. |
| Named property | `search_projected_text_with_limit`: one declared searchable+FTS projection, same native filter/view/limit. Actual `SearchHit.score` is **negated SQLite BM25**, descending higher better; native order corresponds to raw BM25 ascending then cursor. No fusion/native snippets. Historical relaxed view may derive values in Engine scratch; persistent stored index inspection stays separate. |
| BM25F substrate | Public doc-hidden `bm25f_search`, default weights1/1/1,b0.75,k1=1.2. Kind/body/status scoring, descending then cursor; active/current nodes with native dependency eligibility, unfiltered/native current time; own-node validity windows are ignored (not strict ReadView). Inspector truncates native ranked output, never claims native candidate limit. Reject requested filter or relaxed/custom-time view before invocation. Hydration is a later read, not query-snapshot payload. |

All routes are model-free under None session. Body/property query uses native
plain-text compiler, not arbitrary FTS/SQL; preserve input exactly. BM25F native
tokenizer query interpretation is separately labeled. No rank tuning/private
MATCH reconstruction. The [SQLite reference](https://www.sqlite.org/fts5.html#the_bm25_function)
confirms negative raw BM25; Engine source, not stale method summary, establishes
property score negation. Effective tokenization comes from bounded actual
`sqlite_schema.sql`, separately from registry override.

## Protocol, bounded reads and identity

`indexed {handle, key?, offset, limit}` reads representations for an exact canonical
owner; attribute handles use their recorded owner. Body/edge, property and
fielded index rows are selected by exact cursor, never equal body/logical ID.
Each representation exposes index/field, stored UTF8 window, byte length/
continuation, configured versus actual tokenizer/DDL, declaration, origin handle
and independent context. Bound list200,response2MiB,overview windows4KiB, selected window64KiB, shared SQL
progress/deadline. Missing materialization means absence in this read, not
projection failure/search ineligibility. Only fixed table/column SQL; no mutation.

`lexical {route,query,property,filter,view,limit}` uses deny-unknown typed inputs,
query1–4096 bytes, metadata operands1–4096 bytes, at most8 attribute equalities,
limit1–100. Filter exposes native kind/source_type/status/created_after and
declared attribute equality only. Decimal integers travel as exact strings.
View flags are independent native relaxations; optional valid_as_of explicit.
Default query view is strict, while D01 stored inspection includes history/
inactive by default. These are different semantics, visible in controls.

Native calls allocate full bodies and relaxed scratch corpora. Before invocation,
an owned read bounds relevant stored canonical/index text to64MiB,100000 rows,
16MiB per value using lengths, not payload preload. Incomplete size accounting
refuses `budget_exhausted`; this is resource refusal, not a capability pass.
Relaxed property scratch admission additionally charges all searchable declarations times all canonical node rows/body bytes plus attribute-name bytes and baseline property rows, with checked arithmetic; aliases cannot amplify scratch outside the cap. Small corpus supplies mandatory positives; F06 exercises bounded refusal/cancel
on large fixture. All native work stays in existing child/30s deadline. Cancel
kills child, expires attachment, discards late data. No imaginary native interrupt;
owned SQL retains progress cap. BM25F never claims a native limit it lacks.

Preserve original rank gaps/value as round-trip decimal strings, typed native
ID/namespace/class/cursor, representative branch, bounded original body prefix
with original byte count/completeness, actual snippet spans/windows/highlights/
cut and query start/end. No omitted body is labeled complete. Body/property
payloads and Engine snippets have query scope; later policy/identity/index reads
have separate context. No authenticated frozen token/shared-snapshot claim.
Preserve native projection cursor without treating it as snapshot identity.

Resolve origins by class+physical cursor+recorded revision in bounded later
transaction. Zero/one/many remain unresolved/exact/choice. Only verified handles
open. Representative cursor proves its own origin; body dedup does not prove all
equal-text contributors. Derived canonical records stay separate. Unresolvable
content is withheld; never replace original result by current logical-ID fetch.
BM25F cursor/value-only body is explicitly later hydration. Property/BM25F match
spans remain unavailable, never query-guessed. Indexed text is an independent
observation; no copied compiled MATCH for later highlight.

## Policy and UI

D01 gates entire text/ID/snippet/evidence/link delivery using existing fresh
per-read classification/real queued scope. Withhold disallowed hits and retain
original rank gaps. Revalidate every result origin and anchor on Back/copy.
Generation changes invalidate before effects; late navigation cannot invalidate
or replace newer selection. Existing payload/structural scopes remain distinct.
Telemetry payloads never enter canonical lexical routes; all8 launch combinations
compose; never delay/reconstruct physical deletion.

Record detail offers Indexed text; global Text retrieval opens keyboard-operable
workspace with route/field/filter/view/limit, indexed text, ranked cards, inert
byte-safe native highlights, actual score family/direction, query context, later
origin context and exact origin/Back. Preserve controls/selection/index offsets/
initiating focus on return. Refresh creates new observation. Copy original result
text/snippet revalidates owners and preserves bytes/truncation. Full later content
uses existing detail/chunks. Error/empty/invalid/budget/policy-filtered/cancelled/
evicted differ. Comparison60 consumes same observations; no pin UI/persistence.

## Verification

Per-behavior real RED commit then GREEN: indexed owner/content metadata; native
body/property and authored numeric/UTF8 facts; BM25F formula/rejected controls/
bounds; policy/reference/resource; actual browser record→index→query→origin→Back/
keyboard/copy/late state. Real Engine writes, independent identities/tiny corpus
facts; direct Engine comparison adds conformance only. Property checks cover
wire values/bounds/references. Worker cancel/queue/deadline/close and F06 reuse
existing harness. Direct Sol-high design review before code; fresh Sol-high
code review and Terra afterward. Focused checks during work, one clean full gate,
truthful failures/skips and concise status. Installed/all-view/human remains70;
unfinished lexical basics stay30.

Design review: direct Sol-high identified historical-property scratch alias amplification and BM25F own-validity semantics. Both are incorporated above before implementation; no private compiler/extractor is copied.
