---
title: Slice 60 — test bodies, independent oracles and TDD sequence
status: PLAN_READY
target_release: 0.8.29
observed_on: 2026-11-15
---

# Slice 60 — test bodies, independent oracles and TDD sequence

## Current F11 test sequence

Entry453b640f includes qualified subset5271 and integrated50
`9036507bc98ef31c05ae334aa22ec5101323c7e7`, completed70 and accepted80 design.
The initial source reconciliation found both flags missing. After independent
plan acceptance, actual per-unit assertion RED→GREEN covers superseded exposure,
null/presence validation and inactive/combined exposure. All10 frozen tests now
pass in search_views/null_view_flags. [Execution](execution.md) retains exact
subjects and the original timed-edge fixture failure/correction. Existing50
feedback guard/race tests remain controls; fetch's own four-view parity does not
replace search-view proof. Final coherent review/source qualification remain.

Approved pre-fix sequence (now executed through scoped GREEN):

- F11a RED: baseline-callable real MCP JSON superseded selection, authored
  old/current and equal-body revision occurrences plus same-build SDK positional
  ID/body/order/score controls in both formats and both routes. Structured scores
  equal SDK raw values; compact scores equal their accepted three-decimal display.
  Stage false/absent identity equality, true identity difference, fixed validity
  and stored-view continuation/explicit-false override checks before GREEN.
- F11b RED: inactive and combined flags select authored current/pending/deleted/
  superseded nodes and edges under their actual validity/source-closure rules.
  Stage malformed bool types and filter conjunctions before inactive GREEN.
  Replace the obsolete true-flag refusal with wrong-type validation before the
  related GREEN; freeze those assertions while repairing implementation.
- Stage both-format repeated-ref historical-search→feedback composition before
  F11 GREEN. Recordable hybrid names of ambiguous revisions must write zero
  labels; mixed unique/ambiguous calls are atomic. Text/telemetry-off retain
  not_recorded. Existing50 same-cursor/acquisition/final-epoch tests remain
  controls. Fetch remains latest eligible, not exact searched-revision lookup.
- Stage both-format constrained budgets and flag/filter continuation traversal
  before the corresponding fix. Require whole-hit progress for no-gap claims;
  tiny budgets keep accepted zero-progress/refusal semantics. Preserve hosted
  continuation_exact=false and reopen/cross-store/expired refusal controls.
- Each actual behavior gets its observed assertion RED commit then separate
  GREEN. Already passing assertions remain controls; no fabricated failure,
  missing-helper compile RED or repeat test solely for attribution.

## Original subset oracle mapping

The table and F01–F14 register below retain the original preimplementation
baseline and authored oracles. F01–F10 and the direct-filter/budget portions of
F12–F14 are implemented/qualified in [execution](execution.md); their old source
descriptions are dated evidence,
not current gaps. F11 and its affected composition are now scoped passing as
above; this is not final integrated qualification.
Paths below use `src/rust/crates/` prefix.

| Need / requirement → AC | Actual body and current oracle limit | Future failing assertion and independent expected result |
| --- | --- | --- |
| OUT → OUT | MCP config::empty_object_yields_documented_defaults; schema::every_tool_declares_draft_2020_12_input_and_output_schemas; protocol::tools_list_and_capabilities_are_static_across_configs_and_contents require current structured path. | F01 config accepts explicit text/structured, default text; strict invalid values name key. F02 default tools/list has seven names/unchanged inputs, no outputSchema; explicit structured has schemas. Start callers through current APIs/JSON so RED is behavior failure, not missing enum/helper compilation. |
| OUT → OUT | common::ok/err unwrap structured; OUTPUT_SCHEMAS is global OnceLock; RawClient/Spawned also unconditionally validate structured. | F03–F09 grow caller-visible compact grammar one tool at a time. Every response has exactly one TextContent and no structuredContent; explicit structured text equals serde JSON of the same schema-valid value. Use per-connection schema capture for mixed-format tests; never let cache from one connection validate another. |
| REF → REF | fetch_expand::fetch_returns_one_item_per_reference_in_request_order asserts seven outcomes; hints::hit_line_refs_are_quoted_in_text_blocks checks quoted odd IDs but old line grammar. | F04/F06/F07 refs containing spaces/quotes/backslashes/LF/CR/pipes/combining and astral Unicode round-trip unchanged. Graph refs and kinds follow actual accepted validation; invalid store name remains invalid. Search/expand body pipes/header words cannot create new rows. |
| REF/BUD → REF/BUD | fetch_expand::fetch_bodies_are_cut_to_fit_the_budget checks prefixes; current JSON-quoted bodies are not byte frames. | F06 hand-authored body `a\nrecord "l:fake" kind="x" bytes=4\n😀\n` is recovered byte-for-byte by reading n UTF-8 bytes and one framing LF. Include zero length, CRLF, embedded NUL body if write accepts it, final newline, and duplicate refs interleaved with missing/malformed/h:/p: errors. Count/order stays exact or whole call gets documented empty-body budget refusal. |
| BUD → BUD | search::every_response_fits_its_budget; hints::hints_count_against_the_budget recompute actual old structured+human-text bytes. | F10 final text count is ceil(bytes/4), not chars; hand-chosen lengths cross9/10,99/100,999/1000 with hints/cut/escapes. Structured fit is both exact final JSON copies; displayed used preserves documented exclusion. Assert convergence≤4 with real requests and pure boundary cases. Add independent tiny-budget zero-progress success with truthful cut/completeness, no continuation/hints after fallback and final fit; mandatory metadata overflow still refuses. No search snippet recut. |
| BUD/FTS4 → BUD/FTS5/FTS6 | search::continuations_page_without_gaps_or_duplicates_up_to_the_ranked_cap compares100 refs to same-route full result. shape properties cover offsets only. | F10/F14 independent finite ranked fixture and same-build SDK control: every format keeps ranked prefixes, own continuation resumes offset+kept, complete no-gap traversal through100 only when every nonterminal budget admits at least one whole leading hit. Keep arbitrary smaller budgets: independently expect either an advancing ordered prefix or exact zero-progress/refusal outcome below, never silently exclude cases. Search hint fallback cannot lose the sole otherwise-fitting hit. |
| SIG → SIG | search::explain_attaches_a_per_hit_explanation_without_run_varying_ids verifies trace/per-hit; text renderer omits trace/CE/highlights and hosted caveat. schema validates structured only. | F04/F05/F09 parsed text preserves all kept fields, exact JSON explanations, requested-empty vs absent highlights, hosted continuation_exact=false, projection dense readiness/affected kinds/runtime reason and current error candidates/detail/busy. Field census fails on an unclassified serialized path. |
| VIEW → VIEW, FTS2/3 | search::invalid_arguments_are_typed_invalid_requests currently rejects include_superseded; as_of_selects_the_validity_window_and_is_reported checks lower/interior/exclusive upper for both modes. fetch_expand same-view SDK parity covers all four fetch flag combinations, only default search. | F11 explicit search flags false/true individually/both yield hand-specified current/old/pending/deleted nodes and edges at fixed instants plus exact SDK parity. Flags never bypass mandatory edge expiry/source closure. Wrong type fails bad_argument; no include_out_of_window surface. |
| FTS1/5 → FTS1/FTS6 | search::filters_are_hybrid_only_and_constrain_hybrid_results requires five text refusals, hybrid doc only. | F12 selected text filter actually succeeds and returns only expected ID, then explicit/config text each filter/conjunction/empty attributes. Old refusal baseline must fail the new success assertion at RED SHA. Keep hybrid exact-doc control. |
| FTS2 → FTS2 | engine::selective_filter_precedes_node_cap_and_node_scoped_attributes_exclude_edges has120 stronger nonmatches+target; filtered_node_edge_prefix_and_empty_filter_preserve_order_and_scores checks same-engine prefixes. | F12 MCP fixture has>100 stronger nonmatching nodes, one lower lexical target and equal-body edge. Establish target absent in unfiltered top100, present at filtered limit1; attributes AND excludes edge and second-attribute mismatch gives empty. Independent expected target/kind/source_type map plus SDK parity. |
| FTS3/5 → FTS3/FTS4 | engine::attribute_validation_and_view_axes_compose_with_direct_text_filter rejects unknown/rankable-only even query nohit. metadata_keeps_projected_sentinel_time_and_missing_row_semantics uses actual projection timestamp/sentinel and missing provider-free rows. | F13 malformed string/int/map values reject at wire; unknown/nonfilterable attr returns bad_argument + InvalidFilterError even nohit. Projected empty status and created_at exactly bound match; bound+1 and status=open do not. Missing metadata never matches body JSON's fake status/time, including relaxed/history view. |
| FTS3 → FTS4 | SDK direct route calls core search_text_only_filtered_with_snippets; engine direct route uses no query-vector/embed/CE/graph API. Provider-free MCP text already works without filters. | F12/F13 filtered real fresh None-provider store succeeds, text mode/arm/no vector degradation. A narrow test-seams dispatch witness at actual SDK call records direct-text once/hybrid zero; source audit follows that SDK call to engine. Distinguish this route witness from quantitative engine-inference instrumentation; no production numeric provider is introduced. |
| FTS4 → FTS5/VIEW | search::continuations_are_stale_after_a_reopen_tampered_or_mixed_with_arguments covers old limit/tamper/reopen. session stores canonical request/time/offset only. | F14 each predicate/true flag changes canonical identity; attribute key order and absent/explicit false flags do not. Continuation overrides (including empty attrs/false flags) reject; same predicates survive pages and hosted writes. Fixed-time SDK control establishes each page's current filter eligibility, never no-gap promise after writes; retain hosted false caveat and cross-store/reopen/LRU refusal. |

“FTS” requirements and acceptance keys expand to R29-60-FTS and AC29-60-FTS;
OUT/REF/BUD/VIEW/SIG similarly use R29-60-/AC29-60-. F01–F14 are planned
behavior units below, not canonical AC IDs or a claim of14 passing tests.
One source oracle can constrain several units; this does not manufacture
independent correctness evidence from adapter-versus-itself comparison.

## Real-store fixtures and controls

Use SDK writes/configure_projections to make fixtures, real MCP calls through
rmcp/stdio and immutable same-state SDK reads. Do not open the same database
through a second Engine while adapter holds its lock: compute expected control
before adapter open, or register the same Arc<SDK Engine> in hosted mode and
make both reads against it. Pin validity time and use identical snippet
max_bytes/head/windows/highlights/run window. Historical attribute validation
comes from the reader snapshot, not a body-only adapter filter.

Positive projected status/time control requires real existing metadata. Prepare
it through the accepted existing projection path in an owned default-embedder
fixture, close, then read with explicit None provider; do not synthesize status,
mutate metadata to make a query pass or reinterpret JSON. Fresh None-provider
fixtures cover absent metadata independently. Existing engine test uses a
TestEmbedderChoice to create metadata; that does not mean SDK/MCP exposes such
injection. If required owned model artifact is unavailable, mark that selected
positive control unexecuted; do not claim FTS4 complete from negative cases.
No model fetch or GPU session is authorized by this PhaseA plan.

For views, use exact authored reference lists and bodies at lower/interior/
exclusive upper instants; qualify each source/kind/attribute/status/time field
individually before conjunctions. Include real supersession/inactive transition,
node/edge parity and duplicate bodies; under an accepted50 successor, use its
record-identity oracle and rerun parity rather than pinning body dedup forever.
For Unicode highlights, assert source substring bytes and offsets remain valid
after CR/LF→space-per-byte flattening, with composed/combining/astral examples.
Properties supplement these hand-selected expectations; do not build expected
framing using the production renderer or replace human goldens with generated
snapshots. Error negative controls include unknown store candidates, SDK invalid
filter detail and seam-driven busy object without redefining admission timing.

## Tiny-budget oracle and existing property adaptation

Before the corresponding F10 GREEN, adapt the existing complete property
`hints_properties.rs::hints_on_may_shrink_kept_but_offsets_stay_consistent`
(lines145–189 at bootstrap). It currently generates budgets256..6000 and
unconditionally expects the continuation chain to return the entire ranked
list; the approved two-copy structured format can validly stop with zero kept.
Do not increase the generator minimum, prop_assume/filter out tiny budgets,
accept arbitrary partial lists or use production-shaper output to decide whether
the test should demand progress. Preserve hints-on≤hints-off, ordered-prefix,
o-offset-skip, final-fit and bounded-termination assertions for every generated
case, including those that cannot progress.

Use a separate test-only reference oracle derived from authored wire fields and
fixed hint templates: independently assemble expected payload/text for each
ranked leading prefix, final-count accounting and hints-off fallback, without
calling production prepare_search_page, shape_search_page, finish, renderers or
hint planners to compute the expected outcome. Generation retains the existing
ASCII synthetic bodies/short refs with known snippets plus dedicated Unicode
boundaries. Handle widths can use independently fixed grammar lengths for byte
sizing; handle identity itself is checked by independent canonical-hash cases.
This oracle enumerates possible prefix sizes and independently determines the
largest fitting leading prefix and whether hints fallback is required. It must
also distinguish mandatory zero-hit overflow before comparing production output.

Author fixed expected cases before implementation, including the reviewed
one-hit120-ASCII-snippet structured witness (295 pre-hint tokens when using that
exact witness's fields), a corresponding text page, a budget that fits one hit
only after dropping hints, a genuinely empty input and mandatory metadata
larger than the budget. Record complete authored fields and independent final
byte calculations, not a universal295 threshold. Reproduce the reviewer witness
exactly or use a separately authored field-complete specimen with its own fixed
byte count; never copy a production render as the oracle.

For each generated page, compare against one independently selected outcome:

- Progress: kept≥1 and exactly the expected leading prefix; next offset is
  start+kept if continuation is warranted. Full traversal/no gaps follows only
  if this progress condition holds at every nonterminal page on unchanged state.
- Zero progress with remaining candidates: kept=0, budget.cut=true,
  completeness=cut_by_budget, no continuation and hints absent after the
  hints-off retry, exact final count/fit, with no recut or skipped leading hit.
  Accumulated refs equal the independently expected prefix up to this stopping
  page, not the whole ranked list; the test proves the stated stopping reason.
- Mandatory metadata overflow: typed bad_argument; no oversized successful
  page, fabricated advancing handle or lost-error-field shortcut.
- Truly empty ranked input: kept=0, cut=false, empty/empty_not_ready as applicable;
  optional hints follow independent final fit and never manufacture continuation.

Keep generated small-budget coverage distinct from a second traversal property
whose oracle verifies whole-hit fit for every nonterminal page. The zero-progress
regression must genuinely fail at its own RED SHA for its changed visible
behavior (for example old-format incorrectly retained hit/count or final fit),
not merely fail the obsolete unconditional all-results assertion. Commit the
reviewed property-contract adaptation and independent expected cases before
GREEN; test expectations remain read-only during the fix. If existing behavior
already satisfies a case, retain it as a control rather than claiming new RED.
No historical fixture generation/extra ignored fields or EP03 PINNED change is
allowed by this correction.

## Historical oracle preservation and exact adaptation

`tests/fixtures/hints_off_baseline.jsonl` is immutable. Its authored full raw
JSON-RPC bytes include old human content and old budget sizes; merely selecting
structured cannot preserve those bytes under the approved JSON text rule.
Do not regenerate it, delete the semantic regression or silently loosen every
field. Adapt `hints_baseline.rs` only as this reviewed contract permits:

- Parse the authored historical structured payloads; compare unaffected fields
  recursively, rejecting extra/missing fields, changes to types/values/order and
  accidental state/verdict deletion in structured. Preserve raw fixture bytes.
- Exact changed-path allowlist is `result.content[0].text` and
  `result.structuredContent.budget.used_tokens`. budget.max_tokens remains exact.
  Budget-constrained search additionally allows only trailing result removal,
  consequent budget.cut/completeness/response_id/continuation changes; validate
  each against kept count, canonical args and independent final-byte accounting.
  Default-false canonical omission preserves historical hashes when kept refs
  and offsets are identical; do not blanket-ignore handles.
- Budget-constrained fetch additionally allows only UTF-8 body-prefix length
  and its body_cut/budget.cut; compare full authored fixture-builder body and
  exact item ref/kind/error/order. Expand allows body prefix/cut, trailing target
  removal and consequent bound/budget.cut only where its budget actually binds.
  No arbitrary field/path, score, explanation, degradation, ref, kind, source,
  error or unchanged-page order is masked.
- At ample budget, compare the full retained historical semantic payload, apart
  from budget accounting, and require identical ordered refs/values. For a
  constrained page, independently assert prefix/continuation correctness rather
  than permitting arbitrary subsets; a page that exceeds the authored reference
  coverage requires explicit additional human-intent oracle review.
- Within each current format, config hints=false versus per-call hints=false
  must remain byte-identical, including final budget/handles. Hints on preserves
  ample-budget selected semantics; its extra bytes/cuts follow the hints ADR.
  Both formats retain deterministic pinned two-process sessions.
- Add explicit comparator negative controls: wrong score/source/error/ordered ref
  or added unknown field is rejected; false budget/handle/count is rejected;
  cutting a middle hit rather than a suffix is rejected. These guard the oracle
  adaptation, not a new product behavior or a passing-on-arrival RED exception.

Independent Astra design review accepted this exact allowlist in principle;
parent confirms terminal60 design PASS after tiny-budget FIX1. Preserve it during
source work; a material allowlist/oracle change requires renewed independent review. The EP03 healthy-byte PINNED golden is a separate engine/native oracle;
leave it unchanged. Approved formatting is not a waiver for other byte contracts.

## Original RED/GREEN dependency order

For every changed behavior: add tests against existing public APIs/JSON, run
selected test until its contract assertion genuinely fails, commit that RED with
raw assertion/command/exit evidence, then separate GREEN; tests are read-only
while fixing. No
compile-error RED, new helper missing at compile time, “not RED” coverage commit
or pass-on-arrival exception counts. New integration assertions must be staged
before the final still-missing behavior they exercise, not counted as separate
RED after implementation. Do not reopen product scope just to manufacture RED.

1. F01a strict JSON result_format acceptance through from_json_str (no new-field
   reference at RED), then separate GREEN introducing the public String field.
   F01b then constructs that now-existing field with invalid values and requires
   Adapter::new validation at its own runtime RED before separate GREEN. Stage
   JSON null/type/unknown/case controls before F01a fix; passing controls are not
   claimed as new RED. Default text is observed through F02/F03 public calls.
2. F02 discovery output schemas depend only on server format, get_tool/list
   agree, same-format content/config independent; separate GREEN.
3. F03 list_stores compact lines/format envelope and quoted stores.
4. F04 search compact fields/explanations/rounding/highlights/hosted caveat.
5. F05 orient readiness/affected/runtime text and approved omissions.
6. F06 fetch raw framing/order/per-item typed details and UTF-8 cuts.
7. F07 expand line grammar/order/body-cut/bound.
8. F08 use_store/feedback complete compact representations and JSON structured
   serialization. Split these into distinct RED/GREEN units if fixes are distinct.
9. F09 tool errors in each format with current candidates/detail/busy; final
   all-tool/error field census/composition test is staged while this still fails.
10. F10 final counting/convergence/format-specific cuts, hints fallback,
    independent zero-progress/metadata-overflow outcomes and sufficient body-cut
    fetch budget; adapt the existing small-budget property before GREEN as above.
    Split each independently changed behavior
    into its own observed RED/GREEN rather than one universal budget GREEN.
11. F11 search existence flags and canonical effective view with real selections.
12. F12 remove text-filter refusal and forward complete direct SDK filters;
    keep each newly claimed filter behavior's observed failing RED before fix.
13. F13 projected metadata/invalid attr/nohit and provider-free route composition;
    if forward-all-fields GREEN would already pass these, stage them as part of
    the corresponding pre-fix F12 RED, never later “new RED” passing coverage.
14. F14 flags/filters continuation identity/override and both-format composition;
    complete gap-free traversal is conditioned on every nonterminal page fitting
    at least one whole leading hit. Preserve arbitrary small-budget independent
    zero-progress/prefix/refusal coverage separately. Stage each changed behavior before its corresponding F10/F11/F12 fix where
    applicable. Preserve inherited paging/stale/queue controls as regression
    verification, not new behavior commits claiming RED.

This sequence is a dependency plan, not permission for a big-bang GREEN. Exact
splits/selector names are recorded with observed failures on the completed10
baseline; if a proposed behavior already works, do not change product solely to
force failure. Record it as an existing control and implement only real deltas.

## Test-by-test implementation register

Prospective selectors below name independent assertions, not executed tests or
promised failures. Run the exact selector on its own RED SHA; record command,
assertion, actual exit and0pass/1fail where applicable before separate GREEN.
A test passing on arrival is a retained control, never an artificial RED. Every
row with a distinct missing behavior gets its own RED/GREEN pair; stage related
matrix/composition assertions before that same fix can make them pass. Do not
break working behavior, add a missing-helper compile failure or claim later
passing coverage as RED. Freeze each tested oracle during its repair.

| Unit / prospective selector | Independent runtime assertion at RED; bounded GREEN |
| --- | --- |
| F01a `result_format_json_accepts_only_the_two_values` | Existing from_json_str accepts valid text/structured; baseline rejects unknown key. Add strict key/parser/default String only; wrong/null/type/case controls stage here. |
| F01b `embedded_result_format_is_revalidated` | After F01a introduces the public field, invalid hand-built Config must fail Adapter::new naming result_format. Add embedded validate; no missing-field RED. |
| F02 `default_discovery_has_no_output_schema` | Omitted format lists seven unchanged names/inputs with no outputSchema; get/list agree. Explicit structured retains schemas. Clear only text route schemas; per-connection mixed-format harness prevents global-cache bleed. |
| F03 `list_stores_text_has_one_block_and_quoted_names` | Default result has one text/no structuredContent and authored store/default/readiness rows. Wire-envelope branch first; no other tool renderer is completed here. |
| F04a `search_compact_rows_keep_quotes_and_scores` | Raw search rows parse hostile refs/kinds/src exactly and show independently rounded3dec SDK score/CE, ordered IDs and CR/LF-flattened snippet bytes. Implement search grammar only. |
| F04b `search_text_retains_diagnostics_and_highlights` | Requested trace/per-hit JSON and absent/empty/nonempty highlights survive with valid byte coordinates. Implement these missing fields; raw typed scores stay exact. |
| F04c `hosted_continuation_reports_best_effort` | Hosted page includes continuation_exact=false when typed output does. Add caveat rendering; no state-read/drift authority. |
| F05 `orient_text_keeps_real_readiness_fields` | Authored projection/embedding/runtime/affected-kind facts survive; only approved unavailable placeholders omitted. Add orient renderer without B2 state/verdict invention. |
| F06a `fetch_frames_exact_utf8_in_request_order` | Hand-authored header-like/CRLF/NUL-if-valid/astral/trailing-LF bodies recover by bytes; duplicate/missing/malformed refs preserve one outcome each. Implement raw framing, no body normalization. |
| F06b `fetch_item_errors_keep_existing_details` | Missing/unsupported/ambiguous item error has exact class/code/message/remedy/candidates/detail, where real typed output has them. Implement per-item detail rendering, not new errors. |
| F07 `expand_has_one_ordered_line_per_target` | Hand-authored graph with hostile refs/edge/kind and CR/LF/pipes retains hop/order/body/cut/bound. Implement expand grammar, not graph semantics. |
| F08a `use_store_text_quotes_default_name` | Session switch returns exact quoted store and subsequent default action. Implement use_store representation only. |
| F08b `feedback_text_keeps_envelope_and_reason` | Real accepted/rejected feedback retains store/epoch/time/status/reason/degradation/budget. Implement feedback representation only. |
| F09a `structured_text_is_exact_final_payload_json` | Representative completed tool's explicit structured text equals serialization of final structuredContent and schema validates. Add common serialization; stage all seven tool/error matrix assertions before this fix. |
| F09b `text_tool_error_keeps_all_typed_fields` | Wire error isError=true/text-only and complete candidates/detail/busy including empty-vs-absent. Add selected error renderer; final field census staged before last missing field fix. |
| F10a `text_count_measures_final_utf8` | Independently measured final text count includes quotes/hints/framing/count digits and fits max. Add≤4-render convergence, staging Unicode/digit-boundary properties first. |
| F10b `structured_fit_counts_both_final_copies` | Hand-authored final JSON proves both-copy fit and displayed used exclusion; old human text cannot satisfy it. Add converged structured accounting and exact transmitted S(u). |
| F10c `search_keeps_whole_prefix_and_retries_without_hints` | Authored independently sized leading prefixes choose expected kept count/handle; sole fit hit survives hints-off retry. Fix only whole-hit shaping/fallback; no snippet recut/skip. |
| F10d `tiny_budget_zero_progress_is_truthful` | Independent specimen predicts zero-hit cut_by_budget/no continuation/no hints/final fit or mandatory-overflow bad_argument. Before GREEN adapt arbitrary256..6000 property with independent prefix oracle; no filtering or raised minimum. |
| F10e `fetch_cut_hint_budget_returns_uncut_records` | Independent complete target-format fetch size is sufficient on unchanged state; constrained bodies remain UTF-8 prefixes, all outcomes retained or typed refusal. Fix body shaping and target-format suggested budget at separate pairs if independently missing. |
| F10f `expand_budget_cuts_bodies_before_suffix_targets` | Independent authored graph sizing predicts prefixes then trailing-target drop/bound=budget; mandatory metadata overflow refuses. Add only missing format-specific cut logic. |
| F11a `search_superseded_view_and_handle_are_effective` | Raw include_superseded changes exact authored old/current selection and canonical identity; false/absent hashes match. Add this field/view/hash/override rules. |
| F11b `search_inactive_view_and_handle_are_effective` | Raw include_inactive and both flags choose authored pending/deleted/current nodes/edges at fixed bounds, SDK exact control. Add inactive path; invalid types and continuation false override assertions stage before fix. |
| F12 `direct_text_forwards_the_closed_filter` | Explicit/config text accepts each five fields/conjunctions and returns authored expected IDs, >100 nonmatches pre-cap target, AND attrs excluding edges, empty-filter control. All cases run to refusal at this RED before one narrow complete SearchFilter forwarding fix; do not manufacture per-field partial production filtering. |
| F13 matrix staged with F12 | Unknown/nonfilterable even nohit yields InvalidFilter/bad_argument; actual projection empty-status/inclusive-time positive vs missing-row/body-JSON negatives; None-provider direct dispatch once/hybrid zero. Stage before F12 GREEN; controls already passing are not separate RED claims. |
| F14 matrix staged with F10/F11/F12 | Every filter/true flag affects identity; key order/default false does not; overrides refuse; same-state progress-capable pages traverse100, arbitrary budgets independently stop/refuse. Hosted writes preserve eligibility/caveat without no-gap promise; inherited stale/reopen rules remain controls. |

Pure parsing/round-trip/accounting properties accompany the corresponding wire
behavior before GREEN. Comparator adaptation/negative controls are reviewed
oracle maintenance, not fabricated product REDs; historical JSONL and EP03
PINNED bytes remain immutable. If a row is already satisfied after an earlier
coherent fix, it must have been staged before that fix or remain honestly an
existing control. This register adds no test-only policy exception.

## Verification after source work

Strict closed10 preflight and fresh sole-writer checkout creation were completed
by parent before this reconciliation. Fresh Astra-medium plan review precedes
new tests/code. Wait for the parent-allocated native window before provisioning
or execution. Run the canonical ownership check before installers; create own
venv/modules and source-bound noneditable native through canonical helpers.
Never execute sibling mutable environments or native artifacts. Canonical test
children use this checkout's nonce-qualified candidate; installed production
checks clear PYTHONPATH. Preserve real failures/interruptions/skips and strict
ptrace requirements. Accepted earlier evidence is reusable without reexecution;
new source still requires applicable qualification and independent final review.

Focused commands use the owned checkout and selected features, for example
`cargo test -p fathomdb-mcp --test config`, `--test schema`, `--test search`,
`--test fetch_expand`, `--test hints`, new format/filter targets, then retained
protocol/determinism/feedback/admission/lifecycle and relevant engine/SDK suites.
Each is preceded by applicable lint/typecheck; raw diagnostics/exits retained.
Run format-specific stdio/default config and explicit structured checks; no test
helper may inject structured into tests claiming to verify default text.
Existing old human-text assertions move to the compact branch or become exact
structured JSON-text assertions under the reviewed contract, before related
GREEN. Never change their intent just to fit current output.

Run scoped Markdown/interface/state validators as applicable; full agent verify
once at final clean source, with checkout-owned venv/tools and strict ptrace
executor. Missing preflight stops source qualification. Both-format official
conformance uses selected pinned owned tools and records current applicability;
the old scratch report is not current evidence, nor is bridge HTTP behavior a
stdio pass. Block1 exit owns composed feature/performance/smoke/remeasure after
selected20–70; final combined160 and90's80+exit prerequisites stay distinct.

## Integrated feedback prerequisite

Integrated50 implements seq-375's Unique/Ambiguous fold and atomic named-label
refusal after acquisition/final epoch validation. Existing feedback tests cover
mixed sets, unknown membership, unmentioned ambiguity, telemetry/text precedence,
acquisition/reopen races and compact refusal. They are retained controls, not
new60 REDs. F11 adds composition using newly reachable historical revisions;
stage that composition before the flag fix, as above. .30 richer occurrence
labels remain separate and add no .29 reference ABI.
