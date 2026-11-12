# Slice 100 exact coverage and gap census

Current read-only census baseline:
`1099978cd7d7aef6e745cfc850ddb8f9b57050d3`, branch
`llm/0.8.29-s100-continuation`. The initial ad102de and prior actual20 at2f6913c
are historical. [Design](design.md#current-parent-delta-and-evidence-validity)
records current qualified10/20/60/70/30-P and selective changed-input attribution.
The task `receipts/source-census.json` records22 unchanged selected file hashes,
changed engine paths and retained actual20 log identity. Entire-src/Cargo.lock
equality from the old proposal does not carry forward. No product test ran here.

Tests below are in `src/rust/crates/fathomdb-engine/tests/`; modules are in its
`src/`. Current complete bodies were reread for82 recovery/physical,83 availability/
routes,85 crash/physical/scope and95 visibility; selected complete raw-WAL/BUSY,
COMMIT/retry property, visibility mapping and orphan-span bodies were also read.
Relevant queue/delete/sanitation/finalizer, worker/open/offline and checkpoint
source was inspected. This is a bounded census, not all-source exhaustiveness.

“Covered” attributes a named existing assertion and its historical execution
limits. C14's eight-control reuse has recorded bounded actual20 Astra PASS;
fresh current proposal review is separate. This table supplies no fresh100 run.
A5 now assesses honest fault-model attribution; the original OR minimum is
addressed by transaction-interior C2/C3, while C13 remains unproven.

| Cell / local AC | Exact existing test / fault position / policy | Asserted post-state and precise disposition |
| --- | --- | --- |
| C1 /A1,A2 | `slice85_crash::actual_process_exits_preserve_atomic_intent_removal_redaction_and_receipt`: `before_queue_commit`, `after_queue_commit`; Explicit. | Real child exit73 checked under5s watchdog; reopen target+survivor visible; before queue no ticket, after queue logical state unchanged; accepted/reused ticket completes. Covered admission COMMIT rollback/survival, not rich projection/history assertions. Source `erasure_queue` before/after owned tx.commit. |
| C2 /A1 | Same85 parent: `before_delete_commit`, `after_delete_commit`;82 `crash_admission_units_and_completion_replay_preserve_scope_and_debt`;82 `dependent_crash_and_pre_repair_closure_without_root_journal_remain_retryable`. | Before-delete hook is inside active IMMEDIATE after deletion+root/activation writes, before commit; exit73 proves active transaction rollback. After commit target absent/root owed; target write fenced; retry preserves survivor.85 explicitly reopens persisted policy and checks logical state. Covered transaction interior, not every physical family at that crash. Source `erasure::{purge_inner,excise_source_inner}`. |
| C3 /A1 |82 crash parent at `before_sanitation_commit`; `crash_after_positive_work_preserves_started_continuation_and_segment_geometry`; Autonomous. | Before commit occurs after actual FTS merge and progress store inside `physical_erasure::transaction`; reopen asserts family0. Positive multi-segment/>64-leaf fixture exits73 after committed unit, asserts started1 and nontrivial geometry; retry retains4 survivors. Covered rollback+positive continuation.82 parent child waits have no watchdog, unlike85; this is a harness limit, not claimed product deadlock. |
| C4 /A1,A6 | `physical_erasure::tests::failed_commit_rolls_back_and_leaves_writer_retryable`; `retry_sequences_preserve_atomic_progress_and_survivors`. | Deferred FK makes actual COMMIT fail after positive insert; transaction returns error/autocommit/row0 and retry succeeds.24-case bool sequences retain root/family0/survivor then complete. Worker deliberately stopped for exact state observation, so this property is NOT autonomous qualification. Covered storage COMMIT/refailure oracle, no process/host fault. |
| C5 /A1,A3 |82 `external_sink_cleanup_before_process_exit_is_idempotently_replayed`: `after_sink_redaction`; Autonomous. | Real exit73 after rename and before durable pending-sink discharge; target ID absent in original file, offline refuses still-owed sink debt, authenticated reattach+retry clears debt. Covered external/durable split.85 crash fixture has no telemetry sink; its redaction hooks alone do not establish this file boundary. |
| C6 /A1,A3 |85 crash parent at `before_redacted_commit`, `after_redacted_commit`, `before_completion_commit`, `after_completion_commit`; Explicit. | Exit73/reopen target absent, survivor present, logical Removed; after completion asserts Complete; retry to Complete all cases. Before redacted hook is inside scrub/root tx; before completion is inside finalizer AFTER successful checkpoint. Covered real transactions and checkpoint→receipt replay window. Parent does NOT assert exact pre-retry intermediate phase/root scrub for every case. |
| C7 /A2 |83 `process_crash_reopen_resumes_without_target_or_followup_engine_calls`; three after-delete/unit/redacted phases; `committed_debt_finishes_on_idle_engine_without_another_call`; Autonomous. | Real exits73; independent SQLite observer waits for no root/nonterminal physical closure without follow-up Engine call, then close and scan main marker. Covered autonomous continuation. Main scan is after close; this one test does not prove live WAL bytes or unchanged rich survivor/history after crash. `open` starts worker only Autonomous; `physical_maintenance` owns continuation. |
| C8 /A3,A4 |82 `complete_erasure_removes_positive_terms_from_every_family_and_preserves_history`, `survivor_only_database_agrees_on_fts_order_and_score_bits`;85 scope `exclusive_complete_covers_dependency_four_fts_families_mapped_vectors_and_survivor_statistics`. | Positive four FTS families/target bytes, exact deletion, survivor FTS rowIDs/history snapshots, independent survivor-only order/score bits;85 additionally dependent rows, positive non-head vector900000, raw vector_default/_vector_rows zero and main/live WAL marker absence. Covered success control across policies; NOT combined abrupt rich-crash proof. |
| C9 /A3,A4 |83 routes `queued_projection_is_drained_before_delete_and_cannot_publish_afterward`; `visibility_admission_tests::vector_mapping_cleanup_remains_active_while_visibility_is_coalesced`; `projection_registry::span_erasure_tests::row_erasure_removes_orphan_span_without_mapping_trigger`. | Actual queued worker holds deletion until release; target canonical/search absent and target cursor spans0 after drain. Separate coalesced tx deletes mapping and directly asserts span0; registry unit proves direct erasure of an orphan span without a mapping trigger. `_fathomdb_vector_spans` is itself registered by write_cursor and included in zero proof.85 rich fixture creates non-head spans but its final loop checks vector/mapping, not explicit span count. Existing direct span proof is covered; direct post-crash projection/span observation remains retained G100-2P. An exhaustive all-history/non-head conjunction is optional, not an established defect/gate. |
| C10 /A2,A3 |85 physical `committed_failure_keeps_global_fence_through_handle_release_and_explicit_reopen`, `pinned_external_reader_keeps_redacted_ticket_fenced_until_actual_truncate`, sibling/global-fence and finalizing-key tests. | After-delete Storage failure retains Removed/global write fence through handle close/reopen, exact retry unlocks writes. External pinned reader returns Finalizing/Removed/WalCheckpoint; survivor reads work; real blocker release permits Complete. Sibling Queued/terminal Complete status reports global fence without changing outcome. Covered explicit policy/refusal/retry, not active-checkpoint crash. |
| C11 /A2,A4 |85 physical `offline_completion_publishes_same_activated_receipt_without_activating_queued_intent`, orphaned-root/unknown-policy/partial-finalization refusal tests;82 malformed sanitation test. | Offline Queued stays Queued/target visible; after activation fault offline publishes same Complete/target absent. Corrupt root/ticket/phase refuses;82 malformed sanitation raw main+WAL unchanged with positive WAL. Covered conservative recovery. Orphaned explicit-root test itself compares main only; do not claim every corrupt case has WAL checksum assertion. |
| C12 /A3,A4 |83 routes `owed_debt_routes_match_independent_survivor_only_stores_before_caps_and_statistics`; `erasure_completeness::{erasure_wal_bytes_absent,erasure_busy_yields_incomplete_not_success}`. | Independent survivor-only controls compare read/list/text/property/frozen/hybrid/graph/evidence across three distributions during owed debt; positive span fixture. Separate AC080 raw main+LIVE-WAL scan with secret/control before close; real reader pin gives typed incomplete. Covered success/refusal/routing; no exhaustive interrupted combination. Existing5s bound is retained; no new product deadline. |
| C13 /A5 | `erasure::checkpoint_erasure_at_rest`, `wal_runtime::wal_checkpoint_truncate_once`, worker `checkpoint`, offline completion. | Existing actual-checkpoint observations are before/after call; BUSY is a real checkpoint refusal. No inspected test/control proves termination in active frame-copy/TRUNCATE work. **G100-1 missing fault-position coverage**, not a product defect. Before-completion crash is after checkpoint, never relabel it inside. |
| C14 /A6 | Historical83/85/95 records; closed10 implementation104dcbc/closure d7d928; accepted bounded actual20 collector6eee2e9/final657edeb inference. | Current1099978 selective22-file hashes match collector6eee for core erasure, named controls and Rust caller; whole src and Cargo.lock differ. Retained log65288B/2390lines SHA256aacbdb427f6a809d34ef046fe38ebebf5b8e01256f21e32a2d5e338d53bbd2a2;1611–1629 matches2tests/eightchild launches/1pass/1ignored/0fail. Unfiltered serial workspace feature closure and unchanged parent assertions support historical eight-control execution inference. Prior bounded actual20 Astra PASS already accepted that inference with quiet phase/PID/binary-digest limits. Collector212pass/2fail plus affected corrections is assembled evidence.70 changes query reporting; no whole current-binary equivalence or fresh100 execution is claimed. Fresh v1/v2 semantic and exact v3 design PASS are recorded; parent scoped checks and different fresh final DOC/OUTPUT PASS apply to exact imported11-file diffc0e80622, as bound in review-log. No fresh runtime or proof-predicate qualification follows. |

G100-1: no attributable abrupt exit inside active native checkpoint work. This
is optional under the original OR minimum, not a new mandatory fault gate.

G100-2 has three concrete retained O3-16 predicates: **2H**, unchanged survivor
historical versions after interruption (C2/C3/C7 versus non-crash C8); **2P**,
direct projection/span state after crash/reopen (C2/C6 versus C8/C9); **2W**,
independent main-plus-live-WAL absence after post-crash completion before close
(C7/C6 versus non-crash C8/C12). These remain unmet even though an all-in-one,
every-family/every-hook conjunction is not an established requirement. None is
a demonstrated product defect. [Minimal proposed coverage cells](test-plan.md#minimal-coverage-only-proposal)
name the exact two faults, healthy control, independent witnesses and needed
passing-coverage exception; no test/source/runtime authority is inferred.

Fresh v1 Astra design PASS accepted the bounded documentation census and original
OR interpretation. Exact v2 semantic design PASS covers the concrete coverage
proposal; exact v3 design PASS covers its documentary corrections. Parent scoped
checks and different fresh final DOC/OUTPUT PASS cover the imported stable parent
candidate; [review log](review-log.md) binds exact subjects and limits.
No whole100/D28-09 closure, coverage exception or retained-predicate waiver follows.
