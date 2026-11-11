---
title: Slice 80 local acceptance
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 acceptance

These are design acceptance targets and future90 behavior oracles, not passing
test receipts or canonical AC additions. Tests are planned in [test-map](test-map.md).

This documentary consolidation incorporates the independently bounded
Astra-medium PASS for aggregate v2, exact source proposal SHA-256
`a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`.
The consolidated packet's narrow consistency recheck is pending; this is no
new ABI/policy approval or full80/source-entry approval. All original packets
remain frozen. Parent source stays clean2f6913 with no imports.

| Local acceptance | Falsifiable observation / design completion |
| --- | --- |
| S80-A1 | All store-bound success shapes omit envelope state; search omits verdict. Structured discovery permits their absence; orient.not_in_this_release is exactly [state,verdict,kinds,counts], without a coverage entry. Both text and structured retain every real readiness/degradation value. Structured kinds/counts retain their existing shape. |
| S80-A2 | Existing empty/degraded/cut/default/queue controls remain mapped to actual tests. New70 labels use its accepted integrated vocabulary, without reconstructing a new failure taxonomy. No reserved weak_results/index_not_ready/retry_after_ms appears. |
| S80-A3 | Every tool returns unknown_argument for an unknown key, even alongside a wrong known-key type; accepted equals sorted tools/list inputSchema.properties. Reversing JSON key order changes nothing. Exact64-byte and multibyte-over64 cases obey the bound. Known-key-only malformed input keeps bad_argument. Valid calls allocate no corrective-detail payload. |
| S80-A4 | A real known response plus used=[foreign-b,foreign-a,foreign-b], unused=[foreign-c] returns that occurrence order. Wrong-store/expired/reopened/telemetry-off/text-mode membership precedence matches the existing path; membership failure records zero feedback labels.51 short offenders reports50+omitted1; an over128-byte offender is omitted whole and counted. Selected50 ambiguity tests are required separately after its ruling. |
| S80-A5 | No-default, configured default, use_store switch and separate sessions agree across top-level value, row flags, hint and quoted header, including hints=false. Store-less call addresses the sampled default. Guide remains within its existing1200-byte cap. |
| S80-A6 | Stored mapped-span fixture gives R4/K3/U1/T1/B16/V6/unattributed0 with exact shares1/3 and6/16; profile.dimension=8 remains numeric, while wire count/byte/fraction components use decimal strings. Passing ownerless/bodyless/dual-owner mappings increment only unattributed_heads; existing invalid-predicate and caller-specific completion failures remain errors in their current scope. Empty/unknown/known-empty/no-vector/pending cases obey design. Stable reopen/already-required full-status refresh produces numeric output; cold/changed cache produces only its tagged reason. Added aggregate work on orient is fixed token work, with no validation scan or vector_default probe; rollback after attempted DML conservatively invalidates. Same-snapshot/token/native/both-format checks follow O80-10–12. |
| S80-A7 | Same-snapshot actual vector participant with provider end9 returns scored_bytes9 for the exact selected row; unknown provider returns None; known0 returns Some(0); no vector contribution and standalone selection return None. Identical bodies/different records cannot lend extents. Stored malformed/missing physical tuple errors cannot become None. |
| S80-A8 | Ordinary/frozen/text-only, node/edge, mixed-arm and selected50 duplicate-ref/version cases retain positional sidecar binding. A concurrent later span commit cannot contaminate an older reader; post-mint mutation retains frozen refusal. UTF-8 boundaries and role-tagged bytes use explicit hand-authored byte oracles. All native/public conversions preserve zero and absence. |
| S80-A9 | Snippets off adds zero coverage lookups/sidecar allocations; text-only adds zero coverage lookups/provider calls. Hybrid snippets add at most one indexed span query per final actual vector-participant record, independent of snippet max_point_queries, never a store scan. Both-format final budget tests include every new byte and retain60 zero-progress/refusal rules. |
| S80-A10 | Concrete source-bound design revision has an actual independent Astra-medium verdict; findings are resolved or explicitly held open. Actual50/60/70 settlement, release requirements/AC recheck, owner approval and separate Block1 exit receipt are recorded before90. Aggregate v2's bounded design PASS is retained, while this packet's consistency recheck and full80 approval remain held. Each later behavior has real assertion RED→GREEN evidence and a fresh independent final review. |

Canonical qualification remains existing AC-031 (fallback), AC-060a (typed
errors), AC-068a/b (FFI strings), AC-069 (sanitized diagnostics), AC-074 (governed
parity), applicable AC-072/075/076/081 performance/reader gates and tracked
AC-083. These do not themselves test the new S80 behaviors. Readiness/span
storage and queue passes are regression evidence, not new REDs. This Slice80 documentary work ran none of them.
