---
title: Slice 80 local acceptance
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 acceptance

These are design acceptance targets and future90 behavior oracles, not passing
test receipts or canonical AC additions. Tests are planned in [test-map](test-map.md).

This revision uses integrated source
`e1cb1bd5311f68da8be98bbe133b9cecd5ab56d4`, selected identity/packing
seq-374 and feedback seq-375, and Slice50's independently reviewed
`10a72e5a9` carrier design. Selected design is distinguished from implemented API.
The earlier aggregate-v2 and consolidation consistency reviews passed; they are
not repeated. Fresh Astra-medium review passed on revision
`f2f4831554ae165420a31890cf102642ce28b21d`, and the owner accepted it in seq-384.
Seq392 permits90 after fresh integrated source fit while measurements continue;
no runtime completion follows this design acceptance.

| Local acceptance | Falsifiable observation / design completion |
| --- | --- |
| S80-A1 | All store-bound success shapes omit envelope state; search omits verdict. Structured discovery permits their absence; orient.not_in_this_release is exactly [state,verdict,kinds,counts], without a coverage entry. Both text and structured retain every real readiness/degradation value. Structured kinds/counts retain their existing shape. |
| S80-A2 | Existing empty/degraded/cut/default/queue controls remain mapped to actual tests. New70 labels use its accepted integrated vocabulary, without reconstructing a new failure taxonomy. No reserved weak_results/index_not_ready/retry_after_ms appears. |
| S80-A3 | Every tool returns unknown_argument for an unknown key, even alongside a wrong known-key type; accepted equals sorted tools/list inputSchema.properties. Reversing JSON key order changes nothing. Exact64-byte and multibyte-over64 cases obey the bound. Known-key-only malformed input keeps bad_argument. Valid calls allocate no corrective-detail payload. |
| S80-A4 | A real known response plus used=[foreign-b,foreign-a,foreign-b], unused=[foreign-c] returns that occurrence order. Wrong-store/expired/reopened/telemetry-off/text-mode membership precedence matches the existing path; membership failure records zero feedback labels.51 short offenders reports50+omitted1; an over128-byte offender is omitted whole and counted. Selected50 seq-375 ambiguity tests separately require atomic bad_argument only after acquisition/final epoch check on otherwise-recordable calls; unready/reopened/telemetry-off/text-mode retain not_recorded, and mixed unique/ambiguous refs write zero labels. |
| S80-A5 | No-default, configured default, use_store switch and separate sessions agree across top-level value, row flags, hint and quoted header, including hints=false. Store-less call addresses the sampled default. Guide remains within its existing1200-byte cap. |
| S80-A6 | Stored mapped-span fixture gives R4/K3/U1/T1/B16/V6/unattributed0 with exact shares1/3 and6/16; profile.dimension=8 remains numeric, while wire count/byte/fraction components use decimal strings. Passing ownerless/bodyless/dual-owner mappings increment only unattributed_heads; existing invalid-predicate and caller-specific completion failures remain errors in their current scope. Empty/unknown/known-empty/no-vector/pending cases obey design. Stable reopen/already-required full-status refresh produces numeric output; cold/changed cache produces only its tagged reason. Added aggregate work on orient is fixed token work, with no validation scan or vector_default probe; rollback after attempted DML conservatively invalidates. Same-snapshot/token/native/both-format checks follow O80-10–12. |
| S80-A7 | Same-snapshot actual vector participant with provider end9 returns scored_bytes9 for the exact selected row; unknown provider returns None; known0 returns Some(0); no vector contribution and standalone selection return None. Identical bodies/different records cannot lend extents. Only a validated NULL byte_end returns provider-unknown None; missing/multihead/misowned/malformed/physical-missing tuples refuse, including checked u32 overflow. Nested graph fusion cannot turn its synthetic vector input into participation. |
| S80-A8 | Ordinary/frozen/text-only, node/edge, mixed-arm and selected50 duplicate-ref/version cases retain positional sidecar binding. A concurrent later span commit cannot contaminate an older reader; post-mint mutation retains frozen refusal. UTF-8 boundaries and role-tagged bytes use explicit hand-authored byte oracles. All native/public conversions preserve zero and absence. |
| S80-A9 | Snippets off adds zero coverage lookups/sidecar allocations; text-only adds zero coverage lookups/provider calls. Hybrid snippets add at most one indexed span query per final actual vector-participant record, independent of snippet max_point_queries, never a store scan. Both-format final budget tests include every new byte and retain60 zero-progress/refusal rules. |
| S80-A10 | Concrete source-bound design revision has an actual independent Astra-medium verdict; findings are resolved or explicitly held open. This revision reconciles selected50 design, integrated60/70 and release requirements/AC mappings. Actual50 implementation reconciliation and explicit full80 owner acceptance are retained. Seq392 permits90 after fresh integrated source fit while measurements continue; Block1 measurement/disposition and qualification exits remain final, with the same approved72episodes once on final90. Aggregate-v2 and prior consolidation PASS remain historical; updated-design review is distinct. Each later behavior has real assertion RED→GREEN evidence and a fresh independent final review. |

Canonical qualification remains existing AC-031 (fallback), AC-060a (typed
errors), AC-068a/b (FFI strings), AC-069 (sanitized diagnostics), AC-074 (governed
parity), applicable AC-072/075/076/081 performance/reader gates and tracked
AC-083. These do not themselves test the new S80 behaviors. Readiness/span
storage and queue passes are regression evidence, not new REDs. This Slice80 documentary work ran none of them.
