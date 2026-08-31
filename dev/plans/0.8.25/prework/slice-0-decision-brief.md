---
title: 0.8.25 Slice 0 — decision brief
status: PROPOSED
target_release: 0.8.25
---

# Slice 0 decision brief

This brief supplies Slice 6 decisions. A recommendation is not HITL approval.

| ID | Decision or finding | Recommendation | Owner / destination | State |
| --- | --- | --- | --- | --- |
| P25-ENV-01 | Current Python shim imports from `main`, not the release worktree | Add/document a wheel-built, external-venv agent verification path; never use editable install through the shim | Slice 7 decision | Proposed |
| P25-ENV-02 | Rust 1.95.0 and local x86 target match repository policy | Retain; add targets only in a feature/platform slice that needs them | Existing infrastructure | Resolved/no change |
| P25-ENV-03 | Host npm 11.19.0 differs from manifest package-manager 11.12.1 | Slice 1 determines whether to align the local tool or retain with compatibility evidence | Slice 1, then Slice 7 if included | Open |
| P25-ENV-04 | Two authorized RTX 3090s are available; sandbox access fails; `nvcc` is absent | Use unconfined GPUs for evaluation and the existing CUDA 12.6 runner/image for artifacts; exclude K620 | Feature measurements / existing release infrastructure | Resolved/no change |
| P25-ENV-05 | ptrace works only outside the sandbox | Run unchanged strict gates unconfined; never weaken or skip them | Every applicable verification slice | Resolved/no change |
| P25-ENV-06 | Tegra route exists but this host cannot certify it | Preserve the existing Jetson workflow; allocate only concrete new contract gaps | Owning feature slice / Slice 75 audit | Resolved/no change |
| P25-ENV-07 | Windows CPU/native routes exist; Windows CUDA is separate and not locally proven | Require Windows CPU SDK parity; keep Windows CUDA outside 0.8.25 unless Slice 6 explicitly includes a concrete route | Owning feature slice / future release | Proposed boundary |
| P25-ENV-08 | Shared `data` and `node_modules` links can leak mutation across checkouts | Treat as read-only convenience; copy mutable inputs and use private caches for runs | Every run plan | Resolved policy |
| P25-INFRA-01 | 0.8.25 lacked its release-state file and status board | Establish both through generated-view tooling | Slice 0 | Complete |
| P25-INFRA-02 | New data-plane contracts need platform, lifecycle, concurrency, and retrieval-only proof | Keep verification local to each Slice 10+ contract and audit the combined surface in Slice 75 | Slices 10–75 | Already allocated |
| P25-INFRA-03 | Generic release-state completion rendering treats `landed` as `origin/main`; 0.8.25 completes work on a release branch | Generalize branch-completion identity and its verification without a release-number special case | Slice 7 decision | Proposed |
| P25-INFRA-04 | The relocated worktree retained Rust target artifacts compiled with the removed `/tmp/fathomdb-release-0.8.25` path; a full workspace run failed 24 targets on stale binary/fixture/script paths | Select a fresh release-bound target directory, rebuild from source, verify no executable/dep metadata retains the removed checkout path, and rerun the unchanged serial workspace suite | Slice 7 decision | Proposed |

## Slice 0 disposition

The release environment is ready for proposal work through Slice 6. Feature or
Python-runtime implementation is not ready to close until P25-ENV-01 is decided
and, if included, implemented in Slice 7. Slice 1 is the next dependency.
