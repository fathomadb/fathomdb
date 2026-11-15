# Slice 130 test and evidence map

[Active owner dispositions](plan.md#phase-a-scope): seq-381 moves Tegra CUDA13
unit A to .31; D/E and existing CUDA12.6 obligations remain .29. Owner seq-387
excludes unavailable non-Tegra ARM+GPU B/C from known-working environments and
every .29 release gate; they remain unqualified with no future release assigned;
new x86 CUDA13 delivery/compiler work is .31 and real110 resets are HELD .30.
References to120 artifacts below mean applicable independently selected artifacts,
not a mandatory new x86 package or blanket120 dependency.

The original census was inspection-only; integrated D observer and portable E
sampler source coverage is reused. Native D/E remains unexecuted. Full files read for release tests,
early-init/fork controls and documented contracts; related device/build/SDK paths
were inspected by relevant symbols. Preserve existing assertions/gold. Test names
below for new work are proposals, not claims that files/tests already exist.

| Exit | Existing inspected oracle/substrate | Required delta and intended assertion |
| --- | --- | --- |
| A1 future .31 / A2 excluded from .29 | [Allocation v2](../../../../adr/ADR-0.8.28-attributable-cuda-allocation-witness.md), [Tegra verifier](../../../../../scripts/release/verify-tegra-gpu-witness.py), [preflight verifier](../../../../../scripts/release/verify-cuda-preflight-witness.py), [artifact contract](../../../../../scripts/release/cuda-artifact-contract.sh) | Future-only Tegra13 (.31 A)/unqualified non-Tegra (B outside .29 by seq-387) SDK installed/source-consumer cells with real forwards, correct method/PID/UUID/context/artifact identity, missing-library/driverless/forcedCUDA/CPU controls. Extend existing producer/validator only for a demonstrated selected-contract gap; do not write another framework. |
| A3 outside .29 (seq-387) | [Shared init](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_driver_init.rs), [Node init](../../../../../src/rust/crates/fathomdb-napi/src/cuda_early_init.rs), [Python import](../../../../../src/rust/crates/fathomdb-py/src/lib.rs), [existing import tests](../../../../../src/python/tests/test_tegra_pool_early_init.py) | Existing exact-off/all-CPU/driverless/per-caller outcomes stay. Proposed generic_arm_import_does_not_init_after_reviewed_gate requires an actual accepted change and failing assertion of observed initialization before GREEN; no test is promised until C evidence justifies the gate. |
| A3 outside .29 (seq-387) | [Complete five-case fork suite](../../../../../src/python/tests/test_tegra_pool_fork_after_init.py), [retained Node heap consumer](../../../../../scripts/tests/tegra_node_early_cuinit_consumer.mjs) | New generic ARM enabled/off, explicit fork-before-CUDA/spawn/clean-server-forkserver and parent-after-negative controls; main-first/worker-first Node and exact bounded heap settings. Existing Jetson marker skips do not count as generic ARM proof; required live skips fail qualification. |
| A4 | [Complete eight release tests](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_pool_release_tests.rs), [three ignored live release tests](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_pool_slice31_tests.rs) | Existing model-count/last-only trim/error/loss semantics are retained. Live tests gpu_orin_64_dropping_the_embedder_releases_pool_memory_without_a_caller_synchronize, gpu_orin_64_close_release_also_trims_a_max_threshold_pool and gpu_orin_64_max_pool_stays_reserved_until_the_last_model_closes remain real Orin evidence obligations, not new passing-on-arrival TDD work. |
| A4 | [Pool decision/readers](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_pool_policy.rs), [core lifecycle](../../../../../src/rust/crates/fathomdb-engine/src/runtime_lifecycle.rs), [Rust close](../../../../../src/rust/crates/fathomdb-sdk/src/engine.rs), [Node close](../../../../../src/rust/crates/fathomdb-napi/src/engine.rs) | Integrated observer controls encode passive/nonfabricated measurements. Cross-SDK max_first_close_keeps_exact_reservation_before_survivor_forward requires positive before value, unchanged after value without intervening allocation, then unchanged same-input survivor output; last_close_releases/threshold_zero/implicit_drop/process_singleton_retains need real SDK/model ownership and counters. A smaller still-positive reservation must fail the first-close oracle. No mocked DB. A missing hook alone is not a product release defect. |
| A5 | [Historical G8 record](../../../0.8.28/features/slice-31/qualification/2026-10-15-ORIN-G8-SOAK-32295692f56b.md), [retained Python runner](../../../runs/0.8.28-slice-31/harness/pool_smoke.py) | Existing .29 sampler gains real installed-worker adapters and bounded concurrent supervision; historical bytes stay intact and primary sampling omits churn. Primary E pair uses declared qualification-hook counterparts: every numerical criterion is checked there, on-pool RESERVED mandatory. Review actual paired source/features/dependencies/toolchain/model equivalence; shipping installed lifecycle/v2 proof must actually pass separately, with no shipping numerical plateau claim or automatic second long campaign. Proposed plateau_math_uses_elapsed_time, baseline_is_not_a_scheduled_sample, terminal_windows_use_exact_samples, missing_samples_cannot_pass and primary_sampler_has_no_deliberate_churn use independent hand-computed series/counts or observed allocations, not snapshots/regex-only mirrors. |
| A6 | [Direct mandate](../../direct-execution.md), [prework oracle allocation](../../prework/slice-5/traceability.md), [10 preparation register](../slice-10/preparation-register.md) | Source entry/selected exact inputs and actual review/exit records. No execution tests are invented for proposal metadata, nor broad tamper/path-hardening test campaign. |

## TDD and verification order

Hand-computed schedule boundary fixture: t0=0 is a separate baseline; samples
k1…60 occur at60…3600s (60 total), terminal k31…60 at1860…3600s (30 total).
First terminal median uses1860,1920,1980,2040,2100s; final median uses
3360,3420,3480,3540,3600s. A baseline RSS deliberately different from sample1
must not alter any terminal statistic. Missing k60, duplicate k30, or treating
t0 as one of the60 scheduled observations must fail completeness. Hand-computed
linear RSS values RSS(k)=100+k MiB at exact scheduled times give slope1MiB/min,
first-terminal median133MiB and final median158MiB, hence difference25MiB;
this deliberately fails both proposed bounds. Jitter within5s uses actual times.
This encodes timing intent
independently of a runner loop's inclusive/exclusive range.

For every approved executable behavior change, first run the meaningful assertion
against the pre-fix implementation, preserve observed failure reason/output at
the RED SHA and stage/commit RED separately before GREEN. A syntax/import/missing
file failure is not automatically the intended behavioral RED. Keep tests fixed
during fix-to-spec. Already correct/live characterization is retained and run
without inventing failure or using Slice20's pending special exception.

Use focused lint, typecheck, unit tests before bounded real DB/native/feature
controls. Property-based cases are required for new codec/round-trip transport
if introduced; expected identity/counter fields come from human-specified
examples, not generated oracles. Test measurement math/resume/session order/fresh
DB where used; do not add tamper/digest-drift tests. Source/tool execution changes
require one full stable owned agent verifier and fresh independent Astra-medium
code review. Feature-gated/ignored hardware obligations run separately with
actual live selectors; default cargo success does not execute them.

Docs-only proposal validation is bounded links/traceability/whitespace and scoped
Markdown checks. Missing owned tooling remains a failed/unavailable authoritative
gate, not a product failure or excuse to mutate another checkout. No source,
native, model, hardware or whole-workspace qualification ran in Phase A.

## Remaining binding controls

- Real Rust/Python/Node two-engine lifecycle fixtures use fresh databases and
  installed Python/Node counterparts, preserving exact max equality before
  survivor forward and final5s/50ms settling. Singleton/exit/drop remain separate.
- Adapter controls reject smaller-positive first-close reservation, wrong pool/
  context/PID, ownership change, partial/unavailable/error counters and unsafe
  integer coercion; preserve raw data and property-based exact u64 round trips.
- Controlled subprocess tests require concurrent SDK startup, sequential on/off
  arms, streaming output, bounded requests/close, actual exit propagation and
  complete sibling reaping. They supply no native database qualification.
- Existing wrapper tests preserve default shipping features and explicit
  observer-only counterparts. Existing Python installed smoke gains local-wheel
  consumption without invented Pages provenance or weaker v2/lifecycle checks.
- Existing hand-computed60-sample/time/window/numerical controls remain unchanged;
  qualifying live cases cannot skip and no forced-GC flag is introduced.

One coherent independent Astra-medium design review precedes this binding's
RED/GREEN. Reuse accepted110/140/seam proof; exact qualified inputs gate affected
final runs, not unrelated whole140/160 closure.
