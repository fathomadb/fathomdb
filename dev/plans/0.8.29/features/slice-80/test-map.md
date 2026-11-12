---
title: Slice 80 residual behavior test map
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 test map

This current documentary phase executes no tests. Later90 starts only after
approved80 AND actual Block1 follow-up exit. Fresh final implementation review is mandatory. Every changed
behavior gets its own committed real assertion RED with exact failure evidence
in its commit body, then a separate minimal GREEN with unchanged oracle; no compile-failure/helper-absence RED, regenerated golden
or new passing-coverage exception. Existing regressions remain regressions.

Current documentary source boundary is
`5e4b5b61765aa9ce2557b2349218f44e74803f05`. Historical bounded Phase A,
aggregate-v2 PASS (proposal SHA-256
`a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`)
and consolidation consistency PASS remain recorded in [review log](review-log.md).
Bounded v1 design PASS covers owner patch SHA-256
`3620e6c185568752a6c422e1d37a7c9a577d7c7fe40fef7a8dda8dc45bab2a7e`.
A different fresh reviewer issued FINAL DOC/OUTPUT PASS with no P1/P2 on the
14-file parent diff SHA-256
`709240de171103f26951c123265bbfcdddfa5938df9fea6f2e617e052dda4389`.
Parent scoped Markdown/state/diff checks passed and independently verified78
source pins; [review log](review-log.md) attributes subjects and limits. This v2
updates records/citation only and claims no separate v2 PASS, new ABI/policy,
full80 approval,90 source entry or runtime/resumption GO.
[Current census](census.md) separates integrated code, qualified subsets and holds.

| Unit / requirement | Callable runtime RED on current API | Human oracle and work/error controls |
| --- | --- | --- |
| O80-01 / R1 | MCP Harness.call search/fetch/expand/orient/feedback with real fixture; inspect returned JSON with get and presence predicates. Assert structured state/verdict absent; current structured placeholders fail. Built text omission is a regression control. | Structured discovery is changed to permit absence; text mode omits structuredContent/outputSchema. Orient fixed missing list; current kinds/counts JSON preserved. Retain readiness/degradation values. |
| O80-02 / R3 | Every tool via existing tools/call supplies two unknown keys plus a known bad type, then reverses insertion order. Assert unknown_argument, exact bounded argument and full accepted list from tools/list. Baseline bad_argument fails. | Lexical original-key selection;64-byte exact and64+UTF8/CR/LF/quotes/backslash/NUL keys. Known-key-only failures remain bad_argument. Nested attributes unchanged. Evaluate successful-path allocation delta as a control paired with the failing detail assertion, without assuming zero whole-parser allocations. |
| O80-03 / R4 | Real telemetry-enabled hybrid response from existing feedback harness; call with ordered repeated foreign refs and inspect error JSON. Baseline has no typed code/details. | Exact used→unused order/duplicates,50/51 refs,128/129 raw bytes, all-overlong list=[] with omitted count, hostile escaped bytes. Sink label count unchanged. Store/lookup/membership/not_recorded/epoch-recheck controls retain current precedence. Ambiguous named refs wait50's actual ruling: P-ref singleton and full-R-cursor singleton; preserve the safe off-page different-cursor case. |
| O80-04 / R5 | Existing list_stores/use_store/store-less search calls, hints false; assert top-level default value and exact quoted header. Baseline lacks them. | No-default/configured/switch/session isolation, row flags/hint agree. Value independently of hints; no paths. Both formats and bounded guide wording. |
| O80-05 / R7–9 | In fathomdb-py Rust tests, existing engine test-hook API opens a real database with a deterministic Embedder::truncation_offset, writes/drains/searches with snippets, then existing PySnippetV1::from_rust→Py::new→getattr reads scored_bytes dynamically. Compare value11, not mere attribute existence. Baseline absence yields no value and fails at runtime, with no new helper API/type reference needed to compile. | Known partial/full/0/unknown, vector node/edge, text-only/graph-only/nonparticipant, ordinary/frozen, opt-out. Existing py crate already depends on embedder-api, tempfile and test-hooks engine. Existing tuple/type/UTF8 validation failures must error, not become None; reuse vector-stage proof without a new physical probe. Use existing SQL/worker barriers as in slice60_store_format/snippet_search; no mocked database or paid/model run. |
| O80-06 / R9 | Owned installed Python/TS and MCP default-embedder builds use already approved available local model/artifact fixtures; write the15-byte tagged body below, drain and search hybrid with snippets. Dynamic public value/JSON must equal15 under BGE's full-short-input contract. Baseline missing field fails. Do not run this until the required authorized native/model fixture actually exists. | Separate GREENs for wrapper/TS/MCP mapping, each preceded by that boundary's runtime RED. Full/0/absent/native integer width must not be conflated. Preserve ordered per-hit tuples. No provider-injection SDK, extra production dependency or fabricated MCP synthetic-hit coverage seam. |
| O80-07 / R8/R9 | Pair O80-05's real scored-value expectation with trace/work observations using existing take_slice71_search_statement_trace_for_test and barriers. If necessary, add an observational SQL-event tag to the existing trace before behavior GREEN, retaining a semantic value failure as RED. | Zero coverage work with snippets off/text-only; at most one indexed coverage read per final vector participant; existing lexical count remains≤2*max_point_queries, including38-query fixture. Inspect plans/VM work at two corpus sizes after authorized source/resource entry; timing alone cannot prove bounded work. No whole-vector_spans validator per hit. |
| O80-08 / R8 | Stage same O80-05 scored-value oracle with selected50 same-body/version/ref fixtures and frozen after-validation reader barrier. Baseline lacks value; test remains runtime RED. | Distinct canonical records with partial ends4/11 on identical tagged bodies, source identity and positional pairing. Add ownerless NodesFts twins with same cursor and a canonical edge alias: no span borrowing; real FTS locators stay distinct. Concurrent span update after reader pin returns old span or existing frozen refusal, never mixed-body/end. Rebuild/erase/reopen/rollback retain tuple invariants. |
| O80-09 / R10 | Against integrated60, MCP real hybrid search's final text/structured payload includes expected scored field in bytes and fit calculation. Assert exact scalar and final budget formula together; absence fails runtime. | Small whole-hit fit/no-fit, zero-progress success versus bad_argument if mandatory metadata cannot fit; hint fallback/drop/continuation offset, repeated refs, highlights on/off. No recutting e or adopting unbounded error budget as a success budget. |
| O80-10 / R6/R9 | Existing real projection-status/orient calls, dynamic native/JSON read: assert exact mapped-span carrier with R4/K3/U1/T1/B16/V6/unattributed0 and fractions1/3,6/16. Baseline lacks those values and fails at runtime. | Known-empty, unknown-empty, empty mapped/no-vector/pending cases; u64/i64-count/u128 widths, decimal syntax, null shares and profile scope across Python/TS/both MCP formats; profile.dimension=8 remains numeric, outside decimal-string transport. |
| O80-11 / R6 | Stage with O80-10: stable reopened/full-status-refreshed numeric result, then owned/external commit yields state_changed without numeric keys; refresh via the already-required existing status API yields exact changed counts. | Actual trace/VM work at two corpus sizes: no aggregate vector_default access and no additional full scan on cold/warm orient. Same writer token, postcommit publication, lock order, rollback/erasure/rebuild/reopen/profile/time controls. |
| O80-12 / R6 | Stage with O80-10: passing ownerless/bodyless/dual-owner mapped-span fixtures increment only unattributed_heads; compare exact numeric result dynamically. Baseline lacks carrier and fails at runtime. | First apply the unchanged invalid predicate including COALESCE; old failures remain typed. No reporting/open physical-vector or dual-owner refusal is introduced. Existing completion's separate eligible physical and cross-owner errors retain their own scope and block publication on failure. S80-F1's proposed same-table multiplicity case returns unattributed1 once per mapping, not records2; a second owner with an existing invalid length/boundary still errors before exclusion. Stage that semantic assertion in its own genuine RED, then minimal grouped-stream GREEN, after bounded design disposition. One mapping must never count twice. |

O80-05 is one vertical scored-extent behavior, not a new public provider route.
Rust tests must use the existing public engine calls and private native
conversion, never make a fake database result. First fixture setup must assert
the intended record actually participated in vector retrieval independently of
display branch, using selected50 internal capture/available explanation or the
vector-stage test seam. Explanation is only a test witness; production metadata
must also work with explain=false. Existing FtsRows is cursor keyed; it cannot
supply the exact-record handoff under aliases. No public branch/cursor inference
or physical vec0 probe substitutes for the settled internal carrier. Do not satisfy O80-05 solely by adding the
Python getter while returning guessed body length.

## Exact byte fixtures and properties

Use encode_round_body on parts user="é", assistant="🦀 zed". Hand-written
stored bytes are `\x1e\x11é\n\x1e\x12🦀 zed` (15 UTF-8 bytes): marker0,
user-code1, é[2,4), LF[4,5), marker5, assistant-code6, 🦀[7,11), space11,
zed[12,15). Known head end11 gives scored_bytes11, independent of a lexical
zed match[12,15). With Whole/highlights=true the shown window is[0,15), match
spans/highlights[12,15); with longer surrounding text use exact stored-byte
offsets and validate displayed highlights against snippet.text. End8 is inside
the crab and must fail validation; never round it to11 or report unknown.

Distinct records with identical stored tagged bytes and ends4 versus11 must
retain their own extents under selected50. Equal decoded text with different
role tags is a separate fixture: preserve byte differences, decoded role and
coordinates. Existing prefixed context_item_from_hit IDs and pack_context's
duplicate-ID refusal remain; no renamed identity or dedup to make packing pass.
No change to codec roles, framing, character budgets or malformed-body policy.

Required property cases for the new projection/readout seam: valid UTF-8 bodies
and chosen char-boundary ends; returned Some(e) equals the stored committed end
for the selected participating record,0≤e≤body byte length; invalid boundary/
length/mapping cannot return success. Include tags/NFC/NFD/emoji/empty input.
Stage the properties with O80-05's behavior RED before its GREEN. Existing
standalone golden expectations stay byte-for-byte; new-field checks supplement
their original field projection rather than regenerate snapshots.

## Aggregate runtime oracles and bounded work

R6/A6 now contain the concrete reviewed aggregate v2 contract; O80-10–12 are
required per-behavior runtime RED scenarios before later90 GREEN, not new
passing-only coverage. Use existing projection_status/orient APIs and dynamic
native/JSON extraction so tests compile against the old source and fail on the
wrong semantic values. Add no synthetic database or compile-missing helper.

The numeric denominator is attributable canonical stored mapped spans, not live
E/P/K/U/M. Use design's R4/K3/U1/T1/B16/V6 hand fixture; passing ownerless,
bodyless and dual-owner mappings increment only unattributed_heads. Known0,
unknown0, empty/no-vector/pending/failed cases obey the explicit domains;
invalid-predicate or orphan-span failures cannot become exclusions/unknown.
Read the stored default profile, independent of current runtime presence.

Count actual full validations/executed statements, not only the existing
completion-cache counter. No aggregate-driven vector_default rowid/EXISTS
probe is allowed: pinned vec0Filter_point loads vector payloads despite an
indexed plan. Existing completion's physical checks are not new reporting work.
Keep the original COALESCE validation and original caller-specific refusals.

Warm/cold orient adds only fixed token/summary work. Writer-only seed capture
and reuse share DB/profile/schema/clean visibility generation+nonce and the same
connection's data_version/total_changes. Publish only after commit. Ordinary/
external/projection commits, schema/profile changes, rebuild/recenter, erasure
and reopen prevent unequal-token reuse. After attempted DML rollback,
total_changes may advance and the cache conservatively reports state_changed;
no optimistic rollback-hit assertion remains. Cold/changed cache emits no
numeric fields, while stable reopen or existing full validation refreshes can
produce the real numeric carrier. Time-only eligibility changes leave this
metadata population unchanged; the whole orient envelope is not one snapshot.

Test exact canonical decimal/u64-count/u128-byte and rational domains at native
boundaries without JavaScript Number conversion for counts/bytes/fraction
components. Profile.dimension remains a numeric u32/JSON/TS number and exact
Python int, never a decimal string. Errors in SQL, metadata,
existing validation, lock acquisition or commit stay typed, never cache misses.
Measure added streaming arithmetic separately; no cost/latency pass is claimed
by this source-only packet. Per-hit50 membership and coverage lookups remain
a separate feature and cannot establish aggregate or integrity qualification.

## Existing controls and later verification

Read in full and preserve: mcp tests/hints.rs, hints_properties.rs,
hints_allocation.rs, hints_overhead.rs, feedback.rs, stores.rs, schema.rs and
admission.rs; engine snippet_search.rs, snippet_cost_receipt.rs,
search_result_shape.rs, slice60_store_format.rs; native snippet modules and
Python/TS EP17 tests. Existing allocation gate is hints-off==baseline and
planning≤1+refs, rendering notify-only; existing limit100 overhead block is
200µs/iteration, limit10 notifications10µs p50/25µs p99. Do not claim a timing
pass now or change those gates to absorb reporting work.

After authorized implementation: use owned10/actual20 setup, scoped latency-
ordered checks and one final strict full agent verify for source/native changes.
Preserve preflight/ptrace/platform failures as failures; missing model/native
fixtures are unrun cells. Reconcile final integrated50/60/70, both-format
conformance and original human oracles before independent final review.
160 remains final assembled-release qualification; it cannot serve as the
earlier Block1 exit required before90.
