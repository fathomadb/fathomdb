# Slice 20 continuation test map

All runtime commands are future, from a fresh parent-created sole-writer checkout
after this revision passes independent Astra design review and own setup. Record exact SHA/features/argv/log/
exit/nonzero reached counts; preserve compiler diagnostics verbatim. Existing
Linux accepted evidence stays in execution.md; do not repeat it wholesale.

| Obligation | Exact target / proposed selector | Proof and gate |
| --- | --- | --- |
| A3 primary | cargo test -p fathomdb-mcp --test admission a_drain_before_enqueue_returns_the_call_to_open_wait -- --exact | Approved verification-only seq-369; runtime outstanding. Reached old epoch/counters0, real idle/lock release, stale rejection before bridge, blocked reopen, exact ref/new epoch. |
| A3 deadline | Same target, a_pre_enqueue_drain_keeps_the_original_wait_deadline | Proposed; original absolute budget, exact not_ready/reopening, bounded cleanup. |
| A3 cleanup/shutdown | Same target, a_dropped_pre_enqueue_observation_releases_its_hold and shutdown_at_pre_enqueue_refuses_without_engine_work | Proposed; Drop/abort/panic/release_all release owned token and all actors; shutdown no bridge. No new pre-permit cancellation promise. |
| A3 preservation | cargo test -p fathomdb-mcp --test admission; cargo test -p fathomdb-mcp --test lifecycle | Current complete25/18 controls originally qualified; run affected current bodies with ordinary harness parallelism/TOKIO_WORKER_THREADS2. Preserve queue/FIFO/deadline/cancel/running ownership, idle/LRU and current advertised schema validation. |
| A5 fusion | cargo test -p fathomdb-engine --lib fusion::tests::overflow_chunking_fusion_tripwire -- --exact | Approved verification-only seq-369, live30/3/1 and record path in every assertion. |
| A5 embedder | cargo test -p fathomdb-embedder --features default-embedder --lib candle_bge::tests::overflow_chunking_embedder_tripwire -- --exact | Proposed; live name/private512, no model construction/network/GPU. Confirm actual module-qualified listed name before execution. |
| Sensitivity | Same new selectors in disposable isolated controls | Both reviewed stale controls (typed not_ready/draining replacement and phase/epoch predicate bypass) plus five separate actual-constant changes; unchanged oracle fails named assertion. Coverage controls, never product RED. Parent-created trees/own targets, source candidate untouched. |
| Genuine new defect | Exact new deterministic regression at committed RED, then unchanged test at separate GREEN | Capture failure assertion at RED SHA, preserve tests through fix, independent clean RED replay. Missing API/compile failure or wrong introduced expected value is not semantic RED. |
| Test-seam contract | scripts/tests/test_slice140_test_seam_boundary.py plus affected feature-off/check/clippy | Existing default-negative and dev/release-positive consumers preserved; Rust interface note documents new public test-seam ownership/lifetime. |

## Actual Windows selectors

Build exact committed source with:
`cargo test -p fathomdb-engine --features operator,test-hooks --lib --no-run`.
Then use the same prefix without --no-run, each selector below, followed by
`-- --exact --nocapture`. Actual execution remains HELD beyond capacity-only seq-370; guest format and
compiler/helper/test authority plus qualified scripts/bootstrap/binary/supervisor
are separate prerequisites.
Build20min; each selector90s; zero selected tests/timeout is failure.

1. `slice90_close_review_tests::failed_startup_after_accepted_probe_preserves_error_and_cleans_sqlite_profiles`:
   exact injected Io, all READER_POOL_SIZE+1 profiles safely released after
   callbacks/owners/workers, SQLite0/provider0 and admission reacquisition.
2. `slice90_post_probe_real_error_tests::verification_timeout_precedes_later_visibility_fault_and_retains_provider_only`:
   StartedTimeout precedence, provider1 then explicit-release0, SQLite0, admission.
3. `slice90_post_probe_real_error_tests::accepted_probe_still_preserves_later_visibility_error_and_releases_database_owners`:
   exact visibility Io, workers0/SQLite0/admission. Keep its existing same-path lock.
4. `tests::current_opener_holds_lock_before_admission_classification`:
   reached lock before classification, losing opener cannot install33, fresh37.
   Current unbounded Barrier is safely diagnosed by outer process bound first;
   a repair needs actual regression/failure before changing rendezvous.

Source bodies were reread, not executed. Path mismatch/global overwrite remains a
hypothesis until current deterministic runtime reproduction. No old timed-out
name substitutes for the current accepted-probe policy. Existing FIX3/core static
reviews are neither compiler/bootstrap/runtime approval nor Windows evidence.

## Verification ordering and final evidence

Scoped changed-crate lint/check before targeted tests, then affected targets.
Docs-only use scoped Markdown/state validation, no full gate. If source/tests/
executable scripts change, one required final clean stable-tree agent-verify with
owned tools/native receipts; canonical Rust serial gate does not replace explicit
parallel affected targets or feature-on embedder guard. Preserve all failed
attempts/inner skips/strict-security requirements and resume only failed/remaining
checks as needed. No files change during final qualification.

Fresh source-bound production Python/TS lifecycle and canonical test-hook candidate/
child proof are required if their relevant compiled inputs change; otherwise give
exact input equivalence with prior receipt attribution, never borrow native bytes.
Fresh distinct parent-commissioned Astra implementation/results review precedes
integration. CI/external workflows/registry publication remain outside this task.
