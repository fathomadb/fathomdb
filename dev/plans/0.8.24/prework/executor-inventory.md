---
title: 0.8.24 Slice 0 — executor inventory
status: COMPLETE
target_release: 0.8.24
---

# Executor inventory

**Observed:** 2026-08-23. “Unknown” is intentional: it prevents a plan from
pretending a remote executor is ready when it has not been independently
observed.

| Route | Workflow selector / local evidence | Classification | Implication |
| --- | --- | --- | --- |
| Linux x64 CUDA | `release.yml` uses `[self-hosted, Linux, X64, gpu, cuda-12]`, environment `cuda-unmerged-preflight`, and CUDA 12.6 paths. | Contracted in source; current GitHub online state unknown. | Existing x64 CUDA release route is not evidence for Windows or Tegra. |
| windchill3 runner | Local configuration names `windchill3-fathomdb` for `fathomadb/fathomdb`; logs show “Listening for Jobs” most recently on 2026-08-23 09:47 UTC. | Historical listener evidence only. Current online/busy/label state unknown. | Do not schedule based on this observation alone. |
| Hosted Windows | Existing CI/release jobs use `windows-latest`; existing packaging is CPU Windows MSVC. | Available workflow class; unsuitable as assumed CUDA executor. | It proves neither Windows CUDA toolchain nor GPU access. |
| Remote Windows CUDA | No approved runner label, host/toolkit/GPU facts, or transfer boundary was available locally. | Unknown / decision-blocked. | Slice 40 cannot start until the owner selects or supplies the route. No local Windows build is requested. |
| Jetson/Tegra | The Tegra build contract exists, but no accessible host alias or current remote runner facts were present in the local configuration. | Unknown / decision-blocked. | Slice 30 needs an observed Jetson route with L4T/JetPack, CUDA, Python, Rust, glibc, GPU capability, cache, and clean-install smoke facts. |
| Hosted ARM64 | `ubuntu-24.04-arm` builds/smokes generic ARM64 packages. | Available workflow class. | It is not interchangeable with Jetson/Tegra. |

GitHub runner and environment API queries were attempted once and returned HTTP
403 API rate limiting. They were not retried. System service/neighbor status is
not inspectable from this sandbox; no runner was started, stopped, or changed.

## Required later executor evidence

Before implementing either new target route, record:

1. GitHub selector labels and current online status;
2. host OS, architecture, Python/Rust, CUDA toolkit, compiler, driver, GPU and
   compute capability;
3. trust boundary and artifact transfer/upload path;
4. cache/network prerequisites; and
5. an exact clean installed-package smoke command for the target.

## Slice 0 disposition

No executor change is proposed. The remote Windows CUDA executor and Jetson
runner facts are explicit owner-decision inputs, not hidden prerequisites.

## Evidence

- `.github/workflows/release.yml:336-426, 865-925, 1264-1495`.
- `/home/coreyt/actions-runner/fathomdb/.runner` and its diagnostic logs.
- One GitHub Actions REST API request on 2026-08-23 (rate-limited, HTTP 403).
