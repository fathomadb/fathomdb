---
title: 0.8.24 Slice 40 — draft contracts
status: POSTPONED-TO-0.8.26
target_release: 0.8.24
---

# Slice 40 draft contracts

> **POSTPONED TO 0.8.26 — HITL ruling `seq-258`.** These remain draft inputs
> only; none becomes a 0.8.24 canonical contract or implementation task.

These are local drafts derived from Slice 3. They are not edits to
`dev/needs.md`, `dev/requirements.md`, or `dev/acceptance.md` and must remain
drafts until P24-09 and P24-10 are decided.

| Draft | Proposed contract | Allocation / boundary |
| --- | --- | --- |
| N40-DRAFT | A supported Windows x64 CUDA user can obtain a documented prebuilt SDK artifact without compiling it locally. | Slice 40 selects and implements; CPU routes remain intact. |
| REQ-TARGET-WINDOWS-CUDA-DRAFT | Windows CUDA exists only for the owner-declared Python/npm matrix and explicitly trusted remote builder. Package selection and unsupported routes are documented and deterministic. A Python local-version route is exact-pin, selected-first-party-index only, and exclusive with CPU in one environment. | A public identity/loader change needs an ADR/interface/doc update. |
| R40-PROVENANCE-DRAFT | The builder is either owner-operated external or Actions-restricted to a dedicated selected-repository/selected-workflow runner group, never labels alone. It records immutable source SHA, toolchain/GPU facts, dependencies, artifact digests, and a reviewed transfer to a hosted publisher without build-host registry authority. | Slice 40 candidate route; Slice 60 publisher matrix. |
| AC-TARGET-WINDOWS-CUDA-DRAFT | A clean, installed selected artifact on the named Windows GPU executor completes open/write/search/close/exit and proves resolved selected UUID equals the GPU UUID holding the active computation PID; it retains UUID/model/driver/toolkit, candidate SHA, and artifact digest. | Slice 40 candidate proof; Slice 60 later registry-installed proof. |
| R40-CPU-PRESERVATION-DRAFT | Existing Windows CPU wheel and `fathomdb-native-win32-x64-msvc` package identities/install behavior remain separately valid. | Slice 40 tests selected changes; Slice 60 owns release-wide preservation. |

## Approval rule

P24-09 selects which rows become real. P24-10 selects where the GPU proof can
be produced. Both decisions are deferred to 0.8.26; these rows do not authorize a canonical
contract change, a package name, a workflow, a runner action, or publication.
