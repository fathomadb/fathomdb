---
title: 0.8.24 Slice 0 — owner decision brief
status: PROPOSED
target_release: 0.8.24
---

# Slice 0 owner decision brief

This brief records findings, not decisions. It is input to the Slice 6 HITL
session; no item below is authorized merely because it has a recommendation.

| Decision | Evidence | Recommendation | If deferred |
| --- | --- | --- | --- |
| Tegra public identity | The `+tegra` wheel is deliberately local-only; PyPI has no `fathomdb-tegra` project at query time; same-name ARM64 CPU/CUDA wheels are ambiguous to pip. | Use an explicitly installed, separately named public distribution, subject to owner naming and trusted-publisher approval. | Do not publish Tegra in 0.8.24; keep local build/proof only. |
| Windows CUDA SDK surface | Python and npm already have Windows CPU surfaces; no Windows CUDA artifact contract exists. | Decide Python, npm, or both before artifact design. “Both” is consistent with today’s two SDKs but is not assumed. | Exclude Windows CUDA rather than ship a divergent, undocumented SDK surface. |
| Remote Windows CUDA executor | Hosted Windows is not CUDA evidence; no approved remote CUDA selector/toolchain/GPU route is recorded. | Name/approve a remote Windows CUDA executor and its trust/artifact route. | Windows CUDA cannot enter Slice 40. No local compile is requested. |
| New main CI follow-on | Proportional CI routing is already landed on main and covers Windows/WAL/CI categories. | Start Slice 10 with a no-change presumption; change main only for a demonstrated new target-route gap. | Existing CI remains; no release-branch recreation occurs. |
| Performance branch integration | SCALE-02 has retained result evidence and owner approval on an unmerged performance branch. | Decide whether `experiments/performance-0.8.23-plan-20260821` is a 0.8.24 input. If yes, integrate as one reviewed feature slice with targeted correctness proof, not a new benchmark run. | Defer the engine change; keep the evidence branch separate. |
| External facts availability | GitHub runner/environment/secret-name and Memex-job API queries are currently rate-limited; crates.io returned an access-policy 403. | Re-query only when the service limit clears, then attach metadata to the relevant target slice. | Treat remote executor and Memex-job conclusions as unknown, not negative. |

## Slice 0 outcome

Slice 0 is complete as a discovery slice: all findings have a durable location,
every executor is classified, and unavailable external metadata is explicit.
It made no product, infrastructure, registry, runner, CI, or dependency change.
The next scheduled discovery work is Slice 1 (Dependabot and library sweep).

## Finding index

- [Main CI interface](main-ci-interface.md)
- [Publication topology](publication-topology.md)
- [Executor inventory](executor-inventory.md)
- [CI and release controls](ci-and-release-controls.md)
- [Benchmark evidence index](benchmark-evidence-index.md)
