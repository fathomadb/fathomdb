# Selected decision: repeated search references and feedback

Source observations below use parent `2f6913c1d59ac8ff3a30de835b57290a8635998c`.
Owner 2026-11-14 selects option1 (seq-375): the smallest
atomic refusal of ambiguous named refs for otherwise-recordable calls. The .29
correction is implemented and focused CPU tests pass; broader qualification is
pending.

## Observed boundary and concrete examples

MCP `tools.rs:814` emits `hit.id.to_prefixed()` as `ref`; `tools.rs:1032–1035`
collects returned `(ref,cursor)` pairs into `ResponseContext.refs`, a
`BTreeMap<String,u64>` (`session.rs:32`). `feedback` indexes that single cursor
at `tools.rs:1436–1437`. Two distinct cursors sharing a ref silently retain the
last page occurrence. Body fusion does not ensure unique public refs.

- **Already possible in SDK:** write `note:x` revision1 body `river old`, then
  revision2 body `river new`; historical search with `include_superseded=true`
  keeps both bodies but gives both `l:note:x`. Current MCP search rejects that
  argument (`tests/search.rs:301`), so approved60 forwarding makes this case
  adapter-reachable even without50. This is a source-derived example, not an
  executed fixture. A current node and edge may also share a logical string
  across their separate tables; different bodies avoid body dedup today.
- **Additional exposure under proposed50:** byte-identical anonymous rows from
  two sources keep the same content-hash `h:` but would remain two record hits;
  same-body historical revisions keep the same `l:`. Body identity currently
  collapses them. Different governed IDs with the same body have different refs
  and are a distinct fusion issue, not a repeated-ref feedback case.
- **Fetch is separate:** `parse_ref` accepts only nonempty/NUL-free `l:` values;
  `h:`/`p:` remain unsupported (`tools.rs:354–366`). `fetch` passes logical IDs
  and its own view to `read::get_many` (`1085–1113`). Engine `read.rs:97–124`
  orders eligible revisions by cursor and retains the latest per logical ID.
  Fetch does not promise retrieval of the search-selected historical revision.
  A feedback fix changes none of this and must not claim exact-revision fetch.

## Selected option and alternatives

1. **Reject ambiguous labels, keep refs (selected).** Internally fold each returned ref to
   Unique(cursor)/Ambiguous; one distinct cursor is unique, while zero remains
   unknown under existing membership validation. Repeated occurrences of the
   same cursor are not ambiguous; an unbounded cursor set/count is unnecessary.
   Preserve existing not_recorded precedence and, only for otherwise recordable
   feedback, reject a named `used` or `unused` ref mapping to more than one
   distinct cursor with existing `invalid_request/bad_argument`. Do not pick the first/last cursor or label all
   occurrences. Unique named refs remain usable; unmentioned ambiguous refs
   generate no labels. This changes MR-024's acceptance contract and needs an
   implementation-time interface update under this ruling. It adds no public field or reference ABI.
2. **Add MCP-only response occurrence labels.** Keep logical/content `ref` and
   fetch unchanged; add a response-local `feedback_ref` to search hits, backed
   by cursor/occurrence data already available in the adapter. Rule how feedback
   names those labels, scope them to response/store/epoch, and update output
   schema, text rendering, feedback validation and paging tests. This can enable
   exact per-occurrence attribution without changing Engine or native SDK ABI.
   No new token grammar, input compatibility route or field is chosen here.
3. **Attribute a named ref to every matching cursor.** Small internal multimap
   change, but it records a judgment on hits the caller may not have used. It
   answers entity-level feedback, unlike the current hit-level label meaning;
   not recommended without explicit consumer intent.

Selected option1 is the smallest correction for observed consumers. Memex's
`src/memex/fathom_store.py:4300–4374` consumes SDK body/kind/score/CE/source and
logical `IdSpace.value`; `WorldModelSearchHit` retains logical identity. A
read-only source/doc search of Memex `src`, `dev/fathomdb` and `eval/fathomdb`
found no `response_id`/MCP feedback consumer requiring occurrence labels. That
bounded search is not a claim about every deployed integration. No live database
was opened; no snapshot access authorization was discovered.

## Selected validation precedence and atomicity for option1

Preserve current shape/store/response lookup and reference-membership validation,
then epoch, telemetry-disabled, text-mode and missing-query-ID handling in
`tools.rs::feedback`. Preserve acquisition-time not_ready→not_recorded handling
and the post-acquisition epoch recheck. Only after successful acquisition and
that final epoch check, validate ambiguity across the complete named used/unused
set, before resolving cursor labels or entering `bridge::call`/`record_feedback`.
Thus an unknown ref retains bad_argument precedence; expired/reopened,
telemetry-off and text-only cases retain existing not_recorded outcomes, including
close/reopen between ready_epoch and acquisition. An otherwise recordable
ambiguous label returns bad_argument with **zero** calls to `record_feedback`
and no partial telemetry label. A repeated same-cursor entry retains existing
behavior; overlap/duplicate input-label policy is not changed by this fix.

The runtime RED/GREEN oracle: real telemetry-enabled hybrid response containing two
distinct cursor hits with one ref; feedback names that ref and currently records
one cursor. Assert typed refusal and no label write, then GREEN the internal
multiplicity handling. Independently test unique-ref success, repeated same
cursor, mixed unique/ambiguous named refs (no partial record), unmentioned
ambiguity, stale epoch, telemetry-off, text mode, unknown ref and both formats.
With an ambiguous named ref, pause after ready_epoch and before acquisition:
concurrent unready/close must retain acquisition-time not_recorded; reopen under
a new epoch must retain post-acquisition not_recorded. Both write zero labels
and must not become an early ambiguity bad_argument.
Existing `tests/feedback.rs` was read in full: it asserts unique-ref cursor
attribution, concurrency/epoch/ref validation and no effect on search output;
it did not supply this repeated-ref oracle. The new real repeated-ref fixture
failed because feedback was recorded, then passed with the selected refusal;
focused precedence, atomicity, same-cursor and both-format controls also pass.

## Implementation and follow-up boundary

The .29 fix must precede exposing repeated-reference historical MCP results and
must be composed with50's additional equal-body exposure. Preserve logical fetch
and unchanged helper/pack contracts.50's actual30 baseline/source prerequisites
and independent design review remain; the decision alone grants no source entry.

For .30, review precise per-result feedback using richer occurrence labels, with
explicit response/store/epoch scoping and schema/text/paging consequences. That
review does not postpone or replace the selected atomic .29 refusal. No new
reference ABI, public result field or all-occurrences attribution is selected.
