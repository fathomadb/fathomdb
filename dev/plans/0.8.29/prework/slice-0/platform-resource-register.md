# Slice 0 platform and resource register

Declared support is source evidence; current remote readiness is unmeasured.

| Route | Evidence and remaining prerequisite |
| --- | --- |
| CPU native | [.github/workflows/ci.yml](../../../../../.github/workflows/ci.yml): Linux x64/ARM64, macOS x64/ARM64, Windows x64. Need final source-bound artifact/config and actual selected execution. |
| GPU windchill3 | [.28 status](../../../0.8.28/features/slice-95/status.md): historical runner 168/group 3. Current driver/UUID/ordinal unavailable in executor. Preserve R580/primary12.6; never K620. |
| Windows | Historical runner 31 on MEMEX-AFE8V4I00 and existing VM. Need actual current access/availability; no VM reconfiguration implied. |
| Jetson | Historical runner 22 on ubuntu. Separate classic Tegra, CUDA13/Q5 and genuine non-Tegra ARM driverless claims. No reflash permission. |
| Readiness control | [runner-readiness.yml](../../../../../.github/workflows/runner-readiness.yml) is main-only, identity-only, three-minute bounded. Historical run 42726255745 proves prior delivery, not product or .29 candidate execution. |
| MCP CUDA | Crate publish=false and embed-cuda feature exist. Build, provider runtime smoke and installed consumer support are separate evidence. |
| Browsers | Node native SDK has no implied browser product matrix; only a selected docs/tool flow warrants browser checks. |
| Build/scratch | Root/tmp 16 GiB observed; /mnt/data candidate capacity 914 GiB. Need approved task-owned directory and measured peak headroom including upload/cache post-processing. |
| Corpora/models | Locate actual data, license/access and hashes before measurement. README presence is insufficient. |

Preserve `/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`. GPU faults require
a separately permitted isolated venue, not this inventory. CUDA usage evidence
and policy/artifact choices remain with 120. Missing venue/input evidence is
reported as unmeasured, never as a successful qualification or driver defect.
