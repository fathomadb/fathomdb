---
title: Slice 60 — actual source execution register
status: IN_PROGRESS
target_release: 0.8.29
observed_on: 2026-11-10
---

# Slice 60 — actual source execution register

## Source and resources

Sole writer: `/mnt/data/worktrees/fathomdb-0.8.29-s60-mcp`, branch
`llm/0.8.29-s60-mcp`, dispatched from2f6913c1d59ac8ff3a30de835b57290a8635998c.
Parent owns all worktree creation and release metadata. Entry and repeated own
strict preflight/ownership checks passed before setup. Closed10 is60's only hard
dependency;20 remains a partially integrated subset with its own holds.

Task resources/logs/receipts are exclusively under
`/mnt/data/tmp2/fathomdb-0829-01a1c5a5-s60`. Each named phase below has a complete
`logs/<phase>.log` and `receipts/<phase>.json`: exact argv/features/environment
entrypoint, start/end SHA and tracked delta, real exit, timings and resource
samples. Node25.9.0/npm11.12.1, canonical own noneditable Python3.13.13 venv and
root/TS/Mermaid modules are owned; Cargo home/target and all mutable caches/temp
are private. No sibling mutable environment/artifact, GPU/model/paid work or
provider injection was used. One native phase/jobs2; final canonical Rust jobs1.

The monitor starts only with root≥32GiB, task≥250GiB and MemAvailable≥16GiB.
It stops only owned workload for root<20GiB, task<50GiB, MemAvailable<8GiB,
owned size≥200GiB or swap>64MiB/s across three30-second samples. No recorded
resource stop occurred. No cleanup was performed.

## Actual runtime RED then separate GREEN

Every row below is a behavioral assertion failure (runtime101), committed before
its separate production fix (runtime0). Compiler/setup/fixture authoring failures
remain in earlier logs and are not product RED. Shared fixes cover related
assertions staged together; the rows do not manufacture independent pairs.
Selectors below use `cargo test -p fathomdb-mcp`; complete exact commands and
preceding scoped clippy/check results are in the phase receipts.

| Behavior | RED commit | GREEN commit | Target/selector | RED/GREEN phases |
| --- | --- | --- | --- | --- |
| F01 JSON acceptance | 0b77b4765 | c4129c2d1 | config/result_format_json_accepts_only_the_two_values | f01a-red / f01a-green |
| F01 embedded validation | 888d419aa | b883abe43 | config/embedded_result_format_is_revalidated | f01b-red / f01b-green |
| F02 format-bound discovery | ad738813f | 57cb0d02f | formats/default_discovery_has_no_output_schema | f02-staged-red / f02-green |
| F03 one-block text envelopes | ea119dd71 | 9f38302f1 | formats/list_stores_text_has_one_block_and_quoted_names | f03-red / f03-green |
| F04 search grammar/CE/score | 9c89367b0 | e54688f22 | formats+lib/compact_search_ | f04a-red / f04a-green |
| F04 full diagnostics/highlights | 553eab3c0 | 20127e73e | formats+lib/compact_search_signal_ | f04b-red / f04b-green |
| F04 hosted caveat | 524f685ae | 96de98b06 | formats/hosted_continuation_reports_best_effort | f04c-red / f04c-green |
| F05 orient real fields | 41d17551c | 688c56265 | formats+lib/compact_orient_ | f05-red / f05-green |
| F06 byte frames/order/codec | 39c771533 | e27a01926 | formats+lib/compact_fetch_frames_ | f06a-runtime-red / f06a-green |
| F06/F09 shared full errors | d34ce5b50 | 13538ddf0 | formats+lib/compact_error_ | f06b-f09b-red / f06b-f09b-green |
| F07 expand grammar/codec | 9c04a5ffc | 1aeedc5a4 | formats+lib/compact_expand_ | f07-red / f07-green |
| F08 selected store | 3a0c42965 | d9ec54c6e | formats/compact_use_store_quotes_name_and_preserves_session_action | f08a-red / f08a-green |
| F08 feedback envelope | e6802b62c | 298ffb160 | formats+lib/compact_feedback_ | f08b-red / f08b-green |
| F09 exact structured JSON,7 successes+7 errors | 291cebee5 | b81449a93 | formats/structured_text_is_exact_final_payload_json_for_all_tools_and_errors | f09a-frozen-runtime-red / f09a-green |
| F12/F13/F14 closed text predicates/route/cap/validation/paging | 75e7d5da9 | 512015678 | direct_filters, all4 tests | f12-fixture-corrected-red / f12-reviewed-green |
| F10 final counts/fit/zero progress/hints fallback/prefix property | 18286cd39 | b2e955007 | formats+hints_properties+lib | f10-frozen-red / f10-green then f10-seam-borrow-green |

F10's first GREEN passed real wire/baseline/unit targets but exposed a production
seam returning internal compact text for its explicit structured presentation.
The seam was corrected without changing the frozen assertions. A Rust move-order
compiler error is retained separately. The independently sized zero-progress
specimen observed old kept1 versus expected0; arbitrary256..6000 budgets remain
unfiltered. The independent oracle enumerates authored leading prefixes and
hints-off fallback; complete traversal requires independently proven progress.

Human historical JSONL blob63c3ad7318dacbccbb99c14c7ced2429ff29b058 remains
unchanged, SHA2560cdca8a0b906b9cd0b559c0e870e898fd4637da4b9d6176b99047a93c73070b4.
EP03 PINNED was not regenerated. Comparator negative controls reject score,
source, order, unknown/type/state/verdict/error/count/handle/middle removal and
unjustified empty/shorter fetch bodies. The exact allowed adaptation is in the
committed test plan and independently rechecked v2 proposal. Initial automatic
review rejected the broad-looking edit; the exact rejected proposal was retained,
bounded Astra FIX/PASS followed, and the narrow retry was approved. No bypass.

## Affected integration checks and corrections

`mcp-focused-gate` ran all MCP targets once after scoped lint/check. It retained
four failing targets: old structured hint-line determinism expectation; a newly
authored dual-opener hints fixture; old unconditional budget success; and real
production hints-overhead273.8us above the unchanged200us bound. Other target
passes remain valid scoped evidence, not a full-workspace claim.

Production optimization1f28b1609 caches budget-independent serialization size,
avoids deep Value clones and uses exact fixed-key insertion byte length.
`budget-overhead-final` passes21 unit,18 real wire,6 properties,2 immutable
baseline,1 census and the unchanged overhead gate. Notify-only10-hit warnings
remain visible; the prior measured intermediate optimization was187.7us at100.

Independent Astra passed the exact F12 one-assertion correction: SDK lexical
node Text→text and edge TextEdge→text_edge, rejecting vector/graph. IDs/raw
scores/routes/providers/cap/paging remain asserted. All4 tests then passed.
The first failed GREEN and original four REDs remain.

Independent Astra passed late integration adaptation7a6e9a788f379d2b10808350ef8a1bf00331e2870ed2350a9cc581305a9b59a7.
It uses separate authored SDK fixtures and checks both formats' pinned exact
bytes/hints/paging. Determinism3 and hints14 pass. The shortest mandatory-field
bound could not justify the actual hybrid+explain300 refusal; that failure is
retained in reviewed-integration-targets. Independent Astra passed the stronger
SDK trace control proposal45f5a31fe2ef7f491e9838a527f6abe02b3212a9b9866ae3de13a35d1adcc35d:
read same-state SDK trace before adapter open, close SDK, use explicitly authored
zero-progress envelope and exact two-copy accounting, retain all28 cases and
successful full-fit assertions. reviewed-sdk-budget-control passes scoped lint/
check and the complete21-test search target. This is a late test correction, not a new
product RED, threshold change or policy/oracle waiver.

Supplemental digit-boundary/compact-progress controls and serialized-field
census are additional coverage of existing observed RED behavior, not new RED
pairs. final-budget-boundary-controls-corrected passed after an uncommitted
String-helper typo was corrected before validation. Canonical Engine ownership/module/48-field census and owned developer-tool
preflight pass in structural-and-entry-tools. Parent independently replayed clean
Config RED0b77b4765d using its own detached checkout/resources during an explicitly
released native window; no remaining audit child preceded its return.

## Qualification boundary

The single strict full agent-verify wrapper ran on clean307c92e3 and failed at
pinned-override-rot: the durable manual driver added an unsupported tracked
Cargo-patch location. Earlier canonical lint passes and the actual failure are
retained. Same Astra passed exact four-file packaging repair7038b5b4: no guard
change/new pin, exact root patch/source binding/frozen lock validation, external
materialization only. Static valid/refusal controls passed without native/model
runs. Failed and remaining canonical stages resumed on clean
7038b5b47dc7dd02bc64e612d788fa334ff79b61; there was no second full wrapper.

Assembled resumption exited0:214 registered/ran/passed,0 failed/skipped/excluded.
Rust:2694 passed,0 failed,67 ignored across405 reports; Python1744 passed,
38 skipped; TypeScript625 passed,0 failed,6 skipped of631. Strict security had
0 violations/blockers/downgrades, including real ptrace and live netns. Native
Rust used canonical serial jobs1/test-threads1. Numeric resource floors remained
satisfied, with no stop;285 complete captured logs were retained and no opens
were missed. Raw commands/exits/counts/paths/hashes and original wrapper failure
are indexed by owned task receipts/final-qualified-subset.json and its referenced
resource/retention manifests. Canonical ignored/skipped cases remain visible.

Production Python/TS canonical builds and two installed open/close cycles each
passed on307; exact relevant Rust/Python/TS/manifest/lock inputs are unchanged at
7038. Production Python digest92d8c07ea23dc1c420fd96d8ea5b685dc94c5924dd6bb997d1f20a03fc922ac7
and NAPI digest0f80025e5209d513d277aef964e030093ce142f42ac850242c2c218239b42a73
are separate from test-hook qualification. The canonical fresh child-loaded test
module digest2c681fa7cb5e35811f951283f2757e668b2f881d8601b61d9ec796d000bcaf7e
binds clean7038 and nonceagent-test-3483006; the canonical receipt verifier passed.
Selected Python shared-library path/hash/SONAME and command-local loader path
remain verified. The read-only spill collector preserved native output across
canonical unlink without changing any gate. Its earlier inotify probe failed
before a child due to host watch limits; no global setting changed. The2ms
polling/retained-FD probe and final collector both passed with no missed opens.

F11 search view exposure/repeated-ref feedback remains held for the actual user
ruling and reviewed addendum. Positive MCP projected-status/time qualification
uses a real Default producer and subsequent None reader. The late authored
projected_metadata.rs candidate was compiled only, archived externally at
SHA25616348d6334c1326fa8e5203e925a8b71b242389990378b9924ca7e5aa2bf2ca7,
and removed from the source by normal editing at parent instruction. It never
ran or landed and was not staged before F12GREEN. Manual acceptance will not be
misrepresented as a new RED or automated regression coverage.

Read-only probes found config/tokenizer/weights at current loader pins in the
user cache (743/711396/133466304 bytes, stable hashes). Standing GPU policy
requires the available3090 for real BGE preparation. CPU model execution never
occurred. Source-only manual SDK drivers and actual normal MCP stdio consumers
are external; no new provider/API/helper is introduced in the product.

The bounded GPU v1/v2 proposals received independent FIX for actual XDG cache,
both consumer formats and inherited descriptors. Astra passed final v3 after
same-path model-free FD/namespace probes. Actual run at checkpoint2d8278c72
passed one Default3090 producer (3 vector rows), no download/reranker/witness,
then closed.76 NVML samples bound PID3418425 to selected UUID/cuda:0 with5%
utilization/436MiB. Driver580.178.04/primary12.6 unchanged. Genuine a/edge
status='' and created_at1794366027 were preserved in the completed store.

Separate no-CUDA SDK None14 cases and actual normal MCP structured14 cases
passed. Compact orient initially failed two manual expected field spellings;
existing F05 contract was correct. Same Astra passed exact two-spelling fix and
optional format selection. Text-only resumption passed14 cases/exit0; original
failure/structured pass remain retained, with no repeated producer/structured
run or new product RED claim. Combined receipt is task gpu-metadata/run-001/
manual-qualification.json. Exact executed sources plus separately disclosed
portable operator copies, lock provenance, commands and artifact hashes are
indexed in dev/experiments/0.8.29-slice60-projected-metadata/README.md.

No ref ABI/guard/provider/model workaround, whole60 completion, full green,
20/30/Block1 exit or platform release qualification is claimed. Fresh independent
Astra final implementation/results review remains required; owner does not
self-approve or merge.
