# Slice 140 local acceptance

Status: proposed criteria, no execution. Each A140 number joins the same R140
and T140 number in [requirements](requirements.md) and [test map](test-map.md).

| Criterion | Falsifiable pass evidence | Current disposition |
| --- | --- | --- |
| A140-1 | Accepted110/120 requirement and exact witness select each repair; smaller alternatives are addressed and diff/dependencies exclude rebase, marker cleanup and broad kernel work. | Unselected;110/120 decisions remain held. |
| A140-2 | Selected healthy and loss behavior preserves all affected current CPU/ARM/allocator/linkage/error contracts; public changes have approved ADR/interface updates. Healthy cleanup actually releases resources. | Source census only; no behavior changed. |
| A140-3 | Full reviewed bases/code commits, manifests/features/locks, retained package digests/VCS/source comparison and fork-local qualification receipts bind the exact consumer closure. Root-patch and no-patch registry behavior are separately stated. | Existing C/K source authenticated; no repair package or qualification. |
| A140-4 | Per-behavior observed RED assertion/exits at its commit precede separate GREEN. Selected real-device children reach required typed-loss/no-forward-close/final-drop/normal-exit milestones within frozen bounds. Signal, panic, timeout, kill or skipped cell cannot pass. Healthy/OOM/restart controls pass. | Existing expected SIGSEGV is characterization, not survival. No fault executed. |
| A140-5 | Required new code version is distinct, semver justified and prepared from exact qualified source; chosen pins, lock/pin registry/relied-on items agree. No automatic latest-base advance, stale lock or speculative pin. Publication authority separately recorded before actual release. | No version/pin proposed. FT-1 not activated. |
| A140-6 | Each selected repair has fork-local verdict and affected110/120 consumer handoff; final130 records composed artifact proof or explicit owner disposition. An explicit no-repair-needed result addresses selected requirements. .30 items remain deferred. | WAITING_FOR_REQUIRED_REPAIR_SELECTION, no closure. |

For reset suppression/quarantine, selected design must account for every reachable
temporary, partial-construction and final owner; define safe versus forbidden
post-loss calls, retained-reference accounting and counts/bytes or explicit
opaque-byte limits/reclamation. Unknown bounds block the affected criterion.
Balanced primary release after reset is not categorically forbidden;110's
version-pinned evidence permits it under its stated constraints. Neither ignored
FFI errors nor catching Rust unwind proves a native destructor safe.
