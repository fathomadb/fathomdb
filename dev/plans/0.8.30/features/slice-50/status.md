---
title: Slice 50 runtime, model ownership and resources
status: COMPLETE
slice: 50
target_release: 0.8.30
---

# Slice 50 runtime, model ownership and resources

Finish50 resumes seq-409 from `bc6dba1518cdbccb45bfab9c50a03d606f68012d`.
All six owned rows are complete. Parent owns canonical closure, integration
and worktree cleanup. Product publication is not authorized.

## Scope and review

The unaffected runtime/logging work integrated at
`daf12b6d9839816fab4cd7c72c81e2f10918bf0f` is reused: MCP30/30,
foreground dispatch11/11, alias2/2, close2/2, concurrent close3/3 and
installed logging10/10. These retain exact positive results, bounded rendezvous,
fresh-process committed-state recovery and the original logging6!=3 failures.
The original partial status/evidence remains at that commit and
`/tmp/fathomdb-0830-slice50-evidence`; it is not relabeled as an uninterrupted pass.

Direct Astra-medium passed the remaining plan/design after one consolidated
FIX-1 (`1791509a0`). Direct Sol6.2-high passed the API/ownership body
(`720684e77`), scoped controls (`484243540`) and portable Popen discovery.
Parent Codex passed those bodies and the focused post-gate correction.
Terra independently verified the actual affected bindings, artifacts and exits.

B30-01 consumes [60's completed exact-pin safe-drop proof](../slice-60/status.md):
cudarc `3509cd6bd83b44204be1880e833474ab0f75e060`, unified Candle/kernel
`952edb7dcd1e29563acedd485a3c1ce8ef07a85c`. No owning fork/native campaign
was replayed. Rust SDK/Python `release_models()` and TS `releaseModels()`
now release both owned caches after admitted work ends; overlap returns the
existing overload class without clearing either cache. Both owners detach
before Drop, outside gate/cache locks and the Python GIL. Healthy reload works;
engine Mean ownership and terminal CUDA loss remain separate.
[Ownership ADR](../../../../adr/ADR-0.8.30-explicit-module-model-release.md).

Actual ownership RED failed2/2; the destructor-panic RED exposed the undetached
second owner. GREEN ownership/memoization/boundary scope passed12/12. Collector
schedule/pair/measurement and real-process cleanup REDs preceded their fixes;
final scoped controls passed28/28. Exact thresholds and frozen baselines remain.

## Fresh native proof

| Selection | Actual result |
| --- | --- |
| 3090 Rust SDK | Selected child PASS1/1: actual held reranker lease, typed Busy, exact reload, engine ownership and committed canary close/reopen. Binary `3ec540b0536de2bf19dffd37abbce1957c9a1858d783a9c4c817ff9035f9c09e`. |
| 3090 Python / Node | Each healthy and Busy child PASS/exit0. Native Python `72516fa933f922c55dead473f27dab3d67aaa3759cf2f72f7139e3a3a2f536e4`; Node `c3d5a2e57711a2ab81c477bac0631ba1e1c16a72c2f48d43131ff3c34c59d769`. x86 private observation is `not_built`, with unavailable counters. No new3090 reset ran. |
| Orin Python / Node | Healthy and Busy selections PASS via preserved assembled results. Healthy release observes3→1→3→0 owners and final USED/RESERVED0 within5s, without GC/trim. Mean output stays exact. Native Python `f76d99ec8d522406625759ce07d7bc416a447d897bbf65661b8aaf2b7ae80887`; Node `2767bb4bcde77b04852398b6ddb64df50516d31052f01d5a685c2850d20a5dfc`. |
| Orin new-API loss | One completed-construction child-local reset and immediate fresh recovery per binding: all four children PASS/exit0. The new API releases before subsequent inference, repeated empty release preserves typed loss, final owners0; fresh processes read exact committed state and reload. No same-process recovery or native-residue reclamation is claimed. |

Original x86 observation/launcher setup failures and the Orin disappearing-PID
Busy guard abort remain recorded; the corrected architecture oracle and the
unchanged guard's one Python-only follow-up passed. No unknown peer was waived.

## Default Node comparison and limits

Parent accepts the factual assembled R30 comparison/classification, independently
checked by Terra: same-session ON/OFF60+60 samples, exact positive workload,
zero errors/mismatches, matched Node25.9/addon/model/default-heap bytes and
actual normal exits0. The original protocol result remains **UNMEASURED**.
Its OFF close guard rejected a transitional process state after all samples.
Synchronous inventory ordering binds the failed check to an EMPTY inventory:
29891 fuser records /29888 successful guards = preflight + failed check + final
check; failed ordinal `G+1` is helper3205017, final3205018, both empty.
The known PID3181944/birth57036741 then reaped0. Real CPU controls on Orin
reproduced vanished procfs executable/zombie while Popen was not yet waitable.

Using the unchanged evaluator for a separately stated normal-completion
assessment (raw flags/results untouched):

| Diagnostic | ON | OFF | ON minus OFF |
| --- | --- | --- | --- |
| Terminal RSS slope, MiB/min |0.068735 PASS |2.158951 FAIL (>0.5) |-2.090216 |
| Terminal median RSS growth, MiB |0.15625 |9.71875 | paired median RSS -11.472656 |

ON terminal USED283241476/RESERVED301989888/owners3 were constant; private
reservation range/growth0. OFF private counters remain null. Terminal JS heap
capacity was constant; these observations do not attribute opaque RSS growth.
There is no numerical waiver, protocol PASS, hour plateau or shipping claim.
Historical .29 default FAIL/OFF_NOT_RUN and separate semispace4 results remain.

The first profile entry was NOT_RUN (missing Tegra procfs children listing).
The single final paired follow-up stopped after ON60/60 and exit0 with the
same close observer failure; it remains UNMEASURED/OFF_NOT_RUN. No third pair ran.
The maintained helper now tracks actual owned Popen identities and uses a
bounded50ms reap observation inside unchanged100ms/shared deadlines, requiring
same birth, actual0, proc absence and fresh empty inventory. Live/foreign PID,
missing-birth, fresh-peer and expired-deadline controls still reject.

## Full gate and narrow recovery

The one strict unconfined gate ran on clean `484243540`: exit1 after
3675667ms; registered241, ran238, passed235, failed3, skipped3, excluded0.
AC-036 and AC-037 executed; security reported0 violations/blockers/downgrades.
Only these failed scopes were recovered, with tests kept read-only:

- Pin-check diagnostic retained truthful registry-publication wording; the
  whole affected checker scope passed, including16 qualified-pin controls.
- Exact original PyO3 lib binary could not load `libpython3.13.so.1.0` (127).
  With its owned interpreter's LIBDIR,46/46 passed; the embedded-Python prefix
  warning remains in the log.
- Python package boundary failed2 assertions (1785 passed/36 skipped). The
  existing reviewed-additions registry now contains only the exact release
  declaration and its designated stub digest. Removing only that new block
  reproduces the old digest; frozen baseline/test files were not regenerated.
  Affected package-boundary/parity recovery passed4/4.

All other registered scopes passed, including TS and native receipts. The three
skips are the unselected G/M/D live controls; no waiver was introduced. This is
assembled recovery, not an uninterrupted green full-gate wrapper. No full gate,
workspace, full pytest or passing native scope was replayed.

The prior partial wrapper at2d76eea2 exited1 at Python annotation typecheck;
its cb98f6b7 scoped/remaining recovery passed238/241 with3 explicit live-control
skips. That historical failure and assembled result remain unchanged.

## Evidence and handoff

Local raw gate/RED/recovery logs and protected CUDA subjects:
`/tmp/fathomdb-0830-slice50-finish-evidence`; independent x86 logs:
`/tmp/fathomdb-0830-slice50-terra`. Orin exact artifacts/raw API/loss/profile
records remain under `/tmp/fathomdb-0830-slice50-finish`, especially
`terra-healthy`, `terra-busy-followup`, `terra-loss-{node,python}`,
`default-node-pair-owned-worker` and `default-node-pair-followup`.
Runnable controls stay in `dev/experiments/slice50-release/`,
`dev/tools/slice50_orin_profile.py`, `scripts/slice130_soak.py` and the Rust SDK test.

Seq-412 covers the selected Orin work. Existing inode32899085, UUID, driver and
toolkit are unchanged: private3GiB, owned RSS16GiB, RAM12/8GiB, free swap2/1GiB,
disk32/16GiB, short children120s/profile≤750s and shared session≤30min.
Unprivileged fuser is observed-peer evidence; total GPU bytes and host-wide
peer absence remain unavailable. No paid campaign, host reset or publication ran.
All owned verification processes ended. Parent owns integration and cleanup.

55 retains data/fuzz/crash-WAL/doctor work;70 retains integrated long/security/
numerical qualification, including the inherited200us hints row;75 owns final
installed platforms/native/docs/support. No remaining owned prerequisite is open.
