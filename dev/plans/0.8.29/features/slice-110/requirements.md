# Slice 110 requirements and local acceptance

Status: independently reviewed design; fault envelope/native-residue policy require actual owner
selection. S110 IDs retain original110 identities and join R29-REL-R/G,
R29-3P-17/D29-RESET and O3-17. They are local, not new global ACs. Preserve
REQ-068/AC-082a–d; do not edit locked canonical acceptance to add these IDs.

| Need / requirement | Falsifiable local acceptance |
| --- | --- |
| S110-N1/R1: refuse lost-generation use | S110-A1: in the selected envelope embed, batch, rerank, model load/open and standalone calls return existing typed context-loss payloads. Repeated/concurrent detections emit one process snapshot. No invented vectors, neutral scores, CPU substitution or allocator switching. |
| S110-N2/R2: complete ownership | S110-A2: tracked contexts/clones, tensor/slice/view/event/stream, temporary synchronization guards, modules, BLAS/RNG handles and attached private pools observe one irreversible generation loss before unsafe cleanup. Last-owner drop discovers loss without another forward. Healthy cleanup remains effective. |
| S110-N2/R3: positive teardown/exit | S110-A3: supported reset→immediate close, loss→repeated/concurrent close, standalone last-owner drop finish with explicit post-drop/close markers, normal exit0 and no signal, panic or timeout. Partial-load loss/unwind requires production-seam proof only; real reset during partial load is excluded pending separate design. Workers/locks and real-database reopen obey existing contracts. |
| S110-N3/R4: truthful recovery | S110-A4: no new GPU allocation/device/model admission or reconstruction on the lost device until process restart. Cached reranker/retained first context cannot bypass refusal. Known-lost wrappers make no unsafe native calls. A fresh process computes on the same device after the old process is reaped. |
| S110-N3/R5: explicit cleanup | S110-A5: healthy/OOM controls retain release/retry. After loss there is no per-request ownership quarantine/growth, GPU allocation or repeated native destruction. Abandoned count/known bytes cannot exceed the finite live set at loss; Rust wrappers are reclaimed. Opaque native host bytes have no established hard bound. Owner must accept this exact frozen-residue-until-restart exception to the original count/byte requirement or select separately reviewed isolation. Count alone does not establish a hard byte cap. |
| S110-N4/R6: actual consumers | S110-A6: fork unit/negative/device proof precedes exact qualified pins; positive original Orin private-pool proof and selected installed Rust/Python/Node cases identify actual loaded code/device. Default/synchronous and12/13 support require their own actual cells; missing cells stay unclaimed. |

## Proposed fault envelope

One deliberate in-process `cuDevicePrimaryCtxReset_v2` of the selected primary
context after all participants finish GPU work and no constructor, operation or
destructor can race reset. Live model/tensor/stream handles may remain. Caller
coordination covers Fathom projection workers, direct embed/rerank, other CUDA
libraries and all clone holders; a Fathom mutex cannot impose it externally.
After reset returns, callers may observe loss or immediately close/drop. Queued
calls released afterward refuse before GPU work. No same-process recovery.

Model load/construction retains its admission lease through completion or unwind.
A merely paused load still owns that lease and cannot enter the real reset
envelope. Partial-load loss is seam-only. Future live-reset coverage would need
a separately reviewed handoff: retain fully wrapped partial ownership, synchronize
all submitted GPU work, release every admission lease, and close global reset
admission to all constructors, operations and destructors. Resume must reacquire
admission, observe generation loss and unwind without unsafe native work; only
the separately permitted balanced primary release remains possible. This handoff
is not implemented or selected by this design.

Reset during active kernels/copies, constructor/destructor, identity query or
uncooperative third-party work is excluded. A native call that returns loss
still latches refusal and best-effort cleanup; it does not prove arbitrary
asynchronous-reset survival. Extending support needs owner decision/design.

## Preservation and changed claims

- Ordinary OOM is not terminal loss. Clearing transient error_state cannot clear
  terminal loss; established loss wins over subsequent OOM/errors.
- Explicit CPU/non-CUDA/driverless CPU paths initialize no CUDA; forced CUDA
  remains typed without fallback. Do not substitute another healthy device.
- Close success releases Rust ownership, workers and locks. If residue is
  accepted, it does not assert native GPU/library residue was reclaimed; record
  that distinction in ADR/SDK contracts.
- Preserve existing30-second provider-drain contract. Child deadlines are test
  limits, not a new public close SLA or interruptible-native-call promise.
- After loss diagnostics use cached metadata/null for unavailable native
  counters, never stale pool inspection.
- No public reset/recover operation, new environment switch, storage change,
  backwards-compatibility shim or unrelated maintenance is introduced.
