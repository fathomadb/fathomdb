---
title: Slice 50 runtime implementation and open proof
status: ACTIVE
slice: 50
target_release: 0.8.30
---

# Slice 50 runtime implementation and open proof

Commissioned by seq-408 from `cd64360a63bf06c7d1bc5f27865cca6d7400eff8`.
Direct Astra-medium passed the [reconciled plan](plan.md) and [design](design.md)
after one consolidated FIX-1. Direct Sol6.2-high passed the source changes
after fixing SQLite connection cleanup, bounding the entire new engine
schedule and strengthening the exact MCP recovery results. No product defect
was demonstrated in these bounded schedules; no public API or product
contract changed. This is not Slice50 closure: two required proofs remain open.

## Implemented and measured scope

| Selected row | Actual result |
| --- | --- |
| MCP admission/cancellation | Serial `cargo test --locked -p fathomdb-mcp --features test-seams --test admission -- --test-threads=1` passed30/30. Cancellation now has its own progress token and actual queued notification, retired pre-enqueue lease and empty bridge history. Two surviving waiters and a subsequent call return exact authored fetch items. Started run-limit returns timeout/deadline and retains in_flight1/abandoned1 until actual return; later exact fetches recover and abandoned resets0. One registry refuses a second busy store with busy/store_switch, then switches successfully. Distinct registries simultaneously hold actual admitted calls on distinct databases and return their own exact records. The Slice10 startup timing lead did not recur in these selected serial runs; broader parallel flakiness is not excluded. |
| Engine dispatch/isolation | Existing baseline foreground scope passed10/10. Final scope passed11/11 with one added bounded real-store schedule: A's worker remains occupied after direct Embedder timeout; queued expiry is Overloaded without a replacement call; B writes/reads exact Unicode state and embeds successfully; releasing A allows an exact healthy vector. Existing saturation, error/panic reuse, sparse fallback and frozen-reader cases remain intact. |
| Close/alias/reopen | `--features operator,test-hooks --lib slice20_open_hook_tests` passed2/2, close-review passed2/2 and concurrent-close passed3/3. The targeted canonical/alias startup hook leaves a sibling's open, committed read and later write intact. Existing observation proves SQLite quiescence precedes one shared provider deadline across concurrent/repeated close/Drop, provider-only retention on expiry and accounting0 after return. New ownership/reopen/verify phases each run in a supervised fresh child with a60s watchdog, positive completion marker and normal exit0. Explicit close releases canonical locks while closed handles still live. A fresh child reads both exact originals, writes/reads new sentinels; another child reopens and checks all four records. |
| Logging synchronization | The retained6!=3 failures remain unchanged. One unchanged installed-native run passed after its first callback, exposing the race. RED `accb149fa` waits for actual Admin/finished and fails6!=3. GREEN `01cd67854` asserts each intended started/finished event, every named typed reentry refusal, a distinct Search/finished FIFO fence after both serialized configure calls, and exact persisted configuration records after reopen. The final installed-native module run passed10/10. This repairs the fixture's completion/count assumption; no duplicate product emission was found and no sleep or weakened assertion was used. |
| Default Node resources | **OPEN / NOT_RUN.** Historical .29 default Node FAIL/OFF_NOT_RUN and the separate semispace4 pair remain as recorded in [.29 status](../../../0.8.29/features/slice-130/status.md). No current Orin session/resource authority was established; seq-408 grants no new GPU session. CPU or semispace4 evidence cannot satisfy the selected default private-pool ON/OFF comparison. No new plateau or attribution is claimed. |
| Conditional B30-01 model release | **OPEN / PREREQUISITE_UNAVAILABLE.** Consumed cudarc343b04fa/Candle2ee39305/kernel182ceab0 match the exact revisions in the design. [.29 Slice110](../../../0.8.29/features/slice-110/plan.md) explicitly holds real Orin/3090 reset-close, last-clone and consumer proof. [60](../slice-60/plan.md) has not supplied the required exact-pin safe-drop proof. The design inventories all static model consumers and specifies owned leases/serialized release; no unsafe release API is exercised or shipped, and no CPU-only subset is presented as the complete row. Original Orin reset-survival/TC-281155a1 remains unqualified. |

The quality roadmap is applied to positive work, queue pressure, owned cleanup
and restart. General data cohesion, fuzz, crash/WAL and doctor work remains55;
integrated source/features/security/long/numerical qualification remains70;
final installed-platform/support convergence remains75. The inherited200us
hints criterion remains open with70;40's original failed gate is not relabeled.

## Verification and evidence

Owned Python3.12.3/pip/Ruff0.15.17/Pyright1.1.410 and noneditable CPU ABI3 wheel.
Exact Node26.13.1/npm11.20.0 are installed under
`/tmp/fathomdb-0830-slice50-tools`; root and TypeScript npm modules are owned
by this checkout. Rust1.95.0 is unchanged. The public Node range is unchanged.
Initial sandbox registry DNS failed; the approved-registry executor provisioned
the tools. An initial pytest command selected the source package without its
native module and failed collection; the installed run used `-o pythonpath=''`.
Neither setup failure is behavioral RED. The deterministic logging RED is
separately retained.

Scoped Clippy for engine/MCP all targets with operator/test-hooks/test-seams,
Ruff, Markdown, generated release views and the pinned environment check
passed. Terra independently executed all six runtime selections above with
the same30/11/2/2/3/10 pass counts and verified the actual installed native
module against its source-bound receipt. The wheel SHA-256 is
`5c555b2513e7c883e6c7b2617d432385b62db4266f360a3b90e1373bf2bf6c8d`;
the module SHA-256 is
`8469f797f0bb7604afdec5a7de839963a86b9a790c2a4bcc0cdff929519226bd`.

Terra ran the full gate once on clean
`2d76eea2f75376c2d25f71f7b0cc3f7c063f5c40` in the unconfined executor.
It exited **1** after219.259s at Python typecheck: one fixture annotation
required string fields while raw diagnostic payload access can return None.
Lint and the preceding Rust typechecks passed; Tegra pool lint had its
declared platform skip. The test phase and strict security had not run.
No aggregate test-suite count exists for that stopped wrapper.

The narrow recovery changes only the batch field annotation to object,
preserving unconditional event capture and every exact runtime assertion.
Full-package Pyright then reported0 errors/warnings/informations and the
installed logging scope passed10/10. Sol6.2-high passed this annotation-only
correction.

Terra independently ran the affected and remaining phases on clean
`cb98f6b743f2966012fee7fcb75a39d06f1b48eb`: Python typecheck0 errors,
installed logging10/10, TypeScript typecheck exit0, strict security exit0
and default `agent-test.sh` exit0. AC-036 and AC-037's live catch executed;
security reported zero violations, blockers or downgrades. The test runner
registered241 suites, ran238, passed238, failed0, skipped3 and excluded0.
The skips are the unselected Slice30 G/M/D live controls, requiring
`AGENT_SLICE30_LIVE_CONTROLS=1`. No such controls were activated.

The normal runner deleted passing Rust/Python/TypeScript temporary spills
before their final case aggregates were captured; partial Python/TypeScript
progress logs remain. No aggregate case count is inferred. Rust ignored
branches and Python/TypeScript network branches under
`FATHOMDB_SKIP_NETWORK_TESTS=1` are not runtime passes. The exact independent
runtime selections above retain their actual complete counts. The current
source test-hooks Python module and receipt were retained; module SHA-256
`e041eced43fe0c382fa821fb955e572e7f5692ae4c3fcbae9a319a19ef64d735`.
The production wheel/module above remain their actual2d76eea2 subjects;
the subsequent change is a test annotation and release-local status only.
This is assembled completion of the source-gate components, not a rerun or
an uninterrupted full-gate pass. It does not close the mandatory Node or
conditional model-release rows, nor70/75's remaining qualification.

Raw original/scoped/continuation logs and needed native subjects are retained under
`/tmp/fathomdb-0830-slice50-evidence`. Parent owns canonical release records,
integration and worktree removal. No GPU session, reset, driver/toolkit change,
paid campaign or publication ran. During compilation, free disk fell to18GiB.
After the Rust workspace phase, Terra verified the owned debug incremental
cache inactive and removed only that13GiB cache; compiled executables,
native subjects, receipts, release artifacts and unrelated caches were kept.
The runner was uninterrupted; final free space is30GiB. All owned verification
processes ended. The branch is ready for parent integration of this source
subset, with both mandatory open rows retained for owner disposition.
