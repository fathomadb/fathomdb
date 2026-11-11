---
title: Slice 80 source and authority census
status: PROPOSED
target_release: 0.8.29
slice: 80
---

# Slice 80 census

Read-only source witness: clean `2f6913c1d59ac8ff3a30de835b57290a8635998c`.
10 implementation104dcbc and closure d7d928 are predecessor evidence;20's
Linux repair657e is integrated, while policyA3/A5 and WindowsA6 remain open.
30 has no supplied measured baseline/TC5 inputs. Reviewed contracts and the
parent-supplied60 checkpoint below are distinct from integrated source. No mutable
worker output was treated as source authority;80 ran no build/test/native/DB/GPU.

This documentary consolidation incorporates the independently bounded
Astra-medium PASS for aggregate v2, exact source proposal SHA-256
`a5e4851df2fedcbcce3e7d1b93138980c25e172d0ceb802792d218940bca0e53`.
The consolidated packet's narrow consistency recheck is pending; this is no
new ABI/policy approval or full80/source-entry approval. All original packets
remain frozen. Parent source stays clean2f6913 with no imports.

Parent-supplied60 checkpoint: clean candidate
`307c92e3b4b8c674eec28fc940c879e9054fff90` has source TDD for formats,
filters and budgets, and post-code real-GPU manual SDK plus both MCP formats
14-case PASS. Native/full gate and fresh final review remain pending; F11
feedback ruling remains held. This is not60 completion, final-contract
settlement or integration into the clean2f6913 parent.80 did not run those
checks or read mutable worker output as authority.

The built/residual census below describes frozen2f6913 source, not the later
307c92e60 candidate. Paths start at `src/rust/crates/` unless stated otherwise. Named test
files supporting gap claims were read in full, including any separately read
remainder after output caps. Wider suites are not claimed qualified or absent.

| B2 row | Source witness and existing controls | Genuine residual |
| --- | --- | --- |
| 1 | mcp output.rs Envelope.state/SearchOutput.verdict required; tools.rs envelope/shape builds unavailable. Accepted hints ADR(f) already authorizes omission. | Structured optionality/absence and orient.not_in_this_release;60 already owns text omission, so do not rebuild it. |
| 2/3 | engine hints.rs is pure; mcp hints.rs/tools.rs already build no_results/degraded/cut hints. tests/hints.rs covers empty, empty_not_ready, first canonical cause, hints precedence/bounds/budget/quoting. | Reconcile70's accepted absent/timeout causes and both60 formats. No new weak threshold. |
| 4 | mcp tools.rs parse uses serde deny_unknown_fields and generic bad_argument. edge.rs builds typed input schemas. tests/schema.rs exercises unknown input but no typed accepted-list oracle. | Deterministic typed argument/accepted fields, bounded caller key, fixed remedies. |
| 5 | tools.rs feedback checks membership before epoch/telemetry/text outcomes; session.rs stores BTreeMap<ref,cursor>. tests/feedback.rs covers unique cursor attribution/concurrency/ref/store/epoch/sink/no-result-effect. | Bounded offending occurrences, code/detail fields.50's separate duplicate-ref ambiguity consequence remains unruled; no new ref ABI chosen. |
| 6 | tools.rs list_stores samples default once; row flags/default_store_set already exist. tests/stores.rs + hints.rs prove session isolation/default routing/hint on-off. | Top-level default_store and quoted first-line default; clearer guide sentence.60 owns general quoting and use_store renderer. |
| 7 | registry.rs + bridge.rs have FIFO wait/permit ownership/abandonment. tests/admission.rs covers32 waiters, queue_full/deadline/overload schema, cancellation/shutdown/progress/stuck/no retry_after_ms. | Both-format regression qualification only; no demonstrated new queue defect.60 owns rendering busy details. |
| 8 | schema lib.rs format35 span table survives37; engine vector_spans.rs validates tuple/body length/UTF-8; projection_commit.rs publishes mapping/span/vector/terminal transactionally. tests/slice60_store_format.rs proves unknown/known offsets, invalid offsets, corruption, rebuild/recenter, batch fallback, erasure and rollback. | Orient aggregate reporting and scored field are absent. Existing storage is preserved, not new work. |

## Aggregate authority and reviewed bounded delta

Accepted long-record §7.1.2 requests record/byte shares cached with projection
state so orient adds no scan; it does not mandate live E/P/K/U/M. That old
unapproved field proposal is retired. Aggregate v2's bounded Astra PASS covers
the concrete carrier and architecture now included in [design](design.md).

Source37 stores per-head mapping/span facts, not aggregate counters. The full
vector_spans validator at open.rs:1254 and physical_completion already traverses
mapping/span/canonical metadata and checks orphan spans. It does not inspect
physical vectors or reject a NULL canonical body. Preserve its exact invalid
predicate, including COALESCE body semantics. Existing physical_completion
cross-owner refusal and eligible-member physical-vector checks stay in their
original caller scope; reporting adds no blanket open-time integrity policy.

Pinned sqlite-vec0.1.9 vec0Filter_point at sqlite-vec.c:7363–7400 loads every
vector column even for rowid/EXISTS point probes. Therefore aggregate capture
has no vector_default join/probe/column/schema check. Only metadata arithmetic
is folded into the existing full validation pass; all observed existing errors
remain typed. Valid passing ownerless/bodyless/dual-owner mappings are counted
as unattributed_heads outside canonical record/byte denominators.

The concrete required StoredHeadCoverageV1 uses population stored_mapped_spans,
seven exact counters and nullable unreduced rational shares. The Engine-owned
cache is seeded only by committed full writer-side capture, and served only
when DB/profile/schema/clean visibility generation+nonce/same-connection
PRAGMA data_version/total_changes match. read_projection_status pins this check
and its returned coverage in one read transaction. Existing orient does not
force generation-status validation; cold and changed caches expose no numbers.
Attempted DML rollback may advance total_changes and conservatively invalidate.
Stable reopen or an already-required full-status miss can provide real numbers;
there is no always-current-after-write or whole-orient-snapshot promise.

This is stored metadata scope, including retained history/inactive/out-of-window
records, independent of live time/view/provider health and50's per-hit identity.
No missing-owner, no-vector, pending or failed population is silently treated
as full/known-zero. Exact fields, domains, cache publication and lifecycle rules
are part of design, requirements R6 and acceptance A6, not deferred drafting.

## Per-hit source carriers and read work

engine search_types.rs SearchResult.snippets already parallels final results;
SnippetV1 is non_exhaustive and opt-in. search.rs:2943 builds snippets before
the transaction commits. snippet_query.rs bounds lexical point queries to
2*max_point_queries. Vector input membership is available before fusion, but
the current fuse_three_arms body key and explanation body maps are50's subject.
Vector edges can carry TextEdge: branch is not vector-participation proof.
No existing scored field or public vector-participation bit is present.

Rust SDK exports SnippetV1; accepted70 proposes facade exports. PyO3/NAPI
snippet.rs and read_search.rs explicitly copy fields. Python types.py and
_sdk_search.py construct dataclasses; TS search.ts mapNativeSnippet explicitly
maps fields and native binding.ts declares them. MCP search_page copies
snippet metadata, then60's shaping retains whole hits. Every one needs an
explicit scored-field mapping; adding only the Rust field would be incomplete.

Full snippet_search.rs proves sidecar alignment/off-path identity/text/hybrid/
frozen/read-only/Unicode/edge/vector behavior and exact38 lexical point queries.
snippet_cost_receipt.rs distinguishes measured searches from its ignored sweep.
Python test_ep17_snippets.py, TS ep17-snippets.test.ts and native snippet modules
were read in full. Existing shared parity golden is a human oracle; never
regenerate it to add this field. Existing selector expectations can be compared
as their original field projection with explicit new None checks in new REDs.

Relevant verified memory: agent-access-block1-design-state.md,
long-record-embedding-findings.md, verify-design-against-code-not-just-architecture.md,
tdd-red-must-fail-per-finding.md, slice-prompt-verify-test-claims.md and
0.8.25-slice50-compact-evidence-lessons.md in the canonical main-repo store.
The older acceptance-md-locked-no-feature-acs.md index note is stale about the
maximum AC: current source extends through082. Retain its no-invented-AC rule,
use current source and the release's tracked083 allocation for actual IDs.

Contract witnesses read as proposals:50 phase-a design/feedback-decision,
60 actual20-reconciliation design and70 phase-a design under the parent's
exclusive temporary packages. The pending50 feedback-decision witness supplied
by parent has SHA-256
`0242afd3a7e61ad5b53045b7b70c4cb6e75b75fd4d4b1933909fa881a9e6295a`.
Neither its recommendation nor70 reason additions are silently promoted to
accepted source behavior by this census.
