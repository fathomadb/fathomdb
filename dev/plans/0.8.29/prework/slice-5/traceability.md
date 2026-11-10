# Slice 5 bounded test adequacy traceability

## Evidence identity and selection

Source audit dated November10,2026 at product
`6edc1155f039f87cd0a37660cb27ae4373bbce71`, included by planning HEAD
`2d5041795f0cbbe49d55d81a2710f09e2043b8a9` dated12:11:30-06:00.
Reviewed3 is completed there with both independent Astra PASS verdicts recorded
in its review-log; requirements, acceptance and allocation were reread after
import. Its22 O3 obligations agree with this join, including existing crash
boundaries and conditional graph strengthening. Planning ancestry changes do not
change the inspected product baseline. No tests, native imports, hardware, performance or artifacts
executed in5. All E rows below are source inspections; none is a current PASS.
Paths are repository-relative unless linked. Uninspected broader suites remain
candidates, never negative proof that coverage is absent.

Default Rust registration is `scripts/test-rust-workspace.sh --serial`:
`cargo test --workspace --quiet --no-fail-fast --jobs 1 -- --test-threads=1`.
It does not enable every feature or run ignored tests directly.
`scripts/check.sh` supplies AGENT_LONG=1; `test-feature-complete.sh` separately
uses its matrix/skip inventory and live prerequisites. Neither automatically
qualifies permitted ignored Orin reset tests. Pytest/Node execution needs the
owned environment/native build and its real selector; a fixture on disk is inert.
An ignored child explicitly spawned by a selected nonignored parent is different
from an ignored test with no selected parent. Future owners record exact command,
features, listed/reached tests, skip/exit counts, artifact and host identity.

## Actual inspected oracles

Complete files were read for E1–E6, E8 and E10. E7 additionally reads complete
in-source degradation modules; E9 reads the complete relevant GPU parent/child
bodies and cfg/ignore controls, not all1608 pool-policy test lines. E0 reads
the complete admission file and the owning registry admission interval.

| Key / actual source | Meaningful assertion and practical limit |
| --- | --- |
| E0 `fathomdb-mcp/tests/admission.rs`; `src/registry.rs` within the same crate | `queued_returns_to_zero_on_timeout_cancel_and_panic` probes queued0 after each cause; `in_flight_is_derived_from_permits` holds permit before guard and checks busy/idle; abandoned/cancelled-running controls keep permit until work returns and later search succeeds. Shutdown expects not_ready/draining. Existing `hold_arrivals` is after `QueueTicket::join`; tests use sleeps around it. No deterministic Ready→pre-enqueue reached/release proof follows. |
| E1 `fathomdb-engine/tests/slice35_eligibility_pretruncation.rs`, `slice70_native_eligibility.rs`, `slice95_visibility_coalescing.rs` |150 excluded+one eligible lexical row survives limit1. Dense200 excluded+one eligible gives exact ID/Vector/no fallback for expired/future/superseded/inactive. SQL placement and EXPLAIN indexed-EAV controls complement result assertions. External canonical mutation advances generation and changes nonce; bad armed state refuses mutation/frozen binding. These prove substrate contracts, not a future cache key or all eligibility paths. |
| E2 `fathomdb-engine/tests/pr_g9_rrf_fusion.rs` | Manual formula accumulation/order, vector representative cursor, dedup-on-body and deterministic real-store repeated results are actual intended oracles. `rrf_graph_arm_expands_ranking` accepts Carol OR any count increase; `carol_without` is diagnostic only. Wrong extra result can pass that one assertion; this proves weak oracle, not a product defect. |
| E3 `fathomdb-mcp/tests/search.rs` | `filters_are_hybrid_only_and_constrain_hybrid_results` requires text bad_argument/hybrid exact doc. `as_of_selects_the_validity_window_and_is_reported` checks lower/interior/exclusive upper/now. Continuations compare100 references/order to same-route ample-budget result; every_response budget helper recomputes ceil((serialized structured+text bytes)/4), including explain/mode variations. Same-route paging cannot independently prove upstream rank/identity; current tests expect placeholders/structured content. Feature-gated mismatch control refuses real default-embedder open until explicit None, not a degraded handle. |
| E4 `fathomdb-mcp/tests/feedback.rs`, `hints_allocation.rs` | Seven concurrent query captures join relevant IDs to the right response; wrong-store/unknown-ref,256-response expiry, reopen epochs and exact output noninterference are asserted. Allocation fixture blocks off-baseline and planning>1+refs; render excess is printed NOTIFY only. These synthetic shaping/allocation controls do not qualify all real requests or future formats. |
| E5 `fathomdb-engine/tests/slice60_store_format.rs` | Unknown endpoints remain NULL through rebuild; écho known end is5, invalid1 publishes no span/vector-row tuple; missing/mismatch/orphan/nonUTF8 live and reopen states refuse. Child batch case asserts batch actually ran, malformed batches trigger scalar work,48 exact byte extents; parent explicitly invokes ignored child. Span insert failure checks four physical/terminal counts0. Storage is already built; no public scored-coverage reporting proof follows. |
| E6 `fathomdb-engine/tests/snippet_selector.rs` | Rule examples assert actual window text/positions and highlights; generated t4 checks determinism, byte cap, grapheme-safe sorted/disjoint windows/spans, exact independently joined text, whole-body behavior and actual shown match. Unicode library boundaries are shared technology, so add explicit boundary expectations for changed routes. Pure selector properties do not prove DB query selection or SDK rendering. |
| E7 `fathomdb-engine/tests/ep03_degradation_bytes.rs`; py/napi `src/read_search.rs::degradation_tests`; `src/python/tests/test_ep03_degradations.py`, `src/ts/tests/ep03-degradations.test.ts` | Rust preserves healthy fixed-result bytes against pre-field PINNED. Both native modules hold a real provider worker, require real overload, convert exact vector/embedder_overloaded and assert healthy absence, including expand. Python/TS stand-ins check wrapper translation and absent/null/empty handling. Preserve all layers; wrapper controls alone are not native/production reachability. Test-only Caller injection does not authorize production numeric providers. Never regenerate PINNED to obtain green. |
| E8 `fathomdb-engine/tests/slice82_physical_erasure.rs`, `slice85_crash.rs`, `slice85_queue.rs` | Positive terms in all four FTS families, exact12/12 removal, survivor/history rowIDs unchanged, zero shadow/main+WAL markers, survivor-only ranking/score bits and busy-checkpoint fence are asserted. Crash parent requires exit73 at eight before/after queue/delete/redaction/completion commits, reopens target/survivor/ticket states, then retries to Complete. Ignored crash_child is invoked explicitly by its normal parent. Queue identity/scope/fence survives reopen. These real crash tests exist; no inside-checkpoint/power-loss proof is implied. |
| E9 `fathomdb-embedder/src/cuda_pool_policy_tests.rs` GPU bodies | Healthy ignored Orin embedder control exercises batches, reserved_high bound and drop→reserved_current=used_current=0. Reset parent requires typed-loss marker, one snapshot, teardown-start, absent torn-down marker and SIGSEGV11; child performs reset and two typed-loss calls before drop. cfg is Linux+aarch64+tegra-pool+embed-cuda; explicit Orin lock/venue is required. Parent `.output()` has no timeout; future safe-exit control needs bounded supervision as well as positive completion. Healthy library drop is not SDK close/exit after fault. |
| E10 `scripts/tests/test_tegra_gpu_witness.py` | Synthetic v2 parser fixtures rederive signed USED delta, exact64MiB control/release, UUID/ordinal, uint64 string context and tagged shape; reject reserved method, stalev1, wrong arithmetic/identity and mixed fields. Strong negative parser oracles are already built. Synthetic values do not establish current hardware allocation, release or installed SDK lifecycle. |
| E11 [reviewed Slice7 mapping](../slice-7/test-mapping.md) | Inherited actual full-body audit: direct verifier fixture is test_agent_verify_markdown_scope.sh plus test_ci_verify_observability.sh; exact tag route is test_tag_cuda_route.sh. Bootstrap, workflow/static, security and release metadata controls retain their distinct limits.5 consumes this audit; it does not rerun or recertify those bodies. |

Source paths E0–E9 use prefix `src/rust/crates/` except explicit Python/TS paths.
The [Slice4 alignment](../slice-4/alignment-matrix.md) supplies source contract
findings with its own review status; it cannot turn E rows into executions.

## Complete release-obligation join

Every row uses N29-X→R29-REL-X→R29-3P-number→same-number O3 oracle in the
reviewed [Slice3 table](../slice-3/acceptance.md). Single delivery owner is below;
design/parity/integration assistance creates no second implementation owner.
Canonical ancestry and conflicts are in [requirements](requirements.md).

| R29-3P / O3 | Need X / delivery owner | Actual evidence, residual and proposed disposition |
| --- | --- | --- |
|01 | D /10 | E11 owned setup/native identity. Reject wrong source/features/digest despite timestamp sentinel; current tooling/native readiness unmeasured. Retain requirement; bounded new receipt/entry-point behavior only. |
|02 | D /10 | Reviewed1 dependency consuming checks are inherited candidates; no fresh dependency execution. Selected Rustls/DOMPurify closure gets its exact changed-boundary controls, no blanket upgrades. |
|03 | D /10 | Reviewed1 exporter/parser inventory is inherited. New supported-subset parser needs expected declarations/tier/exclusion output and loudly failing malformed controls; don't silently ignore syntax. |
|04 | D,K /10 | Reviewed2/6 authority/document inventory. Source37/current12-crate/native setup descriptions are document checks, not runtime proof. Preserve canonical IDs; resolve explicit AC052 authority conflict through9. |
|05 | D /10 | E11 exact .29 tag positive/negative fixture; preserve .28 strict controls and live held/peeled identity duty. Static routing is not dispatch/package proof. |
|06 | A,R /20 | E0 strong accounting retained; add reached/release Ready→enqueue race and no-old-engine/permit/queue/idle witnesses. Hygiene/parallel-warning failures and tripwire-constant selectors need20 verification; assert actual accepted constants, not duplicate literals. Current residual not reproduced by5. |
|07 | S,K /30 | E1/E3 establish functional substrate, not current latency. Measure fixed inputs/current100k and write-interleaved costs; retain failed outcomes and tracked AC083 versus existing hard gates. |
|08 | S /40 | E1 plus new expected IDs across actual old/new snapshots, time/view, rollback/external writes/database/branch. Shared production eligibility comparison alone is inadequate; reject/defer cache if invalidation overhead defeats benefit. |
|09 | S,K /50 | E2/E6 preserve accepted manual ranking/Unicode intent. Successor first; equal-body/different-ID/source/revision/node/edge cases need independent selected IDs/provenance/explanation/seed oracle, plus exact route equivalence. Strengthen graph assertion only if touched. |
|10 | A /60 | E3 current refusal proves successor delta. AC29-60-FTS1–6 select >100 stronger nonmatches+eligible limit1, actual SDK same-view parity, every filter/flags/boundary and no inference. |
|11 | A /60 | E3/E4 actual serialized budget/page controls remain. Both formats/all tools/errors, quoted refs/framing and ample-budget semantic parity need new format-specific controls; no same-route correctness claim. |
|12 | K,A /70 | E7 strong overload/native/healthy controls retained. New bounded reason/precedence and supported remediation use real production reachability plus exact native artifacts; test-only Caller is no product API. |
|13 | A /80 design | E4/E5 and reviewed3 built-hint assertions constrain complete B2 census. Set omission/denominator/view/coordinate/unknown/nonvector rules before90; no calibration or storage rebuild. |
|14 | A /90 | E0/E4 preserve built2/3/6/queue/feedback; add only80-approved omission, accepted-args/offending-ref/default-store residuals in both formats. Requires actual Block1 exit AND approved80. |
|15 | A,K /90 | E5/E6 storage/properties retained. New independent known-head/snippet-window intersections and approved aggregation/coordinates/unknown rules pass every affected SDK and rendering format.80 design,70 parity support. |
|16 | R /100 | E8 already covers eight real commit crashes. Inventory all reached old fault positions before adding only uncovered transaction/checkpoint interruption post-state. Include survivor/history/projection/span/fence/WAL/retry and healthy control; no migration/power-loss claim. |
|17 | R,G /110 | E9 SIGSEGV characterization is existing defect evidence. Reviewed remedy needs positive bounded completed teardown/close/drop/exit with typed loss and allocation lifetime, healthy control and permitted isolated GPU venue. |
|18 | G /120 | E10 existing parser/identity machinery retained. Selected CUDA13 product artifact needs exact toolkit/PTX/driver/library/architecture/distribution identity and real installed behavior; historical library proof alone insufficient. Policy ratification stays explicit. |
|19 | G,K /130 | E9/E10 counters/parser are substrate only. Final selected pins/ABI/UUID/pool/context and SDK open/embed/close/exit need attributable USED release plus frozen soak rule/healthy controls. Selected Tegra/non-Tegra12/13 cells remain separate. |
|20 | G /140 conditional | Qualified baseline remains; only concrete110/120 repair gets fork-local failing-first proof, release closure and pin qualification. No source/fork run here; B/C .30 never gates .29. |
|21 | S,A /150 conditional | E2 graph limit informs only selected study/touched seam. Individual authorization, frozen native/held-out paired rule and caps precede runs; existence of queue/hints is not benefit. Preserve negative/inconclusive results. |
|22 | all /160 | Consume all selected exits/dispositions, E11 delivery and current affected composed/native artifact evidence. AC054 pending finalizer and AC052 same-version conflict stay explicit. Preserve original .28 failures; separate publication authority and fresh registry lifecycle proof. |

This table establishes proposal mapping, not exhaustive tests for every surface.
Known contract questions go to their named owner/9; uninspected paths remain
unqualified. Future exact selector and source/artifact/configuration receipts,
not this table, establish executed evidence.
