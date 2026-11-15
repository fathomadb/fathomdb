---
title: 0.8.31 receiving allocation — CUDA13 delivery and quantitative evidence
status: PROPOSED
target_release: 0.8.31
---

# 0.8.31 CUDA receiving allocation

Owner 2026-11-14 seq-378 moves proposed new CUDA13 delivery/compiler-safety work
from .29 Slice120 here; seq-379 accepts bounded quantitative unknowns/current
importance ratings and moves quantitative tests/evidence here. This is a receiving
allocation, not a full release plan, accepted package contract, source-ready
checkpoint or permission to build/publish. Current ratings remain unchanged;
unknown comparable shares remain UNKNOWN, not zero or inferred adoption.

| Allocation / authority | Requirements and acceptance | Sequence |
| --- | --- | --- |
| New CUDA13 delivery/compiler safety; TC-d244fe8e-4f77-4fd3-966b-a6d6308c82d7; seq-378 | Review actual release package names/versions/channel, Python variant exclusivity, Node explicit pre-load selection/no fallback, native release/CUDA identity on both variants, external Rust-root patch/pins and CLI scope. Before13 compilation, resolve6111901 against the actual generated/consumed kernel pipeline with independently reviewed safety evidence. Exact artifacts must satisfy linkage/PTX/ABI/license/SBOM rules and real installed CPU/auto/forced-GPU persisted write/read/reopen/close/exit proof. | Reread the preserved [.29 proposal](../0.8.29/features/slice-120/design.md), [requirements](../0.8.29/features/slice-120/requirements.md) R2–R12 and [test map](../0.8.29/features/slice-120/test-map.md); historical .29 coordinates are not accepted .31 coordinates. Reconcile source/pins/policy, settle concrete choices, independent design review, genuine per-behavior RED/GREEN, then exact owned builds/consumer tests and final review. No13 build before compiler-safety disposition. |
| Tegra CUDA13 qualification; existing TC-b267d940-1add-4d08-a596-a7669ae4a500; seq-381 | Qualify the actual selected Tegra CUDA13 target/stack/artifact and installed/source Rust/Python/Node consumer scope, with real forward, persisted database lifecycle, typed policy controls, actual selected device/allocator and checked allocation witness. Missing artifact/venue remains unqualified. Preserve the original [.29 A acceptance](../0.8.29/features/slice-130/acceptance.md) and [A protocol](../0.8.29/features/slice-130/campaign-protocol.md) as future .31 inputs. | The previously directed CloudJetson venue still requires actual JetPack7.2/root/data-handling vetting, device/stack/loader verification, access, written numeric quote/cap/duration/data scope and concrete owner session authorization. Reconcile actual artifact/compiler prerequisites and independent review before any run. No marketing-based support inference, live qualification or publication authority follows from allocation. |
| Quantitative tests/evidence; TC-48442a08-8feb-4776-930a-d6496d627440; seq-379 | Define comparable dated windows, denominators and alias mapping; distinguish MEASURED/REPORTED/INFERRED, exclude known confounders and retain missing shares as UNKNOWN. Test any implemented aggregation with independently specified complete/missing-window fixtures. Record what actual evidence changes importance or actions; any changed policy needs owner ratification. | Reuse the bounded November review and its exact missing-query specification. Reconcile current data availability before selecting a bounded collection; do not rerun settled decisions merely to confirm them or claim old data as fresh measurements. |

Package choices in the old proposal remain proposals. No actual CUDA13 build,
qualified compiler safety, quantitative measurements, installed candidate or
support expansion is claimed. Existing .29 CUDA12/current-artifact obligations,
130B–E and existing CUDA12.6 remain .29. Seq-381 explicitly moves Tegra CUDA13
130A here; this does not defer B–E or qualify any CUDA13 cell.
Broad fork maintenance/reset review stays [.30](../0.8.30/release-followups.md).
windchill3 stays R580 with primary CUDA12.6; no K620 test, primary symlink change,
driver change, automatic toolkit substitution or publication follows.
