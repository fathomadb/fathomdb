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
