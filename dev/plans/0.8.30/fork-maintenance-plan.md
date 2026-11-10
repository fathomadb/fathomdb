---
title: 0.8.30 — Candle rebase and broader fork/kernel maintenance
status: PROPOSED
target_release: 0.8.30
placement: owner-approved 2026-11-10, steward seq-367
---

# 0.8.30 — Candle rebase and broader fork/kernel maintenance

## Authority and release boundary

Steward `seq-367` records the owner's November10 ruling: put the work outlined
in B/C into .30 and focus .29 on A. **Placement in .30 is approved.** Detailed
requirements, selected upstream revision/GPU target matrix, implementation and
qualification remain to be designed and reviewed in .30. This is the receiving
work allocation, not an activated/full .30 release plan or completed work claim.

[0.8.29 Slice140](../0.8.29/features/slice-140/plan.md) now contains only required
reset/artifact repairs and versioning necessary to release those repairs.
Nothing below gates .29 close. Integrate .29's qualified repairs before final
.30 fork qualification so they survive upstream rebasing.

## Work units

| Unit / stable identity | Required planning and implementation work | Acceptance and qualification |
| --- | --- | --- |
| F30-REBASE / TC-509dc8a6 | Choose a justified upstream Candle release/revision; compare our K1–K9 deltas with upstream; retain only patches needed for current capabilities; adapt consumers directly to selected APIs | Independently reviewed delta/removal map, exact upstream/fork identity, one coherent cudarc family, current CPU/ARM/private-pool/driverless/embedding/rerank/reset behavior and installed-consumer evidence. |
| F30-MARKER / TC-93d11583 | Review removal of unused fathomdb-private-pool marker independently of the actual private-pool primitive; update fork FATHOMDB.md removal policy before removal | Feature-resolution and consumer checks prove removing the marker does not remove the pool functionality or break selected consumers. |
| F30-MULTIARCH / TC-323f15b0 | Design/build selected native SASS/cubin targets with appropriate PTX fallback; update kernel loading/selection; address architecture-specific BF16/F8 availability and single-target MoE coverage | Inspect actual artifact targets/ISA/kernels; execute independent output and unsupported-target controls on selected venues; measure cold-start/JIT, artifact-size and build-time tradeoffs. |

F30-MULTIARCH expands GPU code generation and packaging. The exact target list
is not yet selected. Known .29 baseline: x86 compute75 PTX excludes >=80-only
BF16/F8 kernels, product embed/rerank useF32, and MoE's archive contains
single-target native code without PTX fallback. New kernel availability does
not automatically select a lower-precision product mode, claim a measured
performance gain or add generic server-ARM/SBSA platform support.

## Requirements/design structure

- N30-F1: consumers need a maintained dependency base with truthful supported
  features and preserved current correctness/resource behavior.
- N30-F2: selected GPUs need correctly chosen kernels with measured delivery/
  startup costs and explicit unsupported cases.
- R30-F1/A30-F1: every retained/removed fork delta has source evidence and a
  current requirement; no obsolete-API shim is added for a pre1.0 rewrite.
- R30-F2/A30-F2: upstream/cudarc/feature/API/pin changes form a coherent reviewed
  migration. Existing private-pool, driverless, ARM safety and required .29 reset
  repairs remain qualified on selected final artifacts.
- R30-F3/A30-F3: declared native/PTX/dtype/MoE target coverage matches actual
  bytes and execution; unsupported combinations fail plainly. Native selection
  and PTX fallback are tested separately, not inferred from compilation.
- R30-F4/A30-F4: baseline/treatment cold-start, steady behavior, package size and
  build time use fixed comparable inputs; report neutral/negative results too.
- R30-F5/A30-F5: marker cleanup preserves actual pool API/features; qualification
  and pin/consumer identity follow the final selected upstream/build design.

These are work-local draft criteria, not canonical AC additions. Before code,
write fork-local requirements/design/test mappings and obtain independent review.
Prefer rebase before final multiarch implementation/qualification to reuse the
chosen upstream kernel infrastructure; marker investigation can proceed
independently. Only the final selected combination establishes product support.
No .30 slice numbers or release-state closures are invented by this allocation.

## Verification, policy and carryover

Apply the [quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md):
real output/resource controls, selected boundary/property cases and installed
consumer proof; defer general GPU fuzzing and exhaustive architecture matrices.
New evidence motivating this allocation is fork drift plus current single-target
kernel/dtype limitations; no product performance win has been demonstrated.

The full .30 release plan must run its own required
[CUDA usage/policy review](../../design/cuda-version-support-review.md).
The [.29 review](../0.8.29/research/cuda-version-adoption.md) is a dated input,
including compiler6111901 exposure/mitigation and missing quantitative shares,
not a .30 usage refresh. Keep accepted toolkit/PTX/driver/hardware policy until
owner-ratified changes; preserve windchill3 R580/primary12.6 and never runK620.
Venue qualification, cost bounds and product precision/support changes remain
explicit design/owner decisions. Do not reflash, provision or publish by inference.

Use observed failing behavior tests before fixes, retain current meaningful
oracles, and qualify each fork before promoting exact pins to FathomDB. Fork
work lives under each fork's fathomdb/; FathomDB records product consumption
proof. Scoped validators cover plans; source phases receive the required final
full gate and independent review. Preserve stable TC identities while moving
release allocation; all three items stay open until their .30 exits are met.
