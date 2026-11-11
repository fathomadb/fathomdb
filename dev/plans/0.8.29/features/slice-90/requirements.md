---
title: "Slice 90: requirements"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 requirements

These local requirements refine R29-3P-14/15, not locked canonical ACs.
Every proposed public shape is conditional on exact final80 approval. The
aggregate-v2 revision identified in [census](census.md) supersedes80's old R6/A6
missing-coverage direction; its fields are not an accepted ABI today.

| Requirement / need | Required behavior if approved | Acceptance |
| --- | --- | --- |
| S90-R1 / N1 | Omit unavailable-only envelope state and search verdict in both formats; structured schema permits absence. Orient lists exactly [state,verdict,kinds,counts] once in not_in_this_release. Preserve real readiness/degradation/completeness and structured kinds/counts placeholders;60 already owns their text omission. | A1 |
| S90-R2 / N2 | Before typed parsing/store work, unknown top-level keys produce invalid_request/unknown_argument, least original key in UTF-8 lexical order bounded to64 bytes including ellipsis, complete sorted schema-derived accepted names and fixed per-tool remedy. Known-key-only malformed input retains bad_argument; nested user map keys retain semantics. No successful-call corrective payload construction. | A2 |
| S90-R3 / N2 | Feedback membership error is invalid_request/refs_not_in_response with offending occurrences in used-then-unused request order, duplicates retained. Include first50 eligible refs of at most128 raw UTF-8 bytes, omit overlong refs whole; refs_omitted counts every excluded occurrence and is absent at0. Preserve existing store/lookup/membership/epoch/telemetry/text precedence, zero labels on failure and final50's separately ruled ambiguity policy. | A3 |
| S90-R4 / N3 | Required nullable list_stores.default_store shares one session sample with row flags/hint. Text first line includes default=<JSON string or null> with hints=false too. Guide explains store-less session-default routing within1200 bytes. No change to use_store action. | A4 |
| S90-R5 / N4 | Required ProjectionRuntimeStatus.vector_coverage is the stored_mapped_spans tagged union; MCP orient carries it once at top level. Aggregate population includes exactly attributable committed mapped heads under stored default profile, including retained history; it is neither live-view membership nor physical-vector certification. Passing ownerless/bodyless/dual-owner mappings contribute only unattributed_heads. | A5 |
| S90-R6 / N4 | Available R=K+U, T≤K, V≤B; known-only record share T/K and unseen-byte share V/B are exact unreduced fractions, null at denominator0. NULL end stays unknown even for empty body; no-vector/pending/failed records without heads enter no count. Counts are u64≤i64::MAX, sums/fraction components u128. | A6 |
| S90-R7 / N4 | Capture only during already-required full span validation on one writer snapshot; preserve original invalid predicate/orphan errors and separate completion refusals. No second store traversal, physical vector_default access/probe/schema check, persisted counter, migration/rebuild/reprojection or full capture on ordinary/explained reader searches. New row-stream arithmetic inside the existing full pass is explicit O(N) work. | A7 |
| S90-R8 / N4 | Publish seed only after successful read transaction commit; Engine-local same-writer token/profile validation permits O(1) coverage work during projection_status/orient. Cold/mismatch yields cache_cold/state_changed without counts. Invalid metadata/SQL/commit/cache poison is typed error, never unavailable. Rollback can conservatively invalidate via total_changes. No always-current guarantee or whole-orient snapshot claim. | A8 |
| S90-R9 / N5 | Existing opt-in SnippetV1 gains scored_bytes:Option<u32>: Some(e) is [0,e) of this returned record's actual contributing stored head vector, Some(0) known-empty; None asserts no known extent. Capture exact accepted50 key/actual vector participation, use same reader transaction, validate tuple/body/UTF-8/width and propagate errors. Never borrow by body/ref/arm/explanation or recompute tokenizer cut. | A9 |
| S90-R10 / N5 | SDK/facade/native/wrappers carry aggregate tags/identity/exact numeric limits and scored scalar consistently. Only aggregate counts/byte totals/fraction components use canonical decimal strings in MCP/NAPI/TS; Rust retains u64/u128 and PyO3/Python exact ints. profile.dimension remains numeric under its existing contract (for example8, never string "8"); scored scalar retains its approved numeric carrier. Standalone snippet_for has None. Snippets off adds no coverage work; text-only adds no scored-span reads/provider calls. Per-hit lookups bounded to final vector participants, not corpus size. | A10 |
| S90-R11 / N6 | Both renderers/schema/census include all new success bytes in60 accounting. Preserve whole-hit page cuts, hint fallback, zero-progress success and fixed-metadata refusal; diagnostic errors remain finitely bounded unbudgeted errors with no hints. Preserve queue/run/permit/shutdown, frozen refusals, eligibility/ranking and existing span publication. | A11 |
| S90-R12 / N6 | Source entry requires actual integrated selected20–70 reread/reconciliation, exact signed80 revision, Block1 follow-up exit receipt and independent Astra design PASS. Each behavior receives observed staged/committed runtime assertion RED before GREEN, fresh final Astra review and required owned-tool/full source gate before integration. | A12 |

Canonical CRUD belongs in the accepted implementation PR: MR-024, MR-031.1,
MR-050 and MR-071 omission/detail wording; retained MR-072 hints; maintained
[interfaces](../../../../interfaces/) and long-record/EP17 contracts. Do not mint
new ACs, change governed operations, edit historical format SQL or mutate goldens.
