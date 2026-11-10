# Slice 0 platform and resource register

Declared support is source evidence at `6edc1155f`; current remote readiness is
unmeasured. Resource numbers below are inherited November10 approximately13:35UTC
observations, not freshly sampled capacity. No qualification or venue action ran.

| Route | Evidence and remaining prerequisite |
| --- | --- |
| CPU native | [.github/workflows/ci.yml](../../../../../.github/workflows/ci.yml): Linux x64/ARM64, macOS x64/ARM64, Windows x64. Need final source-bound artifact/config and actual selected execution. |
| GPU windchill3 | [.28 status](../../../0.8.28/features/slice-95/status.md): historical runner 168/group 3. Current driver/UUID/ordinal unavailable in executor. Preserve R580/primary12.6; never K620. |
| Windows | Historical runner 31 on MEMEX-AFE8V4I00 and existing VM. Need actual current access/availability; no VM reconfiguration implied. |
| Jetson | Historical runner 22 on ubuntu. Separate classic Tegra, CUDA13/Q5 and genuine non-Tegra ARM driverless claims. No reflash permission. |
| Readiness control | [runner-readiness.yml](../../../../../.github/workflows/runner-readiness.yml) is main-only, identity-only, three-minute bounded. Historical run 42726255745 proves prior delivery, not product or .29 candidate execution. |
| MCP CUDA | Crate publish=false and embed-cuda feature exist. Build, provider runtime smoke and installed consumer support are separate evidence. |
| Browsers | Node native SDK has no implied browser product matrix; only a selected docs/tool flow warrants browser checks. |
| Build/scratch | Root/tmp 16 GiB observed; /mnt/data candidate capacity 914 GiB. Task-owned /mnt/data use is scope-approved;10 selects exact owned path/access and measured peak headroom including upload/cache post-processing. |
| Corpora/models | Locate actual data, license/access and hashes before measurement. README presence is insufficient. |

Preserve `/mnt/data/tmp2/fathomdb-s95-publish-gpu-20261109`. GPU faults require
a separately permitted isolated venue, not this inventory. CUDA usage evidence
and policy/artifact choices remain with 120. Missing venue/input evidence is
reported as unmeasured, never as a successful qualification or driver defect.

Selection and handover:

| Scope | Selecting / executing owner | Required identity and stop condition |
| --- | --- | --- |
| Stable CPU matrix | 160 selects combined Rust/Python/Node native proof using existing CI/release routes;10 prepares local tools | Exact candidate/features/target/ABI/artifact digests and supported runner; no blanket platform PASS from workflow presence. Browser cells remain inapplicable unless a concrete docs/tool flow is selected. |
| CUDA product / library cells | 120 defines selected12.6/13.0 artifact contracts;130 qualifies affected platforms;110 owns permitted reset venue | Match toolkit/PTX, driver, loaded library, GPU UUID/ordinal and chosen feature. Historical runner success, library qualification and compiled MCP CUDA binary each leave runtime/installed-consumer proof outstanding. |
| Required forks | 140 qualifies necessary110/120 repairs before pin adoption | A only under seq-367; repair-release versioning conditional. Candle rebase/marker cleanup/multiarch remain .30 and do not block .29. |
| Build resource reservation | 10 chooses approved task-owned /mnt/data path and phase budget | Existing preflight disk floor plus measured peak/stop threshold on root/tmp and output filesystems; missing budget or insufficient space stops affected builds. No implicit cleanup. |
| Measurement inputs / contention | 30 and affected100/110/120/130 owner | Corpus location/license/hash, model revision/digest, scratch identity and host/GPU/VM reservation. Missing input or conflicting venue use blocks that campaign. |

9 receives explicit dispositions for inaccessible inputs/venues and unmeasured
capacity. Such a disposition can complete proposal intake; it cannot qualify the
platform or waive a selected mandatory downstream acceptance criterion. Support
or toolkit policy changes, actual fault venues and publication retain separate
owner gates. Keep format37 and the query scaffold; no database migration belongs
in environment preparation.
