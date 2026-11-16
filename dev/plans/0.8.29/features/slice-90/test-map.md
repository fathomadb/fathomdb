---
title: "Slice 90: behavior and verification map"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 behavior and verification map

[Acceptance](acceptance.md) is the human-intent oracle. Names below are planned
new test cases, not existing tests or executed results. Every Rn maps to An;
units below map those acceptances to concrete paths/controls. Real databases
and current existing entry points come first. No mock database/golden rewrite.

## Runtime RED units after approved source fit

| Unit / acceptance | Planned test and actual path | Required baseline RED and bounded GREEN |
| --- | --- | --- |
| U1 / A1 | mcp tests/schema.rs + real calls: s90_unsupported_fields_are_absent_and_disclosed, parameterized text/structured; schema check on actual returned success. | Baseline runtime presence of state/verdict and absence of orient disclosure fail assertions. Stage before changing output types/constructors/schema/renderers.60 text omission is a pre-existing control, not a new RED. |
| U2 / A2 | schema.rs: s90_unknown_argument_is_schema_derived_and_precedes_bad_type; all7 tools, reversed key order, nested attributes,64-byte/multibyte boundary. Call existing tools/list and call_tool. | Current actual error is bad_argument without accepted/argument; runtime code/detail equality fails. Implement only shared precheck/error fields/rendering; typed malformed known-only remains control. |
| U3 / A3 | feedback.rs: s90_foreign_reference_occurrences_are_bounded_and_ordered; obtain real response, send foreign duplicates; inspect real sink after error. Include51 and over128-byte offenders. | Current generic bad_argument lacks ordered refs/code; assert against JSON, not missing Rust fields. Stage membership tests before replacing that branch. Keep implemented50 atomic ambiguity refusal after acquisition/final epoch; preserve existing response lookup/mode/epoch controls. The existing membership-precedence test intentionally asserts old bad_argument; amend only that accepted code change with its new diagnostic RED, retaining its zero-label and ordering oracle. |
| U4 / A4 | stores.rs: s90_default_store_is_explicit_with_hints_disabled; two real clients, none/configured/switched default, store-less search; initialized guide bytes. | Current top-level default_store/header absent despite correct row flags/hint; dynamic result/header assertions fail. Change only new value/header/guide. |
| U5 / A5/A6 | PyO3 projection.rs cfg(test) + real Engine/test-hook provider: s90_real_projection_status_reports_exact_mapped_spans; hand fixture mixed known/unknown,0-byte, exclusions, pending/no-head and stable reopen. Convert existing read_projection_status via existing PyProjectionRuntimeStatus::from_rust and observe with Python getattr. MCP real orient and installed Python/TS use approved existing local-model fixtures; U8 stages and witnesses their aggregate REDs before any shared converter GREEN in this unit. | Existing engine status/converter compiles; Python hasattr/vector_coverage assertion fails at runtime. Do not stage code referencing missing Rust fields/helpers. One new aggregate carrier/capture unit asserts cold plus real full refresh/reopen exact values and wire object before GREEN. Implement public types, full capture/open seed and existing status path only after witnessed RED. |
| U6 / A7/A8 | After U5, s90_coverage_never_reuses_changed_writer_snapshot; real local write/rollback, external committed connection, profile/schema/rebuild/erase/reopen; s90_failed_capture_does_not_publish using existing transaction/SQL failure seams. | Each changed invalidation/publication behavior needs its own actual failing assertion at its parent revision before its fix. Do not label controls that already pass as RED or manufacture broken behavior to obtain RED. If the U5 implementation already fulfills an invariant, record regression qualification; no new production change may claim that passing control as its RED. Keep errors/availability distinct. |
| U7 / A9 | PyO3 snippet.rs cfg(test) existing from_rust of real Engine search snippet: s90_scored_bytes_follows_exact_vector_participant, hand provider end9, unknown,0, nonparticipant; same-body/different-row/repeated-ref fixtures under accepted50. Python getattr on actual converted snippet; no new field reference. | Current converted snippet lacks scored_bytes at runtime. Stage real per-hit fixture/assertions, then exact-key participation capture, bounded lookup and converter changes only after U8 has staged and witnessed the genuine installed Python/TS scored-field REDs. Missing helper/field compiler errors do not count. Rust direct-field tests can follow the new field's introduction as controls. |
| U8 / A5/A6/A9/A10 | Installed Python/TS aggregate status and scored-snippet tests use existing calls plus dynamic accesses, avoiding missing-field compile failures. Stage and run aggregate carrier tests before U5 converter GREEN and scored-field tests before U7 converter GREEN, on the actual parent-source owned native artifacts. Use test_slice22_projection_status.py and slice22-projection-status.test.ts for existing status calls, EP17 suites for search/snippet calls. Include ordinary/frozen/text, UTF-8/0 and exact numeric transport. Search-expand is a Rust engine control; do not invent a new installed search-expand or evidence-plus-snippet API. | U8 is prerequisite RED staging interleaved with U5/U7, not a later claim of new RED after shared converters already pass. Both installed languages must show semantic missing/dropped/rounded-field assertion failures before ANY shared converter change that could make those tests pass. Then keep those staged oracles through GREEN. Later reruns are qualification only. Missing native/model inputs hold that converter GREEN; another checkout's artifact or stub is no substitute. A wrapper object stub is supplementary malformed-payload coverage only. Newly discovered transport defects get their own RED/GREEN. |
| U9 / A11 | Current integrated60 formats.rs and located budget/paging suites: s90_new_metadata_counts_toward_final_fit and s90_new_error_details_roundtrip_in_both_formats. Real orient/search/error output; boundary budgets/digit widths/quotes; unchanged-state page traversal only where leading whole hit fits. | Capture actual byte-fit/grammar failure before changing finishing/shaping. Some failures may already be prevented by60's generic accounting; classify those as preserved controls, not invented new TDD work. No redundant production budget rewrite. |

U8 aggregate RED staging precedes U5 converter GREEN; U8 scored-field RED staging
precedes U7 converter GREEN. Its table position is not execution after those
GREENs. Rebuilding a native artifact without staged baseline failure does not
satisfy the installed-language RED requirement.

U5's deterministic partial/unknown native fixture is feasible today: PyO3 and
NAPI dev-dependencies already enable engine/test-hooks; existing test provider
and real temporary files can project explicit offsets. No production caller-
provider API or model download is added. Native public end-to-end controls use
source-bound owned artifacts and already-approved local fixture/model paths;
if those inputs are missing, stop affected execution and report the missing
input instead of using another checkout's artifacts. No such execution is allowed
during this documentation refresh.

The first semantic assertion in each RED must be the claimed missing behavior,
not setup/import/compile/typecheck failure. Record command, exit, failing assertion,
exact source/native artifact and feature/env in the RED commit body. Stage or
commit before production edit. GREEN preserves staged oracles. Reviewer checks
actual command/assertion results and the diff; no detached RED audit campaign
or supplemental receipt is required. A common shell runner cannot paper over
import errors or stale native builds. No passing-on-arrival automation is
labelled RED. Documentation-only
changes have scoped verification, not fake behavioral tests.

## Independent properties and work controls

| Acceptance | Planned proof, with manually specified invariants |
| --- | --- |
| A5/A6/A10 | Property-test legal per-head length/end/unknown fixtures and exact counter/fraction sums against simple test-side integer oracle; include multibyte char boundaries, known-empty, all-unknown and excluded mappings. Check u64/i64-count/u128 boundary transport using real typed converter values. For both variants assert profile.dimension8 remains numeric8 in actual MCP/NAPI/TS output and exact Python int8, while counters/bytes/fraction components alone use MCP/NAPI/TS decimal strings. Preserve existing dimension validation; do not coerce it to a string. Malformed decimals/tags/missing fields/out-of-range and inconsistent fractions produce exact typed path errors. No snapshot regeneration. |
| A7/A8 | SQL trace/statement/work counters on two parameterized corpus sizes: warm AND cold orient must add no span/owner traversal/full-status call; capture must add no vector_default access/probes/schema checks and no second full pass. Inspect actual statement work, not merely EXPLAIN “indexed”. Report added full-pass row/arithmetic overhead separately; no new latency threshold or benefit claim. |
| A8 | Real file/snapshot sequences: capture then external commit before publication, rollback coalescing branch, local non-visibility write detected by total_changes, generation/nonce/profile mutation, erasure, rebuild, reopen. Tokens from a reader connection cannot populate writer cache. Failed token/SQL/commit/poison paths stay typed error, never unavailable or stale available. |
| A9/A10 | Same selected key appears via vector/text/graph; text-labelled vector edges, equal bodies, repeated public refs, retained historical versions, source/visibility and provider changes. Compare hand UTF-8 bytes in original stored role-tagged bodies; None never becomes0/full. Frozen post-mint mutation retains existing refusal. A controlled pinned reader cannot acquire a later endpoint. |
| A10 | Counters prove snippets-off0 participation-set/sidecar/coverage work, text-only0 span/provider calls, standalone0 reads; hybrid≤one indexed span lookup per final actual vector participant, separate from lexical≤2*max_point_queries. Validate physical tuple only in this bounded path. |
| A11 | Existing hints/admission/shutdown/frozen/filter controls on same final60/70/90 build. Preserve no retry_after_ms, no weak_results/index_not_ready, queue wait then run-to-completion, permit retention after abandonment, hint quoting/bounds/fallback and exact response/continuation prefix semantics. |

New trace/cost/operator tools, if needed, belong in the implementation repository
with parameterized rerun command/env/output/feature requirements and an indexed
experiment README; commit before using their results. Raw logs alone may live
in scratch. Use existing hooks first; no tampered corpus, fixtures, threshold,
benchmark skip, test filter or regenerated golden to improve reported outcomes.

## Existing evidence and execution order

Fully reread on the current integrated source: MCP schema.rs, stores.rs,
feedback.rs and formats.rs; engine snippet_search.rs, slice60_store_format.rs
and snippet_cost_receipt.rs; Python test_ep17_snippets.py and
test_slice22_projection_status.py; TS ep17-snippets.test.ts and
slice22-projection-status.test.ts; both native snippet modules. Current
status/sidecar/routing/ambiguity/format/accounting controls do not assert the
new90 fields. This is inspection, not an execution pass.

Concrete existing contract assertions affected by accepted80: schema.rs
output_schemas_are_not_vacuous currently accepts list_stores without the new
required default_store; formats.rs list_stores_text_has_one_block_and_quoted_names
pins its old header; feedback.rs repeated_reference_feedback_keeps_existing_not_recorded_and_membership_precedence
pins generic membership bad_argument. Update those intentional old-shape
assertions with their corresponding new semantic REDs. Keep their original
nonvacuity, routing, membership precedence and atomic zero-label intent.
No new RED oracle or human golden is relaxed to pass the implementation.

Engine snippet_search.rs already asserts sidecar alignment/off equality,
vector-only Start versus lexical Match, frozen same-results and reader-only
work; its lexical trace bound remains2*max_point_queries. Stored span tests
already assert unknown offsets, provider UTF-8 offsets, tuple rollback,
retained history, invalid tuple/orphan refusal and erasure. Preserve them and
add coverage lookup counters separately; do not rebuild span storage/selector.
Py/TS pure snippet parity tests remain unchanged human goldens: standalone
scored metadata is None/absent, so add independent field assertions rather than
regenerating their expected fixture objects.

Other affected hints/admission/generation/frozen/binding error/parity suites
are located controls; reread complete files before prescribing any assertion
change. Preserve applicable accepted predecessor results. Historical source
passes cannot prove the unimplemented90 fields.

After actual source GO: focused lint→typecheck→unit tests per unit, then real
owned Py/TS native/SDK/MCP seams. The parent owns one required full
scripts/agent-verify.sh on stable final integrated code with checkout-owned
.venv/native artifacts; respect
strict ptrace executor requirement. Run appropriate feature/performance gates
and scoped interface/Markdown validators. One scoped pass is not full workspace
green. Get one fresh independent Astra-medium implementation/final review, integrate
exact reviewed revision and rerun affected integration seams.160 owns composed
close, not90 source entry. Missing prerequisites remain explicit failures/holds.

## Refusal and final applicability controls

U7 must include semantic error oracles for missing mapping/span/physical vector,
misplaced/extra/nonzero head, ambiguous canonical owner, body mismatch, malformed
integer/end/UTF-8 boundary and u32 overflow. Only after full tuple validation
may NULL produce unknown None. Exercise original-vector participation before
nested graph fusion, canonical node/edge separation, ownerless FTS numeric
collisions and same-key final position after CE/limit, evidence/explain off.
These are per-hit refusal tests, not broader aggregate/open integrity rules.
U5 keeps ownerless/bodyless/dual-owner passing mappings in unattributed_heads
without new physical probes or refusal policy; existing invalid predicate and
completion refusals remain separate regression controls.

Final source qualification assesses changed90 consumers of160 Q2–4/Q10–12/
Q14–17. New Rust/native fields and converters can invalidate earlier installed
package/strict-TS observations; new default MCP metadata/schema/budget can
invalidate specific conformance and smoke/item4 output/paired results. Keep
earlier Block1 results as their baseline's evidence, then rerun affected final
scopes under actual permission/readiness. Under seq392, the same approved
72episodes run once on final integrated90 before local close; no pre90 smoke
is required for source entry. No automatic full160/model replay,
new paid-call grant or90↔160 source-entry cycle follows.
