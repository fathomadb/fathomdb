# Proposed decision: repeated search references and feedback

Documentation/source reconciliation at parent `3fa16620ac30cbe3ed66f5bfe542f2f7a62b8982`.
No ruling, implementation or runtime reproduction is claimed.

## Observed boundary and concrete examples

MCP tools.rs `search_page` emits `hit.id.to_prefixed()` as `ref`; `search`
collects returned `(ref,cursor)` pairs from `page[..kept]` into
`ResponseContext.refs`, a `BTreeMap<String,u64>` (session.rs). In `feedback`,
both named-label cursor lists use `entry.refs[reference]`. Two distinct cursors
sharing a ref silently retain the last page occurrence. Body fusion does not
ensure unique public refs.

- **Already possible in SDK:** write `note:x` revision1 body `river old`, then
  revision2 body `river new`; historical search with `include_superseded=true`
  keeps both bodies but gives both `l:note:x`. Current MCP search rejects that
  argument (`tests/search.rs:339–357`), and integrated60 still holds that
  forwarding under F11. Its future accepted addendum would make this case
  adapter-reachable even without50. This is a source-derived example, not an
  executed fixture. A current node and edge may also share a logical string
  across their separate tables; different bodies avoid body dedup today.
- **Additional exposure under proposed50:** byte-identical anonymous rows from
  two sources keep the same content-hash `h:` but would remain two record hits;
  same-body historical revisions keep the same `l:`. Body identity currently
  collapses them. Different governed IDs with the same body have different refs
  and are a distinct fusion issue, not a repeated-ref feedback case.
- **Fetch is separate:** `parse_ref` accepts only nonempty/NUL-free `l:` values;
  `h:`/`p:` remain unsupported (tools.rs `parse_ref`). `fetch` passes logical IDs
  and its own view to `read::get_many` (tools.rs `fetch` → `read::get_many`). Engine `read.rs:97–124`
  orders eligible revisions by cursor and retains the latest per logical ID.
  Fetch does not promise retrieval of the search-selected historical revision.
  A feedback fix changes none of this and must not claim exact-revision fetch.

## Cursor-space source proof and its limits

Engine filter.rs `body_fts_rank_sql_view` selects both search_index.write_cursor
and search_index.rowid. SearchHit carries the stored write_cursor; the FTS
locator is separate. The supported ownerless fixture in
slice71_search_statement_trace.rs inserts cursor999999 without an explicit
rowid. No inspected source invariant makes write_cursor injective over proposed
RecordKeys. Two ownerless NodesFts rowids can share a stored cursor; an ownerless
node-FTS record can also share a cursor with Canonical(Edge,cursor), because its
ownership join is to canonical_nodes, not canonical_edges. Distinct records may
therefore have the same ref and cursor, or different refs and the same cursor.
Cursor equality proves neither selected-record sameness nor safe attribution.

Engine telemetry.rs `capture_telemetry` does not exclude ownerless hits: it
substitutes `[erased]` for a stable ID only when neither canonical table has the
cursor, then still captures the complete result. `capture_telemetry_with_sink`
emits result_ids as u64 cursors and arm_of as a cursor-keyed map;
`record_feedback` consumes u64 cursor labels. A cursor alias can therefore
collapse arm attribution or make a label address multiple selected records.
The guard below prevents ambiguous MCP label writes; it does not repair or
upgrade the existing cursor-only event schema, `[erased]` semantics, SDK direct
feedback contract or historical telemetry. This is source-derived reachability,
not a runtime reproduction, prevalence measurement or production-defect claim.

## Options for HITL

1. **Conservatively refuse non-injective labels, keep refs (recommended).**
   A named visible ref is recordable only if it occurs exactly once in the
   visible returned page and its cursor occurs exactly once in the complete
   captured query result. Visible-ref or captured-cursor repetition is ambiguous
   regardless of whether cursors/refs/bodies/branches happen to agree. This requires both
   unambiguous selected-reference attribution and injective mapping into the
   existing telemetry cursor space. Do not infer same-record identity from a
   shared cursor. Preserve existing membership/not_recorded precedence; reject
   otherwise-recordable ambiguity with existing invalid_request/bad_argument
   before any label writes. This conservative policy also refuses repeated
   same-record occurrences within either ambiguity domain; preserving them would
   require a proven internal record-key handoff, which is not proposed. No new engine/public carrier,
   field, schema or reference ABI is needed by this adapter-only guard.
   This changes MR-024 acceptance and requires the actual human ruling/interface
   update before implementation; it is not selected by this proposal.
2. **Add MCP-only response occurrence labels (unselected).** Such labels can
   distinguish visible references, but cannot make an aliased telemetry cursor
   injective. Keep the complete-captured-result cursor guard unless a separately
   reviewed telemetry identity design is selected. Any feedback_ref grammar,
   rendering, schema and lifecycle change remains unselected; it is not a
   remedy for cursor collisions by itself.
3. **Attribute a ref to all matching records (unselected).** This would change
   hit-level judgment into entity/group feedback, and existing cursor labels
   still cannot distinguish aliases. It needs explicit consumer intent and a
   separately reviewed representation; no multimap or telemetry redesign is
   authorized by this proposal.

Recommendation1 is the smallest correction for observed consumers. Memex's
`src/memex/fathom_store.py:4300–4374` consumes SDK body/kind/score/CE/source and
logical `IdSpace.value`; `WorldModelSearchHit` retains logical identity. A
read-only source/doc search of Memex `src`, `dev/fathomdb` and `eval/fathomdb`
found no `response_id`/MCP feedback consumer requiring occurrence labels. That
bounded search is not a claim about every deployed integration. No live database
was opened. Runtime census scope is already approved; its exact consistent
snapshot and safe access method remain unresolved. The current callsite census
is pinned to Memex9cbe455888a50b1a69cc15cb9c0b35399416496d.

## Proposed validation precedence and atomicity for option1

Define R as **all hits in the SearchResult captured for this exact query_id**,
retained as `SearchRun.hits` before `search_page`, offset skipping or budget
shaping. R includes earlier continuation-window ranks and budget-hidden later
hits in that same engine call; it is not only `page[..kept]`. It excludes other
query events and pre-fusion/database candidates absent from that captured
result. `shape::page_window` already bounds that call's ranked window to100.
No extra query or public/engine identity handoff is required.

Define P as exactly the returned `page[..kept]` after offset/budget shaping.
Compute saturating counts (0/1/many) of public refs over P, and separately of
write_cursor over R. ResponseContext membership contains **only refs in P**.
For each member retain a private SafeCursor(cursor)/Ambiguous target: SafeCursor
only when its ref has exactly one occurrence in P and that occurrence's cursor
has exactly one occurrence in R. Otherwise retain Ambiguous even if only one
cursor alias is visible. Hidden-only ref strings do not become members; hidden
records contribute to the complete cursor proof, not visible-ref multiplicity.
A singleton ref in P may recur off page with a different cursor and still be
safe. Conversely, any off-page record sharing its cursor makes it unsafe even
when that other record has a different ref. Do not refuse unrelated aliases.
Never replace the complete cursor proof with visible-page counts or query a
later live state. This uses existing hit fields; it exposes no
RecordKey/cursor/hidden-ref list and requires no new engine/public carrier ABI.

Preserve current shape/store/response lookup and **visible-ref** membership
validation, then epoch, telemetry-disabled, text-mode and missing-query-ID guards
(tools.rs `feedback`). A string absent from the returned page retains existing
nonmember bad_argument precedence, even if its cursor aliases a visible member. If a hidden hit has the
same ref text as a visible hit, the string is already a member; its safety
still follows the P-ref/R-cursor counts. Same text alone outside P does not
make the visible reference ambiguous. Only after the not_recorded guards, validate every named used/unused member's private target;
any Ambiguous target returns generic bad_argument before acquiring/recording
labels, with **zero** record_feedback calls and no partial write. Do not reveal
hidden refs, cursors or multiplicities in the error. Safe named refs remain
usable when unrelated ambiguity is unmentioned; a named ref whose cursor aliases
an unmentioned/hidden record is itself unsafe. Keep acquisition-time epoch
recheck and existing duplicate/overlapping input-label policy unchanged.

Future genuine RED units after ruling: first a real telemetry-enabled response
with repeated visible ref, then a distinct unit with visible unique refs whose
cursors alias in R. For the latter include two ownerless rowids and ownerless
node FTS versus canonical edge, different-body/different-ref controls, same-ref/
same-cursor record twins after identity GREEN, and aliases before/after the
returned page or omitted by budget. Assert typed refusal and no label write;
current cursor-only mapping supplies the source-derived failure hypothesis,
not an observed RED here. Stage each failing runtime oracle before its own
narrow guard GREEN. Independently cover unknown/hidden named refs, mixed safe/
unsafe named labels (atomic refusal), safe label with unrelated unmentioned
ambiguity, stale epoch, telemetry-off, text mode, missing query ID, both formats
and continuation query_id isolation. Explicitly accept a singleton visible ref
that recurs off page only at a different cursor; reject the converse same-cursor/
different-ref alias. Conservative repeated visible-ref/captured-cursor refusal
is intentional; no same-cursor acceptance control survives. Existing
`tests/feedback.rs` pins unique attribution/guards but supplies no cursor-alias
or complete-window oracle. No test, telemetry or database execution ran here.

## What can proceed without this answer

Source/consumer census, identity ADR options, exact-rank proof, snippet design and
independent design review can proceed. Integrated60's formats/direct-filter
work and
logical fetch semantics stay intact. Production identity/repeated-ref behavior
waits for the actual ruling;50's hard30 baseline/source prerequisite also stays.
The question neither selects a new ABI nor defers work to .30.
