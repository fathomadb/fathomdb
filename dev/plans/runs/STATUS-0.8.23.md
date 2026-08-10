# STATUS — FathomDB 0.8.23

> **Board of record.** The single writer is
> `dev/plans/release-state-0.8.23.json`; the release plan is
> `dev/plans/plan-0.8.23.md`.

## Current state

<!-- BEGIN GENERATED release-state:0.8.23:status-current-state -->**Next is Slice 0 (CUDA-CONTRACT), NOT_STARTED.** Landed on `origin/main`:  — verified reachable, not asserted.<!-- END GENERATED release-state:0.8.23:status-current-state -->

| Scope | Current contract |
| --- | --- |
| CUDA artifact | Linux x86_64 Python and npm native packages only. |
| Runtime default | CPU; CUDA is explicit through `FATHOMDB_EMBED_DEVICE=cuda:N`. |
| Publication | Held: this release completes implementation and rehearsal only. |
| GPU trust boundary | Restricted `fathomdb-gpu-release` organization runner group; only the verified release workflow ref may use it. |

## Slice ladder

| Slice | Scope | Status |
| ---: | --- | --- |
| 0 | CUDA contract, dependency probe, and protected runner gate | Not started. |
| 10 | Memex integration feedback, readiness, and lifecycle characterization | Not started; depends on Slice 0. |
| 5 | CUDA build, package, release gate, and installed-artifact smokes | Not started; depends on Slices 0 and 10. |

## Immediate next action

| | |
| --- | --- |
| **Immediate next action** | <!-- BEGIN GENERATED release-state:0.8.23:status-next-action -->**Commission Slice 0 (CUDA-CONTRACT)** — CUDA artifact contract, dependency probe, and protected runner gate. **Remaining ladder:** 0 → 10 → 5.<!-- END GENERATED release-state:0.8.23:status-next-action --> |

## Release hold

No `v0.8.23` tag or registry publication is authorized by this board. The
release-closing rehearsal will retain evidence for a later explicit publication
decision.
