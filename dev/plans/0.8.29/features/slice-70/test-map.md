---
title: Slice 70 — source assertions and per-behavior TDD map
status: PROPOSED
target_release: 0.8.29
slice: 70
---

# Slice 70 test map

This maps [R70-1–8 / A70-1–8](requirements.md) to existing source assertions
and the reviewed staged tests. This specification was frozen before execution;
[status](status.md) and the [execution map](execution-map.md) now record observed
RED/GREEN evidence. Candidate descriptions below retain their planning meaning;
only the actual receipts/counts establish executed coverage.
[Execution map](execution-map.md) is the ordered staging contract;
[oracle inventory](oracle-successors.md), [strict historical amendment](historical-oracle-successor.md)
and [pinned illustrative diff](oracle-successors.proposed.diff) are attached test
specifications, not optional scratch addenda. Inputs passed parent-owned Astra
review; consolidation and actual integrated-baseline rechecks passed.

## Actual source-entry reconciliation

Actual entry589467 and integrated qualified partial60 subset5271 are recorded
in the [plan](plan.md#actual-source-entry-reconciliation) and [status](status.md).
All22 historical product/test pins match; approved scope, exact oracles and
runtime RED ordering stay unchanged. The same independent Astra baseline-fit
review passed with zero material findings before implementation; see the
[review log](review-log.md). Executed pairs and outstanding qualification are
recorded in [status](status.md) and the [execution map](execution-map.md).

## Existing controls inspected at entry589467

| Source body, relative to repository | Actual assertion / qualification limit |
| --- | --- |
| engine `tests/ep03_degradation_bytes.rs::degradations_leave_results_and_soft_fallback_byte_identical` | ByteSum real-store healthy search compares projection_cursor/soft_fallback/results Debug bytes to pre-field PINNED and requires empty degradations. Preserve literal bytes; never regenerate/normalize/weaken them. |
| engine `tests/ep03_embedder_overloaded.rs` | Real one-worker gate proves full queue refusal and queued-no-start expiry, lexical hits/no vector hits; frozen/evidence and both expand routes propagate overload and healthy absence. Current holder assertion requires started-timeout list empty; change that approved expectation in the timeout RED, not after GREEN. |
| py/napi `src/read_search.rs::degradation_tests` | Real store plus held worker produces queued expiry; Rust native conversion asserts exact vector/embedder_overloaded and healthy None for search and expand. This is already engine/native overload coverage, not a wrapper mock. |
| Python `tests/test_ep03_degradations.py`; TS `tests/ep03-degradations.test.ts` | Stand-in native results assert wrapper search/expand mapping and absent/null/empty conventions. These do not drive native dispatch. Preserve this distinct layer. |
| engine `tests/slice90_foreground_dispatch.rs` | Direct timeout/errors/panic/overload/close, ordinary timeout lexical hit, ordinary/frozen provider-error fallback, ordinary panic unwind, frozen panic Storage and snapshot-before/concurrent-write-after controls. Existing timeout test does not assert a timeout label. |
| engine `tests/slice30_cuda_error_kinds.rs` | All three exact typed CUDA query errors survive ordinary and frozen search. Non-CUDA provider failure still succeeds sparse; projection retry stable codes and direct embed errors are separate controls. |
| engine `tests/slice30_embedding_readiness.rs`; Python/TS slice30 readiness suites | Real pending body-edge yields blocked current report and immediate typed drain failure; no terminal dispatch when provider absent; refused open preserves pending work. Installed Python/TS currently assert unsupported caller remedy and body privacy. Those exact expectations need approved successor RED. |
| engine `tests/snippet_selector.rs`; `tests/snippet_search.rs` | Prior inspection of selector cases cover defaults/ranges, byte assembly, NFD widening and grapheme edges; search entry/range/sidecar bodies traced. Existing t4 properties are inherited from completed5 E6, not freshly run here. No assertion that all large suite bodies were reread. |
| py `src/snippet.rs::parity_fixture_tests`; Python/TS EP17 files | Prior inspection: shared authored fixture expected objects and installed search/text/frozen options/ranges/byte slicing are present. Python tests also cover unbounded integers; TS tests reject malformed and misspelled options. Passing fixtures are existing controls, not new RED. |
| facade `tests/reexports.rs`; SDK `src/lib.rs` absence doctests | Facade census omits six types; SDK already exports them and closes engine/provider/recovery bypasses. A facade export compile diagnostic proves only type reachability, never a runtime behavior RED. |
| engine `tests/slice70_runtime_policy.rs`; embedder `src/device_policy.rs::resolve_device` | CPU-only forced cuda:0 open is expected typed CudaNotCompiled, never implicit CPU. Source Auto policy permits limited CPU fallback; private pool kinds error for both auto/forced. Actual current artifact/venue remains to qualify. |

## Ordered behavior changes and independent oracles

Each behavior-changing row must have its own observed runtime RED commit/stage
before its separate minimal GREEN. The already approved facade export pair is
compile reachability, kept explicitly separate. Record command, actual exit/assertion and SHA in commit
body/receipts; audit RED at its own SHA. No missing-helper compile failure may
substitute for runtime RED, no passing-on-arrival exception or broad core GREEN.
Keep tests read-only during fix-to-spec. On a second same-mode failure reread
test/authority/memory and revise the approach before any third attempt.

| Step / requirements | Proposed test seam and intended behavioral RED → GREEN |
| --- | --- |
| T70-1 / R70-4 | Change installed Python/TS readiness/drain exact-remedy expectations to the approved two values; add central real-store readiness+drain equality assertion for graph-edge and ordinary vector pending kinds. Observe old unsupported word fail, then remove only that word centrally. Preserve blocked payload/privacy/pending-state and semantic-provider controls. |
| T70-2 / R70-1,7,8 | Real None-provider seeded store through ordinary/frozen/evidence/expand asserts exact `reason.as_str()==embedder_absent` using existing entrypoints, expected lexical logical IDs and no Vector branch. Stage all real engine/native/installed/MCP assertions and the five exact oracle-owner successors before the shared absence GREEN, using existing as_str and arm APIs. Enum/union implementation follows observed runtime RED; native assertions never arrive after it. Compare explicit text-only healthy control: no query-provider attempt/vector label. |
| T70-3 / R70-1,2,7,8 | Arm a healthy-then-held real provider with reached channel, short absolute deadline and bounded release/join cleanup. Require observed started_ns, exact `embedder_timeout` and seeded text hits across ordinary/frozen/evidence/both expand; held-slot accounting and healthy subsequent call remain. Update old holder expectation in this RED. Contrast separately observed queued-no-start overload. Provider-returned Timeout/Failed/invalid-vector controls stay sparse without new labels. |
| T70-4 / R70-3,2 | Use existing search_query_vector directly in an embedding.rs cfg(test) unit on its real Engine.embed_dispatch Arc. A channel-held request and actual Engine.close yield the helper's old Ok(None) versus required Err(Closing), before any reader dispatch. No factoring, manufactured DispatchError, missing-helper compile failure or later-reader refusal. Retain existing public ordinary/frozen shutdown and worker-retention controls and deadline-wins semantics. Non-close Cancelled is deferred unchanged; remove its proposed new mapping RED. A private cancellation fixture cannot establish public reachability. |
| T70-5 / R70-5 | Add external facade consumer that types actual hit.id and snippet fields with all six approved exports; baseline should fail named unresolved imports. This is the API-export contract's compile RED only; keep it separate from T70-1–4 runtime evidence. Then add six re-exports; run existing SDK consumer and absence proofs unchanged. |
| T70-6 / R70-7 | New words map through real-engine PyO3/NAPI search and expand conversions; stage these assertions within each reason’s RED before engine GREEN, observing missing runtime reason rather than unavailable enum imports. Installed public Python/TS None-provider cases assert absence word and omission on text-only in the absence RED. Existing generic wrapper fixtures/real overload/healthy native controls are rerun; do not append passing-on-arrival new-word stand-ins. Public union type checks prove vocabulary only. No production caller-provider admission. |
| T70-7 / R70-7 | After actual60 integration, stage real MCP hybrid None-provider assertions under both formats in the absence RED: exact canonical reason list and first-entry degraded hint; text mode has no vector reason. Before reason GREEN, execute separate T70-7a absent and T70-7b timeout hint translation pairs through existing hints::degraded with literal Degradation strings, asserting exact serialized hint. Baseline returns None, a runtime failure without new variant imports. Stage schema-validity and translation runtime RED separately per word before adding engine/output HintCause and parse/from_engine mappings. These mapping fixtures do not claim real MCP timeout fault. Retain60 budget/framing/paging controls. |
| T70-8 / R70-6,8 | Correct only proven closed-interval docs. Rerun existing shared fixture, options, Unicode/grapheme properties and installed-native EP17 cases. Additional selector tests require a demonstrated behavior defect and observed RED; do not add duplicate passing tests. Preserve exact PINNED and original authored shared fixture bytes. |

Engine test seam providers use TestEmbedderChoice only; production Default/None
remains. No mock database. Expectations derive from fixed seeded logical IDs,
manual rank contributions and literal authored byte ranges, never freshly
captured output. Selective examples include `é`, `cafe\u0301`, astral emoji and
ZWJ sequences; calculate expected byte endpoints before running the selector.

## Forced versus automatic and installed controls

Use current CPU-only default-embedder artifact in child processes for forced
cuda:0 typed refusal; compare explicit cpu and auto permitted CPU success with
actual device report and fresh installed open/search/close/exit. Private-pool
typed failures retain existing unit/native converters; these source controls do
not claim a real GPU OOM/reset or cross-platform campaign. No driver/toolkit edit,
GPU fault injection, K620 execution, model download or paid venue is allocated.
If built-in success requires unavailable approved local weights, record that
qualification gap; provider-free success cannot substitute for built-in CPU.

Installed Python and Node tests use checkout-owned artifacts and runtime paths,
features and SHA/digests recorded after10. None-provider absence/remedy/Unicode
are production-reachable. Timeout uses real engine→Rust native conversion plus
existing generic wrapper mapping unless an already sanctioned installed test seam is found;
this is not installed end-to-end timeout proof. No public injection is added to
close that limit. Report skipped/unreachable cells explicitly to final review.

## Validation order after source dispatch

Use exact checked-in scripts/tools on the source checkout. Scoped lint, then
typecheck, then unit/native/installed tests; short-circuit failures and retain raw
diagnostics. Candidate Rust selectors: engine `--features test-hooks --test
ep03_embedder_overloaded`, `--test ep03_degradation_bytes`,
`--test slice90_foreground_dispatch`, `--test slice30_cuda_error_kinds`,
`--test slice30_embedding_readiness`, snippet selectors/search, new70 suite;
py/napi `--lib` selected native modules; facade reexports and SDK behavior/surface.
Actual completed10 helper options are --check-ownership, --provision and --build.
In70's parent-created checkout, run the nonmutating ownership check before any
mutators, --provision for owned setup and --build for source-bound wheel rebuilds.
Validate .cache/agent/python-native.json source/lock/features/interpreter/origin/
digests. Reread any actually integrated20 changes, final manifests and actual base
before executing commands;10 closure alone supplies no70 environment or artifact.

Python/TS selectors: existing EP03, readiness, EP17 plus new70 cases. Canonical
Python tests and their children must import the exact sanctioned test-hooks
source candidate through the actual agent-test nonce/source/digest gate and
checkout-scoped PYTHONPATH; record child path/digest/nonce where children matter.
Separate installed-production absence/remedy/Unicode/lifecycle controls run from
an external cwd with PYTHONPATH cleared and recorded site-packages/native digest.
Production pyproject features and the test-hooks candidate stay distinct; never
substitute has-hook presence for identity. Record selected/executed/skipped counts.
Run affected MCP search/hints/schema/format/budget suites after60 source lands.
Use scoped Markdown/interfaces/surface-pin checks. On a final clean code tree run
required agent-verify once; missing owned setup stops preflight. Fresh independent
Astra final review samples actual RED evidence and verification limits. Block1
follow-up exit and combined160 gates remain separate, not claimed by this map.

Completed10 evidence is input only. At the planning freeze no70 environment had
been provisioned; current owned setup/execution is recorded in [status](status.md).
Actual104dcbc passed fresh Astra implementation review and closed at d7d928;
214 registrations/18 behavior pairs and assembled qualification retain original
failures, affected reruns and skips. There is no uninterrupted full-green claim
or new70 RED exception. Actual new checkout capacity, exact tool/module ownership
and source/native identity must be established independently; never reuse10's
venv, node_modules, wheel, native module or mutable Cargo target to qualify70.
Actual20 Linux subset657edeb76 is integrated at parent2f6913c1; its evidence-only
hook repairs/census and original failures/affected reruns/skips were reread.
20 A3/A5 policy and Windows A6 remain held.70 hard dependency stays closed10.
At the planning freeze60 owned the native window and parent2f6913c1 was a
read-only witness. Actual integrated60 was subsequently reread at assigned589467;
parent returned the sole native window to70 after its independent remedy RED audit.
No simultaneous parent native process or borrowed70 environment is claimed.

## Exact genuine Closing RED on existing APIs

Candidate placement: `embedding.rs` cfg(test) unit module, e.g.
`query_helper_returns_closing_when_its_engine_closes`. It accesses existing
private Engine fields from a descendant module, so it needs no production
signature/helper and no new Engine field. Use existing test-provider opener.

1. Open a real temporary database with one healthy provider worker and a long
   dispatch deadline (e.g.5s). Arm the provider only after successful open so
   setup/probe calls cannot consume the query gate. Hold only a unique query
   string; provider sends a reached notification before bounded release wait.
2. Clone `engine.embed_dispatch` into the helper thread. Call existing
   `search_query_vector(&dispatch, query, &mut Vec::new())`, report the entire
   Result and reason words over a channel. Capture the outcome before assertions;
   don't construct DispatchError::Closing, cancel a reply or close only a bare
   dispatcher. The tested request must already have reached the actual provider.
3. Run `engine.close()` in its own thread. Real close calls dispatcher.close,
   waking that helper waiter with actual Closing. Receive helper outcome with
   a deadline comfortably shorter than the5s dispatch deadline; require
   `Err(EngineError::Closing)` and empty degradations. Baseline is `Ok(None)`
   with empty degradations: this is the sole intended behavioral RED.
4. Observe outer close separately: it must remain pending while the provider is
   held, then finish after release. Do not mistake close completion, drain's
   refusal or a subsequent public search error for the helper result. Existing
   close/direct/public reader controls remain preservation evidence.
5. Before any assert/panic can run, install a cleanup owner containing release
   sender and all thread handles. On normal return, recv timeout or panic it
   sends release, waits for completion notifications and joins both threads.
   Provider has its own finite rescue timeout. Keep a short test-only existing
   dispatcher drain budget if needed to bound failure cleanup; don't change
   production timeout or shutdown arbitration. Collect both outcomes first,
   release and join, then assert. A cleanup timeout is a harness failure, never
   an alternative product RED. No blocked provider or detached test thread.
6. Stage/commit and actually execute this test at unchanged mapping. Only after
   its runtime assertion fails, add `Err(DispatchError::Closing) =>
   Err(EngineError::Closing)` in existing search_query_vector, preserving typed
   CUDA match, panic and all other outcomes. Run the unchanged exact selector.
   Actual runtime logs/selected counts and clean RED/GREEN SHAs await source phase.

No new public ordinary/frozen close regression which passes before the fix is
added as if it were the helper RED. Public/frozen propagation is assessed with
existing controls plus the common Result boundary: frozen maps its helper error
through SearchReaderError::Evidence; ordinary uses `?` before reader dispatch.

## Practical execution order and oracle constraints

Follow the linked [execution map](execution-map.md) in order: supported-remedy
pair; absent hint/schema pair; timeout hint/schema pair; complete absence pair
with all supported native/installed controls and [exact60 successors](oracle-successors.md)
staged before shared GREEN; reached started-timeout pair; actual-close/helper
pair; separate facade compile-reachability pair. The exact Closing sequence
above remains authoritative. Docs-only interval corrections reuse existing tests.
No missing helper/enum compile error is runtime RED, and no late generic copying
fixture or artificial passing coverage pair is added.

Candidate60 source/test pins at307c remain unchanged at7038; non-product packaging
paths are recorded in source-pins.json. Full actual integrated60 hash/diff reread
precedes source GO;60 complete closure is not an extra hard prerequisite.
Text/direct-filter/manual controls remain unchanged. Real70 RED receipts and
scoped artifacts now exist as [status](status.md) records; final source-bound
qualification and fresh independent review remain separate.

Human-intent oracles stay exact: EP03 PINNED literal bytes and shared authored
snippet fixture are untouched; seeded logical identities, ranking contributions
and byte endpoints are computed before executing. Only reviewed reason metadata
changes. Existing rank/eligibility/snapshot/CUDA/frozen-Storage versus ordinary
unwind assertions retain meaning. No threshold/verdict/default/pin or schema37
change, query-scaffold deletion or migration is included. Slice120 owns major-
aware CUDA artifact guidance; current12-only advice is not a universal new oracle.

Release trace remains R29-REL-A/K → R29-3P-12/D29-SDK → O3-12 → R70-1–8/A70-1–8.
REQ-052/AC-056 govern exact installed artifacts without pretending70 proves all
release targets; REQ-053/AC-074/079 govern exports/SDK boundaries; AC-068a/b and
AC-069 preserve FFI string validation/privacy. No new canonical AC is minted.
Block1 combined exit and160 remain separate; no generic extra gate or broad
rerun is introduced by this reconciliation.
