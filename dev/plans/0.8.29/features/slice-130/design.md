# Slice 130 design and current-source census

[Active owner dispositions](plan.md#phase-a-scope): seq-381 moves Tegra CUDA13
unit A to .31; B–E and existing CUDA12.6 obligations remain .29;
new x86 CUDA13 delivery/compiler work is .31 and real110 resets are HELD .30.
References to120 artifacts below mean applicable independently selected artifacts,
not a mandatory new x86 package or blanket120 dependency.

## Inspected baseline and boundaries

November10,2026: repository HEAD ad102de7630141762aee28123fac57cbef9f61d5,
clean before drafting. Metadata activation is not product qualification.10's
reported source candidate1cc5701697eb93ab91e80fe297484cccd482b596 and its corrected
suite evidence remain pending final review/integration/actual closure. This
phase does not import, execute or borrow that checkout's tools/native modules.
Before source entry reread final10/110/120/140 and recompute affected inputs.

Read canonical memory index and verified entries:
windchill3-never-change-driver-or-primary-cuda.md,
forks-coreyt-cudarc-and-candle-fathomdb.md,
cudarc-fork-qualification-and-tegra-pool-facts.md,
verify-design-against-code-not-just-architecture.md,
tdd-red-must-fail-per-finding.md, acceptance-md-locked-no-feature-acs.md,
benchmark-tooling-no-tamper-tests.md and keep-measurement-tools-in-repo.md.
Memory paths are in the canonical external store, not repository citations.
Current acceptance reaches AC-082d: the older memory's highest-AC number is
historical; its prohibition on inventing canonical IDs still applies.

## Existing substrate versus actual delta

| Inspected source/evidence | What exists / consequence / remaining delta |
| --- | --- |
| embedder/src/cuda_driver_init.rs | Shared exact `off` opt-out, all-compiled-components exactCPU skip, driver-absent outcome and per-caller cuInit records. It currently runs on aarch64 CUDA, not only verified Tegra. Pure unit tests already cover these decisions; retain them. |
| napi/src/cuda_early_init.rs; py/src/lib.rs | Node registration after dlopen, guarded once across environments; Python import hook under aarch64 CUDA+tegra-pool. No constructor is needed. C is hypothesis/evidence work before any non-Tegra gating change. |
| embedder/src/cuda_pool_policy.rs | Existing decision/private pool/model guard/live count, context-alive check, last-model synchronize+trim; unsynchronized and synchronized readers are crate-private cfg(test). D needs cross-crate observation transport, not a replacement allocator/release mechanism. |
| embedder/src/cuda_pool_release_tests.rs | Complete read of all8 tests: last-model-only trim, static model retention, fallback/other ordinal/inert guard, loss/error/panic/underflow. These are established coverage, not new failing RED. |
| embedder/src/cuda_pool_slice31_tests.rs | Ignored real Orin single-model threshold0 release, max release and two-model last-close tests already exist. Keep actual live selector; merely listing ignored tests proves no .29 run. |
| engine/src/runtime_lifecycle.rs; SDK/native close wrappers | Engine releases default provider after scheduler/close work; Node awaits core close through its promise; Rust wrapper calls core close. Scheduler-held references can outlive close briefly under Q34. D must observe real final model release, not equate close-return to immediate trim. |
| Python test_tegra_pool_early_init.py / test_tegra_pool_fork_after_init.py | Existing import/opt-out/all-CPU and five Jetson fork/spawn cases; current fork suite deliberately requires Jetson and lacks generic ARM/forkserver proof. Preserve complete existing assertions. |
| release/cuda-artifact-contract.sh; Tegra distribution ADR | Existing12.6 x86 target75 and Orin87, classic-Tegra Pages loader route; generic ARM npm artifact is CPU-only. B cannot use either CPU package or sm87 binary as generic discrete GPU proof. A Thor/JetPack7.2 candidate must settle120 routing/target identity, not force classic-Tegra selection. |
| .28 G8 soak record and runs/0.8.28-slice-31/harness/pool_smoke.py | Historical Python sampler deliberately creates200000dicts each minute. Old60-minute pool-on Node+12.4%, Python+10.5%; pool-off Python+10.6% failed old drift rule. Q39 accepted measured characterization, not rewritten PASS. E adds a new rule/primary sampler; do not mutate historical runner/rows. |
| v2 allocation ADR/implementation | Method selection distinguishes integrated private USED from discrete global-free. This is execution/allocation proof, not RESERVED release. Preserve all original v1 failed records. |

Source paths in this table are beneath `src/rust/crates/fathomdb-` for named
crates, `src/python/tests/` for Python tests and `scripts/` for release scripts;
the [test map](test-map.md) provides direct links.

## A (future .31) / B (active .29): platform and installed-consumer architecture

Use existing install/lifecycle/witness primitives and actual selected applicable artifact
identities; retain true driverless ARM control. Record source-built SDK binaries
separately from distributed Python/npm artifacts. No new generic ARM distribution
is invented to make a cell pass. Each requested missing artifact goes back to its artifact owner
for an explicit selected build contract or remains unmeasured.

AWS confirms [g5g uses Graviton2 and T4G](https://aws.amazon.com/ec2/instance-types/g5g/)
(read November10). This supports venue class only, not present capacity, price,
access or CUDA-stack compatibility. Freeze one compatible target from observed
hardware and reviewed build inputs; do not add .30 multiarchitecture fatbins.
For future .31 unit A, CloudJetson vetting and actual JetPack7.2/root/data-handling
answers remain pending;
no new vendor assertion, Thor support, private-pool admission or local reflash
is inferred. Existing class gating applies; an integrated device without actual
private allocator cannot satisfy v2 allocation proof by switching to shared UMA.

## C: bounded non-Tegra initialization study

Hypothesis: generic discrete ARM can initialize at the first actual CUDA probe
without the Orin/V8 address-space hazard; removing import-only initialization
there could permit child CUDA use after an otherwise CUDA-free parent import.
This is not established. Compare accepted enabled-hook and exact opt-out first,
with instrumentation identifying actual init calls/outcomes and allocator.
Only after disconfirmation controls and independent review propose a minimal
non-Tegra gate plus accepted ADR/interface update. No blanket lazy-init rewrite.

Node main-first/worker-first tests use process-level environment set before any
worker creation, no per-worker policy mutation. [Node25.9 workers](https://nodejs.org/download/release/v25.9.0/docs/api/worker_threads.html)
document independent JS execution and environment-copy/native-addon caveats;
multiple environments do not imply separate process CUDA decisions (read
November10). Preserve registration outside the loader lock and process Once.
Bound heap controls to a preregistered payload inherited from the accepted Orin
heap fixture after final-baseline read; exact size/command is still to be bound,
not improvised by allocating until failure. No Node worker termination claim.

Use explicit Python start methods. [Python3.13 multiprocessing](https://docs.python.org/3.13/library/multiprocessing.html)
documents inherited fork state, fresh spawn and forkserver preload inheritance
(read November10). A clean-server forkserver positive control must avoid CUDA
imports in both server preload and imported main before child startup; record
the actual import points. Do not describe every forkserver configuration as safe.
Fork after parent CUDA remains the known hazardous negative; child is bounded,
parent must still forward afterward. No active-CUDA fork or reset injection.

## D: observational seam proposal

The material seam has independent Astra acceptance. Current source adds passive
transport and named unavailable/lost/untracked metadata; successful checked native
counter reads still require110's actual qualified tracked adoption. Use dedicated nondefault `cuda-pool-observer` throughout:
embedder `=[]`; engine `=["fathomdb-embedder/cuda-pool-observer"]`; SDK, Py and
NAPI each `=["fathomdb-engine/cuda-pool-observer"]`. It implies no CUDA/model,
`test-hooks` or `operator` feature, and no ordinary feature enables it. Native
transport follows the existing separate-module/registration convention without
activating broad mutable test hooks. Shipping feature lists must reject it.
The [proposed ADR addendum](../../../../adr/ADR-0.8.29-sdk-qualification-observation.md#slice130-d-addendum)
extends the currently closed diagnostic catalog before source implementation;
the [binding-support contract](../../../../interfaces/rust.md#private-cuda-pool-observation)
fixes names and wire types without adding governed operations.

### Decided ownership and actual driver APIs

Current `cuda_pool_policy.rs` owns driver `DECISION: OnceLock<Decision<Arc<CudaMemPool>,
Arc<CudaContext>>>`. `PrivateDecision` already has `ordinal`, `pool`,
`first_context`, `context_id`, atomic `live_models`, configured `max_size_bytes`,
`release_threshold` and cached `loss`. Its cfg(test) synchronized reader explicitly
synchronizes; its unsynchronized reader directly reads the pool but has no
qualification transport or tracked-generation requirement. Neither is the new
production observer. Published-loss handling from110's accepted98-test CPU
subset remains authoritative and unchanged.

At selected Cudarc `cb36642fad90c1796693c36ee59e3d3d162f4410`, borrowed
`CudaContext::primary_lifetime()`, token `recorded_id()`/`loss()`,
`CudaContext::mem_pool()` and `CudaMemPool::raw()` are available. The first two
token observations do no native dispatch. `used_current()`/`reserved_current()`
read attributes through the attached token's `admit()`; admission can perform
the qualified liveness probe but never synchronizes or trims. Under that same
outer admission, `CudaContext::uuid()` reads the decided device UUID. Borrow
these existing objects; check pool association and recorded context identity,
check cached loss before admission and again before returning success. Read no
high-water/default-pool counters to substitute for private current counters.

110 has not adopted this tracked construction/model wiring into the product.
The observer must return `untracked` without native reads when the decided
context has no actual shared token; it must not make a replacement token,
context, pool or registry, nor query a default context. Its healthy driver
branch is integrated only against actual qualified110 adoption, never a guessed
API or an untracked path. Fallback decisions return named `no_private_pool`
with their recorded reason and unavailable private identity/counters. The
observer reads `DECISION.get()`, never `get_or_init`, and borrows only decision
objects. It creates no model ownership, GPU allocation/forward, retain,
synchronize, trim or counter reset. Bounded host serialization is diagnostic cost.

Read live model count before and after the checked UUID/current-counter reads.
If the before/after live counts differ, report `ownership_changed` with
unavailable counters. Equal counts do not exclude close/open ABA or intervening
activity; no epoch or model-guard change is introduced. The tuple is not an
atomic GPU snapshot. Cached/published loss returns `lost`
without native enrichment; an actual read failure returns `read_error` with its
cached code/message. Missing decision, unsupported build and untracked ownership
remain distinct from these failures. Only an all-successful checked observation
may report zero CURRENT counters. Ordinal, configured size/threshold and live
model count may truthfully be zero even when current counters are unavailable.
Preserve optional observed identity/first-loss cause;
do not query driver error names or manufacture a measured driver code.

### Source and test boundary after seam review

Confine new read-only logic/types to `cuda_pool_observation.rs` in embedder, with
feature-gated borrowed access inside `cuda_pool_policy.rs`; add engine and native
`cuda_pool_observation.rs` transport modules, SDK's exact static forwarder and
minimal manifest/lib registrations. Update the existing closed parity catalog
and its existing positive/negative controls; no general name-pattern exemption.
Keep production constructors, model guards/drop, allocator admission and pins
unchanged. Actual110 remains the only tracked-generation authority.

First extract the existing unsynchronized-reader decision/read path behind a
CPU-testable borrowed seam without changing its behavior. A new semantic test
must demonstrate that its actual current behavior attempts a counter read for a
published-lost private decision, contrary to the new observer's zero-dispatch
contract; record that observed assertion failure before adding the observer's
refusal gate. This is a new qualification behavior, not a claim that the
historical test-only reader is a shipped lifecycle defect. Subsequent focused
RED/GREEN cases cover absent/fallback/untracked decisions, failed admission or
counter read (including partial-read refusal), ownership change and exact
successful counters. Use counting driver seams, never a mocked database.
Property-based wire round trips preserve arbitrary u64 IDs/counters, including
18446744073709551615, as decimal strings across JSON/Python/Node parsing. Actual
installed SDK/last-close/native smoke remains separate real-database proof;
missing API compilation or a nonexistent hook is not behavioral RED. No build,
CUDA call or native qualification runs during this design step.

Qualification-hook artifacts have separate hashes/features from the selected shipping
artifacts. They establish internal lifecycle accounting through SDK paths;
shipping-artifact lifecycle/witness controls remain necessary and cannot be
replaced by the instrumented build. Derive source/code equivalence explicitly.
The accepted production last-model drop may synchronize/trim; the observer may
not. Proposed settle: monotonic5s maximum after awaited last close/drop, poll
every50ms, retain first zero and model-count transition. No extend/retry/GC.
For max first close, capture the positive settled reservation with both models
alive, admit no further allocation/forward, then require exactly unchanged
RESERVED after the first close before testing the surviving model's forward.
The existing two-model test uses this equality; a positive smaller reservation
could hide an incorrect premature trim and is not sufficient.

## E: revised steady-state evidence

Portable source delivery now provides the [bounded sampler/evaluator](../../../../../scripts/slice130_soak.py),
[measurement controls](../../../../../scripts/tests/test_slice130_soak.py) and
[usage/schema](../../../../experiments/slice130-soak/README.md). The sampler
accepts injected real workload/passive-observer functions; actual D native
transport and installed SDK wiring remain outstanding. Its verdict covers only
instrumented numerical measurement, never whole-E or shipping qualification.
The frozen protocol and all historical evidence remain unchanged.

Fork/copy the retained measurement logic into a new committed .29 harness,
leaving historical runner and logs intact. Primary Python sampler omits explicit
host-allocator churn; keep a bounded sampler and record its cost. Test slope/
window/count math against hand-computed data, session order, fresh DB and real
SDK smoke. No tamper/digest-drift test suite. Exact proposed numerical rule,
arm order, rejection and sample cadence are in [protocol](campaign-protocol.md).

The two primary E arms use D's qualification-hook Node/Python counterpart
artifacts, so the on-pool RESERVED time series is observable without a production
counter surface. This is an explicit technical proposal, not a shipping-binary
soak claim. Bind paired shipping/instrumented hashes and review source/features/
dependencies/toolchain/model equivalence down to passive hook/transport changes;
unexplained lifecycle/allocator/runtime differences block the unit. Mandatory
shipping lifecycle/policy/forward/close/exit and v2 allocation proof come from
actual selected applicable130 consumer executions or explicit equivalent reuse. They
cannot be replaced by the instrumented pair. No additional shipping60-minute
campaign is selected: its numerical plateau remains unmeasured. If direct
shipping plateau is required or equivalence cannot be established, retain an
explicit E hold and return that changed evidence/budget choice to parent.

Schedule t0 is a separate baseline; exactly60 qualifying samples occur at
60k seconds for k1…60, with terminal k31…60 and medians k31…35/k56…60.
The baseline is excluded from the sample count and terminal arithmetic.

Protocol review can settle technical bounds without reopening venue choice.
Run authorization, missing support/policy choices and any changed accepted
threshold remain parent/owner gates. No currently measured plateau claim follows.
