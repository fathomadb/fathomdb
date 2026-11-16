---
title: Slice 80 residual requirements
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 requirements

Local S80 identifiers do not mint canonical ACs or amend locked
`dev/requirements.md` / `dev/acceptance.md`. The release needs map is in
[needs](needs.md); observable acceptance and runtime RED plans are separate.

This revision uses integrated source
`e1cb1bd5311f68da8be98bbe133b9cecd5ab56d4`, selected identity/packing
seq-374 and feedback seq-375, and Slice50's independently reviewed
`10a72e5a9` carrier design. Selected design is distinguished from implemented API.
The earlier aggregate-v2 and consolidation consistency reviews passed; they are
not repeated. The updated contract needs its required Astra-medium review and
full80 owner acceptance. No implementation or90 source entry follows.

| Requirement | Contract | Acceptance |
| --- | --- | --- |
| S80-R1 / N1 | Omit only unavailable-only envelope state and search verdict in both formats; structured schemas permit absence. Orient.not_in_this_release is exactly [state,verdict,kinds,counts]. Coverage is a supported tagged availability result, not a fifth missing capability. Retain real readiness, degradation, completeness and current structured kinds/counts placeholders. | A1 |
| S80-R2 / N1 | Preserve built empty/degraded/cut/default-store hints and queue behavior. Preserve integrated70 embedder_absent, embedder_timeout and embedder_overloaded reason words and hint mapping. No weak_results, index_not_ready inference, retry delay or new hint action. | A2 |
| S80-R3 / N2 | Unknown top-level keys produce invalid_request/unknown_argument, the lexicographically first original key bounded to64 UTF-8 bytes including ellipsis, the complete sorted schema-derived accepted list, and a fixed per-tool remedy. This precheck precedes typed deserialization; nested map keys retain existing semantics. No error-detail strings/lists are constructed on successful calls. | A3 |
| S80-R4 / N2 | Feedback membership failure produces invalid_request/refs_not_in_response with used-then-unused offending occurrences in request order, preserving duplicates; include first50 eligible strings of at most128 raw UTF-8 bytes, omit whole overlong strings, and count every omitted offending occurrence. Existing shape/store/response/membership/not_recorded precedence remains; no labels on a membership error. Selected50 seq-375 separately refuses otherwise-recordable named ambiguous refs atomically with bad_argument after acquisition/final epoch check, before writes; not_recorded precedence stays. | A4 |
| S80-R5 / N3 | list_stores returns required default_store:string\|null from one session sample shared with row flags and hint. Text first line has default=<JSON string or null>, even when hints are disabled; guide names store-less use of the session default. use_store behavior stays. | A5 |
| S80-R6 / N4 | Implement design's required StoredHeadCoverageV1 on the existing projection-status path and orient.vector_coverage, population stored_mapped_spans. Report seven exact u64-count/u128-byte counters and nullable unreduced rational shares only from a committed matching writer-connection seed; otherwise cache_cold/state_changed has no numeric fields. Attribute exactly one non-NULL canonical body per passing mapping; ownerless/bodyless/dual-owner passing mappings increment unattributed_heads outside R/K/U/T/B/V. Preserve the exact existing COALESCE invalid predicate and existing caller-specific refusals. Fold arithmetic into already-required full validation; add no aggregate-driven vector_default access, orient scan, persisted counter, migration or reprojection. Use the full same-connection token and conservative rollback invalidation in design. | A6 |
| S80-R7 / N4 | Candidate per-hit field is SnippetV1.scored_bytes:Option<u32>, only in the existing opt-in snippet sidecar. Some(e) means this returned record contributed a stored head vector with validated body-relative extent[0,e); Some(0) is known-empty. None asserts no known scored extent (including provider-unknown or no vector contribution); it never means zero/full. No SearchHit/SearchResult field addition or new switch. | A7 |
| S80-R8 / N4/N5 | Bind span lookup to the exact Canonical(Node/Edge,cursor) key and original vector-arm participation retained before first fusion, on the same reader transaction. Ownerless(NodesFts,rowid) never queries spans; nested fusion tie preference is not original vector participation. Do not derive it from arm/ref/body/CE/explanation, query-token count, snippet or a later logical fetch. Frozen calls reproduce that snapshot or retain their existing typed refusal. Lookup/validation/width failures propagate errors, never unknown success. | A7/A8 |
| S80-R9 / N5 | Rust engine/facade/SDK, both native layers/wrappers and both MCP formats agree on names, absent/null/zero rules, widths and UTF-8 coordinates. Aggregate count/byte/fraction decimal-string transport and exact Rust/Python integer carriers follow design; no JavaScript Number conversion of those components. Profile.dimension remains u32 in Rust and numeric in JSON/TS (for example8), with an exact Python int; it is not decimal-string encoded. Standalone snippet_for reports None without storage work. Snippets off and provider-free text retain their existing work bounds. | A8/A9 |
| S80-R10 / N5 | New success metadata participates in60's final accounting/shaping. Tool errors remain finitely bounded unbudgeted errors, with no hints and no invented budget envelope. No dropping accepted keys or corrective detail to feign a successful fit. | A9 |
| S80-R11 / N6 | Full80 approval requires this reconciled selected50/integrated60/70 contract, its independent updated-design review and explicit owner acceptance. Existing release requirements/AC mappings below remain applicable; implementation must verify the actual50 carrier before source changes. Bounded v2 PASS alone supplies none of those source-entry conditions.Owner seq392 permits90 source after fresh accepted integrated20/50/60/70 source-contract fit while measurements continue; actual Block1 measurements/dispositions remain final integrated qualification exits, with the same approved72episodes once on final90.160 is later final qualification. | A10 |

Canonical amendments, when implementing the accepted subset, belong in the same
PR: MCP MR-031.1/MR-050 omission wording, MR-024 membership detail and selected50
ambiguity rule, MR-071 format-conditional schemas, MR-072 preserved hints;
`dev/interfaces/{rust,python,typescript,mcp}.md`, snippet design/EP17 and
long-record guard §7.1. No operation or governed-operation manifest is added.
Existing format/wire authority remains unchanged. Stage2 hints is not allocated
to90 by this package;150 must settle placement and default separately.

## Release requirement and AC mapping recheck

Checked against current `dev/requirements.md`, `dev/acceptance.md` and the
release master needs map on the source witness above. These inherited gates
remain controls; O80-01–12 add the missing semantic assertions without minting
canonical ACs or declaring any gate passed.

| Release requirement | Existing canonical obligation | New local assertions |
| --- | --- | --- |
| R29-REL-A / N29-A: usable bounded truthful agent output | REQ-029→AC-031 fallback; MCP MR-024/031.1/050/071/072; existing format/budget/admission contracts | R1–5/R10, A1–5/A9; O80-01–04/09 preserve integrated reasons and budget behavior. |
| R29-REL-K / N29-K: coherent SDK identity/errors/UTF-8 | REQ-053→AC-074 governed parity; REQ-056→AC-060a typed errors; REQ-064→AC-068a/b FFI strings; REQ-065→AC-069 sanitized diagnostics | R6–9, A6–9; O80-05–12 assert exact selected-record extents, integer transport and cache truth at each boundary. No new operation is added. |
| R29-REL-D / N29-D: selected release with actual prerequisite closure | Owned source/native verification and existing release discipline; tracked AC-083 plus applicable AC-072/075/076/081 performance/reader scopes | R11/A10 retains explicit full80 acceptance and seq392 integrated Block1 qualification exits; static input qualification and design PASS do not replace runtime results. |
