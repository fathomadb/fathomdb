---
title: "Slice 90: requirements"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 requirements

These local requirements refine R29-3P-14/15, not locked canonical ACs.
The [full80 design](../slice-80/design.md) is owner-approved at
`f2f4831554ae165420a31890cf102642ce28b21d` (seq384). These are its
implementation requirements, reconciled against the actual integrated50/60/70
source in [census](census.md); the new fields are not implemented yet.

| Requirement / need | Required behavior | Acceptance |
| --- | --- | --- |
| S90-R1 / N1 | Omit unavailable-only envelope state and search verdict in both formats; structured schema permits absence. Orient lists exactly [state,verdict,kinds,counts] once in not_in_this_release. Preserve real readiness/degradation/completeness and structured kinds/counts placeholders;60 already owns their text omission. | A1 |
| S90-R2 / N2 | Before typed parsing/store work, unknown top-level keys produce invalid_request/unknown_argument, least original key in UTF-8 lexical order bounded to64 bytes including ellipsis, complete sorted schema-derived accepted names and fixed per-tool remedy. Known-key-only malformed input retains bad_argument; nested user map keys retain semantics. No successful-call corrective payload construction. | A2 |
| S90-R3 / N2 | Feedback membership error is invalid_request/refs_not_in_response with offending occurrences in used-then-unused request order, duplicates retained. Include first50 eligible refs of at most128 raw UTF-8 bytes, omit overlong refs whole; refs_omitted counts every excluded occurrence and is absent at0. Preserve existing store/lookup/membership/epoch/telemetry/text precedence, zero labels on failure and accepted50's implemented atomic ambiguity refusal. | A3 |
| S90-R4 / N3 | Required nullable list_stores.default_store shares one session sample with row flags/hint. Text first line includes default=<JSON string or null> with hints=false too. Guide explains store-less session-default routing within1200 bytes. No change to use_store action. | A4 |
| S90-R5 / N4 | Required ProjectionRuntimeStatus.vector_coverage is the stored_mapped_spans tagged union; MCP orient carries it once at top level. Aggregate population includes exactly attributable committed mapped heads under stored default profile, including retained history; it is neither live-view membership nor physical-vector certification. Passing ownerless/bodyless/dual-owner mappings contribute only unattributed_heads. | A5 |
| S90-R6 / N4 | Available R=K+U, T≤K, V≤B; known-only record share T/K and unseen-byte share V/B are exact unreduced fractions, null at denominator0. NULL end stays unknown even for empty body; no-vector/pending/failed records without heads enter no count. Counts are u64≤i64::MAX, sums/fraction components u128. | A6 |
| S90-R7 / N4 | Capture only during already-required full span validation on one writer snapshot; preserve original invalid predicate/orphan errors and separate completion refusals. No second store traversal, physical vector_default access/probe/schema check, persisted counter, migration/rebuild/reprojection or full capture on ordinary/explained reader searches. New row-stream arithmetic inside the existing full pass is explicit O(N) work. | A7 |
| S90-R8 / N4 | Publish seed only after successful read transaction commit; Engine-local same-writer token/profile validation permits O(1) coverage work during projection_status/orient. Cold/mismatch yields cache_cold/state_changed without counts. Invalid metadata/SQL/commit/cache poison is typed error, never unavailable. Rollback can conservatively invalidate via total_changes. No always-current guarantee or whole-orient snapshot claim. | A8 |
| S90-R9 / N5 | Existing opt-in SnippetV1 gains scored_bytes:Option<u32>: Some(e) is [0,e) of this returned record's actual contributing stored head vector, Some(0) known-empty; None asserts no known extent. Retain original vector keys before first fusion using existing50 RecordKey; use the same reader transaction. Refuse missing/misplaced/extra head, ambiguous owner, mismatched body, missing physical vector, malformed/non-UTF-8 end and u32 overflow; only a fully validated NULL end is provider-unknown. Never borrow by body/ref/arm/explanation or recompute tokenizer cut. | A9 |
| S90-R10 / N5 | SDK/facade/native/wrappers carry aggregate tags/identity/exact numeric limits and scored scalar consistently. Only aggregate counts/byte totals/fraction components use canonical decimal strings in MCP/NAPI/TS; Rust retains u64/u128 and PyO3/Python exact ints. profile.dimension remains numeric under its existing contract (for example8, never string "8"); scored scalar retains its approved numeric carrier. Standalone snippet_for has None. Snippets off adds no coverage work; text-only adds no scored-span reads/provider calls. Per-hit lookups bounded to final vector participants, not corpus size. | A10 |
| S90-R11 / N6 | Both renderers/schema/census include all new success bytes in60 accounting. Preserve whole-hit page cuts, hint fallback, zero-progress success and fixed-metadata refusal; diagnostic errors remain finitely bounded unbudgeted errors with no hints. Preserve queue/run/permit/shutdown, frozen refusals, eligibility/ranking and existing span publication. | A11 |
| S90-R12 / N6 | Owner seq392 permits source entry with retained full80 approval, accepted independent design and fresh actual qualified20/50/60/70 source-contract fit while measurements continue. Required Block1 measurements/dispositions and qualification/conformance remain integrated qualification exits; the exact seq38872episodes run once on final integrated90 before local close. Each behavior receives observed staged/committed runtime assertion RED before GREEN, fresh final Astra review, owned-tool qualification and the parent-owned final integrated full source gate before local close. | A12 |

Canonical CRUD belongs in the accepted implementation PR: MR-024, MR-031.1,
MR-050 and MR-071 omission/detail wording; retained MR-072 hints; maintained
[interfaces](../../../../interfaces/) and long-record/EP17 contracts. Do not mint
new ACs, change governed operations, edit historical format SQL or mutate goldens.

## Release requirement and locked AC joins

Rechecked the release needs map in [master](../../../plan-0.8.29.md),
[requirements](../../../../requirements.md) and locked
[acceptance](../../../../acceptance.md). No canonical AC or threshold changes.

| Release need / requirement | Existing obligation | Residual assertion / retained control |
| --- | --- | --- |
| N29-A / R29-REL-A | REQ-029→AC-031; MCP MR-024/031.1/050/071/072; both-format budget/admission contracts | R1–4/R11→A1–4/A11; real readiness/degradation, text default, flags/filter/continuation and queue stay controls. |
| N29-K / R29-REL-K | REQ-053→AC-074 parity/typed surface; REQ-056→AC-060a; REQ-064→AC-068a/b; REQ-065→AC-069 | R5–10→A5–10 across Rust, installed native Python/TS and MCP. No governed operation added; existing string admission/sanitization stays. |
| N29-D / R29-REL-D | Owned source/native and applicable AC-072/075/076/081 qualification; tracked AC-083 | R12→A12: seq392 parallel source entry; retained integrated Block1 exits and final affected qualification. AC-083 stays tracked, not a new hard gate. |

The format37/no-migration contract supersedes historical public migration
wording. Current identity follows the accepted retrieval-record ADR; private
Memex runtime snapshots are not prerequisites. No new policy question or
reapproval of50/70/80 is created by this reconciliation.
