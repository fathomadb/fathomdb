# Slice 130 design and current-source census

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

## A/B: platform and installed-consumer architecture

Use existing install/lifecycle/witness primitives and actual selected120 artifact
identities; retain true driverless ARM control. Record source-built SDK binaries
separately from distributed Python/npm artifacts. No new generic ARM distribution
is invented to make a cell pass. Each requested missing artifact goes back to120
for an explicit selected build contract or remains unmeasured.

AWS confirms [g5g uses Graviton2 and T4G](https://aws.amazon.com/ec2/instance-types/g5g/)
(read November10). This supports venue class only, not present capacity, price,
access or CUDA-stack compatibility. Freeze one compatible target from observed
hardware and reviewed build inputs; do not add .30 multiarchitecture fatbins.
CloudJetson vetting and actual JetPack7.2/root/data-handling answers are pending;
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

Add a narrowly gated, hidden qualification hook from embedder through existing
engine/native test-hook mechanisms. The embedder needs an explicit nondefault
qualification feature because cfg(test) readers are unavailable to downstream
SDK builds. Use existing `test-hooks` transport conventions and rejection gate;
exact feature spelling/propagation is part of independent design review before
code. The hook must not become a supported SDK method or production artifact
feature. Cross-crate hidden Rust items still need binding-support contract docs.

Return checked PID/device UUID/ordinal, context ID and opaque pool identity,
USED/RESERVED current counters, configured threshold/max, live model count and
decision status. Read the decided objects only: no init, model/pool/context
construction, synchronize, trim, allocation, CUDA forward or counter reset.
Context/pool IDs remain lossless across Node. Missing private decision, lost
context or failed read is a named non-success, never fabricated zeros. Do not
hold new model references or mutate ownership during observation.

Qualification-hook artifacts have separate hashes/features from120 shipping
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
actual selected120/130 consumer executions or explicit equivalent reuse. They
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
