# Slice 140 required repair design

The110 shared-terminal-lifetime repair is selected under seq376/377. Its
completed selected cudarc CPU qualification and the remaining Candle/product
handoff are described in the [active plan](plan.md#selected-repair-and-remaining-delivery).
The table below retains the original scope selectors; it does not reopen a
completed selection or authorize unrelated work. New CUDA13 delivery/compiler
and130A are .31, real resets and fork B/C are .30. Existing dated source census
and reviews remain evidence for their original subjects.

## Exact candidates and selectors

| Candidate | Existing witness / possible smallest A work | Evidence and decision selecting it |
| --- | --- | --- |
| C140-L generation and cleanup lifetime | C has clear-on-read error_state, distinct context wrappers and nested event/slice/stream/SyncOnDrop/module/BLAS/RNG/context/pool cleanup. BLAS/RNG destroy unwrap; construction can own raw handles before wrapper completion. A reviewed terminal-generation/cleanup seam could be required in cudarc, with only necessary Candle ownership/admission adaptation. |Seq-376 selects quiescent completed-construction/frozen-live-set/no-growth residue with opaque host bytes unbounded; demonstrate every reachable affected cleanup and partial-load path. A BLAS-only patch or top-model quarantine is insufficient evidence. No x86/all-API support inferred from private Orin classification. |
| C140-A selected artifact delivery/correctness | C item9 selects built-major libraries; K static cudart and locked cudaforge0.1.5 compile both PTX/JIT and native MoE archive paths. Possible fork repair only if an independently required existing .29 artifact proof establishes a concrete failure of these paths or a required, reviewed mitigation. |New120 artifact/compiler6111901 work is historical future input for .31 and does not select a .29 repair. Exposure UNKNOWN does not select a patch or toolkit13.2. Broad multiarch/native/PTX/dtype/MoE maintenance remains .30. |
| C140-V necessary repair release | New required C code may need distinct cudarc version; dependent K manifests may need a new qualified release to consume it. | Only a selected required code diff activates FT-1. Determine API/semver impact, publication source/closure and smallest release set; no version guessed now. A pure K kernel patch does not automatically require all Candle packages to change. |
| Outside140: product remedy/selector | Product cuda_libraries.rs always prescribes CUDA12 remedy; TS loader has no selected-major contract. |120 owns these definite product gaps and Q120 decisions. Their presence alone cannot select a fork change. Distinct Node13 identity must reconcile ADR-0.8.25's rejected package alternative. |

Shared terminal state, complete quarantine and process isolation are independently
useful110 alternatives; seq-376 selects shared terminal generation/frozen-live-set policy, while quarantine/isolation remain unselected. No mutex/health query promises
survival of arbitrary uncooperative resets during FFI/kernels/destruction. No
manual Lost injection alone proves real no-forward-close discovery. A retained
whole model does not capture partial loads, per-call temporaries or external
clones. Opaque host allocations are not bounded by pool maxSize; a count bound
without host-byte disposition cannot silently satisfy acceptance.

## Fork ownership and provenance

Parent creates fresh isolated fork worktrees at explicitly reviewed source bases.
Default candidate origins are consumed C/K, with comparison to any chosen later
base and explicit justification before change. Never automatically cut from
origin/main and import .30 work. Fork-local `fathomdb/` owns requirements,
accepted model/design, tests, review verdicts and qualification receipts. Product
owns SDK/error/admission integration. No nested agents or shared writers.

Keep the existing fork-local base, exact source/diff, API/feature/lock and actual
tool identities with the substantive repair. Do not regenerate a duplicate
packet, hash inventory or source census after unchanged reviewed inputs. Reuse
qualified C where K depends on it, qualify affected K, then review coherent
FathomDB consumption. Preserve current pins until
that proof exists. If registry publication is needed, version preparation and
dry-run/review precede separate publication authority; authenticate resulting
archive digest, VCS identity and source/original manifests before registry-pin
adoption. No speculative registry checksum or unpublished package pin.

Update affected product exact pins, Cargo.lock, pinned-override registry,
relied-on FATHOMDB items, lock-set evidence and contract/checker assumptions
together. Root candle-kernels patch supplies static cudart only to repository
builds; external Rust consumers do not inherit it. A registry closure remains a
separate proof, never a false REQ-068 promise for all source builds.

## Qualification and review order

Fresh independent Astra design review covers selected fork-local packet before
code. Fork human-intent tests and negative controls get real behavior RED before
GREEN; initial fork qualification then selects exact pins.110/existing .29 artifact owners integrate/build
affected retained artifacts next;130 reruns affected final composed cells with
its accepted bounded protocol.130's A–E do not expand140's A-only scope. Final
implementation/results review is fresh and independent of the design reviewer.

Fork checks own their artifacts; FathomDB consumes receipts and tests product
integration rather than rerunning fork suites. Use owned environments, relevant
scoped checks and final clean source-phase full gates once. Changes invalidate
only affected evidence after an explicit blast-radius comparison; preserve real
failures/skips and do not relabel historical .28 qualification as .29 proof.

Apply the quality roadmap's bounded state/lifetime anomalies, real resource
postconditions and independently specified outputs. Defer exhaustive CUDA APIs,
architecture/dtype expansion, general GPU fuzzing, broad upstream modernization
and arbitrary fault-race survival. The selected cudarc CPU result covers the reviewed dispatch/lifetime seams;
Candle and product consumption plus required actual-device controls remain
separate obligations. It is not reset-survival proof.
Seq-379 accepts bounded usage unknowns/current ratings, with quantitative evidence in .31; no repeated collection,
new importance/toolkit choice or host modification is authorized by140.

## Proposed local executor for exact-lock fork qualification

**PROPOSED; not adopted and no native GO.** This replaces only the hosted
executor in the [Q23 lock-drift contract](../../../0.8.28/features/slice-31/design.md#72-the-lock-drift-guard-q8-option-b)
for the selected unpublished343/2ee local graph. The
[pool ADR](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md)'s requirement to
compare the product lock with an actual fork qualification run remains. No new
ADR is needed: its decision4 requires fork-owned evidence and strict lock drift,
not a particular executor. Q23's two native Linux architectures remain required.
Astra-medium design acceptance and explicit owner adoption of this executor
replacement precede gate changes; native venue/resource GO is separate.

### Evidence being added

The existing312 CPU controls at `cb36642fad90c1796693c36ee59e3d3d162f4410`
remain source/seam proof. Their root features match the product, but their lock
has nine different dependency versions and lacks the product's bytemuck closure;
they are **not exact-product-lock unit proof**. Selected343 changes only package
version/notices, with the same production source. Fresh filtered product metadata
at the current lock has22 members on each Linux target: the21 non-root
version/source/checksum entries and ten root features equal the genuine old
record; only cudarc's root identity changed. This equality does not supply a new
fork run or native ARM evidence.

Qualify natively on x86_64 and on the existing Orin's aarch64 Linux CPU venue.
An x86 cross-target metadata result cannot replace Orin compilation/execution.
Seq-387 excludes unavailable non-Tegra ARM+GPU cells, not this ARM CPU duty.
No GPU device test, fault/reset, model execution, new venue or toolchain change
is authorized here; unavailable access/tools/resources leave that cell OPEN.

### Exact commands and unchanged test oracles

For each venue, use an isolated fork checkout at exact
`343b04fa73f85ee78754d6ee2baa07b959cc9ccc`, its original tests, owned cached
sources/tools, and a copy of the current product `Cargo.lock`. Preserve literal
Git source identities, all non-root versions/checksums and HTTPS K9 kernels;
ARM uses owned copied Git objects rather than rewriting the product's file URLs.
In an owned scratch copy only, join the fork root and its existing
`fathomdb/lock-consumer` as one resolver2 workspace: add root members
`[".", "fathomdb/lock-consumer"]` and remove the consumer's nested `[workspace]`.
Keep its existing half/bytemuck/use-intrinsics and float8/num-traits/rand_distr
bridge. Do not edit library/test source, weaken an assertion or add dependencies.

Normalize only this scratch lock offline before freezing it; mapping its one
path-root entry to the selected343 Git identity is valid only after checking
clean343 source. Before any native command, both filtered normal closures must
match the product's22 package identities/checksums and ten cudarc root features
using the existing guard's projection. Refuse any re-resolved dependency,
missing/extra node, source/version or feature drift. Preserve actual dependency
features in the existing run output; do not claim the alternate-lock312 result
used this graph. Once frozen, every native Cargo command uses `--offline --locked`.

From the scratch workspace, with `F` exactly the ten root features below:

```bash
F=std,cublas,cublaslt,curand,driver,nvrtc,f16,f8,cuda-version-from-build-system,dynamic-loading
CUDARC_CUDA_VERSION=12060 CUDA_VISIBLE_DEVICES= CARGO_BUILD_JOBS=1 \
  cargo test --offline --locked --workspace --lib --no-default-features \
  -F "$F" --no-run --message-format=json
```

Select only the resulting cudarc library-test executable, never a pre-existing
binary. Execute the already reviewed312 seam names from the accepted selection
operator, one process per name with `--exact NAME --test-threads=1 --format=terse`.
On ARM, its two source-cfg exclusions in `primary_context_constructor_tests`
(`allocator_failure_releases_construction_admission_without_releasing_anchor`,
`allocator_unwind_returns_construction_admission_and_preserves_anchor`) give310
seam tests; these are explicit cfg exclusions, not passing/ignored tests.
Also run the original hosted catalogue's landed `kind=P`/`kind=C` CPU controls
whose cfg matches12060, driver/std and the native architecture:11 names on x86,
12 on ARM, including ARM's `sync_zero_byte_request_is_a_null_pointer_without_a_driver_call`.
Select these from the existing catalogue through `count-tests.py`'s cfg matching,
not from whichever tests happen to list. The nonoverlapping totals are323 x86
and322 ARM; each selected name must run once and pass, zero ignored. No device
or fresh-process GPU name is selected or credited. Original assertions stay
unchanged; missing names, native signal/panic, timeout, early return/skip or a
nonzero command are failures, not a reduced accepted count.

The workspace command builds the existing lock-consumer alongside the exact-lock
unit binary. Its closure/root-feature comparison is mandatory before crediting
that compile. Reuse the successful dc8 x86 wheel/CLI/TC5 build for the product
consumer-compile duty: its actual22 identities/features match this graph; keep
its original source/time/command attribution and do not perform a confirming
build. It cannot replace the new exact-lock unit binary or Orin's native
lock-consumer compile and CPU execution. No ARM compiler/runtime pass follows
from the previously captured ARM metadata.

### Bounded execution and record promotion

Reuse the existing fork qualification supervisor/selected-test operator under
`/mnt/data/worktrees/fathomdb-0.8.29-cudarc-reset-20261114-data/operators/`, rebinding
only owned source/data/native-tool paths, selected343 identity and the explicit
architecture counts above. No new execution framework. Sequential Cargo jobs1;
private cgroup2GiB, swap0, tasks128, CPU100%, runtime300s per phase and outer320s.
Keep its entry floors: MemAvailable10GiB, root free32GiB, task free250GiB; stop
at MemAvailable8GiB, root20GiB, task50GiB, owned storage4GiB or sustained actual
swap64MiB/s for three30s windows. Apply on both native venues before reservation
and during growth; a failing Orin floor remains OPEN, never relaxed silently.
Own caches/target/tmp/logs; retain ordinary raw commands/exits/counts and cleanup
outcome. No borrowed venv/native artifacts, other-session cleanup or retry after
a resource stop. Preparation does not claim execution authorization.

After both native cells pass and independent results review accepts them, keep
the authentic old CI record and its CI run/head/conclusion unchanged as history.
A small separately identified local successor names the exact fork/product lock,
both native commands/results, both target closures/root features and authentic
fork-local provenance. No invented CI identifier or carried-forward old success.
The guard must still enforce exact sources/versions/checksums, coherent family,
both target sets, root features, self-consistent digest and original exit meanings;
only the evidence's executor/provenance branch changes after owner adoption.
Until adoption and actual qualification, its present RED remains valid. Registry
normalization/publication and other release gates retain their existing scope.
