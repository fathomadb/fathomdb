# Slice 140 local acceptance

Active .29 repair selection follows owner seq-376/377: necessary110 source/seam
repairs and independently required existing .29 artifact repairs only. New x86
CUDA13 delivery/compiler work formerly in120 is [.31](../../../0.8.31/cuda-delivery-plan.md)
under seq-378 and cannot create a mandatory .29 repair. Real Orin/3090 reset
children are HELD .30; no .29 live-fault gate, survival or original-defect closure
is inferred. Fork B/C maintenance remains .30. No repair is selected by this note.

Status: proposed criteria, no execution. Each A140 number joins the same R140
and T140 number in [requirements](requirements.md) and [test map](test-map.md).

| Criterion | Falsifiable pass evidence | Current disposition |
| --- | --- | --- |
| A140-1 | Accepted110/existing .29 artifact requirement and exact witness select each repair; smaller alternatives are addressed and diff/dependencies exclude rebase, marker cleanup and broad kernel work. | Policy selected by seq-376; exact repair still unselected. New120 delivery is .31. |
| A140-2 | Selected healthy and loss behavior preserves all affected current CPU/ARM/allocator/linkage/error contracts; public changes have approved ADR/interface updates. Healthy cleanup actually releases resources. | Source census only; no behavior changed. |
| A140-3 | Full reviewed bases/code commits, manifests/features/locks, retained package digests/VCS/source comparison and fork-local qualification receipts bind the exact consumer closure. Root-patch and no-patch registry behavior are separately stated. | Existing C/K source authenticated; no repair package or qualification. |
| A140-4 | Per-behavior observed RED assertion/exits at its commit precede separate GREEN. Selected production source/seam tests reach typed-loss/no-forward-cleanup/partial-unwind milestones; real reset children remain HELD .30 under seq-377. Signal, panic, timeout, kill or skipped cell cannot pass. Applicable healthy/OOM controls pass; post-reset restart proof remains future live work. | Existing expected SIGSEGV is characterization, not survival. No fault executed. |
| A140-5 | Required new code version is distinct, semver justified and prepared from exact qualified source; chosen pins, lock/pin registry/relied-on items agree. No automatic latest-base advance, stale lock or speculative pin. Publication authority separately recorded before actual release. | No version/pin proposed. FT-1 not activated. |
| A140-6 | Each selected repair has fork-local verdict and affected110/existing .29 artifact consumer handoff; final130 records composed artifact proof or explicit owner disposition. An explicit no-repair-needed result addresses selected requirements. .30 items remain deferred. | WAITING_FOR_REQUIRED_REPAIR_SELECTION, no closure. |

For reset suppression/quarantine, selected design must account for every reachable
temporary, partial-construction and final owner; define safe versus forbidden
post-loss calls, retained-reference accounting and counts/bytes or explicit
opaque-byte limits/reclamation. Seq-376 accepts frozen native live-set/no-growth accounting and opaque host bytes without a hard numerical cap; no universal byte bound is proved.
Balanced primary release after reset is not categorically forbidden;110's
version-pinned evidence permits it under its stated constraints. Neither ignored
FFI errors nor catching Rust unwind proves a native destructor safe.
