# Slice 100 needs, requirements and local acceptance

N100 refines N29-R/R29-REL-R: an application needs truthful erasure state and
surviving authorized data after an interruption within the declared fault model.
R29-3P-16/O3-16 in [completed3](../../prework/slice-3/requirements.md) owns this
allocation. Canonical ancestry remains NEED-001/020/021 and existing reliability/
erasure requirements; AC-028a/b/c and R-20-E1/E5→AC-080 constrain retained audit,
projection, survivor and raw-byte success proof. Process exit supplies no new
REQ-031b/AC-034 power/OS-crash proof. These A100 labels are local, not additions
to locked canonical ACs. No existing threshold changes.

Completed10 implementation104dcbcbe92a1eca730d146b99ab17d444df6217 and parent
closure d7d928f8e are integrated at02e30d861beddc8061af0dc22f12106d5e09dd32.
That preparation dependency is satisfied. Accepted70 e875f360 and20 A3/A5 Linux
proof/capacity subset f803a964 are integrated at source-entry3c34cd19.
The f803 canonical Rust execution supports existing-control inference against
identical entry src/lock inputs; the qualified subset at85f7eb219 passed final
independent review. Seq-371 approves100-only verification coverage; seq-372 selects bounded
Q1 on the local unconfined ptrace-capable executor. [Exact decisions](owner-decisions.md)
retain source review, actor/identity/runtime readiness and parent technical GO.
Seq-369 remains20-only. No runtime proof or100 closure follows. Entry preflight/ownership checks, maintained-plan recheck and parent source GO
are complete. Seq-383's one additional pair has now executed at integrated453: native healthy
qualified, interruption UNQUALIFIED without confirmed fault exit or recovery.
No further pair is selected. Recorded design PASS is not product acceptance.

| Need → requirement → acceptance | Independent oracle / census / selector / source |
| --- | --- |
| N100 → R100-1: preserve atomic intent/deletion/sanitation and truthful phases. A100-1: each selected child reaches its named fault, uncommitted state rolls back, committed state survives, no speculative Complete. | Exit73 plus predefined state partition, raw SQL target/root/ticket state and survivor; C1–C6, S1/S2 in [test plan](test-plan.md). `erasure_queue::{queue_physical_erasure,activate_in}`, `erasure::{purge_inner,excise_source_inner}`, `physical_erasure::{sanitize_unit,prepare_redacted,finalize}`. |
| N100 → R100-2: policy/fences remain exact across release/reopen. A100-2: Queued has unchanged visibility; explicit activated debt fences mutations and never starts autonomous work; autonomous reopen actually finishes without target/follow-up calls. | Separate explicit/autonomous controls and independently opened observer, not disabling worker during autonomy proof. C7/C10/C11; S3/S4/S7. `open`, `erasure_maintenance`, `physical_maintenance`, durable root/queue guards. |
| N100 → R100-3: physical success preserves registered boundaries. A100-3: exact zero for canonical/projection/native/span targets, original-sink discharge, identity-free queue/root, successful TRUNCATE precede Complete; capture main/live WAL before close can mask residue. | Positive target population, raw tables/file bytes, recorded original fingerprints and unchanged control records. C5/C8/C9/C12; S4–S6/S8. Shared erasure/projection/span registry and finalizer; G100-2 remains explicitly partial for combined crash assertion coverage. |
| N100 → R100-4: preserve exact selected scope/survivors and permitted replay. A100-4: survivor history/row IDs and independent survivor-only scores/order persist, retry converges, offline leaves Queued untouched and publishes same activated receipt. | Hand-selected targets plus independent survivor-only store, exact history snapshot, real retry/offline controls. C8–C12; S4–S6/S8. `erasure_queue::preview`, physical zero proof, `operator/data_plane::recover_complete_erasures`. |
| N100 → R100-5: qualify the actual interruption model. A100-5: inside-checkpoint coverage requires an attributable active-checkpoint reached witness, not a sleep, pre/post hook, BUSY or unreached child; retain all fault/control outcomes and limitations. | C13/G100-1; selected Q1 scope in [protocol](q1-protocol.md). Pinned SQLite `walCheckpoint` frame-copy position and owned child identity would be the independent boundary witness. Native reached witnesses are qualified; interruption exit/snapshot/recovery remain UNQUALIFIED, not a reproduced product defect. |
| N100 → R100-6: keep qualification honest and changes minimal. A100-6: exact source/tree/lock/features/tool/native/host/selector/exits/reachability receipts, verified closed10 ancestry, own strict preflight and two independent reviews; only real defect repairs receive per-finding RED→GREEN. | C14/G100-3; S1–S9 with own artifact receipts, Git/record checks and independent review. Existing [83](../../../0.8.28/features/slice-83/status.md), [85](../../../0.8.28/features/slice-85/status.md), [95](../../../0.8.28/features/slice-95/status.md) receipts keep original identities; completed10 preparation is not fresh100 proof or a coverage exception. |

Every R/A row maps to N100 and the release FAULT row, and each census row maps
back here. G100-1 and G100-2 stay open or explicitly disposed; a design PASS or
passing representative suite cannot upgrade them to exhaustive qualification.
No public behavior/API/format change is currently proposed. A discovered repair
that changes a public contract must update its maintained interface/ADR in the
same reviewed scope; scope expansion returns for the specific decision.
