---
title: Slice 85 — explicit maintenance erasure
status: COMPLETE
target_release: 0.8.28
---

# Slice 85 — explicit maintenance erasure

HITL approved the Astra-reviewed contract and assigned its implementation to a
separate clean Slice 85 on 2026-11-04. Implementation and independent qualification
are complete on `release/0.8.28` at `a2fb4a647` with format 36.

| Document | Purpose |
| --- | --- |
| [Slice plan](plan.md) | Requirements/acceptance mapping, exact design prerequisites, RED/GREEN sequence and verification |
| [Status](status.md) | Completed acceptance, assembled verification, observed timings and CI limits |
| [Approved contract](../../../../design/explicit-maintenance-erasure/contract.md) | API, mode, queue, maintenance, physical completion, blockers and format boundary |
| [Implementation outline](../../../../design/explicit-maintenance-erasure/implementation-plan.md) | Detailed design/qualification work retained from contract review |
| [Findings](../../../../notes/erasure-optimization-sequence-findings.md) | Delivered Slices 81–84b work and rejected optimization evidence |

The release ladder is 85 → 90. Further optimization stays set aside. This is a
new contract implementation, not a revival of either rejected candidate.
